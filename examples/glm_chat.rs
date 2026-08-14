//! GLM CLI Chat Demo with Tool Use - Using rnk UI
//!
//! Run with: GLM_API_KEY=your_key cargo run --example glm_chat

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::watch;

use rnk::prelude::{Color, Element, FlexDirection, Text};

// Alias rnk's Box to avoid conflict with std::boxed::Box
use rnk::prelude::Box as RnkBox;

#[path = "glm_chat/prompt_box.rs"]
mod prompt_box;
use prompt_box::{clear_live_prompt_box, draw_prompt_box, redraw_prompt_box};
use rnk::components::InteractionOutcome;
use rnk::components::chat::{ChatComposerKeyMap, ChatComposerState, handle_key};
use rnk::hooks::Key;

const API_URL: &str = "https://open.bigmodel.cn/api/anthropic/v1/messages";
const ALLOW_TOOLS_ENV: &str = "RNK_GLM_CHAT_ALLOW_TOOLS";
const TOOL_ROOT_ENV: &str = "RNK_GLM_CHAT_TOOL_ROOT";
const MAX_TOOL_ROUNDS: usize = 8;

#[derive(Debug, Default)]
struct ToolRoundBudget {
    completed: usize,
}

impl ToolRoundBudget {
    const fn permits_execution(&self) -> bool {
        self.completed < MAX_TOOL_ROUNDS
    }

    fn record_completed_round(&mut self) {
        self.completed = self.completed.saturating_add(1);
    }
}

#[derive(Serialize, Clone)]
struct ChatRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<MessageParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct MessageParam {
    role: String,
    content: MessageContent,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Serialize, Clone)]
struct Tool {
    name: String,
    description: String,
    input_schema: Value,
}

#[derive(Deserialize, Debug)]
struct ChatResponse {
    content: Vec<ResponseBlock>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum ResponseBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
}

fn get_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "read_file".to_string(),
            description: "Read file content at specified path".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path"
                    }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "list_files".to_string(),
            description: "List files and folders in specified directory".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path"
                    }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "search_files".to_string(),
            description: "Search for matching filenames in current directory".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Search pattern"
                    }
                },
                "required": ["pattern"]
            }),
        },
    ]
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ToolAuthorization {
    Disabled,
    Prompt { root: PathBuf },
}

impl ToolAuthorization {
    fn from_env() -> io::Result<Self> {
        match env::var(ALLOW_TOOLS_ENV) {
            Ok(value) if value == "1" || value.eq_ignore_ascii_case("true") => {
                let configured = env::var_os(TOOL_ROOT_ENV)
                    .map(PathBuf::from)
                    .unwrap_or(env::current_dir()?);
                let root = configured.canonicalize().map_err(|error| {
                    io::Error::new(
                        error.kind(),
                        format!("cannot resolve tool root {}: {error}", configured.display()),
                    )
                })?;
                Ok(Self::Prompt { root })
            }
            _ => Ok(Self::Disabled),
        }
    }

    fn advertised_tools(&self) -> Vec<Tool> {
        match self {
            Self::Disabled => Vec::new(),
            Self::Prompt { .. } => get_tools(),
        }
    }

    fn review_and_execute(&self, name: &str, input: &Value) -> ToolDecision {
        self.review_and_execute_with(name, input, |root| {
            print!(
                "Approve this one tool call inside {}? [y/N]: ",
                root.display()
            );
            if io::stdout().flush().is_err() {
                return false;
            }
            let mut answer = String::new();
            io::stdin()
                .read_line(&mut answer)
                .is_ok_and(|_| answer.trim().eq_ignore_ascii_case("y"))
        })
    }

    fn review_and_execute_with(
        &self,
        name: &str,
        input: &Value,
        confirm: impl FnOnce(&Path) -> bool,
    ) -> ToolDecision {
        let Self::Prompt { root } = self else {
            return ToolDecision::Denied(format!(
                "Not executed. Restart with {ALLOW_TOOLS_ENV}=1 to enable per-request approval."
            ));
        };

        if !confirm(root) {
            return ToolDecision::Denied("Not executed: denied by the user.".to_string());
        }

        match execute_tool(root, name, input) {
            Ok(result) => ToolDecision::Executed(result),
            Err(error) => ToolDecision::Denied(format!("Not executed: {error}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ToolDecision {
    Executed(String),
    Denied(String),
}

impl ToolDecision {
    fn result(&self) -> &str {
        match self {
            Self::Executed(result) | Self::Denied(result) => result,
        }
    }

    const fn was_denied(&self) -> bool {
        matches!(self, Self::Denied(_))
    }
}

fn execute_tool(root: &Path, name: &str, input: &Value) -> Result<String, String> {
    match name {
        "read_file" => {
            let path = confined_path(root, input)?;
            match fs::read_to_string(&path) {
                Ok(content) => {
                    let lines: Vec<&str> = content.lines().take(100).collect();
                    Ok(format!(
                        "Read {} lines from {}",
                        lines.len(),
                        path.display()
                    ))
                }
                Err(error) => Err(format!("cannot read {}: {error}", path.display())),
            }
        }
        "list_files" => {
            let path = confined_path(root, input)?;
            match fs::read_dir(&path) {
                Ok(entries) => {
                    let files: Vec<String> = entries
                        .filter_map(|e| e.ok())
                        .take(20)
                        .map(|e| {
                            let name = e.file_name().to_string_lossy().to_string();
                            if e.path().is_dir() {
                                format!("{}/", name)
                            } else {
                                name
                            }
                        })
                        .collect();
                    Ok(files.join(", "))
                }
                Err(error) => Err(format!("cannot list {}: {error}", path.display())),
            }
        }
        "search_files" => {
            let pattern = input
                .get("pattern")
                .and_then(Value::as_str)
                .filter(|pattern| !pattern.is_empty())
                .ok_or_else(|| "search_files requires a non-empty string pattern".to_string())?;
            let mut results = Vec::new();
            search_recursive(root, pattern, &mut results, 0, 3);
            if results.is_empty() {
                Ok("No files found".to_string())
            } else {
                Ok(format!("Found {} files", results.len()))
            }
        }
        _ => Err(format!("unknown tool: {name}")),
    }
}

fn confined_path(root: &Path, input: &Value) -> Result<PathBuf, String> {
    let raw = input
        .get("path")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| "tool requires a non-empty string path".to_string())?;
    let requested = Path::new(raw);
    let candidate = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        root.join(requested)
    };
    let canonical = candidate
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", candidate.display()))?;
    if !canonical.starts_with(root) {
        return Err(format!(
            "{} escapes the approved tool root {}",
            canonical.display(),
            root.display()
        ));
    }
    Ok(canonical)
}

fn search_recursive(
    dir: &Path,
    pattern: &str,
    results: &mut Vec<String>,
    depth: usize,
    max_depth: usize,
) {
    if depth > max_depth || results.len() >= 20 {
        return;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if name.contains(pattern) {
                results.push(path.display().to_string());
            }

            let is_real_directory = entry
                .file_type()
                .map(|file_type| file_type.is_dir())
                .unwrap_or(false);
            if is_real_directory && !name.starts_with('.') {
                search_recursive(&path, pattern, results, depth + 1, max_depth);
            }
        }
    }
}

// ===== Claude Code Style UI Components =====

fn render_banner() -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(
            Text::new("GLM Chat CLI")
                .color(Color::Cyan)
                .bold()
                .into_element(),
        )
        .child(
            Text::new("Type 'quit' to exit | 'clear' to clear screen")
                .dim()
                .into_element(),
        )
        .child(
            Text::new(format!(
                "Tools are omitted unless {ALLOW_TOOLS_ENV}=1; enabled calls require approval"
            ))
            .dim()
            .into_element(),
        )
        .into_element()
}

/// Render user message with Claude Code style (> prefix, no background)
fn render_user_message(text: &str) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("> ").color(Color::Yellow).bold().into_element())
        .child(Text::new(text).color(Color::BrightWhite).into_element())
        .into_element()
}

/// Render tool call (Claude Code style: ● ToolName(args))
fn render_tool_call(name: &str, args: &str) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("● ").color(Color::Magenta).into_element())
        .child(Text::new(name).color(Color::Magenta).bold().into_element())
        .child(
            Text::new(format!("(\"{}\")", args))
                .color(Color::Magenta)
                .into_element(),
        )
        .into_element()
}

/// Render tool result (Claude Code style: ⎿ result with indent)
fn render_tool_result(result: &str) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("  ⎿ ").color(Color::Ansi256(245)).into_element())
        .child(Text::new(result).color(Color::Ansi256(245)).into_element())
        .into_element()
}

/// Render thinking block (Claude Code style)
fn render_thinking(text: &str) -> Element {
    let lines: Vec<&str> = text.lines().take(5).collect();
    let has_more = text.lines().count() > 5;

    let mut container = RnkBox::new().flex_direction(FlexDirection::Column).child(
        Text::new("● Thinking...")
            .color(Color::Magenta) // Pink/Magenta color
            .into_element(),
    );

    for line in lines {
        container = container.child(
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("  ").into_element())
                .child(Text::new(line).color(Color::Magenta).dim().into_element())
                .into_element(),
        );
    }

    if has_more {
        container = container.child(
            Text::new("  ...")
                .color(Color::Ansi256(245))
                .dim()
                .into_element(),
        );
    }

    container.into_element()
}

fn render_error(message: &str) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("● ").color(Color::Red).into_element())
        .child(Text::new(message).color(Color::Red).into_element())
        .into_element()
}

fn render_goodbye() -> Element {
    Text::new("Goodbye!").dim().into_element()
}

fn render_cancelled() -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("● ").color(Color::Yellow).into_element())
        .child(
            Text::new("Cancelled")
                .color(Color::Yellow)
                .dim()
                .into_element(),
        )
        .into_element()
}

// Print rnk element to stdout (with newline)
fn print_element(element: &Element) {
    let output = rnk::render_to_string_auto(element);
    println!("{}", output);
}

fn render_assistant_response(text: &str) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("● ").color(Color::BrightWhite).into_element())
        .child(Text::new(text).color(Color::BrightWhite).into_element())
        .into_element()
}

struct RawModeGuard;

impl RawModeGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

/// Read a line in a Claude Code style prompt box with proper CJK backspace handling.
/// Translate a crossterm key event into the library's `Key`.
///
/// The example drives crossterm directly, so it has to build the value the
/// composer expects rather than receiving one from `use_input`.
fn to_rnk_key(code: KeyCode, modifiers: KeyModifiers) -> (String, Key) {
    let mut key = Key {
        ctrl: modifiers.contains(KeyModifiers::CONTROL),
        shift: modifiers.contains(KeyModifiers::SHIFT),
        alt: modifiers.contains(KeyModifiers::ALT),
        ..Key::default()
    };
    let mut input = String::new();

    match code {
        KeyCode::Enter => key.return_key = true,
        KeyCode::Esc => key.escape = true,
        KeyCode::Backspace => key.backspace = true,
        KeyCode::Delete => key.delete = true,
        KeyCode::Left => key.left_arrow = true,
        KeyCode::Right => key.right_arrow = true,
        KeyCode::Home => key.home = true,
        KeyCode::End => key.end = true,
        KeyCode::Char(c) => {
            key.character = Some(c);
            if !key.ctrl && !key.alt {
                input.push(c);
            }
        }
        _ => {}
    }

    (input, key)
}

fn read_line_with_input_box() -> io::Result<String> {
    // The composer owns the draft, so backspace removes a whole grapheme
    // cluster. Popping a `char`, as this loop used to, splits an emoji or a
    // combining sequence into something the user cannot repair.
    let mut composer = ChatComposerState::new();
    let keymap = ChatComposerKeyMap::new();
    let _raw_mode = RawModeGuard::enter()?;

    draw_prompt_box(&composer)?;

    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
            {
                if matches!(code, KeyCode::Char('c')) && modifiers.contains(KeyModifiers::CONTROL) {
                    // Ctrl+C - exit immediately, matching terminal conventions.
                    terminal::disable_raw_mode()?;
                    std::process::exit(0);
                }

                let (input, key) = to_rnk_key(code, modifiers);
                match handle_key(&mut composer, &keymap, &input, &key) {
                    InteractionOutcome::Submitted(text) => {
                        // The composer keeps the draft until the send is
                        // acknowledged; this caller takes the text and is done
                        // with the composer, so it acknowledges immediately.
                        if let Some(token) = composer.pending_submission().map(|p| p.token()) {
                            let _ = composer.acknowledge_success(token);
                        }
                        return Ok(text);
                    }
                    InteractionOutcome::Cancelled => {
                        composer = ChatComposerState::new();
                        redraw_prompt_box(&composer)?;
                    }
                    InteractionOutcome::Changed(_) | InteractionOutcome::Handled => {
                        redraw_prompt_box(&composer)?;
                    }
                    InteractionOutcome::Ignored => {}
                }
            }
        }
    }
}

// Spinner for loading animation with ESC cancellation support
struct Spinner {
    running: Arc<AtomicBool>,
    cancel_rx: watch::Receiver<bool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Spinner {
    fn new(message: &str) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let cancel_tx_clone = cancel_tx.clone();
        let message = message.to_string();

        let handle = std::thread::spawn(move || {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut i = 0;

            // Enable raw mode for key detection
            let _ = terminal::enable_raw_mode();

            while running_clone.load(Ordering::Relaxed) {
                // Check for ESC key
                if event::poll(Duration::from_millis(80)).unwrap_or(false) {
                    if let Ok(Event::Key(KeyEvent {
                        code: KeyCode::Esc, ..
                    })) = event::read()
                    {
                        let _ = cancel_tx_clone.send(true);
                        running_clone.store(false, Ordering::Relaxed);
                        break;
                    }
                }

                // Use ANSI codes for spinner
                print!(
                    "\x1b[2K\r\x1b[33m{} {} \x1b[2m(ESC to cancel)\x1b[0m",
                    frames[i], message
                );
                io::stdout().flush().unwrap();
                i = (i + 1) % frames.len();
            }

            let _ = terminal::disable_raw_mode();
            print!("\x1b[2K\r");
            io::stdout().flush().unwrap();
        });

        Self {
            running,
            cancel_rx,
            handle: Some(handle),
        }
    }

    fn get_cancel_receiver(&self) -> watch::Receiver<bool> {
        self.cancel_rx.clone()
    }

    fn stop(mut self) -> bool {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        *self.cancel_rx.borrow()
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

async fn send_request(
    client: &Client,
    messages: &[MessageParam],
    tools: &[Tool],
    api_key: &str,
) -> Result<ChatResponse, Box<dyn std::error::Error + Send + Sync>> {
    let request = build_request(messages, tools);

    let response = client
        .post(API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("API Error: {}", error_text).into());
    }

    Ok(response.json().await?)
}

fn build_request(messages: &[MessageParam], tools: &[Tool]) -> ChatRequest {
    ChatRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 8192,
        messages: messages.to_vec(),
        tools: (!tools.is_empty()).then(|| tools.to_vec()),
    }
}

/// Send request with cancellation support
async fn send_request_cancellable(
    client: &Client,
    messages: &[MessageParam],
    tools: &[Tool],
    api_key: &str,
    mut cancel_rx: watch::Receiver<bool>,
) -> Result<Option<ChatResponse>, Box<dyn std::error::Error + Send + Sync>> {
    tokio::select! {
        result = send_request(client, messages, tools, api_key) => {
            Ok(Some(result?))
        }
        _ = async {
            loop {
                cancel_rx.changed().await.ok();
                if *cancel_rx.borrow() {
                    break;
                }
            }
        } => {
            Ok(None) // Cancelled
        }
    }
}

fn format_tool_args(input: &Value) -> String {
    if let Some(obj) = input.as_object() {
        obj.iter()
            .map(|(k, v)| {
                let val = match v {
                    Value::String(s) => s.clone(),
                    _ => v.to_string(),
                };
                format!("{}={}", k, val)
            })
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        String::new()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!();
    print_element(&render_banner());
    println!();

    let api_key = match env::var("GLM_API_KEY") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => {
            print_element(&render_error(
                "GLM_API_KEY is required; no provider request was sent.",
            ));
            println!();
            return Ok(());
        }
    };

    let client = Client::new();
    let mut messages: Vec<MessageParam> = Vec::new();
    let tool_authorization = ToolAuthorization::from_env()?;
    let tools = tool_authorization.advertised_tools();

    loop {
        // Use custom input handler for a live Claude Code style prompt box.
        let input = read_line_with_input_box()?;
        clear_live_prompt_box();
        io::stdout().flush()?;

        let input = input.trim();

        match input.to_lowercase().as_str() {
            "quit" | "exit" => {
                println!();
                print_element(&render_goodbye());
                println!();
                break;
            }
            "clear" => {
                print!("\x1b[2J\x1b[H");
                print_element(&render_banner());
                println!();
                continue;
            }
            "" => continue,
            _ => {}
        }

        // Display user message in Claude Code style
        print_element(&render_user_message(input));

        messages.push(MessageParam {
            role: "user".to_string(),
            content: MessageContent::Text(input.to_string()),
        });

        // Handle multi-turn tool calls
        let mut tool_budget = ToolRoundBudget::default();
        loop {
            let spinner = Spinner::new("Thinking...");
            let cancel_rx = spinner.get_cancel_receiver();
            let result =
                send_request_cancellable(&client, &messages, &tools, &api_key, cancel_rx).await;
            let was_cancelled = spinner.stop();

            // Handle cancellation
            if was_cancelled {
                println!();
                print_element(&render_cancelled());
                messages.pop(); // Remove the user message since we cancelled
                println!();
                break;
            }

            match result {
                Ok(Some(response)) => {
                    let mut tool_uses = Vec::new();
                    let mut stop_tool_turn = false;

                    for block in &response.content {
                        match block {
                            ResponseBlock::Thinking { thinking } => {
                                println!();
                                print_element(&render_thinking(thinking));
                            }
                            ResponseBlock::Text { text } => {
                                if !text.is_empty() {
                                    println!();
                                    print_element(&render_assistant_response(text));
                                }
                            }
                            ResponseBlock::ToolUse { id, name, input } => {
                                let args = format_tool_args(input);
                                println!();
                                print_element(&render_tool_call(name, &args));

                                let decision = if !tool_budget.permits_execution() {
                                    ToolDecision::Denied(format!(
                                        "Not executed: tool round limit ({MAX_TOOL_ROUNDS}) reached."
                                    ))
                                } else {
                                    tool_authorization.review_and_execute(name, input)
                                };
                                print_element(&render_tool_result(decision.result()));
                                stop_tool_turn |= decision.was_denied();

                                tool_uses.push((id.clone(), decision.result().to_string()));
                            }
                        }
                    }

                    // Save assistant message
                    let assistant_content: Vec<ContentBlock> = response
                        .content
                        .iter()
                        .filter_map(|b| match b {
                            ResponseBlock::Text { text } => {
                                Some(ContentBlock::Text { text: text.clone() })
                            }
                            ResponseBlock::ToolUse { id, name, input } => {
                                Some(ContentBlock::ToolUse {
                                    id: id.clone(),
                                    name: name.clone(),
                                    input: input.clone(),
                                })
                            }
                            _ => None,
                        })
                        .collect();

                    messages.push(MessageParam {
                        role: "assistant".to_string(),
                        content: MessageContent::Blocks(assistant_content),
                    });

                    if !tool_uses.is_empty() {
                        let tool_results: Vec<ContentBlock> = tool_uses
                            .into_iter()
                            .map(|(id, result)| ContentBlock::ToolResult {
                                tool_use_id: id,
                                content: result,
                            })
                            .collect();

                        messages.push(MessageParam {
                            role: "user".to_string(),
                            content: MessageContent::Blocks(tool_results),
                        });
                        if stop_tool_turn {
                            println!();
                            break;
                        }
                        tool_budget.record_completed_round();
                        continue;
                    }

                    println!();
                    break;
                }
                Ok(None) => {
                    // Already handled above (cancelled)
                    break;
                }
                Err(e) => {
                    println!();
                    print_element(&render_error(&e.to_string()));
                    println!();
                    messages.pop();
                    break;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_tools_are_not_advertised_to_the_provider() {
        let authorization = ToolAuthorization::Disabled;
        let tools = authorization.advertised_tools();
        let request = build_request(&[], &tools);
        let json = serde_json::to_value(request).expect("request serializes");

        assert!(tools.is_empty());
        assert!(json.get("tools").is_none());
    }

    #[test]
    fn every_enabled_tool_call_still_requires_a_user_decision() {
        let root = env::current_dir().expect("repository root");
        let authorization = ToolAuthorization::Prompt {
            root: root.canonicalize().expect("canonical root"),
        };
        let mut prompts = 0;
        let decision =
            authorization.review_and_execute_with("list_files", &json!({"path": "."}), |_| {
                prompts += 1;
                false
            });

        assert_eq!(prompts, 1);
        assert!(decision.was_denied());
        assert!(decision.result().contains("denied by the user"));
    }

    #[test]
    fn canonical_paths_cannot_escape_the_approved_root() {
        let repository = env::current_dir()
            .expect("repository root")
            .canonicalize()
            .expect("canonical repository root");
        let root = repository
            .join("examples")
            .canonicalize()
            .expect("examples root");
        let input = json!({"path": repository.join("Cargo.toml")});

        let error = execute_tool(&root, "read_file", &input).expect_err("escape denied");

        assert!(error.contains("escapes the approved tool root"));
    }

    #[test]
    fn model_controlled_tool_rounds_stop_at_the_budget() {
        let mut budget = ToolRoundBudget::default();
        let mut executed = 0;

        for _ in 0..(MAX_TOOL_ROUNDS + 3) {
            if !budget.permits_execution() {
                break;
            }
            executed += 1;
            budget.record_completed_round();
        }

        assert_eq!(executed, MAX_TOOL_ROUNDS);
        assert!(!budget.permits_execution());
    }

    #[cfg(unix)]
    #[test]
    fn recursive_search_does_not_follow_a_symlink_outside_the_root() {
        use std::os::unix::fs::symlink;
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let base = env::temp_dir().join(format!("rnk-glm-search-{}-{unique}", std::process::id()));
        let root = base.join("root");
        let outside = base.join("outside");
        fs::create_dir_all(&root).expect("root created");
        fs::create_dir_all(&outside).expect("outside created");
        fs::write(outside.join("secret-match.txt"), "secret").expect("fixture written");
        symlink(&outside, root.join("escape-link")).expect("symlink created");

        let mut results = Vec::new();
        search_recursive(&root, "secret-match", &mut results, 0, 3);

        let cleanup = fs::remove_dir_all(&base);
        assert!(
            results.is_empty(),
            "search escaped through symlink: {results:?}"
        );
        cleanup.expect("fixture removed");
    }
}
