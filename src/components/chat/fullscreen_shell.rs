//! A fullscreen chat: scrolling transcript, fixed composer, fixed status bar.
//!
//! This shell owns two things and delegates everything else.
//!
//! It owns **where the regions sit**, which is [`layout`], and the ordering
//! there is the load-bearing decision: bottom regions are paid first, the
//! transcript takes the remainder, and a terminal too short for both is refused
//! rather than drawn overlapping.
//!
//! It owns **where a keystroke goes**, and that routing is total: every key
//! produces a [`FullscreenKeyOutcome`] naming the region that consumed it, so a
//! key that does nothing is distinguishable from a key that went somewhere
//! unexpected. An open overlay captures keys ahead of both the transcript and
//! the composer, because a modal that can be typed through is not modal.
//!
//! Everything else already exists and is not re-implemented here. Row heights,
//! scroll anchoring, bottom-follow and prepend stability belong to
//! [`MessageListState`]; draft editing and submission belong to
//! [`ChatComposerState`]. This shell holds them side by side and keeps their
//! viewports honest across a resize.
//!
//! # How it differs from the inline shell
//!
//! [`InlineChatShell`] commits finished messages into the terminal's own
//! scrollback and forgets them. This one never writes to scrollback at all: the
//! transcript stays in the application's viewport for the whole session, which
//! is why it can scroll back and re-flow on resize and the inline transcript
//! cannot.
//!
//! [`layout`]: self::layout
//! [`MessageListState`]: crate::components::chat::message_list::MessageListState
//! [`ChatComposerState`]: crate::components::chat::ChatComposerState
//! [`InlineChatShell`]: crate::components::chat::InlineChatShell

pub mod layout;

#[cfg(test)]
mod tests;

use std::fmt;

use crate::components::interaction::InteractionOutcome;
use crate::hooks::Key;

use super::composer::{ChatComposerKeyMap, ChatComposerState, ComposerProjection, handle_key};
use super::message_list::{MessageListState, MessageListStateError, ViewportRows};

pub use layout::{FullscreenLayout, FullscreenLayoutError, Region};

/// Which region owns the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum FullscreenFocus {
    /// Keys edit the composer draft.
    #[default]
    Composer,
    /// Keys scroll the transcript.
    Transcript,
}

/// Where a keystroke actually went.
///
/// The region is named in every variant, including the ones that changed
/// nothing. A caller debugging a dead key needs to know *which* region ate it,
/// and an outcome that only says "ignored" cannot answer that.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FullscreenKeyOutcome {
    /// An overlay was open and consumed the key before anything else saw it.
    Overlay,
    /// The composer staged a submission carrying this text.
    Submitted(String),
    /// The composer interaction was cancelled; the draft is intact.
    Cancelled,
    /// The composer draft changed, and now reads this.
    Changed(String),
    /// The focused region consumed the key without visible change.
    Consumed(FullscreenFocus),
    /// No region consumed it; the caller may route it elsewhere.
    Unconsumed(FullscreenFocus),
}

/// The typed result of a focus change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullscreenFocusOutcome {
    /// Focus moved, and now rests here.
    Moved(FullscreenFocus),
    /// Focus already rested here; nothing changed.
    Unchanged(FullscreenFocus),
    /// An overlay is open, so focus is not the shell's to move.
    ///
    /// Reported rather than silently applied: a focus change that takes effect
    /// only after the overlay closes is a change the caller did not ask for.
    HeldByOverlay(FullscreenFocus),
}

/// Every way a fullscreen shell operation can be refused.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FullscreenShellError {
    /// The terminal cannot hold the regions this shell needs.
    Layout(FullscreenLayoutError),
    /// The transcript refused the viewport change.
    Transcript(MessageListStateError),
}

impl fmt::Display for FullscreenShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Layout(error) => fmt::Display::fmt(error, f),
            Self::Transcript(error) => fmt::Display::fmt(error, f),
        }
    }
}

impl std::error::Error for FullscreenShellError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Layout(error) => Some(error),
            Self::Transcript(error) => Some(error),
        }
    }
}

impl From<FullscreenLayoutError> for FullscreenShellError {
    fn from(error: FullscreenLayoutError) -> Self {
        Self::Layout(error)
    }
}

impl From<MessageListStateError> for FullscreenShellError {
    fn from(error: MessageListStateError) -> Self {
        Self::Transcript(error)
    }
}

/// A fullscreen chat holding a transcript, a composer and a status bar.
#[derive(Debug, Clone)]
pub struct FullscreenChatShell {
    transcript: MessageListState,
    composer: ChatComposerState,
    status_rows: u16,
    layout: FullscreenLayout,
    focus: FullscreenFocus,
    overlay_open: bool,
}

impl FullscreenChatShell {
    /// Assembles a shell over an existing transcript and composer.
    ///
    /// The transcript's viewport is set from the computed layout rather than
    /// trusted as given: a transcript built against one height and drawn at
    /// another scrolls to positions that exist nowhere on screen.
    pub fn try_new(
        transcript: MessageListState,
        composer: ChatComposerState,
        width: u16,
        height: u16,
        status_rows: u16,
    ) -> Result<Self, FullscreenShellError> {
        let composer_rows = composer_rows(&composer, width);
        let layout = FullscreenLayout::try_new(width, height, composer_rows, status_rows)?;
        let mut shell = Self {
            transcript,
            composer,
            status_rows,
            layout,
            focus: FullscreenFocus::default(),
            overlay_open: false,
        };
        shell.commit_layout(layout)?;
        Ok(shell)
    }

    /// Returns the current region assignment.
    pub const fn layout(&self) -> FullscreenLayout {
        self.layout
    }

    /// Returns the transcript.
    pub const fn transcript(&self) -> &MessageListState {
        &self.transcript
    }

    /// Returns the transcript mutably, for appends, prepends and scrolling.
    pub const fn transcript_mut(&mut self) -> &mut MessageListState {
        &mut self.transcript
    }

    /// Returns the composer.
    pub const fn composer(&self) -> &ChatComposerState {
        &self.composer
    }

    /// Returns the composer mutably, for application-level acknowledgements.
    pub const fn composer_mut(&mut self) -> &mut ChatComposerState {
        &mut self.composer
    }

    /// Returns which region owns the keyboard.
    pub const fn focus(&self) -> FullscreenFocus {
        self.focus
    }

    /// Reports whether an overlay is capturing keys.
    pub const fn overlay_open(&self) -> bool {
        self.overlay_open
    }

    /// Opens or closes the overlay.
    ///
    /// Focus is left exactly where it was, so closing an overlay returns the
    /// keyboard to whichever region had it — the alternative is a modal that
    /// silently steals focus on the way out.
    pub const fn set_overlay_open(&mut self, open: bool) {
        self.overlay_open = open;
    }

    /// Moves focus, unless an overlay holds the keyboard.
    pub const fn set_focus(&mut self, focus: FullscreenFocus) -> FullscreenFocusOutcome {
        if self.overlay_open {
            return FullscreenFocusOutcome::HeldByOverlay(self.focus);
        }
        if matches!(
            (self.focus, focus),
            (FullscreenFocus::Composer, FullscreenFocus::Composer)
                | (FullscreenFocus::Transcript, FullscreenFocus::Transcript)
        ) {
            return FullscreenFocusOutcome::Unchanged(focus);
        }
        self.focus = focus;
        FullscreenFocusOutcome::Moved(focus)
    }

    /// Routes one keystroke, and re-lays out if the draft changed height.
    ///
    /// The relayout is why this is not simply "call the composer": a draft that
    /// grows past a line boundary changes how many rows the composer needs, and
    /// a transcript still sized for the old layout would draw over it.
    ///
    /// # Errors
    ///
    /// An error means the keystroke *was* applied to the draft and the layout
    /// could not follow it — the terminal is now too short for the composer the
    /// user just grew. The previous layout is still in force, so nothing is
    /// drawn overlapping, but the caller must resolve it rather than draw
    /// another frame. Reported rather than swallowed: silently keeping the old
    /// layout would clip the line being typed with no way to notice.
    pub fn handle_key(
        &mut self,
        keymap: &ChatComposerKeyMap,
        input: &str,
        key: &Key,
    ) -> Result<FullscreenKeyOutcome, FullscreenShellError> {
        if self.overlay_open {
            return Ok(FullscreenKeyOutcome::Overlay);
        }
        if self.focus == FullscreenFocus::Transcript {
            // Scrolling is the transcript's own API, deliberately: routing it
            // through here would duplicate anchor and follow-state rules that
            // already exist and must not diverge.
            return Ok(FullscreenKeyOutcome::Unconsumed(
                FullscreenFocus::Transcript,
            ));
        }

        let outcome = handle_key(&mut self.composer, keymap, input, key);
        self.reflow_for_composer()?;
        Ok(match outcome {
            InteractionOutcome::Submitted(text) => FullscreenKeyOutcome::Submitted(text),
            InteractionOutcome::Cancelled => FullscreenKeyOutcome::Cancelled,
            InteractionOutcome::Changed(text) => FullscreenKeyOutcome::Changed(text),
            InteractionOutcome::Handled => {
                FullscreenKeyOutcome::Consumed(FullscreenFocus::Composer)
            }
            InteractionOutcome::Ignored => {
                FullscreenKeyOutcome::Unconsumed(FullscreenFocus::Composer)
            }
        })
    }

    /// Applies a new terminal size.
    ///
    /// Refused as a whole when the new size cannot hold the regions: a partially
    /// applied resize leaves the layout and the transcript viewport disagreeing,
    /// and the transcript is the one that scrolls off screen.
    pub fn try_resize(&mut self, width: u16, height: u16) -> Result<(), FullscreenShellError> {
        let composer_rows = composer_rows(&self.composer, width);
        let layout = FullscreenLayout::try_new(width, height, composer_rows, self.status_rows)?;
        self.commit_layout(layout)
    }

    /// Recomputes the layout for the composer's current height.
    ///
    /// Returns the new layout, or the error that left the old one in place.
    fn reflow_for_composer(&mut self) -> Result<FullscreenLayout, FullscreenShellError> {
        let width = self.layout.width();
        let height = self.layout.height();
        let composer_rows = composer_rows(&self.composer, width);
        if composer_rows == self.layout.composer().rows() {
            return Ok(self.layout);
        }
        let layout = FullscreenLayout::try_new(width, height, composer_rows, self.status_rows)?;
        self.commit_layout(layout)?;
        Ok(layout)
    }

    /// Adopts `layout`, but only once the transcript has accepted its viewport.
    ///
    /// The order is the point. Writing the layout first and syncing after leaves
    /// the two disagreeing whenever the sync is refused — the layout says the
    /// transcript owns N rows while the transcript is still scrolling as though
    /// it owned M, which is the partially-applied resize this method exists to
    /// prevent. The transcript is asked first, and the layout is adopted only if
    /// it agreed.
    fn commit_layout(&mut self, layout: FullscreenLayout) -> Result<(), FullscreenShellError> {
        let rows = ViewportRows::new(u64::from(layout.transcript().rows()));
        self.transcript
            .try_set_viewport_rows(self.transcript.revision(), rows)?;
        self.layout = layout;
        Ok(())
    }
}

/// How many rows the composer needs for its current draft.
///
/// Clamped by the composer's own maximum, which is where the decision belongs:
/// the composer knows which of its lines may scroll out of view, and the layout
/// does not.
fn composer_rows(composer: &ChatComposerState, width: u16) -> u16 {
    let projection = ComposerProjection::build(composer, width);
    u16::try_from(projection.visible_rows()).unwrap_or(u16::MAX)
}
