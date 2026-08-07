//! The inline chat lifecycle: live region above, native scrollback below.
//!
//! An inline chat owns exactly two things on screen and nothing else. The **live
//! region** holds the message currently streaming plus the composer, and is
//! repainted freely. Everything finished has been handed to the terminal's own
//! scrollback and is no longer addressable.
//!
//! The shell's whole job is deciding when a message crosses between them, and
//! there is one rule:
//!
//! > A message leaves the live region only when a sink [confirms] its commit.
//!
//! Not when it finishes streaming, not when a write returns, not when a
//! completion event arrives — those are all things a caller can observe without
//! the terminal having accepted a byte. Removing on any of them shows the reader
//! a transcript that lost a message.
//!
//! # What repetition costs
//!
//! Streams repeat. A provider can deliver a terminal event twice, a render loop
//! can run at any frequency, and deltas arrive in bursts. None of those may
//! produce a second committed line, so:
//!
//! * deltas never commit — [`stream`] only tracks that a message is live;
//! * a finished message commits under a [`ScrollbackCommitId`], which the sink
//!   deduplicates, so repeated [`finish`] calls collapse into one write;
//! * an [`Unknown`] commit is *latched*. The shell refuses to touch that message
//!   again until a human resolves it through [`resolve`], because retrying a
//!   commit whose bytes may already be on screen is exactly how a transcript
//!   line gets duplicated.
//!
//! [confirms]: ScrollbackCommitOutcome::permits_live_removal
//! [`stream`]: InlineChatShell::stream
//! [`finish`]: InlineChatShell::finish
//! [`resolve`]: InlineChatShell::resolve
//! [`Unknown`]: ScrollbackCommitOutcome::Unknown

#[cfg(test)]
mod tests;

use std::fmt;

use crate::components::interaction::InteractionOutcome;
use crate::hooks::Key;

use super::composer::{ChatComposerKeyMap, ChatComposerState, handle_key};
use super::scrollback::{
    AttemptDisposition, NotCommittedCause, ProjectionContext, ScrollbackCommitId,
    ScrollbackCommitKey, ScrollbackCommitOutcome, ScrollbackContent, ScrollbackContentError,
    ScrollbackNamespace, ScrollbackReceipt, ScrollbackSink, UnknownEvidence,
};
use super::{MessageId, MessageRevision};

/// A message in the live region, and why it is still there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveMessage {
    id: MessageId,
    state: LiveState,
}

impl LiveMessage {
    /// Returns the message's identity.
    pub const fn id(&self) -> MessageId {
        self.id
    }

    /// Returns why the message is still in the live region.
    pub const fn state(&self) -> LiveState {
        self.state
    }
}

/// Why a message has not left the live region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LiveState {
    /// Still receiving deltas. Never committed — it has no final content yet.
    Streaming,
    /// Finished, and a commit attempt provably wrote nothing. Retryable.
    AwaitingRetry,
    /// A commit attempt ended undecidably. Latched until a human resolves it.
    ///
    /// The shell will not attempt this message again on its own. Some of its
    /// bytes may already be in the terminal, and nothing in this process can
    /// find out which.
    AwaitingResolution,
}

impl fmt::Display for LiveState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Streaming => "streaming",
            Self::AwaitingRetry => "awaiting retry after a clean refusal",
            Self::AwaitingResolution => "awaiting human resolution of an undecidable commit",
        })
    }
}

/// What one [`finish`] attempt did to the live region.
///
/// [`finish`]: InlineChatShell::finish
#[derive(Debug)]
pub enum InlineCommitReport {
    /// The commit is confirmed and the message left the live region.
    Fixed {
        /// The commit's original receipt.
        receipt: ScrollbackReceipt,
        /// Whether this attempt wrote, or found the commit already confirmed.
        disposition: AttemptDisposition,
    },
    /// Nothing was written; the message stays live and may be retried.
    Retained {
        /// Why the commit wrote nothing.
        cause: NotCommittedCause,
    },
    /// The outcome is undecidable; the message stays live and is now latched.
    Latched {
        /// What was observed before the outcome became undecidable.
        evidence: UnknownEvidence,
    },
}

impl InlineCommitReport {
    /// Reports whether the message left the live region.
    pub const fn left_live_region(&self) -> bool {
        matches!(self, Self::Fixed { .. })
    }
}

/// How a caller resolved a latched commit, after inspecting the terminal.
///
/// Both answers are assertions about something only a human can see, which is
/// why neither is inferred. Getting one wrong duplicates or loses a line, so the
/// choice is deliberately explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownResolution {
    /// The line is on screen. Drop it from the live region without rewriting.
    AlreadyVisible,
    /// The line is not on screen. Allow one more commit attempt.
    NotVisible,
}

/// Which region owns the keyboard.
///
/// The composer never leaves the live region, so focus decides who *receives*
/// keys, not what is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum InlineFocus {
    /// Keys edit the composer draft.
    #[default]
    Composer,
    /// Keys address the transcript; the composer keeps its draft untouched.
    Transcript,
}

/// The typed result of routing one keystroke through the shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InlineKeyOutcome {
    /// The composer staged a submission carrying this text.
    ///
    /// The draft is deliberately *not* cleared; see [`acknowledge_success`].
    ///
    /// [`acknowledge_success`]: ChatComposerState::acknowledge_success
    Submitted(String),
    /// The interaction was cancelled. The draft is left intact.
    Cancelled,
    /// The draft changed, and now reads this.
    Changed(String),
    /// The key was consumed without changing the draft.
    Handled,
    /// The key was not consumed, and the caller may route it elsewhere.
    Ignored,
    /// The key arrived while the transcript held focus, so the composer saw it
    /// not at all.
    NotFocused,
}

/// The typed result of a focus change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineFocusOutcome {
    /// Focus moved, and now rests here.
    Moved(InlineFocus),
    /// Focus already rested here; nothing changed.
    Unchanged(InlineFocus),
}

/// Every way an inline shell operation can be refused.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InlineShellError {
    /// The message is not in the live region, so there is nothing to finish.
    NotLive {
        /// The message that was addressed.
        id: MessageId,
    },
    /// The message is latched on an undecidable commit.
    ///
    /// Refused rather than retried: this is the whole point of latching.
    AwaitingResolution {
        /// The latched message.
        id: MessageId,
    },
    /// The message is already live, so streaming it again would double-track it.
    AlreadyLive {
        /// The message that was addressed.
        id: MessageId,
    },
    /// The finished text is not safe to commit.
    Content(ScrollbackContentError),
}

impl fmt::Display for InlineShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotLive { id } => {
                write!(f, "message {} is not in the live region", id.get())
            }
            Self::AwaitingResolution { id } => write!(
                f,
                "message {} is latched on an undecidable commit and must be resolved by a human",
                id.get()
            ),
            Self::AlreadyLive { id } => {
                write!(f, "message {} is already in the live region", id.get())
            }
            Self::Content(error) => fmt::Display::fmt(error, f),
        }
    }
}

impl std::error::Error for InlineShellError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Content(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ScrollbackContentError> for InlineShellError {
    fn from(error: ScrollbackContentError) -> Self {
        Self::Content(error)
    }
}

/// An inline chat that commits finished transcript into native scrollback.
///
/// Generic over its sink, so the same lifecycle runs against the default
/// terminal writer or against a durable store. The shell reads the sink's answer
/// and never second-guesses it.
#[derive(Debug)]
pub struct InlineChatShell<S> {
    namespace: ScrollbackNamespace,
    sink: S,
    live: Vec<LiveMessage>,
    composer: ChatComposerState,
    focus: InlineFocus,
}

impl<S: ScrollbackSink> InlineChatShell<S> {
    /// Creates a shell committing under `namespace` through `sink`.
    pub fn new(namespace: ScrollbackNamespace, sink: S) -> Self {
        Self {
            namespace,
            sink,
            live: Vec::new(),
            composer: ChatComposerState::new(),
            focus: InlineFocus::default(),
        }
    }

    /// Returns the namespace every commit is made under.
    pub const fn namespace(&self) -> &ScrollbackNamespace {
        &self.namespace
    }

    /// Returns the sink.
    pub const fn sink(&self) -> &S {
        &self.sink
    }

    /// Returns the composer, which is always in the live region.
    pub const fn composer(&self) -> &ChatComposerState {
        &self.composer
    }

    /// Returns the composer mutably.
    pub const fn composer_mut(&mut self) -> &mut ChatComposerState {
        &mut self.composer
    }

    /// Returns every message still in the live region, in arrival order.
    pub fn live_messages(&self) -> &[LiveMessage] {
        &self.live
    }

    /// Returns the live state of one message, if it is live.
    pub fn live_state(&self, id: MessageId) -> Option<LiveState> {
        self.entry(id).map(|entry| entry.state)
    }

    /// Returns which region currently owns the keyboard.
    pub const fn focus(&self) -> InlineFocus {
        self.focus
    }

    /// Moves focus, reporting whether it actually changed.
    pub const fn set_focus(&mut self, focus: InlineFocus) -> InlineFocusOutcome {
        if matches!(
            (self.focus, focus),
            (InlineFocus::Composer, InlineFocus::Composer)
                | (InlineFocus::Transcript, InlineFocus::Transcript)
        ) {
            return InlineFocusOutcome::Unchanged(focus);
        }
        self.focus = focus;
        InlineFocusOutcome::Moved(focus)
    }

    /// Routes one keystroke to the composer when it holds focus.
    pub fn handle_key(
        &mut self,
        keymap: &ChatComposerKeyMap,
        input: &str,
        key: &Key,
    ) -> InlineKeyOutcome {
        if self.focus != InlineFocus::Composer {
            return InlineKeyOutcome::NotFocused;
        }
        match handle_key(&mut self.composer, keymap, input, key) {
            InteractionOutcome::Submitted(text) => InlineKeyOutcome::Submitted(text),
            InteractionOutcome::Cancelled => InlineKeyOutcome::Cancelled,
            InteractionOutcome::Changed(text) => InlineKeyOutcome::Changed(text),
            InteractionOutcome::Handled => InlineKeyOutcome::Handled,
            InteractionOutcome::Ignored => InlineKeyOutcome::Ignored,
        }
    }

    /// Puts a message into the live region as actively streaming.
    ///
    /// Deltas need no further calls: a streaming message has no final content,
    /// so there is nothing a delta could commit. This is what makes a burst of
    /// them free.
    pub fn stream(&mut self, id: MessageId) -> Result<(), InlineShellError> {
        if self.entry(id).is_some() {
            return Err(InlineShellError::AlreadyLive { id });
        }
        self.live.push(LiveMessage {
            id,
            state: LiveState::Streaming,
        });
        Ok(())
    }

    /// Commits a finished message, and removes it from the live region if the
    /// sink confirms.
    ///
    /// Safe to call repeatedly with the same arguments, and safe to call for a
    /// message that never streamed. A duplicate terminal event is ordinary
    /// provider behaviour rather than a caller error, so it is answered rather
    /// than refused: the identity is derived from the content, so the sink
    /// recognises the repeat and reports it as already committed without
    /// writing again.
    ///
    /// The one refusal is a latched message. Nothing may touch that until a
    /// human has resolved it.
    pub fn finish(
        &mut self,
        id: MessageId,
        revision: MessageRevision,
        canonical: &str,
        context: ProjectionContext,
    ) -> Result<InlineCommitReport, InlineShellError> {
        if self.live_state(id) == Some(LiveState::AwaitingResolution) {
            return Err(InlineShellError::AwaitingResolution { id });
        }
        let content = ScrollbackContent::try_new(canonical, context)?;
        let key = ScrollbackCommitKey::new(self.namespace.clone(), id, revision);
        let commit_id = ScrollbackCommitId::new(key, content.identity(), context);

        Ok(match self.sink.commit(&commit_id, &content) {
            ScrollbackCommitOutcome::Committed {
                receipt,
                disposition,
            } => {
                self.remove(id);
                InlineCommitReport::Fixed {
                    receipt,
                    disposition,
                }
            }
            ScrollbackCommitOutcome::NotCommitted { cause } => {
                self.set_state(id, LiveState::AwaitingRetry);
                InlineCommitReport::Retained { cause }
            }
            ScrollbackCommitOutcome::Unknown { evidence } => {
                self.set_state(id, LiveState::AwaitingResolution);
                InlineCommitReport::Latched { evidence }
            }
        })
    }

    /// Resolves a latched message with what a human observed on screen.
    ///
    /// Returns the message's new live state, or [`NotLive`] if it is not latched.
    ///
    /// [`NotLive`]: InlineShellError::NotLive
    pub fn resolve(
        &mut self,
        id: MessageId,
        resolution: UnknownResolution,
    ) -> Result<Option<LiveState>, InlineShellError> {
        if self.live_state(id) != Some(LiveState::AwaitingResolution) {
            return Err(InlineShellError::NotLive { id });
        }
        Ok(match resolution {
            // The caller asserts the bytes are on screen. Dropping without
            // rewriting is the only action that does not duplicate the line.
            UnknownResolution::AlreadyVisible => {
                self.remove(id);
                None
            }
            UnknownResolution::NotVisible => {
                self.set_state(id, LiveState::AwaitingRetry);
                Some(LiveState::AwaitingRetry)
            }
        })
    }

    fn entry(&self, id: MessageId) -> Option<&LiveMessage> {
        self.live.iter().find(|entry| entry.id == id)
    }

    /// Records a live state, adding the message if it was not tracked.
    ///
    /// The insert matters: a message that completed in one shot never streamed,
    /// and an undecidable commit on it must still be latched. Updating in place
    /// only would drop exactly the states that need to survive.
    fn set_state(&mut self, id: MessageId, state: LiveState) {
        match self.live.iter_mut().find(|entry| entry.id == id) {
            Some(entry) => entry.state = state,
            None => self.live.push(LiveMessage { id, state }),
        }
    }

    fn remove(&mut self, id: MessageId) {
        self.live.retain(|entry| entry.id != id);
    }
}
