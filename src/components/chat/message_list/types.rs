//! Validated scalar and configuration types for the message list.
//!
//! Row counts, offsets and viewport heights are separate types on purpose. The
//! bug this component exists to fix was code that used a message *count* where
//! a *row* offset belonged; making them different types stops that class of
//! mistake at compile time rather than at the second page of a transcript.

use std::num::NonZeroU64;

use super::error::MessageListStateError;

/// A message height in terminal rows. Always at least one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageRows(NonZeroU64);

/// Why a row count was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MessageRowsError {
    /// A message that occupies no rows cannot be positioned or scrolled to.
    Zero,
}

impl core::fmt::Display for MessageRowsError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Zero => write!(formatter, "a message must occupy at least one row"),
        }
    }
}

impl std::error::Error for MessageRowsError {}

impl MessageRows {
    /// Builds a row count, rejecting zero.
    ///
    /// Only a value that passes through here can reach a measurement outcome,
    /// so the state machine never has to handle a zero-height message.
    pub fn try_new(raw: u64) -> Result<Self, MessageRowsError> {
        NonZeroU64::new(raw).map(Self).ok_or(MessageRowsError::Zero)
    }

    /// The number of rows.
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// A row coordinate, counted from the top of the list or of one message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RowOffset(u64);

impl RowOffset {
    /// The first row.
    pub const ZERO: Self = Self(0);

    /// Builds a row offset.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// The raw offset.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A viewport height in terminal rows. Zero means nothing is visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ViewportRows(u64);

impl ViewportRows {
    /// Builds a viewport height.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// The raw height.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// The list's own generation counter, separate from message content revisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageListRevision(NonZeroU64);

impl MessageListRevision {
    /// The revision a freshly built list publishes.
    pub const INITIAL: Self = Self(NonZeroU64::MIN);

    /// The numeric revision.
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    pub(super) fn checked_next(self) -> Result<Self, MessageListStateError> {
        self.get()
            .checked_add(1)
            .and_then(NonZeroU64::new)
            .map(Self)
            .ok_or(MessageListStateError::StateRevisionOverflow {
                revision: self.get(),
            })
    }
}

/// Which rendering variant a message is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MessageVariantKey(u64);

impl MessageVariantKey {
    /// Builds a variant key.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// The raw key.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// How far a collapsible message is expanded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MessageExpansionKey(u64);

impl MessageExpansionKey {
    /// Builds an expansion key.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// The raw key.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Identifies one structural part of a message shell, such as a role header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageStructureSlotKey(u64);

impl MessageStructureSlotKey {
    /// Builds a slot key.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// The raw key.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Rows a non-textual part of the shell occupies.
///
/// Role headers, status markers, block spacing, padding and borders all take
/// rows that the text flows know nothing about. Measuring only the body is how
/// a list ends up scrolling to coordinates the renderer never paints.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageStructuralSegment {
    slot: MessageStructureSlotKey,
    rows: RowOffset,
}

impl MessageStructuralSegment {
    /// Builds a structural segment.
    pub const fn new(slot: MessageStructureSlotKey, rows: RowOffset) -> Self {
        Self { slot, rows }
    }

    /// Which part of the shell this is.
    pub const fn slot(&self) -> MessageStructureSlotKey {
        self.slot
    }

    /// How many rows it occupies.
    pub const fn rows(&self) -> RowOffset {
        self.rows
    }
}

/// Horizontal space the shell takes from the outer width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct HorizontalInsets {
    /// Cells consumed on the left.
    pub left: u16,
    /// Cells consumed on the right.
    pub right: u16,
}

impl HorizontalInsets {
    /// Builds insets.
    pub const fn new(left: u16, right: u16) -> Self {
        Self { left, right }
    }
}

/// The non-textual frame a message is drawn in.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageShellMeasureConfig {
    outer_width: u16,
    horizontal_insets: HorizontalInsets,
    structural_segments: Vec<MessageStructuralSegment>,
}

impl MessageShellMeasureConfig {
    /// Builds a shell config.
    ///
    /// The insets must leave at least one cell for text, and each structural
    /// slot may appear once: a repeated slot would count the same header twice
    /// and shift every message below it.
    pub fn try_new(
        outer_width: u16,
        horizontal_insets: HorizontalInsets,
        structural_segments: Vec<MessageStructuralSegment>,
    ) -> Result<Self, MessageListStateError> {
        let insets = u32::from(horizontal_insets.left) + u32::from(horizontal_insets.right);
        if outer_width == 0 || insets >= u32::from(outer_width) {
            return Err(MessageListStateError::InvalidViewportWidth { width: outer_width });
        }

        let mut seen: Vec<MessageStructureSlotKey> = Vec::with_capacity(structural_segments.len());
        for segment in &structural_segments {
            if seen.contains(&segment.slot()) {
                return Err(MessageListStateError::InvalidViewportWidth { width: outer_width });
            }
            seen.push(segment.slot());
        }

        Ok(Self {
            outer_width,
            horizontal_insets,
            structural_segments,
        })
    }

    /// The width the whole message is drawn in.
    pub const fn outer_width(&self) -> u16 {
        self.outer_width
    }

    /// The cells the shell takes on each side.
    pub const fn horizontal_insets(&self) -> HorizontalInsets {
        self.horizontal_insets
    }

    /// The width left for text after the insets.
    pub const fn content_width(&self) -> u16 {
        self.outer_width - self.horizontal_insets.left - self.horizontal_insets.right
    }

    /// The ordered non-textual parts.
    pub fn structural_segments(&self) -> &[MessageStructuralSegment] {
        &self.structural_segments
    }

    /// Total rows the non-textual parts occupy.
    pub fn structural_rows(&self) -> Result<u64, MessageListStateError> {
        self.structural_segments
            .iter()
            .try_fold(0_u64, |total, segment| {
                total
                    .checked_add(segment.rows().get())
                    .ok_or(MessageListStateError::RowArithmeticOverflow)
            })
    }
}

/// Everything about a message that decides how tall it renders.
#[derive(Debug, Clone, PartialEq)]
pub struct MessageCompositeMeasureConfig {
    text_flows: Vec<crate::layout::text_flow::TextFlowCacheIdentity>,
    shell: MessageShellMeasureConfig,
}

impl MessageCompositeMeasureConfig {
    /// Builds a composite config.
    ///
    /// Every text flow must be laid out at the shell's content width. A child
    /// flowed at some other width measures rows the renderer will not paint, so
    /// the mismatch is rejected rather than guessed at.
    pub fn try_new(
        text_flows: Vec<crate::layout::text_flow::TextFlowCacheIdentity>,
        shell: MessageShellMeasureConfig,
    ) -> Result<Self, MessageListStateError> {
        let content_width = usize::from(shell.content_width());
        for identity in &text_flows {
            if identity.options.max_width != content_width {
                return Err(MessageListStateError::InvalidViewportWidth {
                    width: shell.outer_width(),
                });
            }
        }
        Ok(Self { text_flows, shell })
    }

    /// The ordered textual children, in renderer order.
    pub fn text_flows(&self) -> &[crate::layout::text_flow::TextFlowCacheIdentity] {
        &self.text_flows
    }

    /// The frame the children are drawn in.
    pub const fn shell(&self) -> &MessageShellMeasureConfig {
        &self.shell
    }
}
