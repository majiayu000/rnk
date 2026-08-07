//! A fullscreen chat laid out by `FullscreenChatShell`.
//!
//! The point of this example is the arithmetic, so it prints the arithmetic:
//! which rows each region owns, how they change as the terminal resizes and as
//! the draft grows, and how many rows each message actually occupies.
//!
//! Everything here comes from public API. There is no hand-rolled scroll
//! offset, no `.skip(n).take(12)` paging by message count, and no private
//! layout maths — the transcript is indexed by the rows its messages really
//! take, which is the whole reason a 12-row code block and a 1-row
//! acknowledgement can sit in the same list.
//!
//! ```text
//! cargo run --example fullscreen_chat_shell
//! ```

use rnk::components::chat::message_list::{
    HorizontalInsets, MessageCompositeMeasureConfig, MessageExpansionKey, MessageListEntry,
    MessageListState, MessageMeasureOutcome, MessageMeasureRequest, MessageRows,
    MessageShellMeasureConfig, MessageVariantKey, ViewportRows,
};
use rnk::components::chat::{
    ChatComposerKeyMap, ChatComposerState, FullscreenChatShell, FullscreenFocus,
    FullscreenKeyOutcome, FullscreenLayout, MessageId, MessageRevision,
};
use rnk::core::{Style, TextWrap};
use rnk::hooks::Key;
use rnk::layout::text_flow::{
    TextFlow, TextFlowCacheIdentity, TextFlowInput, TextFlowOptions, TextFlowSourceKind,
};

const WIDTH: u16 = 60;
const STATUS_ROWS: u16 = 1;
const MEASUREMENT_CACHE: usize = 256;

/// A transcript with the message kinds that are not one row tall.
const CONVERSATION: &[(u64, &str)] = &[
    (1, "How does the fullscreen shell decide where things go?"),
    (
        2,
        "Bottom regions are paid first. The composer and the status bar get \
         their rows, and the transcript takes whatever is left, because the \
         transcript is the only one of the three that can scroll.",
    ),
    (3, "And if the terminal is too short for all of them?"),
    (
        4,
        "Then the layout is refused rather than clamped. Clamping means drawing \
         two regions over each other, which renders as one region holding \
         garbage from both, and nothing downstream can tell that happened.",
    ),
    (5, "Got it."),
];

/// Builds an entry whose measurement key is derived from its real text.
fn entry(id: u64, text: &str, width: u16) -> MessageListEntry {
    let shell = MessageShellMeasureConfig::try_new(width, HorizontalInsets::new(1, 1), vec![])
        .expect("a positive content width");
    let identity = TextFlowCacheIdentity {
        input: TextFlowInput::plain(text, TextFlowSourceKind::Exact, Style::default()),
        options: TextFlowOptions::new(usize::from(shell.content_width()), TextWrap::Wrap),
    };
    MessageListEntry::new(
        MessageId::new(id),
        MessageRevision::INITIAL,
        MessageVariantKey::new(0),
        MessageExpansionKey::new(0),
        MessageCompositeMeasureConfig::try_new(vec![identity], shell).expect("a valid config"),
    )
}

/// Measures a message by actually wrapping its text at the cached width.
///
/// Not a lookup table: the row count has to come from the same text flow the
/// renderer uses, or the layout and the paint disagree about how tall a message
/// is and the transcript scrolls to rows that are not where it thinks.
fn measure(request: MessageMeasureRequest<'_>) -> MessageMeasureOutcome<String, ()> {
    let mut rows = 0u64;
    for flow in request.key.config().text_flows() {
        match TextFlow::try_build(&flow.input, &flow.options) {
            Ok(built) => rows += built.row_count() as u64,
            Err(error) => return MessageMeasureOutcome::Failed(error.to_string()),
        }
    }
    match MessageRows::try_new(rows.max(1)) {
        Ok(rows) => MessageMeasureOutcome::Measured(rows),
        Err(error) => MessageMeasureOutcome::Failed(error.to_string()),
    }
}

fn print_layout(label: &str, layout: FullscreenLayout) {
    println!("{label}: terminal {}x{}", layout.width(), layout.height());
    for (name, region) in [
        ("transcript", layout.transcript()),
        ("composer  ", layout.composer()),
        ("status    ", layout.status()),
    ] {
        println!(
            "  {name}  rows {:>3}..{:<3} ({} row(s))",
            region.top(),
            region.bottom(),
            region.rows()
        );
    }
    assert!(
        !layout.has_overlap(),
        "regions must never share a row, at any size"
    );
}

fn main() {
    let entries: Vec<MessageListEntry> = CONVERSATION
        .iter()
        .map(|(id, text)| entry(*id, text, WIDTH))
        .collect();

    let transcript = MessageListState::try_new(
        &entries,
        WIDTH,
        ViewportRows::new(20),
        MEASUREMENT_CACHE,
        measure,
    )
    .expect("every message measures");

    let mut shell =
        FullscreenChatShell::try_new(transcript, ChatComposerState::new(), WIDTH, 24, STATUS_ROWS)
            .expect("24 rows is plenty");

    println!("Message heights, as actually wrapped at width {WIDTH}:");
    for (id, _) in CONVERSATION {
        let rows = shell
            .transcript()
            .message_rows(MessageId::new(*id))
            .expect("a known message");
        println!("  message {id}: {} row(s)", rows.get());
    }
    println!(
        "  total: {} rows across {} messages\n",
        shell.transcript().total_rows().expect("measured"),
        CONVERSATION.len()
    );

    print_layout("at startup", shell.layout());

    // A draft that wraps onto a second line takes a row from the transcript and
    // nothing else. The status bar does not move.
    let keymap = ChatComposerKeyMap::new();
    let newline = Key {
        return_key: true,
        shift: true,
        ..Key::default()
    };
    shell
        .handle_key(&keymap, "a two-line draft", &Key::default())
        .expect("the terminal is tall enough");
    shell
        .handle_key(&keymap, "", &newline)
        .expect("tall enough");
    shell
        .handle_key(&keymap, "second line", &Key::default())
        .expect("tall enough");
    println!();
    print_layout("with a two-line draft", shell.layout());

    println!();
    for height in [40u16, 8, 3] {
        match shell.try_resize(WIDTH, height) {
            Ok(()) => print_layout(&format!("resized to {height} rows"), shell.layout()),
            // Refusal is the designed answer, not a failure to handle: there is
            // no correct way to draw a two-line composer, a status bar and a
            // transcript in three rows.
            Err(error) => println!("resized to {height} rows: refused — {error}"),
        }
    }

    println!();
    shell.set_focus(FullscreenFocus::Transcript);
    let routed = shell
        .handle_key(&keymap, "x", &Key::default())
        .expect("routing does no layout work");
    println!("with the transcript focused, a key routes to: {routed:?}");

    shell.set_overlay_open(true);
    let captured = shell
        .handle_key(&keymap, "x", &Key::default())
        .expect("the overlay path does no layout work");
    println!("with an overlay open, a key routes to:        {captured:?}");
    assert_eq!(captured, FullscreenKeyOutcome::Overlay);
}
