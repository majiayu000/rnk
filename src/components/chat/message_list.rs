//! Variable-height message list with row-based scrolling and anchoring.
//!
//! The fixed-height virtual scroll in `components::layout::scrollable`
//! treats the scroll offset and viewport as *item counts*. A chat message can
//! be one row or several hundred, so counting items puts the viewport nowhere
//! near where the renderer paints. This module indexes messages by the terminal
//! rows they actually occupy.
//!
//! Two rules shape the design:
//!
//! - **A position is a row inside a message, not a global offset.** Content
//!   arriving above the reader must not move what they are reading.
//! - **A height is cached under everything that can change it.** Content,
//!   width, styles, variant and expansion all key the measurement, so a message
//!   that changed is re-measured and one that did not is never re-measured.
//!
//! ```rust
//! use rnk::components::chat::message_list::{
//!     MessageListState, MessageMeasureOutcome, MessageRows, ViewportRows,
//! };
//! # use rnk::components::chat::message_list::{
//! #     HorizontalInsets, MessageCompositeMeasureConfig, MessageExpansionKey, MessageListEntry,
//! #     MessageShellMeasureConfig, MessageVariantKey,
//! # };
//! # use rnk::components::chat::{MessageId, MessageRevision};
//! # fn entry(id: u64) -> MessageListEntry {
//! #     let shell = MessageShellMeasureConfig::try_new(40, HorizontalInsets::new(0, 0), vec![])
//! #         .unwrap();
//! #     let config = MessageCompositeMeasureConfig::try_new(vec![], shell).unwrap();
//! #     MessageListEntry::new(
//! #         MessageId::new(id), MessageRevision::INITIAL,
//! #         MessageVariantKey::new(0), MessageExpansionKey::new(0), config,
//! #     )
//! # }
//! // This measure callback always succeeds, so its failure and cancellation
//! // types are `Infallible`.
//! use std::convert::Infallible;
//!
//! let entries = [entry(1), entry(2)];
//! let state = MessageListState::try_new::<Infallible, Infallible, _>(
//!     &entries,
//!     40,
//!     ViewportRows::new(10),
//!     64,
//!     |_request| MessageMeasureOutcome::Measured(MessageRows::try_new(3).unwrap()),
//! )?;
//!
//! assert_eq!(state.total_rows()?, 6);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod cache;
mod error;
mod height_index;
mod key;
mod key_snapshot;
mod state;
mod types;
mod view_state;

#[cfg(test)]
pub(crate) mod tests;

pub use error::{
    MessageCompositeMeasureError, MessageListMeasureError, MessageListRenderError,
    MessageListStateError,
};
pub use key::{MessageListEntry, MessageMeasureKey, MessageMeasureKeyHandle};
pub use state::MessageListState;
pub use types::{
    HorizontalInsets, MessageCompositeMeasureConfig, MessageExpansionKey, MessageListRevision,
    MessageRows, MessageRowsError, MessageShellMeasureConfig, MessageStructuralSegment,
    MessageStructureSlotKey, MessageVariantKey, RowOffset, ViewportRows,
};
pub use view_state::{
    BottomFollowState, MessageAnchor, MessageListMutation, MessageListObservation,
    MessageListUpdate, MessageMeasureOutcome, MessageMeasureRequest, MessageResizeConfigOutcome,
    MessageResizeConfigRequest, VisibleMessageRange, VisibleMessageSlice,
};

use crate::core::Element;
use crate::layout::text_flow::{TextFlow, TextFlowError};

/// Measures a message by flowing each textual child and adding the shell rows.
///
/// This is the contract the renderer has to match. Measuring only the message
/// body is the common mistake: role headers, status markers, spacing between
/// blocks, padding and borders all take rows, and leaving them out puts every
/// message below off by the amount that was missed.
pub fn try_measure_composite(
    request: MessageMeasureRequest<'_>,
) -> Result<MessageRows, MessageCompositeMeasureError<TextFlowError>> {
    let config = request.key.config();
    let mut total = 0_u64;

    for (child_index, identity) in config.text_flows().iter().enumerate() {
        let flow = TextFlow::try_build(&identity.input, &identity.options).map_err(|source| {
            MessageCompositeMeasureError::TextFlowFailed {
                child_index,
                source,
            }
        })?;
        let rows = u64::try_from(flow.row_count())
            .map_err(|_| MessageCompositeMeasureError::RowArithmeticOverflow)?;
        total = total
            .checked_add(rows)
            .ok_or(MessageCompositeMeasureError::RowArithmeticOverflow)?;
    }

    let structural = config
        .shell()
        .structural_rows()
        .map_err(|_| MessageCompositeMeasureError::RowArithmeticOverflow)?;
    total = total
        .checked_add(structural)
        .ok_or(MessageCompositeMeasureError::RowArithmeticOverflow)?;

    MessageRows::try_new(total).map_err(MessageCompositeMeasureError::MessageRows)
}

/// Renders the visible part of a message list.
#[derive(Debug, Clone, Copy)]
pub struct MessageList<'a> {
    state: &'a MessageListState,
}

impl<'a> MessageList<'a> {
    /// Borrows a list for rendering. Nothing here mutates or measures.
    pub const fn new(state: &'a MessageListState) -> Self {
        Self { state }
    }

    /// Builds one element per visible message, in list order.
    ///
    /// Before each call the entry and its key are checked against each other.
    /// A key that no longer describes its entry means the caller would draw new
    /// content into geometry measured for the old content — text overlapping
    /// the message below it — so that is a typed failure rather than a frame.
    pub fn try_into_element<RenderFailure, R>(
        self,
        mut render: R,
    ) -> Result<Element, MessageListRenderError<RenderFailure>>
    where
        R: FnMut(
            &MessageListEntry,
            &MessageMeasureKeyHandle,
            &VisibleMessageSlice,
        ) -> Result<Element, RenderFailure>,
    {
        let range = self.state.visible_range()?;
        let mut root = Element::box_element();
        root.style.flex_direction = crate::core::FlexDirection::Column;

        for slice in &range.slices {
            let entry = self.state.entries().get(slice.message_index).ok_or(
                MessageListStateError::UnknownMessageId {
                    message_id: slice.message_id,
                },
            )?;

            if entry.message_id() != slice.message_id {
                return Err(MessageListStateError::MeasurementIdentityMismatch {
                    message_id: slice.message_id,
                }
                .into());
            }

            let key = &slice.measure_key;
            if key.as_key().message_id() != entry.message_id()
                || key.as_key().content_revision() != entry.content_revision()
                || key.as_key().variant() != entry.variant()
                || key.as_key().expansion() != entry.expansion()
                || key.as_key().config() != entry.measure_config()
            {
                return Err(MessageListStateError::MeasurementIdentityMismatch {
                    message_id: entry.message_id(),
                }
                .into());
            }

            let child = render(entry, key, slice).map_err(|source| {
                MessageListRenderError::RenderFailed {
                    message_id: entry.message_id(),
                    key: key.clone(),
                    message_rows: slice.message_rows.clone(),
                    source,
                }
            })?;
            root.add_child(child);
        }

        Ok(root)
    }
}
