//! GLM CLI Chat Demo with Tool Use - Using Tink UI (Old Version with Custom Rendering)
//!
//! This is the old version that includes custom rendering implementation.
//! It demonstrates how to implement text wrapping, layout calculation, and
//! element rendering from scratch. Kept for educational purposes.
//!
//! The new version (glm_chat.rs) uses rnk's built-in render API instead.
//!
//! Run with: GLM_API_KEY=your_key cargo run --example glm_chat_old

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
use unicode_width::UnicodeWidthChar;

use rnk::core::Style;
use rnk::layout::LayoutEngine;
use rnk::prelude::{Color, Display, Element, FlexDirection, Text};
use rnk::renderer::Output;

// Alias tink's Box to avoid conflict with std::boxed::Box
use rnk::prelude::Box as TinkBox;

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

// ===== Text Wrapping Helpers =====

/// Wrap text to fit within max_width columns
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0usize;

    for ch in text.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);

        if ch == '\n' {
            // Explicit newline - flush current line
            result.push(current_line.clone());
            current_line = String::new();
            current_width = 0;
        } else if current_width + ch_width > max_width {
            // Need to wrap - first try to find a break point
            if current_line.is_empty() {
                // Character is too wide for a single line, just add it
                current_line.push(ch);
                current_width = ch_width;
            } else {
                // Push current line and start new one
                result.push(current_line.clone());
                current_line = ch.to_string();
                current_width = ch_width;
            }
        } else {
            current_line.push(ch);
            current_width += ch_width;
        }
    }

    if !current_line.is_empty() {
        result.push(current_line);
    }

    if result.is_empty() {
        result.push(String::new());
    }

    result
}

// ===== Rendering =====

/// Calculate the actual height needed for an element, considering text wrapping
fn calculate_element_height(element: &Element, max_width: u16) -> u16 {
    let mut height = 1u16;
    let available_width = if element.style.has_border() {
        max_width.saturating_sub(2)
    } else {
        max_width
    };
    let padding_h = (element.style.padding.left + element.style.padding.right) as u16;
    let available_width = available_width.saturating_sub(padding_h);

    // Check for multiline spans with wrapping
    if let Some(lines) = &element.spans {
        let mut total_lines = 0usize;
        for line in lines {
            // Reconstruct the full line text and calculate wrapped height
            let line_text: String = line.spans.iter().map(|s| s.content.as_str()).collect();
            let wrapped = wrap_text(&line_text, available_width as usize);
            total_lines += wrapped.len();
        }
        height = height.max(total_lines as u16);
    }

    // Check text_content with wrapping
    if let Some(text) = &element.text_content {
        let wrapped = wrap_text(text, available_width as usize);
        height = height.max(wrapped.len() as u16);
    }

    // Recursively check children and accumulate height for column layout
    let mut child_height_sum = 0u16;
    for child in &element.children {
        let child_height = calculate_element_height(child, max_width);
        child_height_sum += child_height;
    }

    // Use child height sum if we have children
    if !element.children.is_empty() {
        height = height.max(child_height_sum);
    }

    height
}

/// Render a single text span at position, handling wrapping
fn render_text_span(
    output: &mut Output,
    text: &str,
    x: u16,
    y: u16,
    max_width: u16,
    style: &Style,
) {
    let wrapped_lines = wrap_text(text, max_width as usize);
    for (i, line) in wrapped_lines.iter().enumerate() {
        output.write(x, y + i as u16, line, style);
    }
}

/// Main render-to-string function
fn render_to_string(element: &Element, width: u16) -> String {
    let mut engine = LayoutEngine::new();
    engine.compute(element, width, 100);

    // Calculate actual height considering text wrapping
    let height = calculate_element_height(element, width);

    let mut output = Output::new(width, height);
    render_element_recursive(element, &engine, &mut output, 0.0, 0.0, width);
    output.render()
}

/// Recursively render element tree
fn render_element_recursive(
    element: &Element,
    engine: &LayoutEngine,
    output: &mut Output,
    offset_x: f32,
    offset_y: f32,
    container_width: u16,
) {
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

    // Calculate available width for text
    let inner_x =
        x + if element.style.has_border() { 1 } else { 0 } + element.style.padding.left as u16;
    let inner_y =
        y + if element.style.has_border() { 1 } else { 0 } + element.style.padding.top as u16;
    let padding_h = (element.style.padding.left + element.style.padding.right) as u16;
    let inner_width = w.saturating_sub(if element.style.has_border() { 2 } else { 0 } + padding_h);

    // Render text content with wrapping
    if let Some(text) = &element.text_content {
        render_text_span(output, text, inner_x, inner_y, inner_width, &element.style);
    } else if let Some(lines) = &element.spans {
        // Render rich text with spans and wrapping
        let mut line_offset = 0u16;
        for line in lines {
            // Combine spans into a single line for wrapping calculation
            let line_text: String = line.spans.iter().map(|s| s.content.as_str()).collect();
            let wrapped = wrap_text(&line_text, inner_width as usize);

            for (wrapped_idx, wrapped_line) in wrapped.iter().enumerate() {
                // For simplicity, render the whole wrapped line with the first span's style
                // (proper implementation would track character positions across spans)
                let span_style = if !line.spans.is_empty() {
                    let span = &line.spans[0];
                    let mut style = element.style.clone();
                    if span.style.color.is_some() {
                        style.color = span.style.color;
                    }
                    if span.style.background_color.is_some() {
                        style.background_color = span.style.background_color;
                    }
                    if span.style.bold {
                        style.bold = true;
                    }
                    if span.style.italic {
                        style.italic = true;
                    }
                    if span.style.dim {
                        style.dim = true;
                    }
                    if span.style.underline {
                        style.underline = true;
                    }
                    style
                } else {
                    element.style.clone()
                };

                output.write(
                    inner_x,
                    inner_y + line_offset + wrapped_idx as u16,
                    wrapped_line,
                    &span_style,
                );
            }
            line_offset += wrapped.len() as u16;
        }
    }

    // Recursively render children
    for child in element.children.iter() {
        render_element_recursive(child, engine, output, x as f32, y as f32, container_width);
    }
}

// ===== Claude Code Style UI Components =====

fn render_banner() -> Element {
    TinkBox::new()
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
        .into_element()
}

/// Render user message with Claude Code style (> prefix, no background)
fn render_user_message(text: &str) -> Element {
    TinkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("> ").color(Color::Yellow).bold().into_element())
        .child(Text::new(text).color(Color::BrightWhite).into_element())
        .into_element()
}

/// Render tool call (Claude Code style: ● ToolName(args))
fn render_tool_call(name: &str, args: &str) -> Element {
    TinkBox::new()
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
    TinkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("  ⎿ ").color(Color::Ansi256(245)).into_element())
        .child(Text::new(result).color(Color::Ansi256(245)).into_element())
        .into_element()
}

/// Render thinking block (Claude Code style)
fn render_thinking(text: &str) -> Element {
    let lines: Vec<&str> = text.lines().take(5).collect();
    let has_more = text.lines().count() > 5;

    let mut container = TinkBox::new().flex_direction(FlexDirection::Column).child(
        Text::new("● Thinking...")
            .color(Color::Magenta) // Pink/Magenta color
            .into_element(),
    );

    for line in lines {
        container = container.child(
            TinkBox::new()
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
    TinkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("● ").color(Color::Red).into_element())
        .child(Text::new(message).color(Color::Red).into_element())
        .into_element()
}

fn render_prompt() -> Element {
    TinkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("> ").color(Color::Yellow).bold().into_element())
        .into_element()
}

fn render_goodbye() -> Element {
    Text::new("Goodbye!").dim().into_element()
}

fn render_cancelled() -> Element {
    TinkBox::new()
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

// Direct ANSI print for AI response (bypasses layout engine to avoid indentation)
fn print_assistant_response(text: &str) {
    // ● prefix in default color, then white text
    // \x1b[97m = bright white, \x1b[0m = reset
    println!("\x1b[97m● {}\x1b[0m", text);
}

/// Read a line of input with proper CJK character handling
/// Uses raw mode to correctly handle backspace for wide characters
fn read_line_with_cjk() -> io::Result<String> {
    let mut input = String::new();
    let mut stdout = io::stdout();

    terminal::enable_raw_mode()?;

    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
            {
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
                        let total_width: usize =
                            input.chars().map(|c| c.width().unwrap_or(1)).sum();
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

        // Clear the line where user typed (move up and clear)
        // The user input was already echoed, so we need to replace it with formatted version
        print!("\x1b[1A\x1b[2K"); // Move up one line and clear it

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
                                    print_assistant_response(text);
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
