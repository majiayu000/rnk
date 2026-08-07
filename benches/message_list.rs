//! Message list benchmarks.
//!
//! These exist to hold a specific line: the cost of scrolling and of streaming
//! one message must not grow with the size of the transcript. If `visible_range`
//! or `try_update` starts scaling with message count, the index has regressed
//! into a scan and a long chat will stutter.

use rnk::components::chat::message_list::{
    HorizontalInsets, MessageCompositeMeasureConfig, MessageExpansionKey, MessageListEntry,
    MessageListState, MessageMeasureOutcome, MessageMeasureRequest, MessageRows,
    MessageShellMeasureConfig, MessageVariantKey, RowOffset, ViewportRows,
};
use rnk::components::chat::{MessageId, MessageRevision};
use rnk::core::{Style, TextWrap};
use rnk::layout::text_flow::{
    TextFlowCacheIdentity, TextFlowInput, TextFlowOptions, TextFlowSourceKind,
};

fn main() {
    divan::main();
}

const WIDTH: u16 = 80;
const VIEWPORT: u64 = 40;

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

fn measure(request: MessageMeasureRequest<'_>) -> MessageMeasureOutcome<(), ()> {
    let source = &request.key.config().text_flows()[0].input.source;
    let rows: u64 = source
        .split_once(':')
        .and_then(|(_, rows)| rows.parse().ok())
        .unwrap();
    MessageMeasureOutcome::Measured(MessageRows::try_new(rows).unwrap())
}

/// A transcript of `count` messages with heights cycling 1, 4, 12, 40.
///
/// Mixed heights matter: a list of uniform messages would let a naive
/// implementation look fast for the wrong reason.
fn transcript(count: u64) -> MessageListState {
    let heights = [1_u64, 4, 12, 40];
    let entries: Vec<MessageListEntry> = (1..=count)
        .map(|id| entry(id, heights[(id as usize) % heights.len()]))
        .collect();
    MessageListState::try_new::<(), (), _>(
        &entries,
        WIDTH,
        ViewportRows::new(VIEWPORT),
        4096,
        measure,
    )
    .unwrap()
}

#[divan::bench(args = [100, 1_000, 10_000])]
fn build(bencher: divan::Bencher, count: u64) {
    bencher.bench(|| transcript(divan::black_box(count)));
}

/// Should be flat across sizes: a lookup plus the visible slices, nothing more.
#[divan::bench(args = [100, 1_000, 10_000])]
fn visible_range_at_the_bottom(bencher: divan::Bencher, count: u64) {
    let state = transcript(count);
    bencher.bench(|| divan::black_box(&state).visible_range().unwrap());
}

/// Same, from the middle, where a scan-based implementation pays the most.
#[divan::bench(args = [100, 1_000, 10_000])]
fn visible_range_in_the_middle(bencher: divan::Bencher, count: u64) {
    let mut state = transcript(count);
    let middle = state.total_rows().unwrap() / 2;
    state
        .try_scroll_to(state.revision(), RowOffset::new(middle))
        .unwrap();
    bencher.bench(|| divan::black_box(&state).visible_range().unwrap());
}

/// One streaming edit. Should be flat: a point update, not a rebuild.
#[divan::bench(args = [100, 1_000, 10_000])]
fn stream_one_message(bencher: divan::Bencher, count: u64) {
    bencher
        .with_inputs(|| transcript(count))
        .bench_local_values(|mut state| {
            let mut rows = 1_u64;
            for _ in 0..8 {
                rows += 1;
                state
                    .try_update::<(), (), _>(state.revision(), entry(count, rows), measure)
                    .unwrap();
            }
            state
        });
}

/// Loading older history in front of the reader.
#[divan::bench(args = [100, 1_000, 10_000])]
fn prepend_history(bencher: divan::Bencher, count: u64) {
    let older: Vec<MessageListEntry> = (1..=20).map(|id| entry(1_000_000 + id, 3)).collect();
    bencher
        .with_inputs(|| transcript(count))
        .bench_local_values(|mut state| {
            state
                .try_prepend::<(), (), _>(state.revision(), &older, measure)
                .unwrap();
            state
        });
}

/// A terminal resize: every message is re-flowed at the new width.
#[divan::bench(args = [100, 1_000])]
fn resize(bencher: divan::Bencher, count: u64) {
    bencher
        .with_inputs(|| transcript(count))
        .bench_local_values(|mut state| {
            let new_width = WIDTH / 2;
            state
                .try_resize::<(), (), _, _>(
                    state.revision(),
                    new_width,
                    ViewportRows::new(VIEWPORT),
                    |request| {
                        let shell = MessageShellMeasureConfig::try_new(
                            new_width,
                            HorizontalInsets::new(0, 0),
                            vec![],
                        )
                        .unwrap();
                        let source = request.old_key.config().text_flows()[0]
                            .input
                            .source
                            .clone();
                        let identity = TextFlowCacheIdentity {
                            input: TextFlowInput::plain(
                                source,
                                TextFlowSourceKind::Exact,
                                Style::default(),
                            ),
                            options: TextFlowOptions::new(
                                usize::from(shell.content_width()),
                                TextWrap::Wrap,
                            ),
                        };
                        rnk::components::chat::message_list::MessageResizeConfigOutcome::Rebuilt(
                            MessageCompositeMeasureConfig::try_new(vec![identity], shell).unwrap(),
                        )
                    },
                    measure,
                )
                .unwrap();
            state
        });
}
