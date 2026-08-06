//! Fixture builders shared by the message-list tests.

use crate::components::chat::message_list::{
    HorizontalInsets, MessageCompositeMeasureConfig, MessageExpansionKey, MessageListEntry,
    MessageListState, MessageMeasureOutcome, MessageMeasureRequest, MessageRows,
    MessageShellMeasureConfig, MessageVariantKey, ViewportRows,
};
use crate::components::chat::{MessageId, MessageRevision};
use crate::core::{Style, TextWrap};
use crate::layout::text_flow::{
    TextFlowCacheIdentity, TextFlowInput, TextFlowOptions, TextFlowSourceKind,
};

/// Default width every fixture lays out at.
pub(crate) const WIDTH: u16 = 40;

/// A text flow identity for `source` at `max_width`.
pub(crate) fn text_identity(source: &str, max_width: usize) -> TextFlowCacheIdentity {
    TextFlowCacheIdentity {
        input: TextFlowInput::plain(source, TextFlowSourceKind::Exact, Style::default()),
        options: TextFlowOptions::new(max_width, TextWrap::Wrap),
    }
}

fn config_with_source(source: &str) -> MessageCompositeMeasureConfig {
    let shell = MessageShellMeasureConfig::try_new(WIDTH, HorizontalInsets::new(0, 0), vec![])
        .expect("valid shell");
    let identity = text_identity(source, usize::from(shell.content_width()));
    MessageCompositeMeasureConfig::try_new(vec![identity], shell).expect("valid config")
}

/// An entry whose measurement key varies with `source`.
pub(crate) fn entry_with_source(id: u64, source: &str) -> MessageListEntry {
    MessageListEntry::new(
        MessageId::new(id),
        MessageRevision::INITIAL,
        MessageVariantKey::new(0),
        MessageExpansionKey::new(0),
        config_with_source(source),
    )
}

/// An entry that a table-driven measure callback will report `rows` for.
///
/// The height is encoded in the source so that changing it changes the key,
/// the way real content does.
pub(crate) fn entry_with_rows(id: u64, rows: u64) -> MessageListEntry {
    entry_with_source(id, &format!("message {id} of {rows} rows"))
}

/// An entry whose style holds NaN, used to prove key equality is reflexive.
pub(crate) fn entry_with_nan_style(id: u64) -> MessageListEntry {
    let shell = MessageShellMeasureConfig::try_new(WIDTH, HorizontalInsets::new(0, 0), vec![])
        .expect("valid shell");
    let style = Style {
        gap: f32::NAN,
        padding: crate::core::Edges::new(0.0, 0.0, 0.0, f32::NAN),
        ..Style::default()
    };

    let identity = TextFlowCacheIdentity {
        input: TextFlowInput::plain("nan", TextFlowSourceKind::Exact, style),
        options: TextFlowOptions::new(usize::from(shell.content_width()), TextWrap::Wrap),
    };
    MessageListEntry::new(
        MessageId::new(id),
        MessageRevision::INITIAL,
        MessageVariantKey::new(0),
        MessageExpansionKey::new(0),
        MessageCompositeMeasureConfig::try_new(vec![identity], shell).expect("valid config"),
    )
}

/// A measure callback that answers from an id-to-rows table.
pub(crate) fn measure_from_table(
    table: &[(u64, u64)],
) -> impl FnMut(MessageMeasureRequest<'_>) -> MessageMeasureOutcome<(), ()> + '_ {
    move |request| {
        let id = request.key.message_id();
        table
            .iter()
            .find(|(candidate, _)| MessageId::new(*candidate) == id)
            .map(|(_, rows)| {
                MessageMeasureOutcome::Measured(MessageRows::try_new(*rows).expect("non-zero"))
            })
            .unwrap_or(MessageMeasureOutcome::Missing)
    }
}

/// A list whose messages have the given heights, ids `1..=n`.
pub(crate) fn sized_state(
    heights: &[u64],
    viewport_rows: u64,
    _expected_total: u64,
) -> MessageListState {
    let entries: Vec<MessageListEntry> = heights
        .iter()
        .enumerate()
        .map(|(index, rows)| entry_with_rows(index as u64 + 1, *rows))
        .collect();
    let table: Vec<(u64, u64)> = heights
        .iter()
        .enumerate()
        .map(|(index, rows)| (index as u64 + 1, *rows))
        .collect();

    MessageListState::try_new::<(), (), _>(
        &entries,
        WIDTH,
        ViewportRows::new(viewport_rows),
        64,
        measure_from_table(&table),
    )
    .expect("fixture list builds")
}
