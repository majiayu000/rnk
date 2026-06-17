//! Shared contracts for interactive components.
//!
//! Interactive components use these types to describe whether input is enabled
//! and what a key/mouse handler did. Handlers should be pure over explicit
//! state where possible so controlled usage can be tested without a running TUI.

/// Whether an interactive component should accept user input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InteractionMode {
    /// Normal interactive behavior.
    #[default]
    Enabled,
    /// Ignore all user input and leave state unchanged.
    Disabled,
    /// Permit focus/navigation where documented, but do not mutate values or
    /// submit a value.
    ReadOnly,
}

impl InteractionMode {
    /// Return true when the mode accepts value-changing input.
    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled)
    }

    /// Return true when all input should be ignored.
    pub fn is_disabled(self) -> bool {
        matches!(self, Self::Disabled)
    }

    /// Return true when navigation is allowed but value changes are blocked.
    pub fn is_read_only(self) -> bool {
        matches!(self, Self::ReadOnly)
    }
}

/// Result returned by an interactive input handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionOutcome<T> {
    /// The input was not relevant or was blocked by the current mode.
    Ignored,
    /// The input was recognized but did not produce a public value.
    Handled,
    /// The component value changed.
    Changed(T),
    /// The component submitted a value.
    Submitted(T),
    /// The interaction was cancelled.
    Cancelled,
}

impl<T> InteractionOutcome<T> {
    /// Return true when the input was consumed.
    pub fn is_handled(&self) -> bool {
        !matches!(self, Self::Ignored)
    }

    /// Return true when the outcome changed state.
    pub fn is_changed(&self) -> bool {
        matches!(self, Self::Changed(_))
    }

    /// Return true when the outcome submitted a value.
    pub fn is_submitted(&self) -> bool {
        matches!(self, Self::Submitted(_))
    }

    /// Return true when the outcome cancelled the interaction.
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }

    /// Borrow the changed or submitted payload, if present.
    pub fn payload(&self) -> Option<&T> {
        match self {
            Self::Changed(value) | Self::Submitted(value) => Some(value),
            Self::Ignored | Self::Handled | Self::Cancelled => None,
        }
    }
}
