//! GLM CLI Chat Demo with Tool Use - Using Tink UI
//!
//! Run with: GLM_API_KEY=your_key cargo run --example glm_chat

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use unicode_width::UnicodeWidthChar;

use tink::core::Dimension;
use tink::layout::LayoutEngine;
use tink::prelude::{
    BorderStyle, Color, Display, Element, FlexDirection, Newline, Position, Text,
};
use tink::renderer::Output;

// Alias tink's Box to avoid conflict with std::boxed::Box
use tink::prelude::Box as TinkBox;

const API_URL: &str = "https://open.bigmodel.cn/api/anthropic/v1/messages";

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

// Tink UI rendering helpers
fn render_to_string(element: &Element, width: u16) -> String {
    let mut engine = LayoutEngine::new();
    engine.compute(element, width, 100);

    let layout = engine.get_layout(element.id).unwrap_or_default();
    let height = (layout.height as u16).max(1);

    let mut output = Output::new(width, height);
    render_element(element, &engine, &mut output, 0.0, 0.0);
    output.render()
}

fn render_element(element: &Element, engine: &LayoutEngine, output: &mut Output, offset_x: f32, offset_y: f32) {
    if element.style.display == Display::None {
        return;
    }

    let layout = match engine.get_layout(element.id) {
        Some(l) => l,
        None => return,
    };

    let x = (offset_x + layout.x) as u16;
    let y = (offset_y + layout.y) as u16;
    let w = layout.width as u16;
    let h = layout.height as u16;

    // Background
    if element.style.background_color.is_some() {
        for row in 0..h {
            output.write(x, y + row, &" ".repeat(w as usize), &element.style);
        }
    }

    // Border
    if element.style.has_border() {
        let (tl, tr, bl, br, hz, vt) = element.style.border_style.chars();
        let mut style = element.style.clone();

        style.color = element.style.get_border_top_color();
        output.write(
            x,
            y,
            &format!("{}{}{}", tl, hz.repeat((w as usize).saturating_sub(2)), tr),
            &style,
        );

        style.color = element.style.get_border_bottom_color();
        output.write(
            x,
            y + h.saturating_sub(1),
            &format!("{}{}{}", bl, hz.repeat((w as usize).saturating_sub(2)), br),
            &style,
        );

        for row in 1..h.saturating_sub(1) {
            style.color = element.style.get_border_left_color();
            output.write(x, y + row, vt, &style);
            style.color = element.style.get_border_right_color();
            output.write(x + w.saturating_sub(1), y + row, vt, &style);
        }
    }

    // Text
    if let Some(text) = &element.text_content {
        let text_x =
            x + if element.style.has_border() { 1 } else { 0 } + element.style.padding.left as u16;
        let text_y =
            y + if element.style.has_border() { 1 } else { 0 } + element.style.padding.top as u16;
        output.write(text_x, text_y, text, &element.style);
    }

    // Children
    let cx = offset_x + layout.x;
    let cy = offset_y + layout.y;

    for child in element.children.iter() {
        if child.style.position == Position::Absolute {
            render_element(
                child,
                engine,
                output,
                child.style.left.unwrap_or(0.0),
                child.style.top.unwrap_or(0.0),
            );
        } else {
            render_element(child, engine, output, cx, cy);
        }
    }
}

// UI Components using Tink
fn render_banner() -> Element {
    TinkBox::new()
        .width(Dimension::Percent(100.0))
        .flex_direction(FlexDirection::Column)
        .child(
            TinkBox::new()
                .border_style(BorderStyle::Round)
                .border_color(Color::Cyan)
                .padding_x(2.0)
                .padding_y(1.0)
                .flex_direction(FlexDirection::Column)
                .child(
                    TinkBox::new()
                        .flex_direction(FlexDirection::Row)
                        .child(
                            Text::new("GLM Chat CLI")
                                .color(Color::Cyan)
                                .bold()
                                .into_element(),
                        )
                        .child(Text::new("  ").into_element())
                        .child(
                            Text::new("with Tool Use")
                                .color(Color::White)
                                .dim()
                                .into_element(),
                        )
                        .into_element(),
                )
                .child(Newline::new().into_element())
                .child(
                    Text::new("Type 'quit' to exit | 'clear' to clear screen")
                        .dim()
                        .into_element(),
                )
                .into_element(),
        )
        .into_element()
}

fn render_tool_call(name: &str, args: &str) -> Element {
    TinkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(
            Text::new("⏺ ")
                .color(Color::Magenta)
                .into_element(),
        )
        .child(
            Text::new(name)
                .color(Color::Magenta)
                .bold()
                .into_element(),
        )
        .child(
            Text::new(format!("({})", args))
                .color(Color::Magenta)
                .into_element(),
        )
        .into_element()
}

fn render_tool_result(result: &str) -> Element {
    TinkBox::new()
        .flex_direction(FlexDirection::Row)
        .padding_left(2.0)
        .child(
            Text::new("⎿ ")
                .color(Color::Magenta)
                .into_element(),
        )
        .child(
            Text::new(result)
                .color(Color::Magenta)
                .into_element(),
        )
        .into_element()
}

fn render_thinking(text: &str) -> Element {
    let lines: Vec<&str> = text.lines().take(5).collect();
    let has_more = text.lines().count() > 5;

    let mut container = TinkBox::new()
        .border_style(BorderStyle::Single)
        .border_color(Color::Yellow)
        .border_dim(true)
        .padding(1)
        .flex_direction(FlexDirection::Column)
        .child(
            Text::new("Thinking")
                .color(Color::Yellow)
                .dim()
                .into_element(),
        )
        .child(Newline::new().into_element());

    for line in lines {
        container = container.child(
            Text::new(line)
                .color(Color::Yellow)
                .dim()
                .into_element(),
        );
    }

    if has_more {
        container = container.child(
            Text::new("...")
                .color(Color::Yellow)
                .dim()
                .into_element(),
        );
    }

    container.into_element()
}

fn render_error(message: &str) -> Element {
    TinkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(
            Text::new("[Error] ")
                .color(Color::Red)
                .bold()
                .into_element(),
        )
        .child(
            Text::new(message)
                .color(Color::Red)
                .into_element(),
        )
        .into_element()
}

fn render_prompt() -> Element {
    TinkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(
            Text::new("❯ ")
                .color(Color::Green)
                .bold()
                .into_element(),
        )
        .into_element()
}

fn render_assistant_text(text: &str) -> Element {
    Text::new(text)
        .color(Color::White)
        .into_element()
}

fn render_goodbye() -> Element {
    Text::new("Goodbye!")
        .dim()
        .into_element()
}

fn render_cancelled() -> Element {
    Text::new("⏹ Cancelled")
        .color(Color::Yellow)
        .dim()
        .into_element()
}

// Print tink element to stdout (with newline)
fn print_element(element: &Element) {
    let (width, _) = crossterm::terminal::size().unwrap_or((80, 24));
    let output = render_to_string(element, width);
    println!("{}", output);
}

// Print tink element to stdout (without newline, for inline prompts)
fn print_element_inline(element: &Element) {
    let (width, _) = crossterm::terminal::size().unwrap_or((80, 24));
    let output = render_to_string(element, width);
    print!("{}", output);
}

/// Read a line of input with proper CJK character handling
/// Uses raw mode to correctly handle backspace for wide characters
fn read_line_with_cjk() -> io::Result<String> {
    let mut input = String::new();
    let mut stdout = io::stdout();

    terminal::enable_raw_mode()?;

    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
                match code {
                    KeyCode::Enter => {
                        // Print newline and exit
                        print!("\r\n");
                        stdout.flush()?;
                        break;
                    }
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        // Ctrl+C - exit
                        terminal::disable_raw_mode()?;
                        std::process::exit(0);
                    }
                    KeyCode::Char(c) => {
                        // Add character to input
                        input.push(c);
                        print!("{}", c);
                        stdout.flush()?;
                    }
                    KeyCode::Backspace => {
                        if let Some(ch) = input.pop() {
                            // Move cursor back by character width and clear
                            let char_width = ch.width().unwrap_or(1);
                            // Move left, print spaces, move left again
                            for _ in 0..char_width {
                                print!("\x08 \x08");
                            }
                            stdout.flush()?;
                        }
                    }
                    KeyCode::Esc => {
                        // Clear input on Escape
                        let total_width: usize = input.chars().map(|c| c.width().unwrap_or(1)).sum();
                        for _ in 0..total_width {
                            print!("\x08 \x08");
                        }
                        stdout.flush()?;
                        input.clear();
                    }
                    _ => {}
                }
            }
        }
    }

    terminal::disable_raw_mode()?;
    Ok(input)
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
                    if let Ok(Event::Key(KeyEvent { code: KeyCode::Esc, .. })) = event::read() {
                        let _ = cancel_tx_clone.send(true);
                        running_clone.store(false, Ordering::Relaxed);
                        break;
                    }
                }

                // Use ANSI codes for spinner
                print!("\x1b[2K\r\x1b[33m{} {} \x1b[2m(ESC to cancel)\x1b[0m", frames[i], message);
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
    // Get API key from environment
    let api_key = env::var("GLM_API_KEY").unwrap_or_else(|_| {
        eprintln!("Warning: GLM_API_KEY not set, using default key");
        "your_api_key_here".to_string()
    });

    let client = Client::new();
    let mut messages: Vec<MessageParam> = Vec::new();
    let tools = get_tools();

    // Print banner
    println!();
    print_element(&render_banner());
    println!();

    loop {
        // Print prompt (inline so user types on same line)
        print_element_inline(&render_prompt());
        io::stdout().flush()?;

        // Use custom input handler for proper CJK backspace support
        let input = read_line_with_cjk()?;
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

        messages.push(MessageParam {
            role: "user".to_string(),
            content: MessageContent::Text(input.to_string()),
        });

        // Handle multi-turn tool calls
        loop {
            let spinner = Spinner::new("Thinking...");
            let cancel_rx = spinner.get_cancel_receiver();
            let result = send_request_cancellable(&client, &messages, &tools, &api_key, cancel_rx).await;
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
                                    print_element(&render_assistant_text(text));
                                }
                            }
                            ResponseBlock::ToolUse { id, name, input } => {
                                let args = format_tool_args(input);
                                println!();
                                print_element(&render_tool_call(name, &args));

                                let tool_result = execute_tool(name, input);
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
