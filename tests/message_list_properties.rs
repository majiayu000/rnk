//! GH-65: the row index must agree with a naive scan under any edit sequence.
//!
//! The Fenwick index exists so lookups and updates stay logarithmic. That is
//! only worth anything if it gives the same answers as walking the list, so
//! these properties check it against an oracle that does exactly that, under
//! randomly generated inserts, removals and height changes.

use proptest::prelude::*;
use rnk::components::chat::message_list::{
    HorizontalInsets, MessageCompositeMeasureConfig, MessageExpansionKey, MessageListEntry,
    MessageListState, MessageMeasureOutcome, MessageMeasureRequest, MessageRows,
    MessageShellMeasureConfig, MessageVariantKey, ViewportRows,
};
use rnk::components::chat::{MessageId, MessageRevision};
use rnk::core::{Style, TextWrap};
use rnk::layout::text_flow::{
    TextFlowCacheIdentity, TextFlowInput, TextFlowOptions, TextFlowSourceKind,
};

const WIDTH: u16 = 40;

fn entry(id: u64, rows: u64) -> MessageListEntry {
    let shell =
        MessageShellMeasureConfig::try_new(WIDTH, HorizontalInsets::new(0, 0), vec![]).unwrap();
    let identity = TextFlowCacheIdentity {
        input: TextFlowInput::plain(
            format!("m{id}:{rows}"),
            TextFlowSourceKind::Exact,
            Style::default(),
        ),
        options: TextFlowOptions::new(usize::from(shell.content_width()), TextWrap::Wrap),
    };
    MessageListEntry::new(
        MessageId::new(id),
        MessageRevision::INITIAL,
        MessageVariantKey::new(0),
        MessageExpansionKey::new(0),
        MessageCompositeMeasureConfig::try_new(vec![identity], shell).unwrap(),
    )
}

/// Measures by reading the height back out of the entry's own source text, so
/// the callback stays a pure function of the key.
fn measure(request: MessageMeasureRequest<'_>) -> MessageMeasureOutcome<(), ()> {
    let source = &request.key.config().text_flows()[0].input.source;
    let rows: u64 = source
        .split_once(':')
        .and_then(|(_, rows)| rows.parse().ok())
        .expect("fixture source encodes its height");
    MessageMeasureOutcome::Measured(MessageRows::try_new(rows).expect("non-zero"))
}

/// What the index should say, computed by walking the list.
fn oracle_containing(heights: &[u64], row: u64) -> Option<usize> {
    let mut consumed = 0_u64;
    for (index, height) in heights.iter().enumerate() {
        consumed += height;
        if row < consumed {
            return Some(index);
        }
    }
    None
}

fn state_for(heights: &[u64], viewport: u64) -> MessageListState {
    let entries: Vec<MessageListEntry> = heights
        .iter()
        .enumerate()
        .map(|(index, rows)| entry(index as u64 + 1, *rows))
        .collect();
    MessageListState::try_new::<(), (), _>(
        &entries,
        WIDTH,
        ViewportRows::new(viewport),
        128,
        measure,
    )
    .expect("list builds")
}

proptest! {
    #[test]
    fn total_rows_equals_the_sum_of_message_heights(
        heights in prop::collection::vec(1_u64..40, 1..30),
    ) {
        let state = state_for(&heights, 10);
        prop_assert_eq!(state.total_rows().unwrap(), heights.iter().sum::<u64>());
    }

    #[test]
    fn visible_slices_tile_the_viewport_without_gaps_or_overlaps(
        heights in prop::collection::vec(1_u64..20, 1..25),
        viewport in 1_u64..30,
        offset in 0_u64..200,
    ) {
        let mut state = state_for(&heights, viewport);
        state
            .try_scroll_to(state.revision(), rnk::components::chat::message_list::RowOffset::new(offset))
            .unwrap();

        let range = state.visible_range().unwrap();
        let mut next_viewport_row = 0_u64;
        for slice in &range.slices {
            prop_assert_eq!(
                slice.viewport_rows.start, next_viewport_row,
                "viewport coverage has a gap or overlap"
            );
            prop_assert!(slice.message_rows.start < slice.message_rows.end);
            prop_assert_eq!(
                slice.message_rows.end - slice.message_rows.start,
                slice.viewport_rows.end - slice.viewport_rows.start,
                "a slice claimed a different number of rows on each side"
            );
            prop_assert!(
                slice.message_rows.end <= heights[slice.message_index],
                "a slice reached past the end of its message"
            );
            next_viewport_row = slice.viewport_rows.end;
        }
        prop_assert!(next_viewport_row <= viewport);
    }

    #[test]
    fn the_first_visible_message_is_the_one_a_naive_scan_finds(
        heights in prop::collection::vec(1_u64..20, 1..25),
        offset in 0_u64..150,
    ) {
        let mut state = state_for(&heights, 5);
        state
            .try_scroll_to(state.revision(), rnk::components::chat::message_list::RowOffset::new(offset))
            .unwrap();

        let range = state.visible_range().unwrap();
        let expected = oracle_containing(&heights, range.scroll_offset.get());
        let actual = range.slices.first().map(|slice| slice.message_index);
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn removing_messages_keeps_the_index_in_step_with_the_order(
        heights in prop::collection::vec(1_u64..20, 2..20),
        victims in prop::collection::vec(0_usize..20, 0..6),
    ) {
        let mut state = state_for(&heights, 6);
        let mut remaining: Vec<u64> = heights.clone();
        let mut ids: Vec<u64> = (1..=heights.len() as u64).collect();

        for victim in victims {
            if remaining.is_empty() {
                break;
            }
            let position = victim % remaining.len();
            let id = ids[position];
            state.try_remove(state.revision(), MessageId::new(id)).unwrap();
            remaining.remove(position);
            ids.remove(position);
        }

        prop_assert_eq!(state.len(), remaining.len());
        prop_assert_eq!(state.total_rows().unwrap(), remaining.iter().sum::<u64>());

        let total: u64 = remaining.iter().sum();
        for row in 0..total {
            let mut state = state.clone();
            state
                .try_scroll_to(state.revision(), rnk::components::chat::message_list::RowOffset::new(row))
                .unwrap();
            let range = state.visible_range().unwrap();
            if range.scroll_offset.get() == row {
                prop_assert_eq!(
                    range.slices.first().map(|slice| slice.message_index),
                    oracle_containing(&remaining, row)
                );
            }
        }
    }

    #[test]
    fn changing_one_height_moves_only_the_boundaries_below_it(
        heights in prop::collection::vec(1_u64..20, 2..15),
        target in 0_usize..15,
        new_height in 1_u64..30,
    ) {
        let mut state = state_for(&heights, 6);
        let position = target % heights.len();
        let id = position as u64 + 1;

        state
            .try_update::<(), (), _>(state.revision(), entry(id, new_height), measure)
            .unwrap();

        let mut expected = heights.clone();
        expected[position] = new_height;
        prop_assert_eq!(state.total_rows().unwrap(), expected.iter().sum::<u64>());
        prop_assert_eq!(
            state.message_rows(MessageId::new(id)).unwrap().get(),
            new_height
        );
    }
}
