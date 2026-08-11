//! GLM provider adapter over rnk's typed conversation and inline shell.
//!
//! The model produces inert `ToolCallContent`. A separate, default-deny
//! `PendingToolRequest` requires an exact human approval before a one-shot
//! workspace operation. Missing `GLM_API_KEY` fails before a client or request
//! is created. The earlier out-of-renderer prompt module is intentionally gone;
//! stdin is only the provider adapter's transport, while public rnk state owns
//! conversation, view projection, and scrollback publication.
//!
//! Run with: `GLM_API_KEY=... cargo run --example glm_chat`

use std::env;
use std::error::Error;
use std::ffi::{CStr, CString};
use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Component, Path};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::os::unix::ffi::OsStrExt;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::os::unix::fs::OpenOptionsExt;

use reqwest::Client;
use rnk::components::chat::scrollback::NativeTerminalSink;
use rnk::components::chat::{
    BlockId, ChatMessage, ChatMessageView, ChatRole, ConversationEvent, ConversationGuard,
    ConversationState, ConversationUpdate, InlineChatShell, InlineCommitReport, MessageBlock,
    MessageBlockEntry, MessageId, MessageMutationGuard, ProjectionContext, ScrollbackNamespace,
    ThemeIdentity, ThinkingContent, ThinkingId, ThinkingStatus, ToolArgument, ToolCallContent,
    ToolCallId, ToolCallStatus, ToolResultContent, ToolResultStatus, TypedValue, UpdateId,
};
#[cfg(test)]
use rnk::components::chat::{DecimalValue, TypedField};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const API_URL: &str = "https://open.bigmodel.cn/api/anthropic/v1/messages";
const MAX_TOOL_DEPTH: usize = 3;
const MAX_TOOL_ENTRIES: usize = 20;
const MAX_TOOL_BYTES: usize = 64 * 1024;
const MAX_TOOL_CYCLES: usize = 8;
const MAX_RESPONSE_BLOCKS: usize = 64;

pub(crate) type AppResult<T> = Result<T, Box<dyn Error + Send + Sync>>;
type OutputShell = InlineChatShell<NativeTerminalSink<io::Stdout>>;

#[derive(Serialize, Clone)]
struct ChatRequest {
    model: &'static str,
    max_tokens: u32,
    messages: Vec<MessageParam>,
    tools: Vec<ToolDefinition>,
}

#[derive(Serialize, Clone)]
struct MessageParam {
    role: &'static str,
    content: Vec<ProviderBlock>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
enum ProviderBlock {
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
struct ToolDefinition {
    name: &'static str,
    description: &'static str,
    input_schema: Value,
}

#[derive(Deserialize)]
struct ChatResponse {
    content: Vec<ResponseBlock>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ResponseBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
}

pub(crate) struct ProviderAdapter {
    client: Client,
    api_key: String,
}

impl ProviderAdapter {
    fn from_environment() -> AppResult<Self> {
        Self::from_optional_key(env::var("GLM_API_KEY").ok(), || {
            Ok(Client::builder().build()?)
        })
    }

    pub(crate) fn from_optional_key(
        api_key: Option<String>,
        build_client: impl FnOnce() -> AppResult<Client>,
    ) -> AppResult<Self> {
        let api_key = api_key.ok_or(ProviderError::MissingApiKey)?;
        if api_key.trim().is_empty() {
            return Err(ProviderError::MissingApiKey.into());
        }
        let client = build_client()?;
        Ok(Self { client, api_key })
    }

    async fn send(&self, state: &ConversationState) -> AppResult<ChatResponse> {
        let request = ChatRequest {
            model: "claude-3-5-sonnet-20241022",
            max_tokens: 8192,
            messages: provider_messages(state)?,
            tools: tool_definitions(),
        };
        let response = self
            .client
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(ProviderError::HttpStatus(response.status().as_u16()).into());
        }
        Ok(response.json().await?)
    }
}

#[derive(Debug)]
enum ProviderError {
    MissingApiKey,
    HttpStatus(u16),
    UnsupportedBlock,
    InvalidToolInput,
    ToolCycleLimit,
}

impl fmt::Display for ProviderError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingApiKey => {
                output.write_str("GLM_API_KEY is required; no request was created")
            }
            Self::HttpStatus(status) => write!(output, "provider returned HTTP status {status}"),
            Self::UnsupportedBlock => {
                output.write_str("conversation contains an unsupported provider block")
            }
            Self::InvalidToolInput => {
                output.write_str("provider tool input is not a closed string object")
            }
            Self::ToolCycleLimit => {
                output.write_str("provider exceeded the closed tool-cycle limit")
            }
        }
    }
}

impl Error for ProviderError {}

#[derive(Debug)]
pub(crate) enum ToolError {
    Denied,
    WrongApproval,
    AlreadyExecuted,
    UnknownTool,
    InvalidPath,
    Symlink,
    DepthLimit,
    EntryLimit,
    ByteLimit,
    InvalidUtf8,
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    UnsupportedPlatform,
    Io(io::Error),
}

impl fmt::Display for ToolError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(match self {
            Self::Denied => "tool request is denied by default",
            Self::WrongApproval => "approval did not match the exact tool call",
            Self::AlreadyExecuted => "tool request has already executed",
            Self::UnknownTool => "tool name is not allowed",
            Self::InvalidPath => "tool path must be a relative path without traversal",
            Self::Symlink => "tool traversal encountered a symbolic link",
            Self::DepthLimit => "tool traversal exceeded the depth limit",
            Self::EntryLimit => "tool traversal exceeded the entry limit",
            Self::ByteLimit => "tool output exceeded the byte limit",
            Self::InvalidUtf8 => "tool file or path is not valid UTF-8",
            #[cfg(not(any(target_os = "linux", target_os = "macos")))]
            Self::UnsupportedPlatform => {
                "secure descriptor-relative workspace tools are unavailable on this platform"
            }
            Self::Io(_) => "workspace I/O failed",
        })
    }
}

impl Error for ToolError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(source) => Some(source),
            _ => None,
        }
    }
}

impl From<io::Error> for ToolError {
    fn from(source: io::Error) -> Self {
        Self::Io(source)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolDecision {
    Denied,
    Approved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolName {
    ReadFile,
    ListFiles,
    SearchFiles,
}

impl ToolName {
    fn parse(value: &str) -> Result<Self, ToolError> {
        match value {
            "read_file" => Ok(Self::ReadFile),
            "list_files" => Ok(Self::ListFiles),
            "search_files" => Ok(Self::SearchFiles),
            _ => Err(ToolError::UnknownTool),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::ReadFile => "read_file",
            Self::ListFiles => "list_files",
            Self::SearchFiles => "search_files",
        }
    }

    const fn argument_name(self) -> &'static str {
        match self {
            Self::ReadFile | Self::ListFiles => "path",
            Self::SearchFiles => "pattern",
        }
    }
}

pub(crate) struct PendingToolRequest {
    call_id: ToolCallId,
    name: ToolName,
    argument: String,
    exact_description: String,
    decision: ToolDecision,
    executed: bool,
}

impl PendingToolRequest {
    pub(crate) fn parse(call_id: &str, name: &str, input: &Value) -> Result<Self, ToolError> {
        validate_call_id(call_id)?;
        let name = ToolName::parse(name)?;
        let fields = string_object(input).map_err(|_| ToolError::InvalidPath)?;
        if fields.len() != 1 || fields[0].0 != name.argument_name() {
            return Err(ToolError::InvalidPath);
        }
        let argument = fields[0].1.clone();
        validate_tool_argument(name, &argument)?;
        let call_id = ToolCallId::new(call_id).map_err(|_| ToolError::InvalidPath)?;
        let exact_description = format!(
            "tool={} call_id_hex={} {}_len={} {}_hex={}",
            name.as_str(),
            hex_bytes(call_id.as_str().as_bytes()),
            name.argument_name(),
            argument.len(),
            name.argument_name(),
            hex_bytes(argument.as_bytes()),
        );
        Ok(Self {
            call_id,
            name,
            argument,
            exact_description,
            decision: ToolDecision::Denied,
            executed: false,
        })
    }

    pub(crate) fn approval_phrase(&self) -> String {
        format!("approve {}", self.exact_description)
    }

    pub(crate) fn exact_description(&self) -> &str {
        &self.exact_description
    }

    pub(crate) fn approve_exact(&mut self, supplied: &str) -> Result<(), ToolError> {
        if supplied != self.approval_phrase() {
            return Err(ToolError::WrongApproval);
        }
        self.decision = ToolDecision::Approved;
        Ok(())
    }

    pub(crate) fn execute_once(&mut self, workspace: &Workspace) -> Result<String, ToolError> {
        if self.decision != ToolDecision::Approved {
            return Err(ToolError::Denied);
        }
        if self.executed {
            return Err(ToolError::AlreadyExecuted);
        }
        self.executed = true;
        match self.name {
            ToolName::ReadFile => workspace.read_file(&self.argument),
            ToolName::ListFiles => workspace.list_files(&self.argument),
            ToolName::SearchFiles => workspace.search_files(&self.argument),
        }
    }
}

pub(crate) struct Workspace {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    root: fs::File,
}

impl Workspace {
    fn current() -> Result<Self, ToolError> {
        Self::from_root(Path::new("."))
    }

    pub(crate) fn from_root(root: &Path) -> Result<Self, ToolError> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let root = fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(root)?;
            Ok(Self { root })
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = root;
            Err(ToolError::UnsupportedPlatform)
        }
    }

    fn read_file(&self, path: &str) -> Result<String, ToolError> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        let mut file = self.open_relative(path, false)?;
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        return Err(ToolError::UnsupportedPlatform);
        let mut bytes = Vec::with_capacity(MAX_TOOL_BYTES.min(8192));
        Read::by_ref(&mut file)
            .take(u64::try_from(MAX_TOOL_BYTES).unwrap() + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_TOOL_BYTES {
            return Err(ToolError::ByteLimit);
        }
        String::from_utf8(bytes).map_err(|_| ToolError::InvalidUtf8)
    }

    fn list_files(&self, path: &str) -> Result<String, ToolError> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        let directory = self.open_relative(path, true)?;
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        return Err(ToolError::UnsupportedPlatform);
        let mut names = Vec::new();
        let mut visited = 0;
        for item in read_directory(&directory, &mut visited)? {
            if item.kind == EntryKind::Symlink {
                return Err(ToolError::Symlink);
            }
            names.push(item.name);
        }
        names.sort();
        bounded_join(names)
    }

    pub(crate) fn search_files(&self, pattern: &str) -> Result<String, ToolError> {
        if pattern.is_empty() || pattern.len() > 128 {
            return Err(ToolError::InvalidPath);
        }
        let mut found = Vec::new();
        let mut visited = 0;
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        self.search_directory(&self.root, "", pattern, 0, &mut visited, &mut found)?;
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        return Err(ToolError::UnsupportedPlatform);
        found.sort();
        bounded_join(found)
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn search_directory(
        &self,
        directory: &fs::File,
        prefix: &str,
        pattern: &str,
        depth: usize,
        visited: &mut usize,
        found: &mut Vec<String>,
    ) -> Result<(), ToolError> {
        if depth > MAX_TOOL_DEPTH {
            return Err(ToolError::DepthLimit);
        }
        for item in read_directory(directory, visited)? {
            if item.kind == EntryKind::Symlink {
                return Err(ToolError::Symlink);
            }
            let name = if prefix.is_empty() {
                item.name.clone()
            } else {
                format!("{prefix}/{}", item.name)
            };
            if name.contains(pattern) {
                found.push(name.clone());
            }
            if item.kind == EntryKind::Directory {
                if depth == MAX_TOOL_DEPTH {
                    return Err(ToolError::DepthLimit);
                }
                let child = open_at(directory.as_raw_fd(), &item.raw_name, true)?;
                self.search_directory(&child, &name, pattern, depth + 1, visited, found)?;
            }
        }
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn open_relative(&self, supplied: &str, directory: bool) -> Result<fs::File, ToolError> {
        let parts = relative_components(supplied)?;
        if parts.is_empty() {
            if directory {
                return duplicate_file(&self.root);
            }
            return Err(ToolError::InvalidPath);
        }
        let mut current = duplicate_file(&self.root)?;
        for (index, part) in parts.iter().enumerate() {
            let last = index + 1 == parts.len();
            current = open_at(current.as_raw_fd(), part, !last || directory)?;
        }
        let metadata = current.metadata()?;
        if (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
            return Err(ToolError::InvalidPath);
        }
        Ok(current)
    }
}

fn validate_call_id(value: &str) -> Result<(), ToolError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
    {
        return Err(ToolError::InvalidPath);
    }
    Ok(())
}

fn validate_tool_argument(name: ToolName, value: &str) -> Result<(), ToolError> {
    let limit = if name == ToolName::SearchFiles {
        128
    } else {
        512
    };
    if value.is_empty() || value.len() > limit || value.chars().any(char::is_control) {
        return Err(ToolError::InvalidPath);
    }
    if name != ToolName::SearchFiles {
        let path = Path::new(value);
        if path.is_absolute()
            || path.components().any(|part| {
                !matches!(part, Component::Normal(_))
                    && !(value == "." && matches!(part, Component::CurDir))
            })
        {
            return Err(ToolError::InvalidPath);
        }
    }
    Ok(())
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
struct DirectoryEntry {
    name: String,
    raw_name: CString,
    kind: EntryKind,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
struct DirectoryStream(*mut libc::DIR);

#[cfg(any(target_os = "linux", target_os = "macos"))]
impl Drop for DirectoryStream {
    fn drop(&mut self) {
        // SAFETY: the pointer came from `fdopendir`, remains uniquely owned,
        // and `closedir` consumes both the stream and its descriptor.
        unsafe {
            libc::closedir(self.0);
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn duplicate_file(file: &fs::File) -> Result<fs::File, ToolError> {
    // SAFETY: `file` owns a live descriptor. `F_DUPFD_CLOEXEC` returns a new,
    // independently owned descriptor or -1 without changing the source.
    let descriptor = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 3) };
    if descriptor < 0 {
        return Err(io::Error::last_os_error().into());
    }
    // SAFETY: the successful `fcntl` result is a new owned descriptor.
    Ok(unsafe { fs::File::from_raw_fd(descriptor) })
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn relative_components(value: &str) -> Result<Vec<CString>, ToolError> {
    if value == "." {
        return Ok(Vec::new());
    }
    validate_tool_argument(ToolName::ReadFile, value)?;
    Path::new(value)
        .components()
        .map(|part| match part {
            Component::Normal(value) => {
                CString::new(value.as_bytes()).map_err(|_| ToolError::InvalidPath)
            }
            _ => Err(ToolError::InvalidPath),
        })
        .collect()
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn open_at(parent: RawFd, name: &CStr, directory: bool) -> Result<fs::File, ToolError> {
    let flags = libc::O_RDONLY
        | libc::O_CLOEXEC
        | libc::O_NOFOLLOW
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: `parent` is a live directory descriptor and `name` is a bounded,
    // NUL-terminated single path component. No raw pointer outlives this call.
    let descriptor = unsafe { libc::openat(parent, name.as_ptr(), flags) };
    if descriptor < 0 {
        let error = io::Error::last_os_error();
        return if error.raw_os_error() == Some(libc::ELOOP) {
            Err(ToolError::Symlink)
        } else {
            Err(error.into())
        };
    }
    // SAFETY: successful `openat` returned a new owned descriptor.
    Ok(unsafe { fs::File::from_raw_fd(descriptor) })
}

#[cfg(target_os = "linux")]
unsafe fn errno_location() -> *mut libc::c_int {
    // SAFETY: forwarded to libc's thread-local errno accessor.
    unsafe { libc::__errno_location() }
}

#[cfg(target_os = "macos")]
unsafe fn errno_location() -> *mut libc::c_int {
    // SAFETY: forwarded to libc's thread-local errno accessor.
    unsafe { libc::__error() }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn read_directory(
    directory: &fs::File,
    visited: &mut usize,
) -> Result<Vec<DirectoryEntry>, ToolError> {
    let current = CString::new(".").expect("a literal dot has no NUL byte");
    // Re-open the descriptor-relative directory instead of duplicating its fd:
    // duplicated directory descriptors share one seek position and would make a
    // second traversal start at EOF, silently bypassing global traversal limits.
    let reopened = open_at(directory.as_raw_fd(), &current, true)?;
    // SAFETY: ownership of this independently opened descriptor transfers to
    // `fdopendir`.
    let stream = unsafe { libc::fdopendir(reopened.into_raw_fd()) };
    if stream.is_null() {
        return Err(io::Error::last_os_error().into());
    }
    let stream = DirectoryStream(stream);
    let mut output = Vec::new();
    loop {
        // SAFETY: errno is thread-local and the stream is exclusively owned.
        unsafe {
            *errno_location() = 0;
        }
        // SAFETY: the stream stays alive and unmoved for the duration of this call.
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            // SAFETY: errno is read immediately after `readdir` returned null.
            let errno = unsafe { *errno_location() };
            if errno != 0 {
                return Err(io::Error::from_raw_os_error(errno).into());
            }
            break;
        }
        // SAFETY: `d_name` is NUL-terminated for a successful `readdir` result
        // and remains valid until the next call on this same stream.
        let raw = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        if raw.to_bytes() == b"." || raw.to_bytes() == b".." {
            continue;
        }
        *visited = visited.checked_add(1).ok_or(ToolError::EntryLimit)?;
        if *visited > MAX_TOOL_ENTRIES {
            return Err(ToolError::EntryLimit);
        }
        let raw_name = raw.to_owned();
        let name = raw.to_str().map_err(|_| ToolError::InvalidUtf8)?.to_owned();
        if name.chars().any(char::is_control) {
            return Err(ToolError::InvalidPath);
        }
        // SAFETY: `directory` and `raw_name` are live; `AT_SYMLINK_NOFOLLOW`
        // inspects the directory entry itself rather than following a link.
        let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
        let result = unsafe {
            libc::fstatat(
                directory.as_raw_fd(),
                raw_name.as_ptr(),
                &mut stat,
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result != 0 {
            return Err(io::Error::last_os_error().into());
        }
        let kind = match stat.st_mode & libc::S_IFMT {
            libc::S_IFREG => EntryKind::File,
            libc::S_IFDIR => EntryKind::Directory,
            libc::S_IFLNK => EntryKind::Symlink,
            _ => EntryKind::Other,
        };
        output.push(DirectoryEntry {
            name,
            raw_name,
            kind,
        });
    }
    Ok(output)
}

fn bounded_join(values: Vec<String>) -> Result<String, ToolError> {
    let joined = values.join("\n");
    if joined.len() > MAX_TOOL_BYTES {
        Err(ToolError::ByteLimit)
    } else {
        Ok(joined)
    }
}

fn string_object(value: &Value) -> Result<Vec<(String, String)>, ProviderError> {
    let object = value.as_object().ok_or(ProviderError::InvalidToolInput)?;
    object
        .iter()
        .map(|(name, value)| {
            value
                .as_str()
                .map(|value| (name.clone(), value.to_owned()))
                .ok_or(ProviderError::InvalidToolInput)
        })
        .collect()
}

fn tool_definitions() -> Vec<ToolDefinition> {
    [
        ("read_file", "Read one bounded workspace file", "path"),
        ("list_files", "List one bounded workspace directory", "path"),
        ("search_files", "Search bounded workspace paths", "pattern"),
    ].into_iter().map(|(name, description, field)| ToolDefinition {
        name, description,
        input_schema: json!({"type":"object","properties":{field:{"type":"string"}},"required":[field],"additionalProperties":false}),
    }).collect()
}

fn provider_messages(state: &ConversationState) -> AppResult<Vec<MessageParam>> {
    let mut messages = Vec::new();
    for message in state.messages() {
        let role = match message.role() {
            ChatRole::Assistant => "assistant",
            ChatRole::User | ChatRole::System | ChatRole::Tool => "user",
        };
        let mut content = Vec::new();
        for entry in message.blocks() {
            match entry.block() {
                MessageBlock::Text(text) | MessageBlock::Markdown(text) => {
                    content.push(ProviderBlock::Text { text: text.clone() });
                }
                MessageBlock::ToolCall(call) => content.push(ProviderBlock::ToolUse {
                    id: call.call_id().as_str().to_owned(),
                    name: call.name().to_owned(),
                    input: arguments_json(call.arguments()),
                }),
                MessageBlock::ToolResult(result) => content.push(ProviderBlock::ToolResult {
                    tool_use_id: result.call_id().as_str().to_owned(),
                    content: result.output().to_owned(),
                }),
                // Provider thinking is visible typed state, never request input.
                MessageBlock::Thinking(_) => {}
                _ => return Err(ProviderError::UnsupportedBlock.into()),
            }
        }
        if !content.is_empty() {
            messages.push(MessageParam { role, content });
        }
    }
    Ok(messages)
}

fn arguments_json(arguments: &[ToolArgument]) -> Value {
    Value::Object(
        arguments
            .iter()
            .map(|argument| (argument.name().to_owned(), typed_json(argument.value())))
            .collect(),
    )
}

fn typed_json(value: &TypedValue) -> Value {
    match value {
        TypedValue::Null => Value::Null,
        TypedValue::Bool(value) => Value::Bool(*value),
        TypedValue::Integer(value) => Value::from(*value),
        TypedValue::Decimal(value) => Value::String(value.as_str().to_owned()),
        TypedValue::String(value) => Value::String(value.clone()),
        TypedValue::List(values) => Value::Array(values.iter().map(typed_json).collect()),
        TypedValue::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|field| (field.name().to_owned(), typed_json(field.value())))
                .collect(),
        ),
    }
}

fn response_blocks(
    response: &ChatResponse,
) -> AppResult<(Vec<MessageBlockEntry>, Vec<PendingToolRequest>)> {
    let mut entries = Vec::new();
    let mut pending = Vec::new();
    for (index, block) in response.content.iter().enumerate() {
        let block_id = BlockId::new(
            u64::try_from(index)?
                .checked_add(1)
                .ok_or(ProviderError::InvalidToolInput)?,
        );
        let value = match block {
            ResponseBlock::Text { text } => MessageBlock::Text(text.clone()),
            ResponseBlock::Thinking { thinking } => MessageBlock::Thinking(
                ThinkingContent::new(
                    ThinkingId::new(format!("thinking-{}", block_id.get()))?,
                    thinking,
                )
                .with_status(ThinkingStatus::Complete),
            ),
            ResponseBlock::ToolUse { id, name, input } => {
                let request = PendingToolRequest::parse(id, name, input)?;
                let call_id = request.call_id.clone();
                let tool_name = request.name.as_str();
                let arguments = vec![ToolArgument::new(
                    request.name.argument_name(),
                    TypedValue::String(request.argument.clone()),
                )?];
                pending.push(request);
                MessageBlock::ToolCall(
                    ToolCallContent::new(call_id, tool_name, arguments)?
                        .with_status(ToolCallStatus::Pending),
                )
            }
        };
        entries.push(MessageBlockEntry::new(block_id, value));
    }
    if entries.is_empty() {
        return Err(ProviderError::UnsupportedBlock.into());
    }
    Ok((entries, pending))
}

#[cfg(test)]
pub(crate) fn gh68_provider_adapter_view() -> AppResult<String> {
    let mut state = ConversationState::new(0, std::num::NonZeroUsize::new(16).unwrap());
    push_completed(
        &mut state,
        ChatRole::User,
        vec![MessageBlockEntry::new(
            BlockId::new(1),
            MessageBlock::Text("Explain the release gate".to_owned()),
        )],
    )?;
    let response = ChatResponse {
        content: vec![ResponseBlock::Text {
            text: "Use typed updates.".to_owned(),
        }],
    };
    let (blocks, pending) = response_blocks(&response)?;
    if !pending.is_empty() {
        return Err(ProviderError::InvalidToolInput.into());
    }
    push_completed(&mut state, ChatRole::Assistant, blocks)?;
    let root = rnk::components::Box::new()
        .flex_direction(rnk::core::FlexDirection::Column)
        .children(
            state
                .messages()
                .iter()
                .map(|message| ChatMessageView::new(message).into_element()),
        )
        .into_element();
    Ok(rnk::render_to_string(&root, 60))
}

#[cfg(test)]
pub(crate) fn gh68_provider_internal_contract() -> AppResult<(usize, usize, usize, usize)> {
    let response = ChatResponse {
        content: vec![ResponseBlock::Text {
            text: "answer".to_owned(),
        }],
    };
    let (blocks, _) = response_blocks(&response)?;
    let (_, pending) = response_blocks(&ChatResponse {
        content: vec![ResponseBlock::ToolUse {
            id: "call-internal".to_owned(),
            name: "search_files".to_owned(),
            input: json!({"pattern":"needle"}),
        }],
    })?;
    let _ = response_blocks(&ChatResponse {
        content: vec![ResponseBlock::Thinking {
            thinking: "reasoning".to_owned(),
        }],
    })?;
    let mut state = ConversationState::new(0, std::num::NonZeroUsize::new(16).unwrap());
    push_completed(&mut state, ChatRole::Assistant, blocks)?;
    let _ = tool_result(ToolCallId::new("call-internal")?, "needle.txt".to_owned());
    let provider = provider_messages(&state)?;
    let arguments = vec![
        ToolArgument::new("null", TypedValue::Null)?,
        ToolArgument::new("bool", TypedValue::Bool(true))?,
        ToolArgument::new("integer", TypedValue::Integer(68))?,
        ToolArgument::new("string", TypedValue::String("value".to_owned()))?,
        ToolArgument::new(
            "list",
            TypedValue::List(vec![TypedValue::String("item".to_owned())]),
        )?,
        ToolArgument::new("decimal", TypedValue::Decimal(DecimalValue::new("1.5")?))?,
        ToolArgument::new(
            "object",
            TypedValue::object(vec![TypedField::new(
                "field",
                TypedValue::String("nested".to_owned()),
            )?])?,
        )?,
    ];
    let typed = arguments_json(&arguments);
    let _ = [
        ProviderError::MissingApiKey.to_string(),
        ProviderError::HttpStatus(500).to_string(),
        ProviderError::UnsupportedBlock.to_string(),
        ProviderError::InvalidToolInput.to_string(),
        ProviderError::ToolCycleLimit.to_string(),
        ToolError::UnknownTool.to_string(),
        ToolError::InvalidUtf8.to_string(),
    ];
    let _ = ToolName::parse("read_file")?;
    let _ = ToolName::parse("list_files")?;
    let _ = ToolName::parse("search_files")?;
    validate_call_id("internal._:-68")?;
    validate_tool_argument(ToolName::ReadFile, "file.txt")?;
    validate_tool_argument(ToolName::ListFiles, "directory")?;
    validate_tool_argument(ToolName::SearchFiles, "needle")?;
    let _ = string_object(&json!({"path":"file.txt"}))?;
    let _ = bounded_join(vec!["a".to_owned(), "b".to_owned()])?;
    let _ = relative_components("directory/file.txt")?;
    let definitions = tool_definitions();
    let mut candidate = ConversationState::new(0, std::num::NonZeroUsize::new(16).unwrap());
    let oversized = (0..=MAX_RESPONSE_BLOCKS)
        .map(|index| {
            MessageBlockEntry::new(
                BlockId::new(u64::try_from(index + 1).unwrap()),
                MessageBlock::Text("x".to_owned()),
            )
        })
        .collect::<Vec<_>>();
    let negative_checks = [
        ProviderAdapter::from_optional_key(Some("key".to_owned()), || Ok(reqwest::Client::new()))
            .is_ok(),
        ToolName::parse("unknown").is_err(),
        validate_call_id("").is_err(),
        validate_call_id(&"a".repeat(65)).is_err(),
        validate_tool_argument(ToolName::ReadFile, "/absolute").is_err(),
        validate_tool_argument(ToolName::SearchFiles, "").is_err(),
        string_object(&Value::Null).is_err(),
        string_object(&json!({"path":1})).is_err(),
        response_blocks(&ChatResponse { content: vec![] }).is_err(),
        push_completed(&mut candidate, ChatRole::User, vec![]).is_err(),
        push_completed(&mut candidate, ChatRole::User, oversized).is_err(),
        bounded_join(vec!["x".repeat(MAX_TOOL_BYTES + 1)]).is_err(),
        relative_components("../escape").is_err(),
    ];
    Ok((
        pending.len(),
        provider.len(),
        typed.as_object().unwrap().len() + definitions.len(),
        negative_checks.into_iter().map(usize::from).sum(),
    ))
}

fn push_completed(
    state: &mut ConversationState,
    role: ChatRole,
    entries: Vec<MessageBlockEntry>,
) -> AppResult<MessageId> {
    let mut candidate = state.clone();
    let id = MessageId::new(candidate.expected_sequence());
    if entries.is_empty() || entries.len() > MAX_RESPONSE_BLOCKS {
        return Err(ProviderError::UnsupportedBlock.into());
    }
    let base = id
        .get()
        .checked_mul(u64::try_from(MAX_RESPONSE_BLOCKS)?)
        .ok_or(ProviderError::InvalidToolInput)?;
    let entries = entries
        .into_iter()
        .enumerate()
        .map(|(position, entry)| {
            let offset = u64::try_from(position)?
                .checked_add(1)
                .ok_or(ProviderError::InvalidToolInput)?;
            let block_id = base
                .checked_add(offset)
                .ok_or(ProviderError::InvalidToolInput)?;
            Ok(MessageBlockEntry::new(
                BlockId::new(block_id),
                entry.block().clone(),
            ))
        })
        .collect::<AppResult<Vec<_>>>()?;
    let message = ChatMessage::new(id, role, entries)?;
    let event = format!("push-{}", candidate.expected_sequence());
    let guard = ConversationGuard::new(candidate.revision());
    apply(
        &mut candidate,
        event,
        ConversationUpdate::push(guard, message),
    )?;
    let message = candidate
        .message(id)
        .ok_or(ProviderError::UnsupportedBlock)?;
    let guard = MessageMutationGuard::new(
        ConversationGuard::new(candidate.revision()),
        id,
        message.revision(),
    );
    let event = format!("complete-{}", candidate.expected_sequence());
    apply(&mut candidate, event, ConversationUpdate::complete(guard))?;
    *state = candidate;
    Ok(id)
}

fn apply(
    state: &mut ConversationState,
    event: String,
    update: ConversationUpdate,
) -> AppResult<()> {
    state.apply_event(ConversationEvent::new(
        UpdateId::new(event)?,
        state.expected_sequence(),
        update,
    ))?;
    Ok(())
}

fn publish(shell: &mut OutputShell, message: &ChatMessage, width: u16) -> AppResult<()> {
    let rendered = rnk::render_to_string(&ChatMessageView::new(message).into_element(), width);
    let report = shell.finish(
        message.id(),
        message.revision(),
        &rendered,
        ProjectionContext::new(width, ThemeIdentity::new(1))?,
    )?;
    match report {
        InlineCommitReport::Fixed { .. } => Ok(()),
        InlineCommitReport::Retained { cause } => {
            Err(format!("scrollback retained message: {cause}").into())
        }
        InlineCommitReport::Latched { evidence } => {
            Err(format!("scrollback commit is undecidable: {evidence}").into())
        }
    }
}

fn prompt_line(prompt: &str) -> AppResult<String> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut line = String::new();
    let read = io::stdin().read_line(&mut line)?;
    if read == 0 {
        return Err("stdin closed".into());
    }
    Ok(line.trim_end().to_owned())
}

fn approve_and_execute(
    request: &mut PendingToolRequest,
    workspace: &Workspace,
) -> AppResult<String> {
    let phrase = request.approval_phrase();
    let supplied = prompt_line(&format!(
        "Tool request {}. Type `{phrase}` to approve: ",
        request.exact_description()
    ))?;
    request.approve_exact(&supplied)?;
    Ok(request.execute_once(workspace)?)
}

fn tool_result(call_id: ToolCallId, output: String) -> MessageBlockEntry {
    MessageBlockEntry::new(
        BlockId::new(1),
        MessageBlock::ToolResult(
            ToolResultContent::new(call_id, output).with_status(ToolResultStatus::Complete),
        ),
    )
}

#[tokio::main]
async fn main() -> AppResult<()> {
    let provider = ProviderAdapter::from_environment()?;
    let workspace = Workspace::current()?;
    let width = rnk::renderer::Terminal::size()?.0;
    if width == 0 {
        return Err("terminal width is zero".into());
    }
    let namespace = ScrollbackNamespace::new("example.glm-chat")?;
    let mut shell = InlineChatShell::new(namespace, NativeTerminalSink::new(io::stdout()));
    let mut state = ConversationState::new(0, std::num::NonZeroUsize::new(128).unwrap());

    loop {
        let input = prompt_line("glm> ")?;
        if matches!(input.as_str(), "quit" | "exit") {
            break;
        }
        if input.trim().is_empty() {
            continue;
        }
        let user_id = push_completed(
            &mut state,
            ChatRole::User,
            vec![MessageBlockEntry::new(
                BlockId::new(1),
                MessageBlock::Text(input),
            )],
        )?;
        publish(
            &mut shell,
            state
                .message(user_id)
                .ok_or(ProviderError::UnsupportedBlock)?,
            width,
        )?;

        for cycle in 0..MAX_TOOL_CYCLES {
            let response = provider.send(&state).await?;
            let (blocks, mut pending) = response_blocks(&response)?;
            let assistant_id = push_completed(&mut state, ChatRole::Assistant, blocks)?;
            publish(
                &mut shell,
                state
                    .message(assistant_id)
                    .ok_or(ProviderError::UnsupportedBlock)?,
                width,
            )?;
            if pending.is_empty() {
                break;
            }
            for request in &mut pending {
                let output = approve_and_execute(request, &workspace)?;
                let result_id = push_completed(
                    &mut state,
                    ChatRole::Tool,
                    vec![tool_result(request.call_id.clone(), output)],
                )?;
                publish(
                    &mut shell,
                    state
                        .message(result_id)
                        .ok_or(ProviderError::UnsupportedBlock)?,
                    width,
                )?;
            }
            if cycle + 1 == MAX_TOOL_CYCLES {
                return Err(ProviderError::ToolCycleLimit.into());
            }
        }
    }
    Ok(())
}
