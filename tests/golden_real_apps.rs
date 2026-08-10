use rnk::components::chat::message_list::{
    HorizontalInsets, MessageCompositeMeasureConfig, MessageExpansionKey, MessageList,
    MessageListEntry, MessageListState, MessageMeasureOutcome, MessageRows,
    MessageShellMeasureConfig, MessageVariantKey, ViewportRows,
};
use rnk::components::chat::scrollback::NativeTerminalSink;
use rnk::components::chat::{
    BlockId, ChatComposerState, ChatMessage, ChatMessageView, ChatRole, ConversationEvent,
    ConversationGuard, ConversationState, ConversationUpdate, FullscreenChatShell, InlineChatShell,
    InlineCommitReport, InlineKeyOutcome, LiveState, MessageBlock, MessageBlockEntry, MessageId,
    MessageMutationGuard, MessageRevision, ProjectionContext, ScrollbackNamespace, ThemeIdentity,
    UpdateId,
};
use rnk::components::{
    Badge, BadgeVariant, Box as RnkBox, Confirm, ConfirmState, Message, Progress, ProgressSymbols,
    SelectInput, SelectItem, Stat, Text, TextArea, TextAreaState,
};
use rnk::core::{Color, Element, FlexDirection};
use rnk::hooks::Key;
use rnk::testing::GoldenTest;
use std::io::{self, Write};
use std::num::NonZeroUsize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommitObservation {
    Fixed,
    Retained,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BenchmarkEvidence {
    CorrectnessOracle,
    SmokeOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HarnessEvidence<'a> {
    head: &'a str,
    evidence: &'a [&'a str],
    environment: &'a str,
    expected_environment: &'a str,
    verified: bool,
    api_key: Option<&'a str>,
    commit_observation: CommitObservation,
    retries: usize,
    exact_test_ignored: bool,
    benchmark: BenchmarkEvidence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HarnessEvidenceError {
    StaleHead,
    EmptyEvidence,
    UnauthenticatedVerification,
    EnvironmentMismatch,
    PlaceholderKey,
    UnknownWasRetried,
    IgnoredExactTest,
    SmokeOnlyPerformanceClaim,
}

fn validate_harness_evidence(
    evidence: &HarnessEvidence<'_>,
    expected_head: &str,
) -> Result<(), HarnessEvidenceError> {
    if evidence.head != expected_head {
        return Err(HarnessEvidenceError::StaleHead);
    }
    if evidence.evidence.is_empty() || evidence.evidence.iter().any(|item| item.trim().is_empty()) {
        return Err(HarnessEvidenceError::EmptyEvidence);
    }
    if !evidence.verified {
        return Err(HarnessEvidenceError::UnauthenticatedVerification);
    }
    if evidence.environment != evidence.expected_environment {
        return Err(HarnessEvidenceError::EnvironmentMismatch);
    }
    if evidence
        .api_key
        .is_some_and(|key| matches!(key, "your-api-key" | "placeholder" | "test-key"))
    {
        return Err(HarnessEvidenceError::PlaceholderKey);
    }
    if evidence.commit_observation == CommitObservation::Unknown && evidence.retries != 0 {
        return Err(HarnessEvidenceError::UnknownWasRetried);
    }
    if evidence.exact_test_ignored {
        return Err(HarnessEvidenceError::IgnoredExactTest);
    }
    if evidence.benchmark == BenchmarkEvidence::SmokeOnly {
        return Err(HarnessEvidenceError::SmokeOnlyPerformanceClaim);
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct AdapterDelta<'a> {
    event_id: &'a str,
    text: &'a str,
    terminal: bool,
}

fn text_message(id: u64, block_id: u64, role: ChatRole, text: &str) -> ChatMessage {
    ChatMessage::new(
        MessageId::new(id),
        role,
        vec![MessageBlockEntry::new(
            BlockId::new(block_id),
            MessageBlock::Text(text.to_owned()),
        )],
    )
    .expect("fixture message is valid")
}

fn mutation_guard(state: &ConversationState, message_id: MessageId) -> MessageMutationGuard {
    let message = state
        .message(message_id)
        .expect("fixture message must already exist");
    MessageMutationGuard::new(
        ConversationGuard::new(state.revision()),
        message_id,
        message.revision(),
    )
}

fn apply_update(state: &mut ConversationState, event_id: &str, update: ConversationUpdate) {
    state
        .apply_event(ConversationEvent::new(
            UpdateId::new(event_id).expect("fixture event id is valid"),
            state.expected_sequence(),
            update,
        ))
        .expect("fixture event must apply");
}

fn apply_adapter_fixture(deltas: &[AdapterDelta<'_>]) -> ConversationState {
    let mut state = ConversationState::new(0, NonZeroUsize::new(16).unwrap());
    let guard = ConversationGuard::new(state.revision());
    apply_update(
        &mut state,
        "user",
        ConversationUpdate::push(
            guard,
            text_message(1, 1, ChatRole::User, "Explain the release gate"),
        ),
    );
    let guard = ConversationGuard::new(state.revision());
    apply_update(
        &mut state,
        "assistant",
        ConversationUpdate::push(guard, text_message(2, 2, ChatRole::Assistant, "")),
    );
    for delta in deltas {
        let update = ConversationUpdate::append_text(
            mutation_guard(&state, MessageId::new(2)),
            BlockId::new(2),
            delta.text,
        )
        .expect("fixture delta is non-empty");
        apply_update(&mut state, delta.event_id, update);
        if delta.terminal {
            let complete = ConversationUpdate::complete(mutation_guard(&state, MessageId::new(2)));
            apply_update(&mut state, "complete", complete);
        }
    }
    state
}

fn conversation_view(state: &ConversationState) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .children(
            state
                .messages()
                .iter()
                .map(|message| ChatMessageView::new(message).into_element()),
        )
        .into_element()
}

fn exact_test_names(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("fn "))
        .filter_map(|tail| tail.strip_suffix("() {"))
        .collect()
}

fn markdown_links(source: &str) -> Vec<&str> {
    source
        .split("(")
        .skip(1)
        .filter_map(|tail| tail.split_once(')').map(|(target, _)| target))
        .collect()
}

fn public_example_names(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- `"))
        .filter_map(|tail| tail.split_once('`').map(|(name, _)| name))
        .collect()
}

fn command_lines(source: &str) -> Vec<&str> {
    source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("cargo "))
        .collect()
}

fn chat_flow() -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Message::system("session: deterministic chat").into_element())
        .child(Message::user("Summarize the release gates").into_element())
        .child(Message::assistant("CI, docs, and examples are all checked.").into_element())
        .child(Text::new("> ready").color(Color::Yellow).into_element())
        .into_element()
}

fn git_flow() -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .gap(1.0)
                .child(Text::new("branch").bold().into_element())
                .child(
                    Badge::new("main")
                        .variant(BadgeVariant::Success)
                        .into_element(),
                )
                .child(
                    Badge::new("+2")
                        .variant(BadgeVariant::Warning)
                        .into_element(),
                )
                .into_element(),
        )
        .child(Text::new("M src/testing/harness.rs").into_element())
        .child(Text::new("A tests/golden_real_apps.rs").into_element())
        .child(
            Text::new("checks: clean")
                .color(Color::Green)
                .into_element(),
        )
        .into_element()
}

fn top_flow() -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .gap(2.0)
                .child(Stat::new("CPU", "42%").trend_down("3%").into_element())
                .child(Stat::new("Mem", "1.8G").trend_up("128M").into_element())
                .into_element(),
        )
        .child(
            Progress::new()
                .progress(0.42)
                .width(18)
                .symbols(ProgressSymbols::ascii())
                .show_percent(true)
                .label("load")
                .into_element(),
        )
        .child(Text::new("pid  command      cpu").dim().into_element())
        .child(Text::new("101  rnk-demo     12%").into_element())
        .into_element()
}

fn form_flow() -> Element {
    let mut confirm = ConfirmState::default_yes("Submit profile?");
    confirm.focus_yes();

    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new("Name: Ada Lovelace").into_element())
        .child(
            SelectInput::new(vec![
                SelectItem::new("Engineer", "engineer"),
                SelectItem::new("Designer", "designer"),
                SelectItem::new("Researcher", "researcher"),
            ])
            .highlighted(2)
            .limit(3)
            .into_element(),
        )
        .child(Confirm::new(&confirm).into_element())
        .into_element()
}

fn textarea_flow() -> Element {
    let state = TextAreaState::with_content("fn main() {\n    println!(\"rnk\");\n}");

    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new("editor: src/main.rs").bold().into_element())
        .child(
            TextArea::new(&state)
                .width(36)
                .height(5)
                .line_numbers(true)
                .prompt("| ")
                .into_element(),
        )
        .into_element()
}

#[test]
fn chat_flow_plain_golden() {
    GoldenTest::new("real_app_chat")
        .with_size(80, 12)
        .assert_match(&chat_flow());
}

#[test]
fn chat_flow_ansi_golden() {
    GoldenTest::new("real_app_chat")
        .ansi()
        .with_size(80, 12)
        .assert_match(&chat_flow());
}

#[test]
fn git_flow_plain_golden() {
    GoldenTest::new("real_app_git")
        .with_size(80, 12)
        .assert_match(&git_flow());
}

#[test]
fn top_flow_plain_golden() {
    GoldenTest::new("real_app_top")
        .with_size(80, 12)
        .assert_match(&top_flow());
}

#[test]
fn form_flow_plain_golden() {
    GoldenTest::new("real_app_forms")
        .with_size(80, 18)
        .assert_match(&form_flow());
}

#[test]
fn textarea_flow_plain_golden() {
    GoldenTest::new("real_app_textarea")
        .with_size(80, 12)
        .assert_match(&textarea_flow());
}

#[test]
fn gh68_harness_contract() {
    const EXPECTED_HEAD: &str = "0123456789abcdef0123456789abcdef01234567";
    let offline = [
        AdapterDelta {
            event_id: "offline-1",
            text: "Use typed ",
            terminal: false,
        },
        AdapterDelta {
            event_id: "offline-2",
            text: "updates.",
            terminal: true,
        },
    ];
    let provider = [
        AdapterDelta {
            event_id: "offline-1",
            text: "Use typed ",
            terminal: false,
        },
        AdapterDelta {
            event_id: "offline-2",
            text: "updates.",
            terminal: true,
        },
    ];
    let offline_state = apply_adapter_fixture(&offline);
    let provider_state = apply_adapter_fixture(&provider);
    assert_eq!(offline_state.snapshot(), provider_state.snapshot());
    let view = conversation_view(&offline_state);
    assert_eq!(view.children.len(), 2);

    assert_eq!(
        exact_test_names("#[test]\nfn exact_name() {\n}\nfn helper() {}"),
        ["exact_name"]
    );
    assert_eq!(
        markdown_links("[quickstart](docs/CHAT_QUICKSTART.md)"),
        ["docs/CHAT_QUICKSTART.md"]
    );
    assert_eq!(
        public_example_names("- `chat` — tutorial\n- `rnk_chat` — fullscreen"),
        ["chat", "rnk_chat"]
    );
    assert_eq!(
        command_lines("note\ncargo check --example chat\nother"),
        ["cargo check --example chat"]
    );

    let valid = HarnessEvidence {
        head: EXPECTED_HEAD,
        evidence: &["typed-conversation", "rendered-view"],
        environment: "ubuntu-24.04",
        expected_environment: "ubuntu-24.04",
        verified: true,
        api_key: None,
        commit_observation: CommitObservation::Fixed,
        retries: 0,
        exact_test_ignored: false,
        benchmark: BenchmarkEvidence::CorrectnessOracle,
    };
    assert_eq!(validate_harness_evidence(&valid, EXPECTED_HEAD), Ok(()));

    let cases = [
        (
            HarnessEvidence {
                head: "a3e36dbae157cda3c7247c89675936e9ce7c5625",
                ..valid.clone()
            },
            HarnessEvidenceError::StaleHead,
        ),
        (
            HarnessEvidence {
                evidence: &[],
                ..valid.clone()
            },
            HarnessEvidenceError::EmptyEvidence,
        ),
        (
            HarnessEvidence {
                verified: false,
                ..valid.clone()
            },
            HarnessEvidenceError::UnauthenticatedVerification,
        ),
        (
            HarnessEvidence {
                environment: "macos-15",
                ..valid.clone()
            },
            HarnessEvidenceError::EnvironmentMismatch,
        ),
        (
            HarnessEvidence {
                api_key: Some("placeholder"),
                ..valid.clone()
            },
            HarnessEvidenceError::PlaceholderKey,
        ),
        (
            HarnessEvidence {
                commit_observation: CommitObservation::Unknown,
                retries: 1,
                ..valid.clone()
            },
            HarnessEvidenceError::UnknownWasRetried,
        ),
        (
            HarnessEvidence {
                exact_test_ignored: true,
                ..valid.clone()
            },
            HarnessEvidenceError::IgnoredExactTest,
        ),
        (
            HarnessEvidence {
                benchmark: BenchmarkEvidence::SmokeOnly,
                ..valid.clone()
            },
            HarnessEvidenceError::SmokeOnlyPerformanceClaim,
        ),
    ];
    for (candidate, expected) in cases {
        assert_eq!(
            validate_harness_evidence(&candidate, EXPECTED_HEAD),
            Err(expected)
        );
    }

    assert_eq!(CommitObservation::Retained, CommitObservation::Retained);
}

#[test]
fn gh68_chat_tutorial_contract() {
    let source = include_str!("../examples/chat.rs");
    for required in [
        "ConversationState::new",
        "ConversationUpdate::push",
        "ConversationUpdate::complete",
        "ChatMessageView::new",
        "ComposerProjection::build",
        "acknowledge_success",
        "acknowledge_failure",
    ] {
        assert!(
            source.contains(required),
            "missing public chat seam: {required}"
        );
    }
    for forbidden in [
        "Vec::<String>",
        ".pop(",
        "UnicodeSegmentation",
        "UnicodeWidthStr",
        "cursor_column()",
        ".graphemes(",
    ] {
        assert!(
            !source.contains(forbidden),
            "tutorial retained private transcript/cursor logic: {forbidden}"
        );
    }

    let state = apply_adapter_fixture(&[
        AdapterDelta {
            event_id: "offline-1",
            text: "Use typed ",
            terminal: false,
        },
        AdapterDelta {
            event_id: "offline-2",
            text: "updates.",
            terminal: true,
        },
    ]);
    assert_eq!(state.messages().len(), 2);
    let rendered = rnk::render_to_string(&conversation_view(&state), 60);
    assert!(rendered.contains("Explain the release gate"));
    assert!(rendered.contains("Use typed updates."));
}

#[test]
fn gh68_fullscreen_example_contract() {
    let source = include_str!("../examples/rnk_chat.rs");
    for required in [
        "FullscreenChatShell::try_new",
        "MessageList::new",
        "ChatMessageView::new",
        ".snapshot().root().border_bounds().height()",
        "slice.message_rows",
        "slice.viewport_rows",
        "scroll_offset_y(offset)",
        "candidate.try_shell()?",
    ] {
        assert!(
            source.contains(required),
            "missing fullscreen seam: {required}"
        );
    }
    for forbidden in [
        "struct ChatMessage {",
        "enum Role {",
        ".skip(",
        ".take(12)",
        "Vec<String>",
    ] {
        assert!(
            !source.contains(forbidden),
            "fullscreen example retained private chat logic: {forbidden}"
        );
    }

    let state = apply_adapter_fixture(&[
        AdapterDelta {
            event_id: "fullscreen-1",
            text: "A deliberately wrapped response that spans several terminal rows ",
            terminal: false,
        },
        AdapterDelta {
            event_id: "fullscreen-2",
            text: "and proves the visible slice is row based.",
            terminal: true,
        },
    ]);
    let entries = state
        .messages()
        .iter()
        .map(|message| {
            let shell = MessageShellMeasureConfig::try_new(24, HorizontalInsets::new(0, 0), vec![])
                .unwrap();
            MessageListEntry::new(
                message.id(),
                message.revision(),
                MessageVariantKey::new(0),
                MessageExpansionKey::new(0),
                MessageCompositeMeasureConfig::try_new(vec![], shell).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let transcript = MessageListState::try_new(&entries, 24, ViewportRows::new(3), 16, |request| {
        let message = state
            .message(request.entry.message_id())
            .expect("entry identity comes from the conversation");
        let element = ChatMessageView::new(message).into_element();
        let engine = rnk::layout::LayoutEngine::new();
        let frame = engine
            .prepare_element_incremental(&element, None, 24, u16::MAX)
            .expect("fixture message has an authoritative snapshot");
        let height = frame.snapshot().root().border_bounds().height();
        MessageMeasureOutcome::<String, ()>::Measured(
            MessageRows::try_new(u64::try_from(height).unwrap()).unwrap(),
        )
    })
    .unwrap();
    let shell =
        FullscreenChatShell::try_new(transcript, ChatComposerState::new(), 24, 5, 1).unwrap();
    assert_eq!(shell.layout().transcript().rows(), 3);

    let mut slices = Vec::new();
    let element = MessageList::new(shell.transcript())
        .try_into_element::<String, _>(|entry, _key, slice| {
            let visible = slice.viewport_rows.end - slice.viewport_rows.start;
            assert_eq!(visible, slice.message_rows.end - slice.message_rows.start);
            slices.push((
                entry.message_id(),
                slice.message_rows.clone(),
                slice.viewport_rows.clone(),
            ));
            Ok(ChatMessageView::new(state.message(entry.message_id()).unwrap()).into_element())
        })
        .unwrap();
    assert!(!slices.is_empty());
    assert!(slices.iter().all(|(_, message, viewport)| {
        message.start < message.end && viewport.start < viewport.end && viewport.end <= 3
    }));
    let rendered = rnk::render_to_string(&element, 24);
    assert!(!rendered.is_empty());
    assert_eq!(
        entries[0].content_revision(),
        MessageRevision::new(1).unwrap()
    );
}

#[derive(Debug, Default)]
struct Gh68BudgetedWriter {
    accepted: Vec<u8>,
    budget: Option<usize>,
}

impl Gh68BudgetedWriter {
    const fn unlimited() -> Self {
        Self {
            accepted: Vec::new(),
            budget: None,
        }
    }

    const fn accepting(budget: usize) -> Self {
        Self {
            accepted: Vec::new(),
            budget: Some(budget),
        }
    }
}

impl Write for Gh68BudgetedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(budget) = self.budget else {
            self.accepted.extend_from_slice(bytes);
            return Ok(bytes.len());
        };
        let room = budget.saturating_sub(self.accepted.len());
        if room == 0 {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "budget exhausted",
            ));
        }
        let count = room.min(bytes.len());
        self.accepted.extend_from_slice(&bytes[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn inline_report(
    writer: Gh68BudgetedWriter,
) -> (
    InlineCommitReport,
    InlineChatShell<NativeTerminalSink<Gh68BudgetedWriter>>,
    rnk::components::chat::SubmissionToken,
) {
    let mut shell = InlineChatShell::new(
        ScrollbackNamespace::new("gh68.inline").unwrap(),
        NativeTerminalSink::new(writer),
    );
    assert!(matches!(
        shell.handle_key(
            &rnk::components::chat::ChatComposerKeyMap::new(),
            "draft",
            &Key::default()
        ),
        InlineKeyOutcome::Changed(_)
    ));
    let enter = Key {
        return_key: true,
        ..Key::default()
    };
    assert_eq!(
        shell.handle_key(
            &rnk::components::chat::ChatComposerKeyMap::new(),
            "",
            &enter
        ),
        InlineKeyOutcome::Submitted("draft".to_owned())
    );
    let token = shell.composer().pending_submission().unwrap().token();
    let id = MessageId::new(1);
    shell.stream(id).unwrap();
    let report = shell
        .finish(
            id,
            MessageRevision::INITIAL,
            "You: draft",
            ProjectionContext::new(40, ThemeIdentity::new(1)).unwrap(),
        )
        .unwrap();
    (report, shell, token)
}

#[test]
fn gh68_inline_example_contract() {
    let source = include_str!("../examples/claude_input_box.rs");
    for required in [
        "InlineChatShell::new",
        "InlineCommitReport::Fixed",
        "InlineCommitReport::Retained",
        "InlineCommitReport::Latched",
        "acknowledge_success(token)",
        "acknowledge_failure(token)",
        "LiveState::AwaitingResolution",
    ] {
        assert!(source.contains(required), "missing inline seam: {required}");
    }
    for forbidden in ["app.println(", "println!(", "submitted_count", "wrap_text("] {
        assert!(
            !source.contains(forbidden),
            "inline example retained a direct publication ledger: {forbidden}"
        );
    }

    let (fixed, mut fixed_shell, fixed_token) = inline_report(Gh68BudgetedWriter::unlimited());
    assert!(matches!(fixed, InlineCommitReport::Fixed { .. }));
    fixed_shell
        .composer_mut()
        .acknowledge_success(fixed_token)
        .unwrap();
    assert_eq!(fixed_shell.composer().text(), "");
    assert!(fixed_shell.live_messages().is_empty());

    let (retained, mut retained_shell, retained_token) =
        inline_report(Gh68BudgetedWriter::accepting(0));
    assert!(matches!(retained, InlineCommitReport::Retained { .. }));
    retained_shell
        .composer_mut()
        .acknowledge_failure(retained_token)
        .unwrap();
    assert_eq!(retained_shell.composer().text(), "draft");
    assert_eq!(
        retained_shell.live_state(MessageId::new(1)),
        Some(LiveState::AwaitingRetry)
    );

    let (latched, mut latched_shell, latched_token) =
        inline_report(Gh68BudgetedWriter::accepting(3));
    assert!(matches!(latched, InlineCommitReport::Latched { .. }));
    latched_shell
        .composer_mut()
        .acknowledge_failure(latched_token)
        .unwrap();
    assert_eq!(latched_shell.composer().text(), "draft");
    assert_eq!(
        latched_shell.live_state(MessageId::new(1)),
        Some(LiveState::AwaitingResolution)
    );
}
