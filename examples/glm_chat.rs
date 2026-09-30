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
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
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
const MAX_LISTED_ENTRIES: usize = 20;

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
            description: "Search for matching regular filenames in current directory; return up to 20 relative paths as a JSON array".to_string(),
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

#[derive(Debug)]
enum ToolAuthorization {
    Disabled,
    Prompt {
        root: PathBuf,
        #[cfg(unix)]
        search_root: fs::File,
    },
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
                Self::prompt(root)
            }
            _ => Ok(Self::Disabled),
        }
    }

    fn prompt(root: PathBuf) -> io::Result<Self> {
        #[cfg(unix)]
        let search_root = {
            // Canonical roots are absolute. Walk every component without following
            // symlinks, then hold the directory for the authorization's lifetime.
            let approved = ApprovedPath {
                display: root.clone(),
                components: root
                    .components()
                    .filter_map(|component| match component {
                        Component::Normal(name) => Some(name.to_os_string()),
                        _ => None,
                    })
                    .collect(),
            };
            open_nofollow(Path::new("/"), &approved, OpenMode::Directory).map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!("cannot open tool root {}: {error}", root.display()),
                )
            })?
        };
        Ok(Self::Prompt {
            root,
            #[cfg(unix)]
            search_root,
        })
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
        let Self::Prompt {
            root,
            #[cfg(unix)]
            search_root,
        } = self
        else {
            return ToolDecision::Denied(format!(
                "Not executed. Restart with {ALLOW_TOOLS_ENV}=1 to enable per-request approval."
            ));
        };

        if !confirm(root) {
            return ToolDecision::Denied("Not executed: denied by the user.".to_string());
        }

        #[cfg(not(unix))]
        let search_root = root;
        let result = match name {
            "search_files" => search_files(search_root, input),
            _ => execute_tool(root, name, input),
        };
        match result {
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
            let approved = approve_confined_path(root, input)?;
            read_approved(root, &approved)
        }
        "list_files" => {
            let approved = approve_confined_path(root, input)?;
            list_approved(root, &approved)
        }
        _ => Err(format!("unknown tool: {name}")),
    }
}

fn search_files(
    #[cfg(unix)] root: &fs::File,
    #[cfg(not(unix))] root: &Path,
    input: &Value,
) -> Result<String, String> {
    let pattern = input
        .get("pattern")
        .and_then(Value::as_str)
        .filter(|pattern| !pattern.is_empty())
        .ok_or_else(|| "search_files requires a non-empty string pattern".to_string())?;
    let mut results = Vec::new();
    #[cfg(unix)]
    search_recursive(root, pattern, &mut results, 0, 3);
    #[cfg(not(unix))]
    search_recursive(root, Path::new(""), pattern, &mut results, 0, 3);
    if results.is_empty() {
        Ok("No files found".to_string())
    } else {
        Ok(format!("Found {} files\n{}", results.len(), json!(results)))
    }
}

struct ApprovedPath {
    display: PathBuf,
    #[cfg(unix)]
    components: Vec<OsString>,
}

#[cfg(unix)]
#[derive(Clone, Copy)]
enum OpenMode {
    File,
    Directory,
}

/// Approve `input`'s path without looking at the filesystem.
///
/// `root` is the already-canonical tool root. The requested target is walked
/// lexically (`..` may not leave `root`, `.` is dropped) and is not
/// canonicalized, so a later symlink swap cannot reuse a checked `PathBuf`.
fn approve_confined_path(root: &Path, input: &Value) -> Result<ApprovedPath, String> {
    let raw = input
        .get("path")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| "tool requires a non-empty string path".to_string())?;
    let requested = Path::new(raw);
    let relative = if requested.is_absolute() {
        requested
            .strip_prefix(root)
            .map_err(|_| escapes_root(requested, root))?
    } else {
        requested
    };

    let mut components: Vec<OsString> = Vec::new();
    for component in relative.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(name) => components.push(name.to_os_string()),
            Component::ParentDir => {
                if components.pop().is_none() {
                    let escaped = if requested.is_absolute() {
                        requested.to_path_buf()
                    } else {
                        root.join(requested)
                    };
                    return Err(escapes_root(&escaped, root));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(escapes_root(requested, root));
            }
        }
    }

    let mut display = root.to_path_buf();
    for component in &components {
        display.push(component);
    }
    Ok(ApprovedPath {
        display,
        #[cfg(unix)]
        components,
    })
}

fn escapes_root(path: &Path, root: &Path) -> String {
    format!(
        "{} escapes the approved tool root {}",
        path.display(),
        root.display()
    )
}

fn read_approved(root: &Path, approved: &ApprovedPath) -> Result<String, String> {
    let mut file = open_for_read(root, approved)
        .map_err(|error| format!("cannot read {}: {error}", approved.display.display()))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|error| format!("cannot read {}: {error}", approved.display.display()))?;
    let line_count = content.lines().take(100).count();
    Ok(format!(
        "Read {line_count} lines from {}",
        approved.display.display()
    ))
}

fn list_approved(root: &Path, approved: &ApprovedPath) -> Result<String, String> {
    let names = list_approved_names(root, approved)
        .map_err(|error| format!("cannot list {}: {error}", approved.display.display()))?;
    Ok(names.join(", "))
}

#[cfg(unix)]
fn open_for_read(root: &Path, approved: &ApprovedPath) -> io::Result<fs::File> {
    open_nofollow(root, approved, OpenMode::File)
}

#[cfg(not(unix))]
fn open_for_read(_root: &Path, approved: &ApprovedPath) -> io::Result<fs::File> {
    fs::File::open(&approved.display)
}

#[cfg(unix)]
fn list_approved_names(root: &Path, approved: &ApprovedPath) -> io::Result<Vec<String>> {
    let file = open_nofollow(root, approved, OpenMode::Directory)?;
    list_from_owned_fd(file.into())
}

#[cfg(not(unix))]
fn list_approved_names(_root: &Path, approved: &ApprovedPath) -> io::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(&approved.display)?.filter_map(|entry| entry.ok()) {
        if names.len() == MAX_LISTED_ENTRIES {
            break;
        }
        let mut label = entry.file_name().to_string_lossy().into_owned();
        if entry.path().is_dir() {
            label.push('/');
        }
        names.push(label);
    }
    Ok(names)
}

#[cfg(unix)]
fn open_nofollow(root: &Path, approved: &ApprovedPath, mode: OpenMode) -> io::Result<fs::File> {
    use std::os::fd::{AsRawFd, OwnedFd};
    use std::os::unix::fs::OpenOptionsExt;

    // O_NOFOLLOW rejects a symlink planted in place of the canonical root.
    // The descriptor stays local to this open; approval does not hold one.
    let root_file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(root)?;
    let mut current: OwnedFd = root_file.into();

    if approved.components.is_empty() {
        return match mode {
            OpenMode::Directory => Ok(fs::File::from(current)),
            OpenMode::File => Err(io::Error::new(
                io::ErrorKind::IsADirectory,
                "is a directory",
            )),
        };
    }

    for (index, component) in approved.components.iter().enumerate() {
        let mode = if index + 1 < approved.components.len() || matches!(mode, OpenMode::Directory) {
            OpenMode::Directory
        } else {
            OpenMode::File
        };
        current = open_child_nofollow(current.as_raw_fd(), component, mode)?;
    }

    Ok(fs::File::from(current))
}

#[cfg(unix)]
fn open_child_nofollow(
    parent_fd: std::os::fd::RawFd,
    component: &std::ffi::OsStr,
    mode: OpenMode,
) -> io::Result<std::os::fd::OwnedFd> {
    use std::ffi::CString;
    use std::os::fd::{FromRawFd, OwnedFd};
    use std::os::unix::ffi::OsStrExt;

    reject_special_component(component)?;
    let name = CString::new(component.as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path component contains NUL"))?;
    let mut flags = libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW;
    if matches!(mode, OpenMode::Directory) {
        flags |= libc::O_DIRECTORY;
    }
    // Safety: parent_fd is held open by the caller and name is NUL-terminated.
    let fd = unsafe { libc::openat(parent_fd, name.as_ptr(), flags) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // Safety: openat just returned a new owned descriptor, or we returned above.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

#[cfg(unix)]
fn reject_special_component(component: &std::ffi::OsStr) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;

    let bytes = component.as_bytes();
    if bytes.is_empty() || bytes == b"." || bytes == b".." || bytes.contains(&b'/') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "refusing special path component",
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn list_from_owned_fd(fd: std::os::fd::OwnedFd) -> io::Result<Vec<String>> {
    let entries = open_dir_from_owned_fd(fd)?;
    read_dir_entries(entries.dir)
}

#[cfg(unix)]
fn open_dir_from_owned_fd(fd: std::os::fd::OwnedFd) -> io::Result<OpenDir> {
    use std::os::fd::AsRawFd;

    // Safety: fd is an open directory descriptor. On failure fdopendir leaves it
    // open and OwnedFd closes it. On success the DIR owns it until closedir.
    let dir = unsafe { libc::fdopendir(fd.as_raw_fd()) };
    if dir.is_null() {
        return Err(io::Error::last_os_error());
    }
    std::mem::forget(fd);
    Ok(OpenDir { dir })
}

#[cfg(unix)]
struct OpenDir {
    dir: *mut libc::DIR,
}

#[cfg(unix)]
impl Drop for OpenDir {
    fn drop(&mut self) {
        if !self.dir.is_null() {
            unsafe {
                libc::closedir(self.dir);
            }
            self.dir = std::ptr::null_mut();
        }
    }
}

#[cfg(unix)]
fn read_dir_entries(dir: *mut libc::DIR) -> io::Result<Vec<String>> {
    use std::ffi::CStr;

    let mut names = Vec::new();
    loop {
        clear_errno();
        // Safety: dir came from fdopendir and OpenDir has not closed it yet.
        let entry = unsafe { libc::readdir(dir) };
        if entry.is_null() {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(0) {
                break;
            }
            return Err(error);
        }
        // Safety: readdir's dirent, including d_name, is valid until the next call.
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        if name.to_bytes() == b"." || name.to_bytes() == b".." {
            continue;
        }
        let directory = entry_type(dir, entry, name)? == libc::DT_DIR;
        let mut label = name.to_string_lossy().into_owned();
        if directory {
            label.push('/');
        }
        names.push(label);
        if names.len() == MAX_LISTED_ENTRIES {
            break;
        }
    }
    Ok(names)
}

#[cfg(unix)]
fn entry_type(
    dir: *mut libc::DIR,
    entry: *mut libc::dirent,
    name: &std::ffi::CStr,
) -> io::Result<u8> {
    // Safety: entry is the current readdir result and name points at its d_name.
    let dtype = unsafe { (*entry).d_type };
    if dtype != libc::DT_UNKNOWN {
        return Ok(dtype);
    }
    let dirfd = unsafe { libc::dirfd(dir) };
    if dirfd < 0 {
        return Err(io::Error::last_os_error());
    }
    let mut info = std::mem::MaybeUninit::<libc::stat>::uninit();
    let rc = unsafe {
        libc::fstatat(
            dirfd,
            name.as_ptr(),
            info.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    let info = unsafe { info.assume_init() };
    Ok(match info.st_mode & libc::S_IFMT {
        libc::S_IFDIR => libc::DT_DIR,
        libc::S_IFREG => libc::DT_REG,
        _ => libc::DT_UNKNOWN,
    })
}

#[cfg(unix)]
fn clear_errno() {
    #[cfg(target_os = "linux")]
    unsafe {
        *libc::__errno_location() = 0;
    }
    #[cfg(target_os = "macos")]
    unsafe {
        *libc::__error() = 0;
    }
}

#[cfg(unix)]
fn search_recursive(
    root: &fs::File,
    pattern: &str,
    results: &mut Vec<String>,
    depth: usize,
    max_depth: usize,
) {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

    if depth > max_depth || results.len() >= MAX_LISTED_ENTRIES {
        return;
    }
    // Open "." from the held root to get an independent directory cursor for
    // each search. Duplicating the descriptor would share readdir's offset.
    // Safety: root is held open and the pathname is a NUL-terminated constant.
    let fd = unsafe {
        libc::openat(
            root.as_raw_fd(),
            c".".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd >= 0 {
        // Safety: openat returned a newly owned directory descriptor.
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        search_from_owned_fd(fd, Path::new(""), pattern, results, depth, max_depth);
    }
}

#[cfg(unix)]
fn search_from_owned_fd(
    fd: std::os::fd::OwnedFd,
    path: &Path,
    pattern: &str,
    results: &mut Vec<String>,
    depth: usize,
    max_depth: usize,
) {
    use std::ffi::{CStr, OsStr};
    use std::os::unix::ffi::OsStrExt;

    if depth > max_depth || results.len() >= MAX_LISTED_ENTRIES {
        return;
    }
    let Ok(entries) = open_dir_from_owned_fd(fd) else {
        return;
    };
    // Safety: entries owns a live DIR until this traversal finishes.
    let parent_fd = unsafe { libc::dirfd(entries.dir) };
    if parent_fd < 0 {
        return;
    }
    while results.len() < MAX_LISTED_ENTRIES {
        // Safety: entries owns the directory and the result stays valid until
        // the next readdir call on this DIR.
        let entry = unsafe { libc::readdir(entries.dir) };
        if entry.is_null() {
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        if name.to_bytes() == b"." || name.to_bytes() == b".." {
            continue;
        }
        let Ok(dtype) = entry_type(entries.dir, entry, name) else {
            continue;
        };
        let component = OsStr::from_bytes(name.to_bytes());
        let child_path = path.join(component);
        if dtype == libc::DT_REG && component.to_string_lossy().contains(pattern) {
            // Relative JSON paths need UTF-8 only below the approved root.
            if let Some(path) = child_path.to_str() {
                results.push(path.to_string());
            }
        } else if dtype == libc::DT_DIR
            && !name.to_bytes().starts_with(b".")
            && depth < max_depth
            && let Ok(child) = open_child_nofollow(parent_fd, component, OpenMode::Directory)
        {
            search_from_owned_fd(child, &child_path, pattern, results, depth + 1, max_depth);
        }
    }
}

#[cfg(not(unix))]
fn search_recursive(
    dir: &Path,
    relative: &Path,
    pattern: &str,
    results: &mut Vec<String>,
    depth: usize,
    max_depth: usize,
) {
    if depth > max_depth || results.len() >= MAX_LISTED_ENTRIES {
        return;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if results.len() >= MAX_LISTED_ENTRIES {
                break;
            }
            let path = entry.path();
            let relative_path = relative.join(entry.file_name());
            let name = entry.file_name().to_string_lossy().to_string();

            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_file() && name.contains(pattern) {
                if let Some(path) = relative_path.to_str() {
                    results.push(path.to_string());
                }
            }
            if file_type.is_dir() && !name.starts_with('.') {
                search_recursive(
                    &path,
                    &relative_path,
                    pattern,
                    results,
                    depth + 1,
                    max_depth,
                );
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
    use std::collections::BTreeSet;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct RemoveOnDrop(PathBuf);

    impl Drop for RemoveOnDrop {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn temp_root(label: &str) -> (RemoveOnDrop, PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let base = env::temp_dir().join(format!("rnk-glm-{label}-{}-{unique}", std::process::id()));
        fs::create_dir_all(base.join("root")).expect("root created");
        let root = base.join("root").canonicalize().expect("canonical root");
        (RemoveOnDrop(base), root)
    }

    fn search_in_root(root: &Path, input: &Value) -> Result<String, String> {
        let ToolAuthorization::Prompt {
            root: _root,
            #[cfg(unix)]
            search_root,
        } = ToolAuthorization::prompt(root.to_path_buf()).expect("root held")
        else {
            unreachable!("prompt authorization");
        };
        #[cfg(not(unix))]
        let search_root = _root;
        search_files(&search_root, input)
    }

    fn listed_names(result: &str) -> BTreeSet<&str> {
        if result.is_empty() {
            BTreeSet::new()
        } else {
            result.split(", ").collect()
        }
    }

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
        let authorization = ToolAuthorization::prompt(root.canonicalize().expect("canonical root"))
            .expect("root held");
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
    fn approved_search_files_returns_usable_matched_paths() {
        let (_scratch, root) = temp_root("search-paths");
        let nested = root.join("sub");
        fs::create_dir(&nested).expect("sub created");
        let expected_paths = [root.join("match.txt"), nested.join("match-child.txt")];
        for path in &expected_paths {
            fs::write(path, "found\n").expect("matching file written");
        }
        fs::write(root.join("other.txt"), "other\n").expect("nonmatching file written");
        let authorization = ToolAuthorization::prompt(root.clone()).expect("root held");

        let decision = authorization.review_and_execute_with(
            "search_files",
            &json!({"pattern": "match"}),
            |_| true,
        );
        assert!(matches!(decision, ToolDecision::Executed(_)));
        let (count, encoded) = decision.result().split_once('\n').expect("count and paths");
        assert_eq!(count, "Found 2 files");
        let paths: Vec<PathBuf> = serde_json::from_str(encoded).expect("JSON paths");
        let paths: BTreeSet<_> = paths.into_iter().collect();
        let expected_paths =
            expected_paths.map(|path| path.strip_prefix(&root).expect("in root").to_path_buf());
        assert_eq!(paths, BTreeSet::from(expected_paths));
        for path in paths {
            execute_tool(&root, "read_file", &json!({"path": path}))
                .expect("returned path can be read");
        }
    }

    #[cfg(unix)]
    #[test]
    fn approved_search_files_keeps_root_after_ancestor_replacement() {
        use std::os::unix::fs::symlink;

        let (_scratch, base_root) = temp_root("search-ancestor");
        let ancestor = base_root.join("ancestor");
        let root = ancestor.join("approved");
        let outside = base_root.join("outside");
        fs::create_dir_all(&root).expect("approved root created");
        fs::create_dir_all(outside.join("approved")).expect("outside root created");
        fs::write(root.join("inside-match.txt"), "inside\n").expect("inside file written");
        fs::write(outside.join("approved/secret-match.txt"), "secret\n")
            .expect("outside file written");
        let authorization = ToolAuthorization::prompt(root.clone()).expect("root held");

        fs::rename(&ancestor, base_root.join("displaced-ancestor")).expect("ancestor displaced");
        symlink(&outside, &ancestor).expect("ancestor replaced with symlink");
        for _ in 0..2 {
            let decision = authorization.review_and_execute_with(
                "search_files",
                &json!({"pattern": "match"}),
                |_| true,
            );
            assert!(matches!(decision, ToolDecision::Executed(_)));
            assert!(
                !decision.result().contains("secret-match"),
                "search reopened the replaced ancestor: {}",
                decision.result()
            );
            let (count, encoded) = decision.result().split_once('\n').expect("count and paths");
            assert_eq!(count, "Found 1 files");
            let paths: Vec<String> = serde_json::from_str(encoded).expect("JSON paths");
            assert_eq!(paths, ["inside-match.txt"]);
        }
        assert!(
            ToolAuthorization::prompt(root).is_err(),
            "initialization must reject a symlink in the root's ancestor"
        );
    }

    // APFS rejects non-UTF-8 names; this fixture needs a Linux filesystem.
    #[cfg(target_os = "linux")]
    #[test]
    fn approved_search_files_returns_relative_paths_from_non_utf8_root() {
        use std::os::unix::ffi::OsStringExt;

        let (_scratch, base) = temp_root("search-non-utf8-root");
        let root = base.join(OsString::from_vec(b"approved-\xff".to_vec()));
        fs::create_dir_all(root.join("sub")).expect("root and child created");
        fs::write(root.join("match.txt"), "root\n").expect("root file written");
        fs::write(root.join("sub/match-child.txt"), "child\n").expect("child file written");
        let authorization = ToolAuthorization::prompt(root.clone()).expect("root held");

        let decision = authorization.review_and_execute_with(
            "search_files",
            &json!({"pattern": "match"}),
            |_| true,
        );
        assert!(matches!(decision, ToolDecision::Executed(_)));
        let (count, encoded) = decision.result().split_once('\n').expect("count and paths");
        assert_eq!(count, "Found 2 files");
        let paths: Vec<String> = serde_json::from_str(encoded).expect("JSON paths");
        let paths: BTreeSet<_> = paths.into_iter().collect();
        assert_eq!(
            paths,
            BTreeSet::from(["match.txt".to_string(), "sub/match-child.txt".to_string()])
        );
        for path in paths {
            execute_tool(&root, "read_file", &json!({"path": path}))
                .expect("relative result can be read under a non-UTF-8 root");
        }
    }

    #[test]
    fn search_files_returns_at_most_twenty_matched_paths() {
        let (_scratch, root) = temp_root("search-cap");
        let mut expected_paths = BTreeSet::new();
        for index in 0..25 {
            let path = root.join(format!("match-{index:02}.txt"));
            fs::write(&path, "found\n").expect("matching file written");
            expected_paths.insert(path.strip_prefix(&root).expect("in root").to_path_buf());
        }

        let result = search_in_root(&root, &json!({"pattern": "match"})).expect("search succeeds");
        let (count, encoded) = result.split_once('\n').expect("count and paths");
        assert_eq!(count, "Found 20 files");
        let paths: Vec<PathBuf> = serde_json::from_str(encoded).expect("JSON paths");
        let paths: BTreeSet<_> = paths.into_iter().collect();
        assert_eq!(paths.len(), 20);
        assert!(paths.is_subset(&expected_paths));
    }

    #[cfg(unix)]
    #[test]
    fn approved_search_files_round_trips_delimiters_in_paths() {
        let (_scratch, root) = temp_root("search-delimiters");
        let path = root.join("match\nchild\"\\.txt");
        fs::write(&path, "found\n").expect("matching file written");
        let authorization = ToolAuthorization::prompt(root.clone()).expect("root held");

        let decision = authorization.review_and_execute_with(
            "search_files",
            &json!({"pattern": "match"}),
            |_| true,
        );
        assert!(matches!(decision, ToolDecision::Executed(_)));
        let (count, encoded) = decision.result().split_once('\n').expect("count and paths");
        assert_eq!(count, "Found 1 files");
        let paths: Vec<String> = serde_json::from_str(encoded).expect("unambiguous JSON paths");
        assert_eq!(
            paths,
            [path
                .strip_prefix(&root)
                .expect("in root")
                .to_str()
                .expect("UTF-8 path")]
        );
        execute_tool(&root, "read_file", &json!({"path": paths[0]}))
            .expect("decoded path can be read");
    }

    #[test]
    fn search_files_counts_only_regular_files_toward_the_cap() {
        let (_scratch, root) = temp_root("search-types");
        for index in 0..25 {
            fs::create_dir(root.join(format!("match-dir-{index:02}")))
                .expect("matching directory created");
        }
        let nested = root.join("match-dir-00");
        let expected_paths = [root.join("match.txt"), nested.join("match-child.txt")];
        for path in &expected_paths {
            fs::write(path, "found\n").expect("matching file written");
        }
        #[cfg(unix)]
        for index in 0..25 {
            std::os::unix::fs::symlink("match.txt", root.join(format!("match-link-{index:02}")))
                .expect("matching symlink created");
        }

        let result = search_in_root(&root, &json!({"pattern": "match"})).expect("search succeeds");
        let (count, encoded) = result.split_once('\n').expect("count and paths");
        assert_eq!(count, "Found 2 files");
        let paths: Vec<PathBuf> = serde_json::from_str(encoded).expect("JSON paths");
        let paths: BTreeSet<_> = paths.into_iter().collect();
        let expected_paths =
            expected_paths.map(|path| path.strip_prefix(&root).expect("in root").to_path_buf());
        assert_eq!(paths, BTreeSet::from(expected_paths));
    }

    #[cfg(unix)]
    #[test]
    fn recursive_search_rejects_a_directory_swapped_for_a_symlink() {
        use std::os::unix::fs::symlink;

        let (_scratch, root) = temp_root("search-swapped-dir");
        let child = root.join("child");
        let outside = root.parent().expect("scratch").join("outside");
        fs::create_dir(&child).expect("child created");
        fs::create_dir(&outside).expect("outside created");
        fs::write(outside.join("secret-match.txt"), "secret").expect("secret written");
        assert!(
            fs::symlink_metadata(&child)
                .expect("child metadata")
                .is_dir()
        );
        fs::rename(&child, root.join("displaced-child")).expect("child displaced");
        symlink(&outside, &child).expect("child replaced with symlink");

        assert!(
            ToolAuthorization::prompt(child).is_err(),
            "swapped child rejected"
        );
        let authorization = ToolAuthorization::prompt(root.clone()).expect("root held");

        let displaced = root.parent().expect("scratch").join("displaced-root");
        fs::rename(&root, &displaced).expect("root displaced");
        symlink(&outside, &root).expect("root replaced with symlink");
        let decision = authorization.review_and_execute_with(
            "search_files",
            &json!({"pattern": "secret-match"}),
            |_| true,
        );
        assert_eq!(
            decision,
            ToolDecision::Executed("No files found".to_string())
        );
    }

    #[cfg(unix)]
    #[test]
    fn recursive_search_keeps_held_directories_after_path_swaps() {
        use std::os::fd::AsRawFd;
        use std::os::unix::fs::symlink;

        let (_scratch, root) = temp_root("search-held-dirs");
        let base = root.parent().expect("scratch");
        let child = root.join("child");
        let outside = base.join("outside");
        fs::create_dir(&child).expect("child created");
        fs::create_dir(&outside).expect("outside created");
        fs::write(root.join("inside-match.txt"), "inside").expect("root file written");
        fs::write(child.join("child-match.txt"), "inside").expect("child file written");
        fs::write(outside.join("secret-match.txt"), "secret").expect("secret written");
        let approved = approve_confined_path(&root, &json!({"path": "."})).expect("root approved");
        let root_file = open_nofollow(&root, &approved, OpenMode::Directory).expect("root held");
        let child_fd =
            open_child_nofollow(root_file.as_raw_fd(), "child".as_ref(), OpenMode::Directory)
                .expect("child held");

        fs::rename(&child, base.join("displaced-child")).expect("child displaced");
        symlink(&outside, &child).expect("child replaced with symlink");
        assert!(
            open_child_nofollow(root_file.as_raw_fd(), "child".as_ref(), OpenMode::Directory)
                .is_err(),
            "a child swapped after its type check must not be opened"
        );
        fs::rename(&root, base.join("displaced-root")).expect("root displaced");
        symlink(&outside, &root).expect("root replaced with symlink");

        let mut results = Vec::new();
        search_from_owned_fd(root_file.into(), Path::new(""), "match", &mut results, 0, 3);
        search_from_owned_fd(child_fd, Path::new("child"), "match", &mut results, 1, 3);
        assert_eq!(results, ["inside-match.txt", "child/child-match.txt"]);
    }

    #[test]
    fn search_files_preserves_no_match_and_invalid_pattern_results() {
        let (_scratch, root) = temp_root("search-empty");
        let result = search_in_root(&root, &json!({"pattern": "missing"}))
            .expect("no-match search succeeds");
        assert_eq!(result, "No files found");
        for input in [json!({}), json!({"pattern": ""}), json!({"pattern": 1})] {
            let error = search_in_root(&root, &input).expect_err("invalid pattern");
            assert_eq!(error, "search_files requires a non-empty string pattern");
        }
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

        let relative = execute_tool(&root, "read_file", &json!({"path": "../Cargo.toml"}))
            .expect_err("parent escape denied");
        assert!(
            relative.contains("escapes the approved tool root"),
            "unexpected parent escape error: {relative}"
        );
    }

    #[test]
    fn empty_tool_path_is_rejected() {
        let root = env::temp_dir();
        for tool in ["read_file", "list_files"] {
            let missing = execute_tool(&root, tool, &json!({})).expect_err("missing path");
            assert_eq!(missing, "tool requires a non-empty string path");
            let empty = execute_tool(&root, tool, &json!({"path": ""})).expect_err("empty path");
            assert_eq!(empty, "tool requires a non-empty string path");
        }
    }

    #[test]
    fn in_root_file_and_directory_still_open() {
        let (_scratch, root) = temp_root("ok");
        fs::write(root.join("note.txt"), "hello\n").expect("note written");
        fs::create_dir(root.join("sub")).expect("sub created");
        fs::write(root.join("sub").join("a.txt"), "a\n").expect("child written");

        let expected = format!("Read 1 lines from {}", root.join("note.txt").display());
        let read =
            execute_tool(&root, "read_file", &json!({"path": "note.txt"})).expect("relative read");
        assert_eq!(read, expected);
        let dotted =
            execute_tool(&root, "read_file", &json!({"path": "./note.txt"})).expect("dot read");
        assert_eq!(dotted, expected);
        let parent = execute_tool(&root, "read_file", &json!({"path": "sub/../note.txt"}))
            .expect("parent read");
        assert_eq!(parent, expected);
        let absolute = execute_tool(&root, "read_file", &json!({"path": root.join("note.txt")}))
            .expect("absolute read");
        assert_eq!(absolute, expected);

        let listed = execute_tool(&root, "list_files", &json!({"path": "."})).expect("list dot");
        assert_eq!(listed_names(&listed), BTreeSet::from(["note.txt", "sub/"]));
        let listed_root =
            execute_tool(&root, "list_files", &json!({"path": root})).expect("list absolute root");
        assert_eq!(listed_names(&listed_root), listed_names(&listed));
        let listed_sub =
            execute_tool(&root, "list_files", &json!({"path": "sub"})).expect("list sub");
        assert_eq!(listed_sub, "a.txt");
    }

    #[test]
    fn directory_listing_is_capped_at_twenty_names() {
        let (_scratch, root) = temp_root("cap");
        for index in 0..25 {
            fs::write(root.join(format!("f{index:02}")), "x").expect("fixture written");
        }

        let listed = execute_tool(&root, "list_files", &json!({"path": "."})).expect("list");
        let names = listed_names(&listed);
        assert_eq!(names.len(), MAX_LISTED_ENTRIES);
        assert!(names.is_disjoint(&BTreeSet::from([".", ".."])));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_components_fail_closed_even_inside_the_root() {
        use std::os::unix::fs::symlink;

        let (_scratch, root) = temp_root("inside-link");
        fs::write(root.join("inside.txt"), "inside\n").expect("inside written");
        fs::create_dir(root.join("real")).expect("real created");
        fs::write(root.join("real").join("inside.txt"), "inside\n").expect("child written");
        symlink("inside.txt", root.join("alias.txt")).expect("file symlink");
        symlink("real", root.join("via")).expect("directory symlink");

        let alias = execute_tool(&root, "read_file", &json!({"path": "alias.txt"}))
            .expect_err("final symlink denied");
        let through = execute_tool(&root, "read_file", &json!({"path": "via/inside.txt"}))
            .expect_err("intermediate symlink denied");
        let listed = execute_tool(&root, "list_files", &json!({"path": "via"}))
            .expect_err("symlink directory denied");
        assert!(!alias.contains("Read "));
        assert!(!through.contains("Read "));
        assert!(!listed.contains("inside.txt"));

        let real = execute_tool(&root, "read_file", &json!({"path": "inside.txt"}))
            .expect("regular file still opens");
        assert!(real.starts_with("Read 1 lines from "));
        let names =
            execute_tool(&root, "list_files", &json!({"path": "."})).expect("list real directory");
        let names = listed_names(&names);
        assert!(names.contains("alias.txt"));
        assert!(names.contains("via"));
        assert!(names.contains("real/"));
        assert!(!names.contains("via/"));
    }

    #[cfg(unix)]
    #[test]
    fn swapped_final_component_is_not_followed() {
        use std::os::unix::fs::symlink;

        let (_scratch, root) = temp_root("toctou");
        let outside = root.parent().expect("scratch").join("outside");
        fs::create_dir_all(&outside).expect("outside created");
        fs::write(outside.join("secret.txt"), "secret\n").expect("secret written");
        fs::write(outside.join("outside-secret.txt"), "name\n").expect("outside name written");

        fs::write(root.join("approved.txt"), "ok\n").expect("approved file written");
        assert!(root.join("approved.txt").is_file());
        assert!(
            !root
                .join("approved.txt")
                .symlink_metadata()
                .expect("metadata")
                .file_type()
                .is_symlink()
        );
        let approved_file =
            approve_confined_path(&root, &json!({"path": "approved.txt"})).expect("file approved");
        fs::remove_file(root.join("approved.txt")).expect("approved file removed");
        symlink(outside.join("secret.txt"), root.join("approved.txt")).expect("file symlink");
        let followed = fs::read_to_string(root.join("approved.txt")).expect("std open follows");
        assert_eq!(followed, "secret\n");
        let read_error =
            read_approved(&root, &approved_file).expect_err("read_file open denies the swap");
        assert!(!read_error.contains("secret"));

        fs::create_dir(root.join("approved-dir")).expect("approved dir created");
        assert!(root.join("approved-dir").is_dir());
        let approved_dir = approve_confined_path(&root, &json!({"path": "approved-dir"}))
            .expect("directory approved");
        fs::remove_dir(root.join("approved-dir")).expect("approved dir removed");
        symlink(&outside, root.join("approved-dir")).expect("dir symlink");
        let followed_names: Vec<_> = fs::read_dir(root.join("approved-dir"))
            .expect("std list follows")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            followed_names
                .iter()
                .any(|name| name == "outside-secret.txt")
        );
        let list_error =
            list_approved(&root, &approved_dir).expect_err("list_files open denies the swap");
        assert!(!list_error.contains("outside-secret.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn swapped_root_directory_is_not_followed() {
        use std::os::unix::fs::symlink;

        let (_scratch, root) = temp_root("root-swap");
        let base = root.parent().expect("scratch").to_path_buf();
        fs::write(root.join("note.txt"), "ok\n").expect("note written");
        let outside = base.join("outside");
        fs::create_dir_all(&outside).expect("outside created");
        fs::write(outside.join("secret.txt"), "secret\n").expect("secret written");
        fs::write(outside.join("outside-secret.txt"), "name\n").expect("outside name written");

        let approved_file =
            approve_confined_path(&root, &json!({"path": "note.txt"})).expect("file approved");
        let approved_dir =
            approve_confined_path(&root, &json!({"path": "."})).expect("directory approved");

        let displaced = base.join("displaced-root");
        fs::rename(&root, &displaced).expect("root displaced");
        symlink(&outside, &root).expect("root replaced with symlink");
        assert!(
            root.symlink_metadata()
                .expect("root metadata")
                .file_type()
                .is_symlink()
        );

        let followed =
            fs::read_to_string(root.join("secret.txt")).expect("std open follows root symlink");
        assert_eq!(followed, "secret\n");
        let followed_names: Vec<_> = fs::read_dir(&root)
            .expect("std list follows root symlink")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            followed_names
                .iter()
                .any(|name| name == "outside-secret.txt")
        );

        let read_error =
            read_approved(&root, &approved_file).expect_err("read_file denies swapped root");
        assert!(
            read_error.starts_with("cannot read "),
            "unexpected read error: {read_error}"
        );
        assert!(!read_error.contains("secret"));
        let tool_read = execute_tool(&root, "read_file", &json!({"path": "note.txt"}))
            .expect_err("read_file tool denies swapped root");
        assert!(
            tool_read.starts_with("cannot read "),
            "unexpected tool read error: {tool_read}"
        );
        assert!(!tool_read.contains("secret"));

        let list_error =
            list_approved(&root, &approved_dir).expect_err("list_files denies swapped root");
        assert!(
            list_error.starts_with("cannot list "),
            "unexpected list error: {list_error}"
        );
        assert!(!list_error.contains("outside-secret.txt"));
        assert!(!list_error.contains("secret"));
        let tool_list = execute_tool(&root, "list_files", &json!({"path": "."}))
            .expect_err("list_files tool denies swapped root");
        assert!(
            tool_list.starts_with("cannot list "),
            "unexpected tool list error: {tool_list}"
        );
        assert!(!tool_list.contains("outside-secret.txt"));
        assert!(!tool_list.contains("secret"));
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
        let root = root.canonicalize().expect("canonical root");
        fs::create_dir_all(&outside).expect("outside created");
        fs::write(outside.join("secret-match.txt"), "secret").expect("fixture written");
        symlink(&outside, root.join("escape-link")).expect("symlink created");

        let result =
            search_in_root(&root, &json!({"pattern": "secret-match"})).expect("search succeeds");

        let cleanup = fs::remove_dir_all(&base);
        assert!(
            result == "No files found",
            "search escaped through symlink: {result}"
        );
        cleanup.expect("fixture removed");
    }
}
