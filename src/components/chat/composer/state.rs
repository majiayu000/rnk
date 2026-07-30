//! Composer state: the draft, its lifecycle, and a checked generation counter.
//!
//! Submitting does not clear the draft. The composer stages a
//! [`PendingSubmission`] and hands the caller a [`SubmissionToken`]; the draft
//! is cleared only when the caller acknowledges success. If the send fails, the
//! user's text is still there — clearing on submit would destroy it at exactly
//! the moment it becomes hardest to reproduce.

use std::num::NonZeroUsize;

use crate::components::interaction::InteractionMode;
use crate::components::textarea::TextAreaState;

/// Default cap on how many rows the composer may grow to.
const DEFAULT_MAX_VISIBLE_LINES: NonZeroUsize = NonZeroUsize::new(10).expect("ten is non-zero");

/// A checked generation for every observable piece of composer state.
///
/// One state-changing action advances this exactly once, even when it changes
/// several fields. A caller holding a stale revision can tell that what it read
/// no longer describes the composer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ComposerRevision(u64);

impl ComposerRevision {
    /// Returns the raw generation.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// The next generation, or `None` on overflow.
    ///
    /// Overflow returns rather than wrapping: a wrapped counter would make a
    /// stale token compare equal to a current one.
    fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// Proof that a submission was staged, and which one.
///
/// Carries the revision it was staged at, so an acknowledgement that arrives
/// after further edits can be rejected instead of clearing newer text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubmissionToken {
    revision: ComposerRevision,
    nonce: u64,
}

impl SubmissionToken {
    /// The revision this submission was staged at.
    pub const fn revision(self) -> ComposerRevision {
        self.revision
    }
}

/// A submission awaiting the caller's verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingSubmission {
    token: SubmissionToken,
    text: String,
}

impl PendingSubmission {
    /// The token identifying this submission.
    pub const fn token(&self) -> SubmissionToken {
        self.token
    }

    /// The exact text that was submitted.
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// Why a state change was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposerError {
    /// The generation counter cannot advance any further.
    RevisionOverflow,
    /// The token does not match the submission currently in flight.
    StaleToken,
    /// There is no submission in flight.
    NoPendingSubmission,
}

/// Draft text, interaction mode, and in-flight submission.
#[derive(Debug, Clone)]
pub struct ChatComposerState {
    text: TextAreaState,
    mode: InteractionMode,
    revision: ComposerRevision,
    pending: Option<PendingSubmission>,
    next_nonce: u64,
    max_visible_lines: NonZeroUsize,
}

impl Default for ChatComposerState {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatComposerState {
    /// Creates an empty, enabled composer.
    pub fn new() -> Self {
        Self {
            text: TextAreaState::new(),
            mode: InteractionMode::Enabled,
            revision: ComposerRevision::default(),
            pending: None,
            next_nonce: 0,
            max_visible_lines: DEFAULT_MAX_VISIBLE_LINES,
        }
    }

    /// Sets how many rows the composer may grow to.
    pub fn with_max_visible_lines(mut self, lines: NonZeroUsize) -> Self {
        self.max_visible_lines = lines;
        self
    }

    /// Sets the interaction mode.
    pub fn with_mode(mut self, mode: InteractionMode) -> Self {
        self.mode = mode;
        self
    }

    /// The current draft text.
    pub fn text(&self) -> String {
        self.text.content()
    }

    /// The current interaction mode.
    pub const fn mode(&self) -> InteractionMode {
        self.mode
    }

    /// Replaces the interaction mode.
    pub fn set_mode(&mut self, mode: InteractionMode) {
        self.mode = mode;
    }

    /// The current generation.
    pub const fn revision(&self) -> ComposerRevision {
        self.revision
    }

    /// The submission awaiting acknowledgement, if any.
    pub const fn pending_submission(&self) -> Option<&PendingSubmission> {
        self.pending.as_ref()
    }

    /// Whether a submission is in flight.
    ///
    /// While one is, the draft is frozen: further edits or a second submit
    /// would change or duplicate what the caller is already sending.
    pub const fn is_submitting(&self) -> bool {
        self.pending.is_some()
    }

    /// The row cap.
    pub const fn max_visible_lines(&self) -> NonZeroUsize {
        self.max_visible_lines
    }

    /// Whether the draft would be accepted as a submission.
    ///
    /// Whitespace-only text is not: sending it is never what the user meant,
    /// and the keystroke should leave the draft alone rather than clear it.
    pub fn has_submittable_text(&self) -> bool {
        !self.text.content().trim().is_empty()
    }

    /// Whether editing is currently allowed.
    pub const fn accepts_edits(&self) -> bool {
        matches!(self.mode, InteractionMode::Enabled) && !self.is_submitting()
    }

    /// Borrows the underlying text state.
    pub const fn text_state(&self) -> &TextAreaState {
        &self.text
    }

    /// Runs `edit` and advances the generation once if it reported a change.
    ///
    /// Every mutation goes through here, so no path can change state without
    /// the counter following, and none can advance it twice.
    pub(super) fn edit(
        &mut self,
        edit: impl FnOnce(&mut TextAreaState) -> bool,
    ) -> Result<bool, ComposerError> {
        if !self.accepts_edits() {
            return Ok(false);
        }

        let before = self.text.content();
        let cursor_before = self.text.cursor();
        let reported = edit(&mut self.text);
        let changed =
            reported || before != self.text.content() || cursor_before != self.text.cursor();

        if !changed {
            return Ok(false);
        }

        self.revision = self
            .revision
            .next()
            .ok_or(ComposerError::RevisionOverflow)?;
        Ok(true)
    }

    /// Stages a submission and returns it, leaving the draft in place.
    ///
    /// Returns `None` when there is nothing worth sending or the composer is
    /// not accepting submissions; neither case touches the draft.
    pub(super) fn stage_submission(&mut self) -> Result<Option<SubmissionToken>, ComposerError> {
        if !self.accepts_edits() || !self.has_submittable_text() {
            return Ok(None);
        }

        let revision = self
            .revision
            .next()
            .ok_or(ComposerError::RevisionOverflow)?;
        let token = SubmissionToken {
            revision,
            nonce: self.next_nonce,
        };

        self.revision = revision;
        self.next_nonce = self.next_nonce.wrapping_add(1);
        self.pending = Some(PendingSubmission {
            token,
            text: self.text.content(),
        });

        Ok(Some(token))
    }

    /// Confirms the staged submission was delivered, and clears the draft.
    pub fn acknowledge_success(&mut self, token: SubmissionToken) -> Result<(), ComposerError> {
        self.take_pending(token)?;
        self.revision = self
            .revision
            .next()
            .ok_or(ComposerError::RevisionOverflow)?;
        self.text.clear();
        Ok(())
    }

    /// Reports that the staged submission failed, and keeps the draft.
    ///
    /// The text is exactly what the user typed, so they can retry or edit it.
    pub fn acknowledge_failure(&mut self, token: SubmissionToken) -> Result<(), ComposerError> {
        self.take_pending(token)?;
        self.revision = self
            .revision
            .next()
            .ok_or(ComposerError::RevisionOverflow)?;
        Ok(())
    }

    fn take_pending(&mut self, token: SubmissionToken) -> Result<PendingSubmission, ComposerError> {
        let pending = self
            .pending
            .as_ref()
            .ok_or(ComposerError::NoPendingSubmission)?;
        if pending.token != token {
            return Err(ComposerError::StaleToken);
        }
        Ok(self.pending.take().expect("checked just above"))
    }
}
