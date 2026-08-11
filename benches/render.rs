#![allow(dead_code)] // Evidence helpers are called when this target is included by the GH68 integration gate.
//! Renderer benchmarks

use rnk::Style;
use rnk::components::chat::message_list::{
    BottomFollowState, HorizontalInsets, MessageCompositeMeasureConfig, MessageExpansionKey,
    MessageListEntry, MessageListState, MessageMeasureOutcome, MessageResizeConfigOutcome,
    MessageRows, MessageShellMeasureConfig, MessageVariantKey, RowOffset, ViewportRows,
};
use rnk::components::chat::scrollback::NativeTerminalSink;
use rnk::components::chat::{
    BlockId, ChatComposerState, ChatMessage, ChatMessageView, ChatRole, ConversationEvent,
    ConversationGuard, ConversationState, ConversationUpdate, FullscreenChatShell, InlineChatShell,
    InlineCommitReport, MessageBlock, MessageBlockEntry, MessageId, MessageMutationGuard,
    MessageRevision, MessageStatus, ProjectionContext, ScrollbackNamespace, ThemeIdentity,
    UpdateId,
};
use rnk::core::{Color, Dimension, Element, FlexDirection};
use rnk::renderer::{ClipRegion, Output, render_to_string};
use std::num::NonZeroUsize;

fn main() {
    divan::main();
}

#[divan::bench(args = [(80, 24), (120, 40), (200, 50), (300, 100)])]
fn output_buffer_creation(size: (u16, u16)) {
    let _output = Output::new(size.0, size.1);
}

#[divan::bench]
fn output_write_ascii() {
    let mut output = Output::new(80, 24);
    let style = Style::default();

    for y in 0..24 {
        output.write(0, y, "Hello, World! This is a test line.", &style);
    }
}

#[divan::bench]
fn output_write_styled() {
    let mut output = Output::new(80, 24);
    let style = Style {
        color: Some(Color::Green),
        bold: true,
        ..Style::default()
    };

    for y in 0..24 {
        output.write(0, y, "Styled text with colors and bold", &style);
    }
}

#[divan::bench]
fn output_write_cjk() {
    let mut output = Output::new(80, 24);
    let style = Style::default();

    for y in 0..24 {
        output.write(0, y, "你好世界！这是一段中文测试文本。", &style);
    }
}

#[divan::bench]
fn output_write_mixed() {
    let mut output = Output::new(80, 24);
    let style = Style::default();

    for y in 0..24 {
        output.write(0, y, "Hello 你好 World 世界 Mixed 混合", &style);
    }
}

#[divan::bench]
fn output_fill_rect() {
    let mut output = Output::new(80, 24);
    let style = Style::default();

    output.fill_rect(10, 5, 60, 14, '█', &style);
}

#[divan::bench(args = [(80, 24), (120, 40), (200, 50)])]
fn output_render_to_ansi(size: (u16, u16)) {
    let mut output = Output::new(size.0, size.1);
    let style = Style::default();

    for y in 0..size.1 {
        output.write(0, y, "Test content for rendering benchmark", &style);
    }

    divan::black_box(output.render());
}

#[divan::bench]
fn output_render_styled_ansi() {
    let mut output = Output::new(80, 24);

    let colors = [
        Color::Red,
        Color::Green,
        Color::Blue,
        Color::Yellow,
        Color::Cyan,
        Color::Magenta,
    ];

    for y in 0..24 {
        let style = Style {
            color: Some(colors[y as usize % colors.len()]),
            bold: y % 2 == 0,
            italic: y % 3 == 0,
            ..Style::default()
        };

        output.write(0, y, "Colorful styled text for benchmark", &style);
    }

    divan::black_box(output.render());
}

#[divan::bench]
fn render_simple_element() {
    let element = Element::text("Hello, World!");
    divan::black_box(render_to_string(&element, 80));
}

#[divan::bench]
fn render_nested_boxes() {
    let mut root = Element::root();

    let mut outer = Element::box_element();
    outer.style.padding = rnk::core::Edges::new(1.0, 2.0, 0.0, 2.0);

    let mut inner = Element::box_element();
    inner.add_child(Element::text("Nested content"));

    outer.add_child(inner);
    root.add_child(outer);

    divan::black_box(render_to_string(&root, 80));
}

#[divan::bench(args = [10, 50, 100])]
fn render_many_text_elements(count: usize) {
    let mut root = Element::root();
    root.style.flex_direction = FlexDirection::Column;

    for i in 0..count {
        root.add_child(Element::text(format!("Line number {}", i)));
    }

    divan::black_box(render_to_string(&root, 80));
}

#[divan::bench]
fn render_styled_text() {
    let mut root = Element::root();

    let mut text = Element::text("Bold and colorful text");
    text.style.bold = true;
    text.style.color = Some(Color::Cyan);

    root.add_child(text);

    divan::black_box(render_to_string(&root, 80));
}

#[divan::bench]
fn output_clip_region() {
    let mut output = Output::new(80, 24);
    let style = Style::default();

    output.clip(ClipRegion {
        x1: 10,
        y1: 5,
        x2: 70,
        y2: 19,
    });

    for y in 0..24 {
        output.write(0, y, "This text should be clipped to the region", &style);
    }

    output.unclip();

    divan::black_box(output.render());
}

#[divan::bench]
fn output_overwrite_wide_chars() {
    let mut output = Output::new(80, 24);
    let style = Style::default();

    // Write wide characters
    output.write(0, 0, "你好世界你好世界你好世界", &style);

    // Overwrite with ASCII
    output.write(2, 0, "AAAA", &style);

    divan::black_box(output.render());
}

#[divan::bench]
fn render_cjk_content() {
    let mut root = Element::root();
    root.style.flex_direction = FlexDirection::Column;

    for _ in 0..10 {
        root.add_child(Element::text("这是一段中文测试文本，用于测试渲染性能。"));
    }

    divan::black_box(render_to_string(&root, 80));
}

#[divan::bench]
fn render_with_colors() {
    let mut root = Element::root();
    root.style.flex_direction = FlexDirection::Column;

    let colors = [Color::Red, Color::Green, Color::Blue, Color::Yellow];

    for (i, color) in colors.iter().enumerate() {
        let mut text = Element::text(format!("Colored line {}", i));
        text.style.color = Some(*color);
        root.add_child(text);
    }

    divan::black_box(render_to_string(&root, 80));
}

#[divan::bench(args = [(40, 12), (80, 24), (120, 40)])]
fn render_full_screen(size: (u16, u16)) {
    let mut root = Element::root();
    root.style.width = Dimension::Points(size.0 as f32);
    root.style.height = Dimension::Points(size.1 as f32);
    root.style.flex_direction = FlexDirection::Column;

    for i in 0..size.1 {
        root.add_child(Element::text(format!(
            "Line {:03}: {}",
            i,
            "x".repeat(size.0 as usize - 10)
        )));
    }

    divan::black_box(render_to_string(&root, size.0));
}

const GH68_WORKLOAD_NAMES: [&str; 5] = [
    "gh68_long_conversation",
    "gh68_high_frequency_streaming",
    "gh68_variable_height_prepend",
    "gh68_continuous_resize",
    "gh68_inline_commit_churn",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Gh68Oracle {
    pub(crate) name: &'static str,
    pub(crate) message_order: Vec<u64>,
    pub(crate) anchor: Option<(u64, u64)>,
    pub(crate) bottom_follow: bool,
    pub(crate) commit_count: usize,
    pub(crate) character_count: usize,
    pub(crate) active_stream_steps: usize,
    pub(crate) expansion_transitions: usize,
    pub(crate) semantic_checksum: u64,
}

fn gh68_text_message(id: u64, role: ChatRole, text: String) -> ChatMessage {
    ChatMessage::new(
        MessageId::new(id),
        role,
        vec![MessageBlockEntry::new(
            BlockId::new(id),
            MessageBlock::Text(text),
        )],
    )
    .expect("GH68 fixture message is valid")
}

fn gh68_apply(state: &mut ConversationState, id: String, update: ConversationUpdate) {
    state
        .apply_event(ConversationEvent::new(
            UpdateId::new(id).expect("GH68 event identity is non-empty"),
            state.expected_sequence(),
            update,
        ))
        .expect("GH68 fixture update is valid");
}

fn gh68_conversation(count: usize) -> ConversationState {
    let mut state = ConversationState::new(0, NonZeroUsize::new(4096).unwrap());
    for index in 0..count {
        let id = u64::try_from(index + 1).unwrap();
        let text = format!(
            "message {index:04}: {}",
            "chat workload ".repeat(index % 7 + 1)
        );
        let push = ConversationUpdate::push(
            ConversationGuard::new(state.revision()),
            gh68_text_message(
                id,
                if index % 2 == 0 {
                    ChatRole::User
                } else {
                    ChatRole::Assistant
                },
                text,
            ),
        );
        gh68_apply(&mut state, format!("push-{index}"), push);
        let complete = ConversationUpdate::complete(MessageMutationGuard::new(
            ConversationGuard::new(state.revision()),
            MessageId::new(id),
            MessageRevision::INITIAL,
        ));
        gh68_apply(&mut state, format!("complete-{index}"), complete);
    }
    state
}

fn gh68_checksum(bytes: impl IntoIterator<Item = u8>) -> u64 {
    bytes.into_iter().fold(0xcbf29ce484222325, |hash, byte| {
        hash.wrapping_mul(0x100000001b3) ^ u64::from(byte)
    })
}

fn gh68_conversation_oracle(
    name: &'static str,
    state: &ConversationState,
    widths: &[u16],
) -> Gh68Oracle {
    let mut rendered = Vec::new();
    let mut characters = 0;
    for width in widths {
        let mut root = Element::root();
        root.style.flex_direction = FlexDirection::Column;
        for message in state.messages() {
            for block in message.blocks() {
                if let MessageBlock::Text(text) = block.block() {
                    characters += text.chars().count();
                }
            }
            root.add_child(ChatMessageView::new(message).into_element());
        }
        rendered.extend_from_slice(render_to_string(&root, *width).as_bytes());
    }
    Gh68Oracle {
        name,
        message_order: state
            .messages()
            .iter()
            .map(|message| message.id().get())
            .collect(),
        anchor: None,
        bottom_follow: true,
        commit_count: 0,
        character_count: characters,
        active_stream_steps: 0,
        expansion_transitions: 0,
        semantic_checksum: gh68_checksum(rendered),
    }
}

pub(crate) fn run_gh68_long_conversation() -> Gh68Oracle {
    gh68_conversation_oracle(GH68_WORKLOAD_NAMES[0], &gh68_conversation(128), &[80])
}

pub(crate) fn run_gh68_high_frequency_streaming() -> Gh68Oracle {
    let mut state = gh68_conversation(1);
    let push = ConversationUpdate::push(
        ConversationGuard::new(state.revision()),
        gh68_text_message(2, ChatRole::Assistant, String::new()),
    );
    gh68_apply(&mut state, "stream-push".to_owned(), push);
    let measure = |request: rnk::components::chat::message_list::MessageMeasureRequest<'_>| {
        MessageMeasureOutcome::<(), ()>::Measured(
            MessageRows::try_new(request.key.content_revision().get() / 32 + 1).unwrap(),
        )
    };
    let entries = state
        .messages()
        .iter()
        .map(|message| gh68_list_entry_with(message.id().get(), 80, message.revision(), 0))
        .collect::<Vec<_>>();
    let mut transcript =
        MessageListState::try_new(&entries, 80, ViewportRows::new(4), 512, measure).unwrap();
    let mut active_rendered = Vec::new();
    for index in 0..256 {
        let message = state.message(MessageId::new(2)).unwrap();
        let append = ConversationUpdate::append_text(
            MessageMutationGuard::new(
                ConversationGuard::new(state.revision()),
                MessageId::new(2),
                message.revision(),
            ),
            BlockId::new(2),
            if index % 9 == 0 { "界" } else { "x" },
        )
        .unwrap();
        gh68_apply(&mut state, format!("delta-{index}"), append);
        let message = state.message(MessageId::new(2)).unwrap();
        assert!(matches!(message.status(), MessageStatus::Streaming));
        transcript
            .try_update(
                transcript.revision(),
                gh68_list_entry_with(2, 80, message.revision(), 0),
                measure,
            )
            .unwrap();
        assert!(matches!(
            transcript.follow_state(),
            BottomFollowState::Following
        ));
        if index % 16 == 0 {
            let mut root = Element::root();
            root.style.flex_direction = FlexDirection::Column;
            for active in state.messages() {
                root.add_child(ChatMessageView::new(active).into_element());
            }
            active_rendered.extend_from_slice(render_to_string(&root, 80).as_bytes());
        }
    }
    let active_anchor = transcript
        .stored_anchor()
        .map(|value| (value.message_id().get(), value.intra_message_row().get()));
    let active_follow = matches!(transcript.follow_state(), BottomFollowState::Following);
    let message = state.message(MessageId::new(2)).unwrap();
    let complete = ConversationUpdate::complete(MessageMutationGuard::new(
        ConversationGuard::new(state.revision()),
        MessageId::new(2),
        message.revision(),
    ));
    gh68_apply(&mut state, "stream-complete".to_owned(), complete);
    let message = state.message(MessageId::new(2)).unwrap();
    assert!(matches!(message.status(), MessageStatus::Complete));
    transcript
        .try_update(
            transcript.revision(),
            gh68_list_entry_with(2, 80, message.revision(), 0),
            measure,
        )
        .unwrap();
    let completed = gh68_conversation_oracle(GH68_WORKLOAD_NAMES[1], &state, &[80]);
    active_rendered.extend_from_slice(&completed.semantic_checksum.to_le_bytes());
    Gh68Oracle {
        anchor: active_anchor,
        bottom_follow: active_follow,
        active_stream_steps: 256,
        semantic_checksum: gh68_checksum(active_rendered),
        ..completed
    }
}

fn gh68_list_entry_with(
    id: u64,
    width: u16,
    revision: MessageRevision,
    expansion: u64,
) -> MessageListEntry {
    let shell =
        MessageShellMeasureConfig::try_new(width, HorizontalInsets::new(0, 0), vec![]).unwrap();
    MessageListEntry::new(
        MessageId::new(id),
        revision,
        MessageVariantKey::new(0),
        MessageExpansionKey::new(expansion),
        MessageCompositeMeasureConfig::try_new(vec![], shell).unwrap(),
    )
}

fn gh68_list_entry(id: u64, width: u16) -> MessageListEntry {
    gh68_list_entry_with(id, width, MessageRevision::INITIAL, 0)
}

pub(crate) fn run_gh68_variable_height_prepend() -> Gh68Oracle {
    let current = (100..132)
        .map(|id| gh68_list_entry(id, 40))
        .collect::<Vec<_>>();
    let measure = |request: rnk::components::chat::message_list::MessageMeasureRequest<'_>| {
        MessageMeasureOutcome::<(), ()>::Measured(
            MessageRows::try_new(
                request.entry.message_id().get() % 11 + 1 + request.key.expansion().get() * 3,
            )
            .unwrap(),
        )
    };
    let mut state =
        MessageListState::try_new(&current, 40, ViewportRows::new(12), 16, measure).unwrap();
    state
        .try_scroll_to(state.revision(), RowOffset::new(20))
        .unwrap();
    let anchor = state.stored_anchor().unwrap();
    let older = (1..17)
        .map(|id| gh68_list_entry(id, 40))
        .collect::<Vec<_>>();
    state
        .try_prepend(state.revision(), &older, measure)
        .unwrap();
    assert_eq!(state.stored_anchor(), Some(anchor));
    let collapsed_rows = state.total_rows().unwrap();
    state
        .try_update(
            state.revision(),
            gh68_list_entry_with(100, 40, MessageRevision::INITIAL, 1),
            measure,
        )
        .unwrap();
    let expanded_rows = state.total_rows().unwrap();
    assert!(expanded_rows > collapsed_rows);
    assert_eq!(state.stored_anchor(), Some(anchor));
    assert!(matches!(
        state.follow_state(),
        BottomFollowState::Paused { .. }
    ));
    state
        .try_update(
            state.revision(),
            gh68_list_entry_with(100, 40, MessageRevision::INITIAL, 0),
            measure,
        )
        .unwrap();
    assert_eq!(state.total_rows().unwrap(), collapsed_rows);
    assert_eq!(state.stored_anchor(), Some(anchor));
    assert!(matches!(
        state.follow_state(),
        BottomFollowState::Paused { .. }
    ));
    let range = state.visible_range().unwrap();
    let transition_rows = [collapsed_rows, expanded_rows, range.total_rows];
    Gh68Oracle {
        name: GH68_WORKLOAD_NAMES[2],
        message_order: older
            .iter()
            .chain(current.iter())
            .map(|entry| entry.message_id().get())
            .collect(),
        anchor: state
            .stored_anchor()
            .map(|value| (value.message_id().get(), value.intra_message_row().get())),
        bottom_follow: matches!(state.follow_state(), BottomFollowState::Following),
        commit_count: 0,
        character_count: range.total_rows as usize,
        active_stream_steps: 0,
        expansion_transitions: 2,
        semantic_checksum: gh68_checksum(
            transition_rows
                .into_iter()
                .flat_map(u64::to_le_bytes)
                .chain(
                    range
                        .slices
                        .iter()
                        .flat_map(|slice| slice.message_id.get().to_le_bytes()),
                ),
        ),
    }
}

pub(crate) fn run_gh68_continuous_resize() -> Gh68Oracle {
    let entries = (1..=48)
        .map(|id| gh68_list_entry(id, 32))
        .collect::<Vec<_>>();
    let measure = |request: rnk::components::chat::message_list::MessageMeasureRequest<'_>| {
        MessageMeasureOutcome::<(), ()>::Measured(
            MessageRows::try_new(request.entry.message_id().get() % 7 + 1).unwrap(),
        )
    };
    let transcript =
        MessageListState::try_new(&entries, 32, ViewportRows::new(9), 64, measure).unwrap();
    let mut shell =
        FullscreenChatShell::try_new(transcript, ChatComposerState::new(), 32, 12, 1).unwrap();
    assert!(matches!(
        shell.transcript().follow_state(),
        BottomFollowState::Following
    ));
    let revision = shell.transcript().revision();
    shell
        .transcript_mut()
        .try_scroll_to(revision, RowOffset::new(20))
        .unwrap();
    let anchor = shell.transcript().stored_anchor().unwrap();
    let sizes = [(80, 24), (41, 16), (120, 40), (24, 12), (96, 28)];
    let mut observed = Vec::new();
    for (width, height) in sizes {
        shell.try_resize(width, height).unwrap();
        let viewport = ViewportRows::new(u64::from(shell.layout().transcript().rows()));
        let revision = shell.transcript().revision();
        shell
            .transcript_mut()
            .try_resize(
                revision,
                width,
                viewport,
                |request| {
                    MessageResizeConfigOutcome::<(), ()>::Rebuilt(
                        gh68_list_entry(request.old_entry.message_id().get(), width)
                            .measure_config()
                            .clone(),
                    )
                },
                measure,
            )
            .unwrap();
        assert_eq!(shell.transcript().stored_anchor(), Some(anchor));
        assert!(matches!(
            shell.transcript().follow_state(),
            BottomFollowState::Paused { .. }
        ));
        observed.extend_from_slice(&width.to_le_bytes());
        observed.extend_from_slice(&height.to_le_bytes());
        observed.extend_from_slice(&shell.transcript().scroll_offset().get().to_le_bytes());
    }
    let revision = shell.transcript().revision();
    shell.transcript_mut().jump_to_bottom(revision).unwrap();
    assert!(matches!(
        shell.transcript().follow_state(),
        BottomFollowState::Following
    ));
    Gh68Oracle {
        name: GH68_WORKLOAD_NAMES[3],
        message_order: entries
            .iter()
            .map(|entry| entry.message_id().get())
            .collect(),
        anchor: Some((anchor.message_id().get(), anchor.intra_message_row().get())),
        bottom_follow: true,
        commit_count: 0,
        character_count: observed.len(),
        active_stream_steps: 0,
        expansion_transitions: 0,
        semantic_checksum: gh68_checksum(observed),
    }
}

pub(crate) fn run_gh68_inline_commit_churn() -> Gh68Oracle {
    let mut shell = InlineChatShell::new(
        ScrollbackNamespace::new("gh68.benchmark").unwrap(),
        NativeTerminalSink::new(Vec::<u8>::new()),
    );
    let mut character_count = 0;
    for id in 1..=64_u64 {
        let content = format!("line {id}");
        character_count += content.chars().count();
        shell.stream(MessageId::new(id)).unwrap();
        let report = shell
            .finish(
                MessageId::new(id),
                MessageRevision::INITIAL,
                &content,
                ProjectionContext::new(80, ThemeIdentity::new(1)).unwrap(),
            )
            .unwrap();
        assert!(matches!(report, InlineCommitReport::Fixed { .. }));
    }
    let count = shell.sink().ledger().len();
    Gh68Oracle {
        name: GH68_WORKLOAD_NAMES[4],
        message_order: (1..=64).collect(),
        anchor: None,
        bottom_follow: true,
        commit_count: count,
        character_count,
        active_stream_steps: 0,
        expansion_transitions: 0,
        semantic_checksum: gh68_checksum((1..=64).flat_map(u64::to_le_bytes)),
    }
}

#[allow(dead_code)] // Shared by the integration correctness gate when this file is included as a module.
pub(crate) fn gh68_workload_oracles() -> Vec<Gh68Oracle> {
    vec![
        run_gh68_long_conversation(),
        run_gh68_high_frequency_streaming(),
        run_gh68_variable_height_prepend(),
        run_gh68_continuous_resize(),
        run_gh68_inline_commit_churn(),
    ]
}

#[divan::bench]
fn gh68_long_conversation() {
    divan::black_box(run_gh68_long_conversation());
}
#[divan::bench]
fn gh68_high_frequency_streaming() {
    divan::black_box(run_gh68_high_frequency_streaming());
}
#[divan::bench]
fn gh68_variable_height_prepend() {
    divan::black_box(run_gh68_variable_height_prepend());
}
#[divan::bench]
fn gh68_continuous_resize() {
    divan::black_box(run_gh68_continuous_resize());
}
#[divan::bench]
fn gh68_inline_commit_churn() {
    divan::black_box(run_gh68_inline_commit_churn());
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Gh68EvidenceError(pub(crate) &'static str);

pub(crate) fn gh68_benchmark_route_contract(
    phase: &str,
) -> Result<&'static str, Gh68EvidenceError> {
    let route = std::env::var("GH68_BENCHMARK_ROUTE").ok();
    let mode = std::env::var("GH68_BENCHMARK_MODE").ok();
    let carries_validation_inputs = mode.is_some()
        || std::env::var_os("GH68_BENCHMARK_BASELINE").is_some()
        || std::env::var_os("GH68_BENCHMARK_EVIDENCE").is_some();
    gh68_benchmark_route_values(
        route.as_deref(),
        phase,
        mode.as_deref(),
        carries_validation_inputs,
    )
}

fn gh68_benchmark_route_values(
    route: Option<&str>,
    phase: &str,
    mode: Option<&str>,
    carries_validation_inputs: bool,
) -> Result<&'static str, Gh68EvidenceError> {
    match route {
        Some("smoke_blocked_no_baseline") => {
            if carries_validation_inputs {
                return Err(Gh68EvidenceError("smoke route carries validation inputs"));
            }
            Ok("performance_status=not_available")
        }
        Some("performance_validation") => match (phase, mode) {
            ("metadata", Some("produce")) | ("comparison", Some("validate")) => {
                Ok("performance_status=validation_required")
            }
            _ => Err(Gh68EvidenceError("benchmark phase and mode disagree")),
        },
        Some(_) => Err(Gh68EvidenceError("unknown benchmark route")),
        None => Err(Gh68EvidenceError("missing benchmark route")),
    }
}

pub(crate) fn gh68_is_regression(candidate: u64, baseline: u64, mad: u64) -> bool {
    candidate > baseline.saturating_mul(12) / 10
        && candidate.saturating_sub(baseline) > mad.saturating_mul(3).max(1_000_000)
}

fn gh68_run(name: &str) -> Gh68Oracle {
    match name {
        "gh68_long_conversation" => run_gh68_long_conversation(),
        "gh68_high_frequency_streaming" => run_gh68_high_frequency_streaming(),
        "gh68_variable_height_prepend" => run_gh68_variable_height_prepend(),
        "gh68_continuous_resize" => run_gh68_continuous_resize(),
        "gh68_inline_commit_churn" => run_gh68_inline_commit_churn(),
        _ => panic!("closed GH68 workload name"),
    }
}

fn gh68_median(samples: &[u64]) -> Result<u64, Gh68EvidenceError> {
    if samples.len() < 15 || samples.contains(&0) {
        return Err(Gh68EvidenceError("invalid samples"));
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    Ok(sorted[sorted.len() / 2])
}

fn gh68_mad(samples: &[u64], median: u64) -> u64 {
    let mut deviations = samples
        .iter()
        .map(|sample| sample.abs_diff(median))
        .collect::<Vec<_>>();
    deviations.sort_unstable();
    deviations[deviations.len() / 2]
}

fn gh68_workload_json(name: &'static str) -> serde_json::Value {
    for _ in 0..3 {
        divan::black_box(gh68_run(name));
    }
    let samples = (0..15)
        .map(|_| {
            let started = std::time::Instant::now();
            divan::black_box(gh68_run(name));
            u64::try_from(started.elapsed().as_nanos())
                .unwrap_or(u64::MAX)
                .max(1)
        })
        .collect::<Vec<_>>();
    let median = gh68_median(&samples).unwrap();
    serde_json::json!({"name":name,"warmup_samples":3,"measured_samples_ns":samples,
        "median_ns":median,"mad_ns":gh68_mad(&samples, median),"unit":"ns"})
}

fn gh68_hex(value: &str, len: usize) -> bool {
    value.len() == len && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn gh68_validate_baseline_head(
    baseline_head: &str,
    implementation_head: &str,
    base_main_sha: &str,
) -> Result<(), Gh68EvidenceError> {
    if !gh68_hex(baseline_head, 40)
        || !gh68_hex(implementation_head, 40)
        || !gh68_hex(base_main_sha, 40)
        || baseline_head != base_main_sha
        || implementation_head == base_main_sha
    {
        return Err(Gh68EvidenceError("baseline identity"));
    }
    Ok(())
}

pub(crate) fn gh68_require_no_regressions<I>(regressions: I) -> Result<(), Gh68EvidenceError>
where
    I: IntoIterator<Item = bool>,
{
    if regressions.into_iter().any(|regression| regression) {
        return Err(Gh68EvidenceError("performance regression"));
    }
    Ok(())
}

#[rustfmt::skip]
fn gh68_environment() -> Result<serde_json::Value, Gh68EvidenceError> {
    let output = std::process::Command::new("rustc").arg("-Vv").output().map_err(|_| Gh68EvidenceError("rustc unavailable"))?;
    if !output.status.success() { return Err(Gh68EvidenceError("rustc failed")); }
    let rustc = String::from_utf8(output.stdout).map_err(|_| Gh68EvidenceError("rustc output"))?;
    if rustc.trim().is_empty() { return Err(Gh68EvidenceError("rustc output")); }
    Ok(serde_json::json!({"rustc_vv":rustc.trim(),"os":std::env::consts::OS,"arch":std::env::consts::ARCH}))
}

#[rustfmt::skip]
fn gh68_rfc3339_now() -> Result<String, Gh68EvidenceError> {
    let seconds = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|_| Gh68EvidenceError("clock before epoch"))?.as_secs();
    let days = i64::try_from(seconds / 86_400).map_err(|_| Gh68EvidenceError("clock range"))?;
    let z = days + 719_468; let era = if z >= 0 { z } else { z - 146_096 } / 146_097; let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400; let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1; let month = mp + if mp < 10 { 3 } else { -9 }; year += i64::from(month <= 2);
    let day_seconds = seconds % 86_400;
    Ok(format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", day_seconds / 3_600, day_seconds % 3_600 / 60, day_seconds % 60))
}

#[rustfmt::skip]
pub(crate) fn gh68_sha256(bytes: &[u8]) -> String {
    const K: [u32; 64] = [0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2];
    let mut data=bytes.to_vec(); let bits=(data.len() as u64).wrapping_mul(8); data.push(0x80); while data.len()%64!=56 { data.push(0); } data.extend_from_slice(&bits.to_be_bytes());
    let mut h=[0x6a09e667_u32,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19];
    for chunk in data.chunks_exact(64) { let mut w=[0_u32;64]; for (i,word) in w[..16].iter_mut().enumerate(){*word=u32::from_be_bytes(chunk[i*4..i*4+4].try_into().unwrap());} for i in 16..64 { let s0=w[i-15].rotate_right(7)^w[i-15].rotate_right(18)^(w[i-15]>>3); let s1=w[i-2].rotate_right(17)^w[i-2].rotate_right(19)^(w[i-2]>>10); w[i]=w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1); } let [mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut hh]=h; for i in 0..64 { let s1=e.rotate_right(6)^e.rotate_right(11)^e.rotate_right(25); let ch=(e&f)^(!e&g); let t1=hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]); let s0=a.rotate_right(2)^a.rotate_right(13)^a.rotate_right(22); let maj=(a&b)^(a&c)^(b&c); let t2=s0.wrapping_add(maj); hh=g;g=f;f=e;e=d.wrapping_add(t1);d=c;c=b;b=a;a=t1.wrapping_add(t2); } for (slot,value) in h.iter_mut().zip([a,b,c,d,e,f,g,hh]) {*slot=slot.wrapping_add(value);} }
    h.iter().map(|word| format!("{word:08x}")).collect()
}

fn gh68_validate_workloads(value: &serde_json::Value) -> Result<(), Gh68EvidenceError> {
    let list = value
        .as_array()
        .ok_or(Gh68EvidenceError("workloads type"))?;
    if list.len() != GH68_WORKLOAD_NAMES.len() {
        return Err(Gh68EvidenceError("workload count"));
    }
    for (item, expected) in list.iter().zip(GH68_WORKLOAD_NAMES) {
        if item["name"] != expected
            || item["unit"] != "ns"
            || item["warmup_samples"].as_u64().unwrap_or(0) < 3
        {
            return Err(Gh68EvidenceError("workload identity"));
        }
        let samples = item["measured_samples_ns"]
            .as_array()
            .ok_or(Gh68EvidenceError("samples type"))?
            .iter()
            .map(|sample| sample.as_u64().ok_or(Gh68EvidenceError("sample value")))
            .collect::<Result<Vec<_>, _>>()?;
        let median = gh68_median(&samples)?;
        if item["median_ns"] != median || item["mad_ns"] != gh68_mad(&samples, median) {
            return Err(Gh68EvidenceError("aggregate mismatch"));
        }
    }
    Ok(())
}

fn gh68_env_path(name: &str) -> Result<std::path::PathBuf, Gh68EvidenceError> {
    let path =
        std::path::PathBuf::from(std::env::var_os(name).ok_or(Gh68EvidenceError("missing path"))?);
    if !path.is_absolute() {
        return Err(Gh68EvidenceError("path not absolute"));
    }
    Ok(path)
}

fn gh68_benchmark_fixture() -> ([serde_json::Value; 5], serde_json::Value) {
    let oracles = gh68_workload_oracles();
    let workloads = GH68_WORKLOAD_NAMES.map(gh68_workload_json);
    let fixture = serde_json::json!({"version":"gh68-chat-workloads-v1","seed":68,
        "message_count":oracles.iter().map(|v|v.message_order.len()).sum::<usize>(),"block_count":oracles.iter().map(|v|v.message_order.len()).sum::<usize>(),
        "character_count":oracles.iter().map(|v|v.character_count).sum::<usize>(),"width_height_sequence":[[80,24],[32,12],[120,40]]});
    (workloads, fixture)
}

fn gh68_build_benchmark_evidence(
    bytes: &[u8],
    head: &str,
    base: &str,
) -> Result<Vec<u8>, Gh68EvidenceError> {
    let baseline: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| Gh68EvidenceError("baseline json"))?;
    if baseline["schema"] != "gh68-benchmark-v1" {
        return Err(Gh68EvidenceError("baseline identity"));
    }
    gh68_validate_baseline_head(baseline["head_sha"].as_str().unwrap_or(""), head, base)?;
    let environment = gh68_environment()?;
    if baseline["environment"] != environment
        || baseline["fixture"]["version"] != "gh68-chat-workloads-v1"
        || baseline["fixture"]["seed"] != 68
    {
        return Err(Gh68EvidenceError("baseline compatibility"));
    }
    gh68_validate_workloads(&baseline["workloads"])?;
    let coordinate = &baseline["coordinate"];
    if coordinate["repository"] != "majiayu000/rnk"
        || coordinate["workflow"].as_str().unwrap_or("").is_empty()
        || coordinate["run_id"].as_u64().unwrap_or(0) == 0
        || coordinate["artifact_name"]
            .as_str()
            .unwrap_or("")
            .is_empty()
    {
        return Err(Gh68EvidenceError("baseline coordinate"));
    }
    let (workloads, fixture) = gh68_benchmark_fixture();
    if baseline["fixture"] != fixture {
        return Err(Gh68EvidenceError("baseline fixture mismatch"));
    }
    let results = workloads.iter().zip(baseline["workloads"].as_array().unwrap()).map(|(candidate, prior)| {
        let candidate_median=candidate["median_ns"].as_u64().unwrap(); let baseline_median=prior["median_ns"].as_u64().unwrap(); let mad=prior["mad_ns"].as_u64().unwrap();
        serde_json::json!({"name":candidate["name"],"candidate_median_ns":candidate_median,"baseline_median_ns":baseline_median,
            "regression":gh68_is_regression(candidate_median, baseline_median, mad)})
    }).collect::<Vec<_>>();
    let artifact = serde_json::json!({"schema":"gh68-benchmark-v1","head_sha":head,"base_main_sha":base,"generated_at":gh68_rfc3339_now()?,
        "environment":environment,"fixture":fixture,"workloads":workloads,"baseline":{"coordinate":coordinate,"source_sha256":gh68_sha256(bytes),
        "head_sha":baseline["head_sha"],"environment":baseline["environment"],"fixture":baseline["fixture"],"workloads":baseline["workloads"]},
        "comparison":{"environment_equal":true,"fixture_equal":true,"relative_threshold":1.2,"absolute_floor_ns":1_000_000,"results":results}});
    serde_json::to_vec_pretty(&artifact).map_err(|_| Gh68EvidenceError("evidence serialization"))
}

pub(crate) fn gh68_benchmark_internal_contract() -> Result<(), Gh68EvidenceError> {
    let samples = (1..=15).collect::<Vec<_>>();
    let median = gh68_median(&samples)?;
    let mad = gh68_mad(&samples, median);
    let measured = gh68_workload_json(GH68_WORKLOAD_NAMES[0]);
    let workloads = GH68_WORKLOAD_NAMES.map(|name| {
        let mut item = measured.clone();
        item["name"] = name.into();
        item
    });
    gh68_validate_workloads(&serde_json::json!(workloads))?;
    let invalid_count = serde_json::json!([]);
    let mut invalid_identity = serde_json::json!(workloads);
    invalid_identity[0]["unit"] = "ticks".into();
    let mut invalid_name = serde_json::json!(workloads);
    invalid_name[0]["name"] = "unknown".into();
    let mut invalid_warmup = serde_json::json!(workloads);
    invalid_warmup[0]["warmup_samples"] = 0.into();
    let mut invalid_samples = serde_json::json!(workloads);
    invalid_samples[0]["measured_samples_ns"] = serde_json::json!(vec![0; 15]);
    let mut invalid_sample_value = serde_json::json!(workloads);
    invalid_sample_value[0]["measured_samples_ns"][0] = "slow".into();
    let mut invalid_aggregate = serde_json::json!(workloads);
    invalid_aggregate[0]["median_ns"] = 0.into();
    let mut invalid_mad = serde_json::json!(workloads);
    invalid_mad[0]["mad_ns"] = u64::MAX.into();
    let base = "b".repeat(40);
    let head = "a".repeat(40);
    let environment = gh68_environment()?;
    let (baseline_workloads, fixture) = gh68_benchmark_fixture();
    let baseline = serde_json::json!({
        "schema":"gh68-benchmark-v1","head_sha":base,"environment":environment,
        "fixture":fixture,"workloads":baseline_workloads,
        "coordinate":{"repository":"majiayu000/rnk","workflow":"gh68","run_id":68,
            "artifact_name":"gh68-approved-baseline"}
    });
    let encoded = |value: &serde_json::Value| serde_json::to_vec(value).unwrap();
    let baseline_bytes = encoded(&baseline);
    let evidence_bytes = gh68_build_benchmark_evidence(&baseline_bytes, &head, &base)?;
    gh68_validate_benchmark_comparison(&evidence_bytes, &baseline_bytes, &head, &base)?;

    let mut invalid_schema = baseline.clone();
    invalid_schema["schema"] = "unknown".into();
    let mut invalid_environment = baseline.clone();
    invalid_environment["environment"]["os"] = "unknown".into();
    let mut invalid_fixture_version = baseline.clone();
    invalid_fixture_version["fixture"]["version"] = "unknown".into();
    let mut invalid_fixture_seed = baseline.clone();
    invalid_fixture_seed["fixture"]["seed"] = 0.into();
    let mut invalid_repository = baseline.clone();
    invalid_repository["coordinate"]["repository"] = "unknown".into();
    let mut invalid_workflow = baseline.clone();
    invalid_workflow["coordinate"]["workflow"] = "".into();
    let mut invalid_run = baseline.clone();
    invalid_run["coordinate"]["run_id"] = 0.into();
    let mut invalid_artifact = baseline.clone();
    invalid_artifact["coordinate"]["artifact_name"] = "".into();
    let mut invalid_fixture = baseline.clone();
    invalid_fixture["fixture"]["character_count"] = 0.into();
    let build_errors = [
        invalid_schema,
        invalid_environment,
        invalid_fixture_version,
        invalid_fixture_seed,
        invalid_repository,
        invalid_workflow,
        invalid_run,
        invalid_artifact,
        invalid_fixture,
    ]
    .map(|value| gh68_build_benchmark_evidence(&encoded(&value), &head, &base));

    let evidence: serde_json::Value = serde_json::from_slice(&evidence_bytes).unwrap();
    let mut wrong_schema = evidence.clone();
    wrong_schema["schema"] = "unknown".into();
    let mut wrong_head = evidence.clone();
    wrong_head["head_sha"] = base.clone().into();
    let mut wrong_base = evidence.clone();
    wrong_base["base_main_sha"] = head.clone().into();
    let mut wrong_environment = evidence.clone();
    wrong_environment["environment"]["os"] = "unknown".into();
    let mut wrong_digest = evidence.clone();
    wrong_digest["baseline"]["source_sha256"] = "0".repeat(64).into();
    let mut wrong_binding_head = evidence.clone();
    wrong_binding_head["baseline"]["head_sha"] = head.clone().into();
    let mut wrong_binding_environment = evidence.clone();
    wrong_binding_environment["baseline"]["environment"]["os"] = "unknown".into();
    let mut wrong_binding_fixture = evidence.clone();
    wrong_binding_fixture["baseline"]["fixture"]["seed"] = 0.into();
    let mut wrong_binding_workloads = evidence.clone();
    wrong_binding_workloads["baseline"]["workloads"][0]["median_ns"] = 0.into();
    let mut wrong_binding_coordinate = evidence.clone();
    wrong_binding_coordinate["baseline"]["coordinate"]["run_id"] = 0.into();
    let mut wrong_result_count = evidence.clone();
    wrong_result_count["comparison"]["results"] = serde_json::json!([]);
    let mut wrong_fixture = evidence.clone();
    wrong_fixture["fixture"]["seed"] = 0.into();
    let mut wrong_environment_flag = evidence.clone();
    wrong_environment_flag["comparison"]["environment_equal"] = false.into();
    let mut wrong_fixture_flag = evidence.clone();
    wrong_fixture_flag["comparison"]["fixture_equal"] = false.into();
    let mut wrong_relative = evidence.clone();
    wrong_relative["comparison"]["relative_threshold"] = 1.into();
    let mut wrong_absolute = evidence.clone();
    wrong_absolute["comparison"]["absolute_floor_ns"] = 0.into();
    let mut wrong_result_name = evidence.clone();
    wrong_result_name["comparison"]["results"][0]["name"] = "unknown".into();
    let mut wrong_candidate_median = evidence.clone();
    wrong_candidate_median["comparison"]["results"][0]["candidate_median_ns"] = 0.into();
    let mut wrong_baseline_median = evidence.clone();
    wrong_baseline_median["comparison"]["results"][0]["baseline_median_ns"] = 0.into();
    let mut wrong_regression = evidence.clone();
    wrong_regression["comparison"]["results"][0]["regression"] = true.into();
    let comparison_errors = [
        wrong_schema,
        wrong_head,
        wrong_base,
        wrong_environment,
        wrong_digest,
        wrong_binding_head,
        wrong_binding_environment,
        wrong_binding_fixture,
        wrong_binding_workloads,
        wrong_binding_coordinate,
        wrong_result_count,
        wrong_fixture,
        wrong_environment_flag,
        wrong_fixture_flag,
        wrong_relative,
        wrong_absolute,
        wrong_result_name,
        wrong_candidate_median,
        wrong_baseline_median,
        wrong_regression,
    ]
    .map(|value| {
        gh68_validate_benchmark_comparison(&encoded(&value), &baseline_bytes, &head, &base)
    });
    let checks = [
        gh68_benchmark_route_values(Some("smoke_blocked_no_baseline"), "metadata", None, false)
            .is_ok(),
        gh68_benchmark_route_values(Some("smoke_blocked_no_baseline"), "metadata", None, true)
            .is_err(),
        gh68_benchmark_route_values(
            Some("performance_validation"),
            "metadata",
            Some("produce"),
            true,
        )
        .is_ok(),
        gh68_benchmark_route_values(
            Some("performance_validation"),
            "comparison",
            Some("validate"),
            true,
        )
        .is_ok(),
        gh68_benchmark_route_values(
            Some("performance_validation"),
            "comparison",
            Some("produce"),
            true,
        )
        .is_err(),
        gh68_benchmark_route_values(Some("unknown"), "metadata", None, false).is_err(),
        gh68_benchmark_route_values(None, "metadata", None, false).is_err(),
        gh68_median(&samples[..14]).is_err(),
        gh68_median(&[0; 15]).is_err(),
        gh68_validate_workloads(&serde_json::Value::Null).is_err(),
        gh68_validate_workloads(&invalid_count).is_err(),
        gh68_validate_workloads(&invalid_identity).is_err(),
        gh68_validate_workloads(&invalid_name).is_err(),
        gh68_validate_workloads(&invalid_warmup).is_err(),
        gh68_validate_workloads(&invalid_samples).is_err(),
        gh68_validate_workloads(&invalid_sample_value).is_err(),
        gh68_validate_workloads(&invalid_aggregate).is_err(),
        gh68_validate_workloads(&invalid_mad).is_err(),
        gh68_environment()?.as_object().is_some(),
        gh68_rfc3339_now()?.ends_with('Z'),
        !gh68_is_regression(1_100_000, 1_000_000, 1),
        gh68_is_regression(3_000_000, 1_000_000, 1),
        gh68_hex(&"a".repeat(40), 40),
        !gh68_hex("short", 40),
        !gh68_hex(&format!("{}z", "a".repeat(39)), 40),
        gh68_validate_baseline_head(&"b".repeat(40), &"a".repeat(40), &"b".repeat(40)).is_ok(),
        gh68_validate_baseline_head("short", &"a".repeat(40), &"b".repeat(40)).is_err(),
        gh68_validate_baseline_head(&"b".repeat(40), "short", &"b".repeat(40)).is_err(),
        gh68_validate_baseline_head(&"b".repeat(40), &"a".repeat(40), "short").is_err(),
        gh68_validate_baseline_head(&"c".repeat(40), &"a".repeat(40), &"b".repeat(40)).is_err(),
        gh68_validate_baseline_head(&"b".repeat(40), &"b".repeat(40), &"b".repeat(40)).is_err(),
        gh68_require_no_regressions([false, false]).is_ok(),
        gh68_require_no_regressions([false, true]).is_err(),
        gh68_require_no_regressions(vec![false, false]).is_ok(),
        gh68_require_no_regressions(vec![false, true]).is_err(),
        build_errors.into_iter().all(|result| result.is_err()),
        comparison_errors.into_iter().all(|result| result.is_err()),
        median == 8,
        mad == 4,
    ];
    if checks.into_iter().all(|passed| passed) {
        Ok(())
    } else {
        Err(Gh68EvidenceError("internal benchmark contract"))
    }
}

pub(crate) fn gh68_benchmark_metadata_contract() -> Result<(), Gh68EvidenceError> {
    if std::env::var("GH68_BENCHMARK_MODE").as_deref() != Ok("produce") {
        return Err(Gh68EvidenceError("mode is not produce"));
    }
    let baseline_path = gh68_env_path("GH68_BENCHMARK_BASELINE")?;
    let evidence_path = gh68_env_path("GH68_BENCHMARK_EVIDENCE")?;
    let head =
        std::env::var("GH68_IMPLEMENTATION_HEAD").map_err(|_| Gh68EvidenceError("missing head"))?;
    let base =
        std::env::var("GH68_BASE_MAIN_SHA").map_err(|_| Gh68EvidenceError("missing base"))?;
    let bytes =
        std::fs::read(&baseline_path).map_err(|_| Gh68EvidenceError("baseline unreadable"))?;
    let output = gh68_build_benchmark_evidence(&bytes, &head, &base)?;
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(evidence_path)
        .map_err(|_| Gh68EvidenceError("evidence create"))?;
    file.write_all(&output)
        .and_then(|_| file.sync_all())
        .map_err(|_| Gh68EvidenceError("evidence write"))
}

fn gh68_validate_benchmark_comparison(
    evidence: &[u8],
    baseline_bytes: &[u8],
    head: &str,
    base: &str,
) -> Result<(), Gh68EvidenceError> {
    let value: serde_json::Value =
        serde_json::from_slice(evidence).map_err(|_| Gh68EvidenceError("evidence json"))?;
    if !gh68_hex(head, 40)
        || !gh68_hex(base, 40)
        || value["schema"] != "gh68-benchmark-v1"
        || value["head_sha"] != head
        || value["base_main_sha"] != base
        || value["environment"] != gh68_environment()?
    {
        return Err(Gh68EvidenceError("evidence identity"));
    }
    gh68_validate_workloads(&value["workloads"])?;
    gh68_validate_workloads(&value["baseline"]["workloads"])?;
    let baseline_source: serde_json::Value =
        serde_json::from_slice(baseline_bytes).map_err(|_| Gh68EvidenceError("baseline json"))?;
    if value["baseline"]["source_sha256"] != gh68_sha256(baseline_bytes)
        || !gh68_hex(
            value["baseline"]["source_sha256"].as_str().unwrap_or(""),
            64,
        )
    {
        return Err(Gh68EvidenceError("baseline digest"));
    }
    gh68_validate_baseline_head(
        baseline_source["head_sha"].as_str().unwrap_or(""),
        head,
        base,
    )?;
    if value["baseline"]["head_sha"] != baseline_source["head_sha"]
        || value["baseline"]["head_sha"] != base
        || value["baseline"]["environment"] != baseline_source["environment"]
        || value["baseline"]["fixture"] != baseline_source["fixture"]
        || value["baseline"]["workloads"] != baseline_source["workloads"]
        || value["baseline"]["coordinate"] != baseline_source["coordinate"]
    {
        return Err(Gh68EvidenceError("baseline binding"));
    }
    let candidates = value["workloads"].as_array().unwrap();
    let priors = value["baseline"]["workloads"].as_array().unwrap();
    let results = value["comparison"]["results"]
        .as_array()
        .ok_or(Gh68EvidenceError("results type"))?;
    if results.len() != 5
        || value["fixture"] != value["baseline"]["fixture"]
        || value["comparison"]["environment_equal"] != true
        || value["comparison"]["fixture_equal"] != true
        || value["comparison"]["relative_threshold"] != 1.2
        || value["comparison"]["absolute_floor_ns"] != 1_000_000
    {
        return Err(Gh68EvidenceError("comparison contract"));
    }
    let mut regressions = Vec::with_capacity(results.len());
    for ((candidate, prior), result) in candidates.iter().zip(priors).zip(results) {
        let cm = candidate["median_ns"].as_u64().unwrap();
        let bm = prior["median_ns"].as_u64().unwrap();
        let mad = prior["mad_ns"].as_u64().unwrap();
        let regression = gh68_is_regression(cm, bm, mad);
        if result["name"] != candidate["name"]
            || result["candidate_median_ns"] != cm
            || result["baseline_median_ns"] != bm
            || result["regression"] != regression
        {
            return Err(Gh68EvidenceError("comparison result"));
        }
        regressions.push(regression);
    }
    gh68_require_no_regressions(regressions)
}

pub(crate) fn gh68_benchmark_comparison_contract() -> Result<(), Gh68EvidenceError> {
    if std::env::var("GH68_BENCHMARK_MODE").as_deref() != Ok("validate") {
        return Err(Gh68EvidenceError("mode is not validate"));
    }
    let evidence = std::fs::read(gh68_env_path("GH68_BENCHMARK_EVIDENCE")?)
        .map_err(|_| Gh68EvidenceError("evidence unreadable"))?;
    let baseline = std::fs::read(gh68_env_path("GH68_BENCHMARK_BASELINE")?)
        .map_err(|_| Gh68EvidenceError("baseline unreadable"))?;
    let head =
        std::env::var("GH68_IMPLEMENTATION_HEAD").map_err(|_| Gh68EvidenceError("missing head"))?;
    let base =
        std::env::var("GH68_BASE_MAIN_SHA").map_err(|_| Gh68EvidenceError("missing base"))?;
    gh68_validate_benchmark_comparison(&evidence, &baseline, &head, &base)
}

#[allow(dead_code)]
pub(crate) fn gh68_sha256_contract() -> bool {
    gh68_sha256(b"abc") == "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
}
