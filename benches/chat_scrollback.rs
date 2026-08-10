//! Scrollback commit benchmarks.
//!
//! These hold two lines that a long session depends on.
//!
//! **Deduplication must not scale with session length.** The confirmed ledger is
//! what makes a repeated terminal event free, and it is consulted on every
//! commit. If that lookup degrades into a scan, the cost of *not* writing a
//! duplicate grows with how much has already been written — which is exactly
//! backwards, since long sessions are where duplicates come from.
//!
//! **Encoding must scale with the message, not the transcript.** A commit's
//! transport encoding touches its own bytes and nothing else.
//!
//! # Baseline
//!
//! Recorded on an Apple M-series laptop, `cargo bench --bench chat_scrollback`,
//! median of 100 samples. Absolute numbers are machine-specific; the *shapes*
//! are the contract.
//!
//! | Benchmark | 100 | 1,000 | 10,000 | Expected shape |
//! |---|---|---|---|---|
//! | `suppress_a_duplicate` | 59 ns | 50 ns | 50 ns | **flat** in session length |
//! | `commit_one_message` | 401 ns | — | — | constant |
//!
//! | Benchmark | 64 B | 1 KiB | 16 KiB | Expected shape |
//! |---|---|---|---|---|
//! | `derive_identity` | 384 ns | 5.6 µs | 88 µs | linear in message bytes |
//! | `encode_transport` | 75 ns | 833 ns | 12.6 µs | linear in message bytes |
//!
//! A regression that matters looks like `suppress_a_duplicate` growing with its
//! argument. That would mean the ledger lookup had become a scan, and the cost
//! of *not* writing a duplicate would grow with how much had already been
//! written — exactly backwards, since long sessions are where duplicates come
//! from.
//!
//! # A caller pattern worth avoiding
//!
//! `stream_then_commit` is quadratic on purpose: 6 µs at 16 deltas, 286 µs at
//! 128, 4.4 ms at 512. It re-derives the whole message's identity on every
//! delta, which is what a caller does if it commits on every frame. The digest
//! is over the full text, so N deltas digest O(N²) bytes.
//!
//! This is not a library regression and it is not fixable inside the sink — it
//! is a measurement of the wrong usage. Derive the identity once, when the
//! message reaches its terminal state. `InlineChatShell` is shaped to make that
//! the easy path: `stream()` takes no content at all, and only `finish()`
//! touches the digest.

use std::io::{self, Write};
use std::num::NonZeroUsize;

use rnk::components::chat::scrollback::NativeTerminalSink;
use rnk::components::chat::{
    MessageId, MessageRevision, ProjectionContext, ScrollbackCommitId, ScrollbackCommitKey,
    ScrollbackContent, ScrollbackNamespace, ScrollbackSink, ThemeIdentity,
};

fn main() {
    divan::main();
}

const WIDTH: u16 = 80;

/// A writer that counts bytes and keeps none, so the benchmark measures the
/// commit path rather than an allocator.
struct SinkWriter {
    written: usize,
}

impl Write for SinkWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.written += buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn context() -> ProjectionContext {
    ProjectionContext::new(WIDTH, ThemeIdentity::new(1)).expect("non-zero width")
}

fn namespace() -> ScrollbackNamespace {
    ScrollbackNamespace::new("bench").expect("non-empty")
}

/// Text long enough that encoding is not lost in the noise.
fn body(id: u64) -> String {
    format!("message {id}: {}", "the quick brown fox. ".repeat(8))
}

fn commit(id: u64) -> (ScrollbackCommitId, ScrollbackContent) {
    let content = ScrollbackContent::try_new(body(id), context()).expect("printable");
    let key = ScrollbackCommitKey::new(namespace(), MessageId::new(id), MessageRevision::INITIAL);
    let commit_id = ScrollbackCommitId::new(key, content.identity(), context());
    (commit_id, content)
}

fn sink(capacity: usize) -> NativeTerminalSink<SinkWriter> {
    NativeTerminalSink::with_capacity(
        SinkWriter { written: 0 },
        NonZeroUsize::new(capacity).expect("non-zero"),
    )
}

/// A session that has already committed `count` messages.
fn filled_sink(count: u64) -> NativeTerminalSink<SinkWriter> {
    let mut sink = sink((count as usize + 16).max(16));
    for id in 1..=count {
        let (commit_id, content) = commit(id);
        sink.commit(&commit_id, &content);
    }
    sink
}

/// One commit's write-and-record path, with nothing else in the ledger.
///
/// Deliberately not parameterised by session length. An earlier version of this
/// benchmark handed each iteration its own pre-filled sink, which made the
/// numbers scale linearly — but the linear part was dropping a 10,000-entry
/// `HashMap`, not committing. The scaling question is asked by
/// `suppress_a_duplicate` instead, where the sink outlives the loop.
#[divan::bench]
fn commit_one_message(bencher: divan::Bencher) {
    let (commit_id, content) = commit(1);
    bencher
        .with_inputs(|| sink(16))
        .bench_local_values(|mut sink| {
            divan::black_box(sink.commit(&commit_id, &content));
        });
}

/// A duplicate terminal event against a session of `count` messages.
///
/// This is the one that must stay flat. It performs no I/O at all — the whole
/// cost is the ledger lookup that proves the line is already on screen — and the
/// sink is built once, outside the timed loop, so no teardown is measured.
#[divan::bench(args = [100, 1_000, 10_000])]
fn suppress_a_duplicate(bencher: divan::Bencher, count: u64) {
    let mut sink = filled_sink(count);
    // The oldest commit: if the lookup ever became a scan, this is the case that
    // would show it first.
    let (commit_id, content) = commit(1);
    bencher.bench_local(|| {
        divan::black_box(sink.commit(&commit_id, &content));
    });
}

/// Building a commit identity, which digests the content.
#[divan::bench(args = [64, 1_024, 16_384])]
fn derive_identity(bencher: divan::Bencher, bytes: usize) {
    let text = "x".repeat(bytes);
    bencher.bench(|| {
        divan::black_box(ScrollbackContent::try_new(text.as_str(), context()).expect("printable"))
    });
}

/// Encoding a commit into its transport stages.
#[divan::bench(args = [64, 1_024, 16_384])]
fn encode_transport(bencher: divan::Bencher, bytes: usize) {
    let content = ScrollbackContent::try_new("x".repeat(bytes), context()).expect("printable");
    bencher.bench(|| divan::black_box(content.encode()));
}

/// A burst of deltas resolving into a single commit.
///
/// Streaming produces many identity derivations and one write, so this measures
/// what a high-frequency stream actually costs at the commit boundary.
#[divan::bench(args = [16, 128, 512])]
fn stream_then_commit(bencher: divan::Bencher, deltas: usize) {
    bencher
        .with_inputs(|| sink(64))
        .bench_local_values(|mut sink| {
            let mut text = String::new();
            for index in 0..deltas {
                text.push_str("delta ");
                // Each delta re-derives the identity of the message so far,
                // which is what a caller committing on every frame would do.
                let content =
                    ScrollbackContent::try_new(text.as_str(), context()).expect("printable");
                if index + 1 == deltas {
                    let key = ScrollbackCommitKey::new(
                        namespace(),
                        MessageId::new(1),
                        MessageRevision::INITIAL,
                    );
                    let commit_id = ScrollbackCommitId::new(key, content.identity(), context());
                    divan::black_box(sink.commit(&commit_id, &content));
                } else {
                    divan::black_box(content);
                }
            }
        });
}
