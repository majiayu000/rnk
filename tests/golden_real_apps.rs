use crossterm::event::KeyModifiers;
use rnk::components::chat::message_list::ViewportRows;
use rnk::components::chat::scrollback::NativeTerminalSink;
use rnk::components::chat::{
    BlockId, ChatComposerState, ChatMessage, ChatMessageView, ChatRole, ConversationEvent,
    ConversationGuard, ConversationState, ConversationUpdate, InlineChatShell, InlineCommitReport,
    InlineKeyOutcome, LiveState, MessageBlock, MessageBlockEntry, MessageId, MessageMutationGuard,
    MessageRevision, ProjectionContext, ScrollbackNamespace, ThemeIdentity, UpdateId,
};
use rnk::components::{
    Badge, BadgeVariant, Box as RnkBox, Confirm, ConfirmState, Message, Progress, ProgressSymbols,
    SelectInput, SelectItem, Stat, Text, TextArea, TextAreaState,
};
use rnk::core::{Color, Element, FlexDirection};
use rnk::hooks::{Key, KeyCodeKind, use_input, use_paste, use_signal};
use rnk::testing::{GoldenTest, TestHarness};
use std::io::{self, Write};
use std::num::NonZeroUsize;

#[cfg(unix)]
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
#[cfg(unix)]
use std::io::Read;
#[cfg(unix)]
use std::os::fd::RawFd;
#[cfg(unix)]
use std::time::{Duration, Instant};

#[path = "../examples/chat.rs"]
#[allow(dead_code)]
mod gh68_chat;
#[path = "../examples/rnk_chat.rs"]
#[allow(dead_code)]
mod gh68_fullscreen;
#[path = "../examples/glm_chat.rs"]
#[allow(dead_code)]
mod gh68_glm;
#[path = "../examples/claude_input_box.rs"]
#[allow(dead_code)]
mod gh68_inline;
#[path = "../benches/render.rs"]
mod gh68_render;

struct Gh68TempDir(std::path::PathBuf);
#[rustfmt::skip]
impl Gh68TempDir {
    fn new() -> Self {
        for nonce in 0..1000_u32 {
            let path = std::env::temp_dir().join(format!("rnk-gh68-{}-{nonce}", std::process::id()));
            match std::fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("cannot create GH68 temp directory: {error}"), }
        }
        panic!("cannot allocate a collision-free GH68 temp directory")
    }
    fn path(&self) -> &std::path::Path { &self.0 }
}
#[rustfmt::skip]
impl Drop for Gh68TempDir {
    fn drop(&mut self) { std::fs::remove_dir_all(&self.0).expect("GH68 temp directory cleanup must succeed"); }
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

fn conversation_fixture() -> ConversationState {
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
    let user_guard = mutation_guard(&state, MessageId::new(1));
    apply_update(
        &mut state,
        "user-complete",
        ConversationUpdate::complete(user_guard),
    );
    let guard = ConversationGuard::new(state.revision());
    apply_update(
        &mut state,
        "assistant",
        ConversationUpdate::push(guard, text_message(2, 2, ChatRole::Assistant, "")),
    );
    let append = ConversationUpdate::append_text(
        mutation_guard(&state, MessageId::new(2)),
        BlockId::new(2),
        "Use typed updates.",
    )
    .unwrap();
    apply_update(&mut state, "assistant-text", append);
    let complete = ConversationUpdate::complete(mutation_guard(&state, MessageId::new(2)));
    apply_update(&mut state, "complete", complete);
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

fn exact_invocation(name: &str) -> bool {
    invocation_has(&std::env::args().collect::<Vec<_>>(), name)
}

fn invocation_has(arguments: &[String], name: &str) -> bool {
    arguments.iter().any(|argument| argument == "--exact")
        && arguments.iter().any(|argument| argument == name)
}

#[cfg(unix)]
struct Gh68PtySession {
    terminal: rnk::renderer::Terminal,
    paste: Option<rnk::hooks::BracketedPasteGuard>,
    fullscreen: bool,
}
#[cfg(unix)]
#[rustfmt::skip]
impl Gh68PtySession {
    fn enter(fullscreen: bool) -> Self {
        let mut terminal = rnk::renderer::Terminal::new();
        if fullscreen { terminal.enter().unwrap(); } else { terminal.enter_inline().unwrap(); }
        terminal.enable_mouse().unwrap();
        let paste=Some(rnk::hooks::BracketedPasteGuard::new().unwrap());
        Self { terminal, paste, fullscreen }
    }
}
#[cfg(unix)]
#[rustfmt::skip]
impl Drop for Gh68PtySession {
    fn drop(&mut self) {
        self.paste.take();
        if self.fullscreen { self.terminal.exit().unwrap(); } else { self.terminal.exit_inline().unwrap(); }
    }
}
#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct Gh68TermiosSnapshot {
    input_flags: libc::tcflag_t,
    output_flags: libc::tcflag_t,
    control_flags: libc::tcflag_t,
    local_flags: libc::tcflag_t,
    control_chars: [libc::cc_t; libc::NCCS],
    input_speed: libc::speed_t,
    output_speed: libc::speed_t,
    #[cfg(any(target_os = "linux", target_os = "android"))]
    line_discipline: libc::cc_t,
}
#[cfg(unix)]
fn gh68_termios(fd: RawFd) -> Gh68TermiosSnapshot {
    let mut value = unsafe { std::mem::zeroed() };
    assert_eq!(unsafe { libc::tcgetattr(fd, &mut value) }, 0);
    Gh68TermiosSnapshot {
        input_flags: value.c_iflag,
        output_flags: value.c_oflag,
        control_flags: value.c_cflag,
        local_flags: value.c_lflag,
        control_chars: value.c_cc,
        input_speed: unsafe { libc::cfgetispeed(&value) },
        output_speed: unsafe { libc::cfgetospeed(&value) },
        #[cfg(any(target_os = "linux", target_os = "android"))]
        line_discipline: value.c_line,
    }
}
#[cfg(unix)]
#[rustfmt::skip]
fn gh68_pty_restoration(fullscreen: bool) {
    for outcome in ["normal", "cancel", "typed-error", "panic"] {
        let pty=native_pty_system().openpty(PtySize{rows:12,cols:40,pixel_width:0,pixel_height:0}).unwrap();
        let fd=pty.master.as_raw_fd().unwrap(); let before=gh68_termios(fd);
        let mut command=CommandBuilder::new(std::env::current_exe().unwrap()); command.args(["--exact","gh68_pty_child","--nocapture"]); command.env("GH68_PTY_CHILD","1"); command.env("GH68_PTY_FULLSCREEN",if fullscreen{"1"}else{"0"}); command.env("GH68_PTY_OUTCOME",outcome); command.env("NO_COLOR","1");
        let mut child=pty.slave.spawn_command(command).unwrap(); let mut reader=pty.master.try_clone_reader().unwrap(); let mut writer=pty.master.take_writer().unwrap(); drop(pty.slave);
        let mut output=String::new(); let mut held=None; let deadline=Instant::now()+Duration::from_secs(20); let mut buffer=[0;4096];
        while !output.contains("<<RESTORED>>") { assert!(Instant::now()<deadline,"PTY timeout: {output}"); let count=reader.read(&mut buffer).unwrap(); assert!(count>0,"PTY closed: {output}"); output.push_str(&String::from_utf8_lossy(&buffer[..count])); if held.is_none()&&output.contains("<<HELD>>") { held=Some(gh68_termios(fd)); writer.write_all(b"\n").unwrap(); writer.flush().unwrap(); } }
        assert!(child.wait().unwrap().success()); let held=held.unwrap(); let after=gh68_termios(fd); assert_eq!(before,after,"the complete termios state must be restored"); assert_eq!(held.local_flags&(libc::ECHO|libc::ICANON),0);
        let hide=output.find("\u{1b}[?25l").unwrap(); let show=output.rfind("\u{1b}[?25h").unwrap(); let mouse_on=output.find("\u{1b}[?1000h").unwrap(); let mouse_off=output.rfind("\u{1b}[?1000l").unwrap(); let paste_on=output.find("\u{1b}[?2004h").unwrap(); let paste_off=output.rfind("\u{1b}[?2004l").unwrap(); assert!(hide<show&&mouse_on<mouse_off&&paste_on<paste_off);
        assert_eq!(output.contains("\u{1b}[?1049h"),fullscreen); assert_eq!(output.contains("\u{1b}[?1049l"),fullscreen);
    }
}

#[test]
#[cfg(unix)]
#[rustfmt::skip]
fn gh68_pty_child() {
    if std::env::var_os("GH68_PTY_CHILD").is_none() { return; }
    let fullscreen=std::env::var("GH68_PTY_FULLSCREEN").unwrap()=="1"; let outcome=std::env::var("GH68_PTY_OUTCOME").unwrap();
    let result=std::panic::catch_unwind(|| -> Result<(),String> { let _session=Gh68PtySession::enter(fullscreen); print!("<<HELD>>\r\n"); std::io::stdout().flush().unwrap(); let mut byte=[0]; std::io::stdin().read_exact(&mut byte).unwrap();
        if fullscreen { let surface=gh68_fullscreen::ChatSurface::try_new(40,12)?; if outcome=="cancel" { let key=Key{escape:true,..Key::default()}; assert!(surface.try_key("",&key)?.1.is_none()); } else if outcome=="typed-error" { surface.try_resize(1,1)?; } }
        else { let mut shell=InlineChatShell::new(ScrollbackNamespace::new("gh68.pty").unwrap(),NativeTerminalSink::new(std::io::stdout())); if outcome=="cancel" { let key=Key{escape:true,..Key::default()}; assert_eq!(shell.handle_key(&rnk::components::chat::ChatComposerKeyMap::new(),"",&key),InlineKeyOutcome::Cancelled); } else if outcome=="typed-error" { shell.stream(MessageId::new(1)).map_err(|error|error.to_string())?; shell.stream(MessageId::new(1)).map_err(|error|error.to_string())?; } }
        if outcome=="panic" { panic!("controlled GH68 unwind"); } Ok(()) });
    match outcome.as_str() { "normal"|"cancel"=>assert!(result.unwrap().is_ok()), "typed-error"=>assert!(result.unwrap().is_err()), "panic"=>assert!(result.is_err()), _=>panic!("closed outcome") }
    print!("<<RESTORED>>\r\n"); std::io::stdout().flush().unwrap();
}

fn chat_flow() -> Element {
    conversation_view(&conversation_fixture())
}

fn gh68_composer_input() -> Element {
    let state = use_signal(ChatComposerState::new);
    let input_state = state.clone();
    use_input(move |input, key| {
        let mut candidate = input_state.get();
        rnk::components::chat::handle_key(
            &mut candidate,
            &rnk::components::chat::ChatComposerKeyMap::new(),
            input,
            key,
        );
        input_state.set(candidate);
    });
    let paste_state = state.clone();
    use_paste(move |event| {
        let mut candidate = paste_state.get();
        rnk::components::chat::handle_key(
            &mut candidate,
            &rnk::components::chat::ChatComposerKeyMap::new(),
            event.content(),
            &Key::default(),
        );
        paste_state.set(candidate);
    });
    Text::new(state.get().text().to_owned()).into_element()
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
#[rustfmt::skip]
fn gh68_harness_contract() {
    let first_temp=Gh68TempDir::new(); let second_temp=Gh68TempDir::new(); assert_ne!(first_temp.path(),second_temp.path());
    let offline = gh68_chat::gh68_offline_adapter_view().unwrap();
    let provider = gh68_glm::gh68_provider_adapter_view().unwrap();
    assert_eq!(offline, provider, "independent offline/provider DTO paths must converge semantically");
    assert_eq!(conversation_view(&conversation_fixture()).children.len(), 2);
    assert_eq!(exact_test_names("#[test]\nfn exact_name() {\n}\nfn helper() {}"), ["exact_name"]);
    assert_eq!(markdown_links("[quickstart](docs/CHAT_QUICKSTART.md)"), ["docs/CHAT_QUICKSTART.md"]);
    assert_eq!(public_example_names("- `chat` — tutorial\n- `rnk_chat` — fullscreen"), ["chat", "rnk_chat"]);
    assert_eq!(command_lines("note\ncargo check --example chat\nother"), ["cargo check --example chat"]);
    assert!(!invocation_has(&[],"selector")); assert!(!invocation_has(&["--exact".to_owned()],"selector")); assert!(invocation_has(&["--exact".to_owned(),"selector".to_owned()],"selector"));
}

#[test]
#[rustfmt::skip]
fn gh68_chat_tutorial_contract() {
    let source = include_str!("../examples/chat.rs");
    for required in ["ConversationState::new","ConversationUpdate::push","ConversationUpdate::complete","ChatMessageView::new","ComposerProjection::build","acknowledge_success","acknowledge_failure"] { assert!(source.contains(required),"missing public chat seam: {required}"); }
    for forbidden in ["Vec::<String>",".pop(","UnicodeSegmentation","UnicodeWidthStr","cursor_column()",".graphemes("] { assert!(!source.contains(forbidden),"tutorial retained private transcript/cursor logic: {forbidden}"); }
    let state = conversation_fixture();
    let rendered = rnk::render_to_string(&conversation_view(&state), 60);
    assert_eq!(rendered, gh68_chat::gh68_offline_adapter_view().unwrap()); assert_eq!([rendered.contains("Explain the release gate"),rendered.contains("Use typed updates.")],[true,true]);
    let mut tutorial=TestHarness::with_size(gh68_chat::app,80,24); tutorial.send_text("harness input"); tutorial.send_key(KeyCodeKind::Enter); tutorial.assert_text_contains("harness input");
    for source in [include_str!("../examples/rnk_chat.rs"),include_str!("../examples/claude_input_box.rs")] { assert!(source.contains("use_paste"),"interactive chat example must register a paste hook"); assert!(source.contains("BracketedPasteGuard::new()"),"interactive chat example must enable bracketed paste for its terminal session"); }
    assert!(include_str!("../src/renderer/runtime.rs").contains("Event::Paste(content)"));
    let complex = "old\r\n界👩‍👩‍👧‍👦e\u{301}";
    let mut harness = TestHarness::with_size(gh68_composer_input, 80, 4);
    harness.send_paste(complex);
    harness.assert_text_contains("old\r\n界👩‍👩‍👧‍👦e\u{301}");
    harness.send_key_with_modifiers(KeyCodeKind::Char('a'),KeyModifiers::CONTROL);
    harness.send_paste("界👩‍👩‍👧‍👦e\u{301}");
    assert!(!harness.render().contains("old"),"paste must replace the active selection");
    harness.send_key(KeyCodeKind::Left);
    harness.send_key(KeyCodeKind::Delete);
    harness.assert_text_contains("界👩‍👩‍👧‍👦");
    assert!(!harness.render().contains("e\u{301}"),"forward delete must remove one combining grapheme");
    harness.send_key(KeyCodeKind::Backspace);
    assert!(!harness.render().contains("👩‍👩‍👧‍👦"),"backspace must remove one ZWJ grapheme");
    let mut composer = ChatComposerState::new();
    rnk::components::chat::handle_key(&mut composer,&rnk::components::chat::ChatComposerKeyMap::new(),"界👩‍👩‍👧‍👦e\u{301}",&Key::default());
    let projection = rnk::components::chat::ComposerProjection::build(&composer, 8);
    assert!(projection.visible_slice().iter().all(|line|unicode_width::UnicodeWidthStr::width(line.as_str())<=8));
}

#[test]
#[rustfmt::skip]
fn gh68_fullscreen_example_contract() {
    #[cfg(unix)]
    gh68_pty_restoration(true);
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

    assert!(gh68_fullscreen::ChatSurface::try_new(24, 2).is_err());
    let mut live=TestHarness::with_size(gh68_fullscreen::app,40,12); assert_eq!(live.runtime_context().borrow().paste_handler_count(),1); live.send_key(KeyCodeKind::Function(1)); live.send_key(KeyCodeKind::Function(1)); live.send_key(KeyCodeKind::Tab); live.send_key(KeyCodeKind::PageUp); live.send_key(KeyCodeKind::PageDown); live.send_key(KeyCodeKind::Tab); live.send_paste("live paste"); live.send_key(KeyCodeKind::Enter); live.resize(31,10); live.send_key_with_modifiers(KeyCodeKind::Char('c'),KeyModifiers::CONTROL);
    let initial = gh68_fullscreen::ChatSurface::try_new(24, 8).unwrap();
    let initial_shell = initial.try_shell().unwrap();
    assert_eq!(
        initial_shell.layout().transcript().rows(),
        6,
        "the stored viewport must be shell-corrected, not stale at one row"
    );
    assert_eq!(
        initial_shell.transcript().viewport_rows(),
        ViewportRows::new(6)
    );
    let focused = initial
        .try_set_focus(rnk::components::chat::FullscreenFocus::Transcript)
        .unwrap();
    let (focused_after_key, submitted) = focused.try_key("ignored", &Key::default()).unwrap();
    assert!(submitted.is_none());
    assert_eq!(
        focused_after_key.try_shell().unwrap().focus(),
        rnk::components::chat::FullscreenFocus::Transcript
    );
    let overlay = focused_after_key.try_set_overlay(true).unwrap();
    assert!(overlay.try_key("blocked", &Key::default()).unwrap().0.try_shell().unwrap().overlay_open());
    assert!(
        overlay
            .try_set_focus(rnk::components::chat::FullscreenFocus::Composer)
            .is_err()
    );
    let mut surface = overlay
        .try_set_overlay(false)
        .unwrap()
        .try_set_focus(rnk::components::chat::FullscreenFocus::Composer)
        .unwrap();
    surface = surface.try_resize(18, 7).unwrap();
    let following = surface.try_shell().unwrap();
    assert_eq!(
        following.transcript().follow_state(),
        rnk::components::chat::message_list::BottomFollowState::Following
    );
    assert!(
        following.transcript().total_rows().unwrap()
            > following.transcript().viewport_rows().get(),
        "the resize scenario must begin with an overflowing transcript"
    );
    surface = surface.try_scroll(-2).unwrap();
    let paused = surface.try_shell().unwrap();
    assert!(matches!(paused.transcript().follow_state(),rnk::components::chat::message_list::BottomFollowState::Paused { .. }));
    let paused_anchor = paused.transcript().stored_anchor().expect("paused viewport has an anchor");
    let stable_ids = surface.message_ids();
    let mut expected_draft = String::new();
    for (width, height, input) in [
        (18, 7, "界"),
        (31, 10, "👩‍👩‍👧‍👦"),
        (24, 8, "e\u{301}"),
        (40, 12, "paste 多字"),
    ] {
        surface = surface.try_resize(width, height).unwrap();
        let resized = surface.try_shell().unwrap();
        assert_eq!(resized.layout().width(), width);
        assert_eq!(surface.message_ids(),stable_ids);
        assert!(matches!(resized.transcript().follow_state(),rnk::components::chat::message_list::BottomFollowState::Paused { .. }));
        assert_eq!(resized.transcript().stored_anchor(),Some(paused_anchor));
        surface = surface.try_paste(input).unwrap();
        expected_draft.push_str(input);
        let edited = surface.try_shell().unwrap();
        assert_eq!(edited.composer().text(),expected_draft);
        assert_eq!(surface.message_ids(),stable_ids);
        assert!(matches!(edited.transcript().follow_state(),rnk::components::chat::message_list::BottomFollowState::Paused { .. }));
        assert_eq!(edited.transcript().stored_anchor(),Some(paused_anchor));
    }
    let enter = Key {
        return_key: true,
        ..Key::default()
    };
    let (submitted_surface, prompt) = surface.try_key("", &enter).unwrap();
    let prompt = prompt.unwrap();
    assert_eq!([prompt.contains("界"),prompt.contains("👩‍👩‍👧‍👦"),prompt.contains("e\u{301}"),prompt.contains("paste 多字")],[true,true,true,true]);
    let submitted_shell=submitted_surface.try_shell().unwrap(); assert_eq!(submitted_shell.transcript().stored_anchor(),Some(paused_anchor)); assert!(matches!(submitted_shell.transcript().follow_state(),rnk::components::chat::message_list::BottomFollowState::Paused { .. }));
    let expected = submitted_surface.revision();
    let moved = submitted_surface.try_resize(32, 10).unwrap();
    let moved_shell=moved.try_shell().unwrap(); assert_eq!(moved_shell.transcript().stored_anchor(),Some(paused_anchor)); assert!(matches!(moved_shell.transcript().follow_state(),rnk::components::chat::message_list::BottomFollowState::Paused { .. }));
    assert!(
        moved
            .try_reply(expected, &prompt)
            .err()
            .unwrap()
            .contains("stale")
    );
    let replied = moved.try_reply(moved.revision(), &prompt).unwrap();
    let replied_shell=replied.try_shell().unwrap(); assert_eq!(replied_shell.transcript().stored_anchor(),Some(paused_anchor)); assert!(matches!(replied_shell.transcript().follow_state(),rnk::components::chat::message_list::BottomFollowState::Paused { .. }));
    assert_eq!(replied.message_ids(), [1, 2, 3, 4]);
    assert_eq!(replied.status(), "ready");
    let painted = rnk::render_to_string(&gh68_fullscreen::render_surface(&replied), 32);
    let plain = strip_sgr(&painted);
    let lines = plain.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 10);
    assert!(
        lines
            .iter()
            .all(|line| unicode_width::UnicodeWidthStr::width(*line) <= 32)
    );
    assert_eq!([painted.contains("rnk-chat · ready"),painted.contains("I received your message")],[true,true]);
}

#[derive(Debug, Default)]
struct Gh68BudgetedWriter {
    accepted: Vec<u8>,
    budget: Option<std::sync::Arc<std::sync::atomic::AtomicUsize>>,
}
#[rustfmt::skip]
impl Gh68BudgetedWriter {
    const fn unlimited() -> Self { Self { accepted: Vec::new(), budget: None } }
    fn accepting(budget: usize) -> Self { Self::shared(std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(budget))) }
    fn shared(budget: std::sync::Arc<std::sync::atomic::AtomicUsize>) -> Self { Self { accepted: Vec::new(), budget: Some(budget) } }
}
#[rustfmt::skip]
impl Write for Gh68BudgetedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(budget) = &self.budget else { self.accepted.extend_from_slice(bytes); return Ok(bytes.len()); };
        let room = budget.load(std::sync::atomic::Ordering::SeqCst);
        if room == 0 { return Err(io::Error::new(io::ErrorKind::BrokenPipe, "budget exhausted")); }
        let count = room.min(bytes.len()); self.accepted.extend_from_slice(&bytes[..count]); budget.fetch_sub(count,std::sync::atomic::Ordering::SeqCst); Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}
#[rustfmt::skip]
fn inline_report(writer: Gh68BudgetedWriter) -> (InlineCommitReport, InlineChatShell<NativeTerminalSink<Gh68BudgetedWriter>>, rnk::components::chat::SubmissionToken) {
    let mut shell = InlineChatShell::new(ScrollbackNamespace::new("gh68.inline").unwrap(), NativeTerminalSink::new(writer));
    assert!(matches!(shell.handle_key(&rnk::components::chat::ChatComposerKeyMap::new(), "draft", &Key::default()), InlineKeyOutcome::Changed(_)));
    let enter = Key { return_key: true, ..Key::default() };
    assert_eq!(shell.handle_key(&rnk::components::chat::ChatComposerKeyMap::new(), "", &enter), InlineKeyOutcome::Submitted("draft".to_owned()));
    let token = shell.composer().pending_submission().unwrap().token(); let id = MessageId::new(1); shell.stream(id).unwrap();
    let report = shell.finish(id, MessageRevision::INITIAL, "You: draft", ProjectionContext::new(40, ThemeIdentity::new(1)).unwrap()).unwrap();
    (report, shell, token)
}

#[test]
#[rustfmt::skip]
fn gh68_inline_example_contract() {
    #[cfg(unix)]
    gh68_pty_restoration(false);
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
        assert!(!source.contains(forbidden), "inline example retained a direct publication ledger: {forbidden}");
    }
    let mut live=TestHarness::with_size(gh68_inline::app,80,12); live.send_paste("inline paste"); live.assert_text_contains("inline paste"); live.send_key_with_modifiers(KeyCodeKind::Char('r'),KeyModifiers::CONTROL); live.send_key_with_modifiers(KeyCodeKind::Char('v'),KeyModifiers::CONTROL); live.send_key(KeyCodeKind::Left); live.send_key(KeyCodeKind::Delete); live.send_text(" publish"); live.send_key(KeyCodeKind::Enter); live.send_key(KeyCodeKind::Escape);

    let (fixed, mut fixed_shell, fixed_token) = inline_report(Gh68BudgetedWriter::unlimited());
    assert!(matches!(fixed, InlineCommitReport::Fixed { .. }));
    fixed_shell.composer_mut().acknowledge_success(fixed_token).unwrap();
    assert_eq!(fixed_shell.composer().text(), "");
    assert!(fixed_shell.live_messages().is_empty());

    let (retained, retained_shell, retained_token) = inline_report(Gh68BudgetedWriter::accepting(0));
    assert!(matches!(retained, InlineCommitReport::Retained { .. }));
    assert_eq!(retained_shell.composer().text(), "draft");
    assert_eq!(retained_shell.composer().pending_submission().unwrap().token(), retained_token);
    assert_eq!(retained_shell.live_state(MessageId::new(1)), Some(LiveState::AwaitingRetry));

    let (latched, mut latched_shell, latched_token) = inline_report(Gh68BudgetedWriter::accepting(3));
    assert!(matches!(latched, InlineCommitReport::Latched { .. }));
    assert_eq!(latched_shell.composer().text(), "draft");
    assert_eq!(latched_shell.live_state(MessageId::new(1)), Some(LiveState::AwaitingResolution));
    let pending=gh68_inline::PendingPublication::new(MessageId::new(1),"You: draft".to_owned(),40,latched_token);
    let before=latched_shell.sink().ledger().len(); assert!(matches!(gh68_inline::resolve_publication(&mut latched_shell,&pending,rnk::components::chat::UnknownResolution::AlreadyVisible).unwrap(),gh68_inline::HumanResolutionReport::AlreadyVisible)); assert_eq!(latched_shell.sink().ledger().len(),before); assert!(latched_shell.live_messages().is_empty());
    let budget=std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(3)); let (retry_latched,mut retry_shell,retry_token)=inline_report(Gh68BudgetedWriter::shared(budget.clone())); assert!(matches!(retry_latched,InlineCommitReport::Latched{..})); budget.store(usize::MAX,std::sync::atomic::Ordering::SeqCst);
    let retry=gh68_inline::PendingPublication::new(MessageId::new(1),"You: draft".to_owned(),40,retry_token); assert!(matches!(gh68_inline::resolve_publication(&mut retry_shell,&retry,rnk::components::chat::UnknownResolution::NotVisible).unwrap(),gh68_inline::HumanResolutionReport::Retried(InlineCommitReport::Fixed{..})));
}

#[test]
#[rustfmt::skip]
fn gh68_provider_example_contract() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let source = include_str!("../examples/glm_chat.rs");
    for required in ["ConversationState::new", "ConversationUpdate::push", "ChatMessageView::new",
        "InlineChatShell::new", "struct PendingToolRequest", "ToolDecision::Denied", "approve_exact",
        "execute_once", "openat", "O_NOFOLLOW", "MAX_TOOL_DEPTH", "MAX_TOOL_ENTRIES", "MAX_TOOL_BYTES"] {
        assert!(source.contains(required), "missing provider seam: {required}");
    }
    for forbidden in ["your_api_key_here", "using default key", "filter_map(|e| e.ok())",
        "unwrap_or((80, 24))", "mod prompt_box"] {
        assert!(!source.contains(forbidden), "unsafe provider residue: {forbidden}");
    }
    assert!(!std::path::Path::new("examples/glm_chat/prompt_box.rs").exists());
    assert_eq!(gh68_glm::gh68_provider_internal_contract().unwrap(),(1,1,10,13));
    let client_builds = AtomicUsize::new(0);
    let missing = gh68_glm::ProviderAdapter::from_optional_key(None, || {
        client_builds.fetch_add(1, Ordering::SeqCst); Ok(reqwest::Client::new())
    });
    assert!(missing.is_err()); assert_eq!(client_builds.load(Ordering::SeqCst), 0);
    let blank = gh68_glm::ProviderAdapter::from_optional_key(Some("  ".to_owned()), || {
        client_builds.fetch_add(1, Ordering::SeqCst); Ok(reqwest::Client::new())
    });
    assert!(blank.is_err()); assert_eq!(client_builds.load(Ordering::SeqCst), 0);
    let root = Gh68TempDir::new();
    std::fs::write(root.path().join("safe.txt"), "safe-content").unwrap();
    std::fs::create_dir(root.path().join("directory")).unwrap();
    std::fs::write(root.path().join("directory/nested.txt"), "nested-content").unwrap();
    let workspace = gh68_glm::Workspace::from_root(root.path()).unwrap();
    assert_eq!(gh68_chat::gh68_offline_adapter_view().unwrap(), gh68_glm::gh68_provider_adapter_view().unwrap());
    let request = |id: &str, name: &str, input: serde_json::Value| gh68_glm::PendingToolRequest::parse(id, name, &input).unwrap();
    let mut list_root=request("call-list-root","list_files",serde_json::json!({"path":"."})); list_root.approve_exact(&list_root.approval_phrase()).unwrap(); assert!(list_root.execute_once(&workspace).unwrap().contains("safe.txt"));
    let mut search_safe=request("call-search-safe","search_files",serde_json::json!({"pattern":"safe"})); search_safe.approve_exact(&search_safe.approval_phrase()).unwrap(); assert!(search_safe.execute_once(&workspace).unwrap().contains("safe.txt"));
    let mut read_directory=request("call-read-directory","read_file",serde_json::json!({"path":"directory"})); read_directory.approve_exact(&read_directory.approval_phrase()).unwrap(); assert!(read_directory.execute_once(&workspace).is_err());
    let mut read_root=request("call-read-root","read_file",serde_json::json!({"path":"."})); read_root.approve_exact(&read_root.approval_phrase()).unwrap(); assert!(read_root.execute_once(&workspace).is_err());
    let mut read_nested=request("call-read-nested","read_file",serde_json::json!({"path":"directory/nested.txt"})); read_nested.approve_exact(&read_nested.approval_phrase()).unwrap(); assert_eq!(read_nested.execute_once(&workspace).unwrap(),"nested-content");
    let mut list_file=request("call-list-file","list_files",serde_json::json!({"path":"safe.txt"})); list_file.approve_exact(&list_file.approval_phrase()).unwrap(); assert!(list_file.execute_once(&workspace).is_err());
    assert!(workspace.search_files("").is_err()); assert!(workspace.search_files(&"x".repeat(129)).is_err());
    assert!(gh68_glm::PendingToolRequest::parse("call-wrong-field","read_file",&serde_json::json!({"wrong":"safe.txt"})).is_err());
    assert!(gh68_glm::PendingToolRequest::parse("call-long-search","search_files",&serde_json::json!({"pattern":"x".repeat(129)})).is_err());
    let mut denied = request("call-1", "read_file", serde_json::json!({"path":"safe.txt"}));
    assert_eq!(denied.execute_once(&workspace).unwrap_err().to_string(), "tool request is denied by default");
    assert!(denied.approve_exact("approve wrong").is_err());
    assert!(denied.approval_phrase().bytes().all(|byte| byte.is_ascii_graphic() || byte == b' '));
    assert!(denied.approval_phrase().contains("tool=read_file"));
    denied.approve_exact(&denied.approval_phrase()).unwrap();
    assert_eq!(denied.execute_once(&workspace).unwrap(), "safe-content");
    assert_eq!(denied.execute_once(&workspace).unwrap_err().to_string(), "tool request has already executed");
    assert!(gh68_glm::PendingToolRequest::parse(
        "call-2", "read_file", &serde_json::json!({"path":"../escape"})
    ).is_err(), "traversal must be rejected before UI or approval");
    std::fs::write(root.path().join("large.txt"), vec![b'x'; 65 * 1024]).unwrap();
    let mut large = request("call-3", "read_file", serde_json::json!({"path":"large.txt"}));
    large.approve_exact(&large.approval_phrase()).unwrap(); assert!(large.execute_once(&workspace).is_err());
    std::fs::create_dir(root.path().join("many")).unwrap();
    for index in 0..21 { std::fs::write(root.path().join(format!("many/{index}")), "x").unwrap(); }
    let mut many = request("call-many", "list_files", serde_json::json!({"path":"many"}));
    many.approve_exact(&many.approval_phrase()).unwrap(); assert!(many.execute_once(&workspace).is_err());
    std::fs::create_dir(root.path().join("fanout")).unwrap();
    for index in 0..21 { std::fs::write(root.path().join(format!("fanout/unmatched-{index}")), "x").unwrap(); }
    let mut fanout = request("call-fanout", "search_files", serde_json::json!({"pattern":"never-matches"}));
    fanout.approve_exact(&fanout.approval_phrase()).unwrap(); assert!(fanout.execute_once(&workspace).is_err());
    #[cfg(unix)]
    {
        let symlink_root=Gh68TempDir::new(); std::fs::write(symlink_root.path().join("target"),"x").unwrap(); std::os::unix::fs::symlink(symlink_root.path().join("target"),symlink_root.path().join("link")).unwrap(); let symlink_workspace=gh68_glm::Workspace::from_root(symlink_root.path()).unwrap();
        let mut list_link=request("call-list-link","list_files",serde_json::json!({"path":"."})); list_link.approve_exact(&list_link.approval_phrase()).unwrap(); assert!(list_link.execute_once(&symlink_workspace).is_err());
        let mut search_link=request("call-search-link","search_files",serde_json::json!({"pattern":"absent"})); search_link.approve_exact(&search_link.approval_phrase()).unwrap(); assert!(search_link.execute_once(&symlink_workspace).is_err());
        std::os::unix::fs::symlink(root.path().join("safe.txt"), root.path().join("link")).unwrap();
        let mut link = request("call-4", "read_file", serde_json::json!({"path":"link"}));
        link.approve_exact(&link.approval_phrase()).unwrap(); assert!(link.execute_once(&workspace).is_err());
        let sandbox = root.path().join("sandbox"); let held = root.path().join("sandbox-held"); let outside = root.path().join("outside");
        std::fs::create_dir(&sandbox).unwrap(); std::fs::create_dir(&outside).unwrap(); std::fs::write(sandbox.join("inside"), "inside").unwrap(); std::fs::write(outside.join("inside"), "outside-secret").unwrap();
        let anchored = gh68_glm::Workspace::from_root(&sandbox).unwrap(); std::fs::rename(&sandbox, &held).unwrap(); std::os::unix::fs::symlink(&outside, &sandbox).unwrap();
        let mut raced = request("call-race", "read_file", serde_json::json!({"path":"inside"})); raced.approve_exact(&raced.approval_phrase()).unwrap(); assert_eq!(raced.execute_once(&anchored).unwrap(), "inside");
        std::fs::remove_file(&sandbox).unwrap(); std::fs::rename(&held, &sandbox).unwrap();
    }
    let deep = root.path().join("d0/d1/d2/d3/d4");
    std::fs::create_dir_all(&deep).unwrap(); std::fs::write(deep.join("needle.txt"), "x").unwrap();
    let mut search = request("call-5", "search_files", serde_json::json!({"pattern":"needle"}));
    search.approve_exact(&search.approval_phrase()).unwrap(); assert!(search.execute_once(&workspace).is_err());
    for (id,name,input) in [("bad\u{1b}]0;x\u{7}","read_file",serde_json::json!({"path":"safe.txt"})), ("ok-id","read\u{9b}file",serde_json::json!({"path":"safe.txt"})), ("ok-id","read_file",serde_json::json!({"path":"safe\u{1b}]8;;x\u{7}.txt"})), ("ok-id","read_file",serde_json::json!({"path":"safe.txt","extra":"x"}))] { let error=gh68_glm::PendingToolRequest::parse(id,name,&input).err().unwrap().to_string(); assert!(error.len()<96); assert!(!error.chars().any(char::is_control)); }
    assert!(gh68_glm::PendingToolRequest::parse(&"a".repeat(65),"read_file",&serde_json::json!({"path":"safe.txt"})).is_err());
}

#[rustfmt::skip]
fn strip_sgr(input: &str) -> String {
    let bytes = input.as_bytes(); let mut output = Vec::with_capacity(bytes.len()); let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != 0x1b { output.push(bytes[index]); index += 1; continue; }
        assert_eq!(bytes.get(index + 1), Some(&b'['), "golden contains a non-SGR escape"); index += 2;
        while index < bytes.len() && !(0x40..=0x7e).contains(&bytes[index]) { index += 1; }
        assert!(index < bytes.len(), "golden contains an unterminated SGR escape"); index += 1; }
    String::from_utf8(output).expect("removing ASCII escapes preserves UTF-8")
}
#[test]
#[rustfmt::skip]
fn gh68_example_convergence_contract() {
    let offline = gh68_chat::gh68_offline_adapter_view().unwrap(); let provider = gh68_glm::gh68_provider_adapter_view().unwrap(); assert_eq!(offline,provider);
    let plain = include_str!("golden/real_app_chat.txt"); let ansi = include_str!("golden/real_app_chat.ansi.txt");
    assert_eq!(format!("{}\n", strip_sgr(&rnk::render_to_string(&conversation_view(&conversation_fixture()), 80))), plain);
    assert_eq!(strip_sgr(ansi), plain, "ANSI normalization may remove styling only");
    for (source, seam) in [(include_str!("../examples/chat.rs"), "ConversationState"), (include_str!("../examples/rnk_chat.rs"), "FullscreenChatShell"), (include_str!("../examples/claude_input_box.rs"), "InlineChatShell"), (include_str!("../examples/glm_chat.rs"), "ChatMessageView")] {
        assert!(source.contains(seam), "example is missing its shared seam: {seam}");
    }
}
#[test]
#[rustfmt::skip]
fn gh68_example_index_contract() {
    let index = include_str!("../examples/README.md");
    let ledger = index.split_once("### Chat example review").unwrap().0;
    let names = public_example_names(ledger);
    for expected in ["chat.rs", "rnk_chat.rs", "claude_input_box.rs", "glm_chat.rs"] {
        assert_eq!(names.iter().filter(|name| **name == expected).count(), 1, "{expected} must have one classification");
    }
    assert!(!names.contains(&"glm_chat/prompt_box.rs"));
    assert!(!std::path::Path::new("examples/glm_chat/prompt_box.rs").exists());
    let line=index.split_once("gh68-public-chat-examples-v1").unwrap().1.lines().find(|line|line.starts_with('{')).unwrap(); let manifest:serde_json::Value=serde_json::from_str(line).unwrap(); let mut keys=manifest.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(); keys.sort_unstable(); assert_eq!(keys,["records","schema"]); assert_eq!(manifest["schema"],"gh68-public-chat-examples-v1"); let records=manifest["records"].as_array().unwrap(); assert_eq!(records.len(),4); let categories=["tutorial","showcase","debug","internal"]; let modes=["offline_inline","interactive_fullscreen","interactive_inline","provider_inline"]; let readers=["new_chat_user","fullscreen_app_author","inline_app_author","provider_integrator"]; for record in records { let mut keys=record.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(); keys.sort_unstable(); assert_eq!(keys,["category","example","purpose","runtime_mode","target_reader"]); assert!(categories.contains(&record["category"].as_str().unwrap())); assert!(modes.contains(&record["runtime_mode"].as_str().unwrap())); assert!(readers.contains(&record["target_reader"].as_str().unwrap())); assert!(!record["purpose"].as_str().unwrap().trim().is_empty()); }
}
#[test]
#[rustfmt::skip]
fn gh68_message_compatibility_contract() {
    assert_ne!(std::any::TypeId::of::<Message>(), std::any::TypeId::of::<ChatMessage>());
    let legacy = rnk::render_to_string(&Message::user("legacy notification").into_element(), 40);
    let typed = rnk::render_to_string(&ChatMessageView::new(&text_message(9, 9, ChatRole::User, "typed conversation")).into_element(), 40);
    assert!(legacy.contains("legacy notification")); assert!(typed.contains("typed conversation"));
}
#[rustfmt::skip]
fn compile_gh68_quickstarts(blocks:&[&str]) {
    let root=Gh68TempDir::new(); let repo=std::env::current_dir().unwrap().canonicalize().unwrap(); let target=root.path().join("target");
    for (index,block) in blocks.iter().enumerate() { let crate_root=root.path().join(format!("quickstart-{index}")); std::fs::create_dir_all(crate_root.join("src")).unwrap(); std::fs::write(crate_root.join("Cargo.toml"),format!("[package]\nname='gh68-quickstart-{index}'\nversion='0.0.0'\nedition='2024'\n[dependencies]\nrnk={{path={}}}\n",serde_json::to_string(&repo).unwrap())).unwrap(); std::fs::write(crate_root.join("src/main.rs"),block).unwrap(); let status=std::process::Command::new("cargo").args(["check","--offline","--quiet"]).current_dir(crate_root).env("CARGO_TARGET_DIR",&target).status().unwrap(); assert!(status.success(),"quickstart block {index} must compile"); }
}
#[rustfmt::skip]
fn validate_gh68_terminal_evidence(matrix:&str) {
    let line=matrix.split_once("gh68-terminal-matrix-v1").unwrap().1.lines().find(|line|line.starts_with('{')).unwrap(); let manifest:serde_json::Value=serde_json::from_str(line).unwrap(); let mut keys=manifest.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(); keys.sort_unstable(); assert_eq!(keys,["cells","schema"]); assert_eq!(manifest["schema"],"gh68-terminal-matrix-v1"); let cells=manifest["cells"].as_array().unwrap(); let required=["os","terminal_emulator","inline","fullscreen","paste","resize","raw_restoration","tmux","ssh"]; assert_eq!(cells.len(),required.len()); let mut ids=cells.iter().map(|cell|cell["id"].as_str().unwrap()).collect::<Vec<_>>(); ids.sort_unstable(); let mut expected=required; expected.sort_unstable(); assert_eq!(ids,expected); let vocabulary=["verified","best_effort","terminal_dependent","unsupported","unverified"]; for cell in cells { let mut keys=cell.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(); keys.sort_unstable(); assert_eq!(keys,["evidence","id","status"]); let status=cell["status"].as_str().unwrap(); let evidence=cell["evidence"].as_str().unwrap(); assert!(vocabulary.contains(&status)); assert!(!evidence.is_empty()); if status=="verified" { assert_ne!(evidence,"none"); } else if status=="unverified" { assert_eq!(evidence,"none"); } }
    let Some(path)=std::env::var_os("GH68_TERMINAL_EVIDENCE") else { assert!(!exact_invocation("gh68_compatibility_matrix_contract"),"exact terminal evidence path is required"); return; }; let path=std::path::PathBuf::from(path); assert!(path.is_absolute()); let evidence:serde_json::Value=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap(); let mut evidence_keys=evidence.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(); evidence_keys.sort_unstable(); assert_eq!(evidence_keys,["cells","head_sha","runner","schema"]); assert_eq!(evidence["schema"],"gh68-terminal-evidence-v1"); assert_eq!(evidence["head_sha"],std::env::var("GH68_IMPLEMENTATION_HEAD").unwrap()); assert_eq!(evidence["cells"],manifest["cells"]); let runner=&evidence["runner"]; let mut runner_keys=runner.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(); runner_keys.sort_unstable(); assert_eq!(runner_keys,["arch","os","rustc_vv"]); let rustc=std::process::Command::new("rustc").arg("-Vv").output().unwrap(); assert!(rustc.status.success()); assert_eq!(runner["rustc_vv"],String::from_utf8(rustc.stdout).unwrap().trim()); assert!(!runner["os"].as_str().unwrap().is_empty()&&!runner["arch"].as_str().unwrap().is_empty());
}
#[test]
#[rustfmt::skip]
fn gh68_public_docs_contract() {
    let quickstart = include_str!("../docs/CHAT_QUICKSTART.md"); let stability = include_str!("../docs/API_STABILITY.md");
    for required in ["## Inline quickstart", "## Fullscreen quickstart", "## Updating a conversation", "## Custom block renderers", "## Keymaps", "## Error handling", "## Non-goals", "```rust\n"] { assert!(quickstart.contains(required), "missing docs contract: {required}"); }
    let blocks=quickstart.split("```rust,compile\n").skip(1).map(|tail|tail.split("\n```").next().unwrap()).collect::<Vec<_>>(); assert_eq!(blocks.len(),2);
    compile_gh68_quickstarts(&blocks);
    for required in ["ConversationState", "ChatMessageView", "InlineChatShell", "FullscreenChatShell", "### `Message` compatibility", "provider-independent"] { assert!(stability.contains(required), "missing maturity contract: {required}"); }
    assert!(gh68_chat::gh68_offline_adapter_view().unwrap().contains("Use typed updates."));
}
#[test]
#[rustfmt::skip]
fn gh68_compatibility_matrix_contract() {
    let matrix = include_str!("../docs/TERMINAL_COMPATIBILITY.md");
    for required in ["## Chat Evidence Matrix", "Evidence kind", "exact checked-out `GITHUB_SHA`", "`unverified`", "no network or secret"] { assert!(matrix.contains(required)); }
    for overclaim in ["all terminals verified", "all platforms verified", "terminal-certified"] { assert!(!matrix.contains(overclaim)); }
    validate_gh68_terminal_evidence(matrix);
    assert_eq!(strip_sgr(include_str!("golden/real_app_chat.ansi.txt")), include_str!("golden/real_app_chat.txt"));
}
#[test]
#[rustfmt::skip]
fn gh68_stress_correctness_contract() {
    let first=gh68_render::gh68_workload_oracles(); let replay=gh68_render::gh68_workload_oracles(); assert_eq!(first,replay);
    assert_eq!(first.iter().map(|item|item.name).collect::<Vec<_>>(), ["gh68_long_conversation","gh68_high_frequency_streaming","gh68_variable_height_prepend","gh68_continuous_resize","gh68_inline_commit_churn"]);
    assert_eq!(first[0].message_order, (1..=128).collect::<Vec<_>>()); assert_eq!(first[1].message_order,[1,2]);
    assert_eq!(first[1].active_stream_steps,256); assert_eq!(first[1].expansion_transitions,0); assert!(first[1].bottom_follow);
    assert_eq!(first[2].message_order.first(),Some(&1)); assert_eq!(first[2].message_order.last(),Some(&131)); assert!(first[2].anchor.is_some()); assert!(!first[2].bottom_follow);
    assert_eq!(first[2].expansion_transitions,2); assert_eq!(first[2].active_stream_steps,0);
    assert!(first[3].semantic_checksum!=0); assert_eq!(first[4].commit_count,64); assert_eq!(first[4].message_order,(1..=64).collect::<Vec<_>>());
}
#[test]
#[rustfmt::skip]
fn gh68_benchmark_metadata_contract() {
    assert!(gh68_render::gh68_sha256_contract());
    gh68_render::gh68_benchmark_internal_contract().unwrap();
    match gh68_render::gh68_benchmark_route_contract("metadata") { Ok("performance_status=validation_required")=>gh68_render::gh68_benchmark_metadata_contract().unwrap(), Ok("performance_status=not_available")=>{ assert!(std::env::var_os("GH68_BENCHMARK_MODE").is_none()); let base="b".repeat(40); let head="a".repeat(40); assert_eq!(gh68_render::gh68_validate_baseline_head(&base,&head,&base),Ok(())); assert_eq!(gh68_render::gh68_validate_baseline_head(&"c".repeat(40),&head,&base),Err(gh68_render::Gh68EvidenceError("baseline identity"))); assert!(!gh68_render::gh68_is_regression(1_100_000,1_000_000,1)); assert!(gh68_render::gh68_is_regression(3_000_000,1_000_000,1)); }, Ok(other)=>panic!("unexpected benchmark status {other}"), Err(error)=>{ assert!(!exact_invocation("gh68_benchmark_metadata_contract"),"exact benchmark route is required: {error:?}"); } }
}
#[test]
#[rustfmt::skip]
fn gh68_benchmark_comparison_contract() {
    match gh68_render::gh68_benchmark_route_contract("comparison") { Ok("performance_status=validation_required")=>gh68_render::gh68_benchmark_comparison_contract().unwrap(), Ok("performance_status=not_available")=>{ assert!(std::env::var_os("GH68_BENCHMARK_EVIDENCE").is_none()); assert_eq!(gh68_render::gh68_benchmark_comparison_contract(),Err(gh68_render::Gh68EvidenceError("mode is not validate"))); assert_eq!(gh68_render::gh68_require_no_regressions([false,false]),Ok(())); assert_eq!(gh68_render::gh68_require_no_regressions([false,true]),Err(gh68_render::Gh68EvidenceError("performance regression"))); }, Ok(other)=>panic!("unexpected benchmark status {other}"), Err(error)=>{ assert!(!exact_invocation("gh68_benchmark_comparison_contract"),"exact benchmark route is required: {error:?}"); } }
}

#[test]
#[rustfmt::skip]
fn gh68_current_head_coverage_contract() { validate_gh68_coverage_evidence(); }
#[rustfmt::skip]
fn validate_gh68_coverage_evidence() {
    let tasks=include_str!("../specs/GH68/tasks.md"); let source=include_str!("golden_real_apps.rs");
    let line=tasks.split("gh68-critical-paths-v1").nth(1).unwrap().lines().find(|line|line.starts_with('{')).unwrap(); let manifest:serde_json::Value=serde_json::from_str(line).unwrap(); let paths=manifest["critical_paths"].as_array().unwrap(); assert_eq!(paths.len(),15);
    let mut names=paths.iter().map(|item|{assert_eq!(item["file"],"tests/golden_real_apps.rs");item["name"].as_str().unwrap()}).collect::<Vec<_>>(); let before=names.len(); names.sort_unstable(); names.dedup(); assert_eq!(names.len(),before);
    for name in &names { assert_eq!(source.matches(&format!("fn {name}()" )).count(),1,"critical selector missing/duplicate: {name}"); }
    let Some(path)=std::env::var_os("GH68_COVERAGE_EVIDENCE") else { assert!(!exact_invocation("gh68_current_head_coverage_contract"),"exact coverage evidence path is required"); return; };
    let path=std::path::PathBuf::from(path); assert!(path.is_absolute()); let bytes=std::fs::read(path).unwrap(); let evidence:serde_json::Value=serde_json::from_slice(&bytes).unwrap(); let object=evidence.as_object().unwrap();
    let mut keys=object.keys().map(String::as_str).collect::<Vec<_>>(); keys.sort_unstable(); assert_eq!(keys,["base_main_sha","branch_collection","changed_executable","critical","head_sha","raw_sha256","schema","toolchain"]);
    assert_eq!(evidence["schema"],"gh68-coverage-v1"); assert_eq!(evidence["head_sha"],std::env::var("GH68_IMPLEMENTATION_HEAD").unwrap()); assert_eq!(evidence["base_main_sha"],std::env::var("GH68_BASE_MAIN_SHA").unwrap()); assert_eq!(evidence["toolchain"],"nightly-2026-01-18"); assert_eq!(evidence["branch_collection"],true);
    let raw_path=std::path::PathBuf::from(std::env::var_os("GH68_COVERAGE_RAW").expect("raw coverage path is required")); assert!(raw_path.is_absolute()); let raw=std::fs::read(raw_path).unwrap(); assert_eq!(evidence["raw_sha256"],gh68_render::gh68_sha256(&raw));
    let changed=&evidence["changed_executable"]; let mut changed_keys=changed.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(); changed_keys.sort_unstable(); assert_eq!(changed_keys,["branch_covered","branch_percent","branch_total","files","line_covered","line_percent","line_total"]); let files=changed["files"].as_array().unwrap(); assert!(!files.is_empty()); let mut paths=files.iter().map(|path|path.as_str().unwrap()).collect::<Vec<_>>(); let before=paths.len(); paths.sort_unstable(); paths.dedup(); assert_eq!(paths.len(),before); assert!(paths.contains(&"tests/golden_real_apps.rs"));
    for kind in ["line","branch"] { let covered=changed[format!("{kind}_covered")].as_u64().unwrap(); let total=changed[format!("{kind}_total")].as_u64().unwrap(); let percent=changed[format!("{kind}_percent")].as_f64().unwrap(); assert!(total>0&&covered<=total&&percent>=80.0); assert!((percent-100.0*covered as f64/total as f64).abs()<0.000_001); }
    let critical=evidence["critical"].as_array().unwrap(); assert_eq!(critical.len(),15); let mut reported=Vec::new(); for item in critical { let mut item_keys=item.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(); item_keys.sort_unstable(); assert_eq!(item_keys,["branch_covered","branch_status","branch_total","file","line_covered","line_total","name"]); reported.push(item["name"].as_str().unwrap()); assert_eq!(item["file"],"tests/golden_real_apps.rs"); let lt=item["line_total"].as_u64().unwrap(); let bt=item["branch_total"].as_u64().unwrap(); assert!(lt>0); assert_eq!(item["line_covered"].as_u64(),Some(lt)); if bt==0 { assert_eq!(item["branch_status"],"not_applicable"); assert_eq!(item["branch_covered"],0); } else { assert_eq!(item["branch_status"],"covered"); assert_eq!(item["branch_covered"].as_u64(),Some(bt)); } } reported.sort_unstable(); assert_eq!(reported,names);
}
#[test]
#[rustfmt::skip]
fn gh68_ci_public_examples_contract() {
    let index=include_str!("../examples/README.md"); let mut indexed=public_example_names(index).into_iter().filter(|name|name.ends_with(".rs")).collect::<Vec<_>>(); let count=indexed.len(); indexed.sort_unstable(); indexed.dedup(); assert_eq!(indexed.len(),count); let mut actual=std::fs::read_dir("examples").unwrap().map(|entry|entry.unwrap().path()).filter(|path|path.extension().is_some_and(|ext|ext=="rs")).map(|path|path.file_name().unwrap().to_str().unwrap().to_owned()).collect::<Vec<_>>(); actual.sort_unstable(); assert_eq!(indexed,actual);
    let workflow=include_str!("../.github/workflows/ci.yml"); for required in ["gh68:","github.event.pull_request.head.sha || github.sha","persist-credentials: false","nightly-2026-01-18","cargo +nightly-2026-01-18 llvm-cov","execution_count - false_execution_count","region_line_counts","nested-region precedence self-check","GH68_COVERAGE_RAW","GH68_TERMINAL_EVIDENCE","GH68_BENCHMARK_ROUTE: smoke_blocked_no_baseline","actions/upload-artifact@v4","GH68_ARTIFACT_DIGEST","      - gh68"] { assert!(workflow.contains(required),"missing GH68 CI contract: {required}"); } assert!(!workflow.contains("validate_gh68_coverage_evidence\" not in function")); let job=workflow.split("  gh68:").nth(1).unwrap().split("  ci-gate:").next().unwrap(); assert!(!job.contains("continue-on-error")); let ordered=["Produce pinned-nightly current-head branch coverage","Derive closed GH68 coverage summary","Run all fifteen exact GH68 selectors","Run complete GH68 target","Run deterministic GH68 benchmark smoke","Bind evidence digests","Upload validated GH68 evidence","Verify immutable artifact receipt"]; let positions=ordered.map(|name|job.find(name).unwrap()); assert!(positions.windows(2).all(|pair|pair[0]<pair[1]));
    let normalize=|value:&str|{let digest=value.strip_prefix("sha256:").unwrap_or(value); (digest.len()==64&&digest.bytes().all(|byte|byte.is_ascii_digit()||(b'a'..=b'f').contains(&byte))).then(||digest.to_owned())}; let bare="ab".repeat(32); assert_eq!(normalize(&bare),Some(bare.clone())); assert_eq!(normalize(&format!("sha256:{bare}")),Some(bare)); assert_eq!(normalize("sha256:xyz"),None);
    let branches=[(10,5_u64,2_u64),(20,0,4)]; let selected=[10]; let (covered,total)=branches.into_iter().filter(|(line,_,_)|selected.contains(line)).fold((0,0),|(covered,total),(_,reported_true_count,false_execution_count)|{let execution_count=reported_true_count+false_execution_count;(covered+u64::from(execution_count-false_execution_count>0)+u64::from(false_execution_count>0),total+2)}); assert_eq!((covered,total),(2,2));
}
