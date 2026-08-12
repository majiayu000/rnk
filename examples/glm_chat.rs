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
use std::path::Path;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolAuthorization {
    DisplayOnly,
    Execute,
}

impl ToolAuthorization {
    fn from_env() -> Self {
        match env::var(ALLOW_TOOLS_ENV) {
            Ok(value) if value == "1" || value.eq_ignore_ascii_case("true") => Self::Execute,
            _ => Self::DisplayOnly,
        }
    }

    fn execute_or_deny(self, name: &str, input: &Value) -> String {
        match self {
            Self::Execute => execute_tool(name, input),
            Self::DisplayOnly => format!(
                "Not executed. Set {ALLOW_TOOLS_ENV}=1 after reviewing the request to allow this demo tool."
            ),
        }
    }
}

fn execute_tool(name: &str, input: &Value) -> String {
    match name {
        "read_file" => {
            let path = input["path"].as_str().unwrap_or("");
            match fs::read_to_string(path) {
                Ok(content) => {
                    let lines: Vec<&str> = content.lines().take(100).collect();
                    format!("Read {} lines", lines.len())
                }
                Err(e) => format!("Error: {}", e),
            }
        }
        "list_files" => {
            let path = input["path"].as_str().unwrap_or(".");
            match fs::read_dir(path) {
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
                    files.join(", ")
                }
                Err(e) => format!("Error: {}", e),
            }
        }
        "search_files" => {
            let pattern = input["pattern"].as_str().unwrap_or("*");
            let mut results = Vec::new();
            search_recursive(Path::new("."), pattern, &mut results, 0, 3);
            if results.is_empty() {
                "No files found".to_string()
            } else {
                format!("Found {} files", results.len())
            }
        }
        _ => format!("Unknown tool: {}", name),
    }
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

            if path.is_dir() && !name.starts_with('.') {
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
                "Tool calls are display-only unless {ALLOW_TOOLS_ENV}=1"
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
    let request = ChatRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 8192,
        messages: messages.to_vec(),
        tools: Some(tools.to_vec()),
    };

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
    let tools = get_tools();
    let tool_authorization = ToolAuthorization::from_env();

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

                                let tool_result = tool_authorization.execute_or_deny(name, input);
                                print_element(&render_tool_result(&tool_result));

                                tool_uses.push((id.clone(), tool_result));
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
