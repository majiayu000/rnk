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
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use reqwest::Client;
use rnk::components::chat::scrollback::NativeTerminalSink;
use rnk::components::chat::{
    BlockId, ChatMessage, ChatMessageView, ChatRole, ConversationEvent, ConversationGuard,
    ConversationState, ConversationUpdate, InlineChatShell, InlineCommitReport, MessageBlock,
    MessageBlockEntry, MessageId, MessageMutationGuard, ProjectionContext, ScrollbackNamespace,
    ThemeIdentity, ThinkingContent, ThinkingId, ThinkingStatus, ToolArgument, ToolCallContent,
    ToolCallId, ToolCallStatus, ToolResultContent, ToolResultStatus, TypedValue, UpdateId,
};
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
    WorkspaceEscape,
    Symlink,
    DepthLimit,
    EntryLimit,
    ByteLimit,
    InvalidUtf8,
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
            Self::WorkspaceEscape => "tool path escapes the canonical workspace",
            Self::Symlink => "tool traversal encountered a symbolic link",
            Self::DepthLimit => "tool traversal exceeded the depth limit",
            Self::EntryLimit => "tool traversal exceeded the entry limit",
            Self::ByteLimit => "tool output exceeded the byte limit",
            Self::InvalidUtf8 => "tool file or path is not valid UTF-8",
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

pub(crate) struct PendingToolRequest {
    call_id: ToolCallId,
    name: String,
    input: Value,
    decision: ToolDecision,
    executed: bool,
}

impl PendingToolRequest {
    pub(crate) fn new(call_id: ToolCallId, name: String, input: Value) -> Self {
        Self {
            call_id,
            name,
            input,
            decision: ToolDecision::Denied,
            executed: false,
        }
    }

    pub(crate) fn approval_phrase(&self) -> String {
        format!("approve {}", self.call_id.as_str())
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
        let fields = string_object(&self.input).map_err(|_| ToolError::InvalidPath)?;
        match self.name.as_str() {
            "read_file" => workspace.read_file(required_field(&fields, "path")?),
            "list_files" => workspace.list_files(required_field(&fields, "path")?),
            "search_files" => workspace.search_files(required_field(&fields, "pattern")?),
            _ => Err(ToolError::UnknownTool),
        }
    }
}

pub(crate) struct Workspace {
    root: PathBuf,
}

impl Workspace {
    fn current() -> Result<Self, ToolError> {
        let root = env::current_dir()?.canonicalize()?;
        Ok(Self { root })
    }

    #[cfg(test)]
    pub(crate) fn from_root(root: &Path) -> Result<Self, ToolError> {
        Ok(Self {
            root: root.canonicalize()?,
        })
    }

    fn resolve(&self, supplied: &str) -> Result<PathBuf, ToolError> {
        let relative = Path::new(supplied);
        if relative.is_absolute()
            || supplied.is_empty()
            || relative
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(ToolError::InvalidPath);
        }
        let joined = self.root.join(relative);
        if fs::symlink_metadata(&joined)?.file_type().is_symlink() {
            return Err(ToolError::Symlink);
        }
        let canonical = joined.canonicalize()?;
        if !canonical.starts_with(&self.root) {
            return Err(ToolError::WorkspaceEscape);
        }
        Ok(canonical)
    }

    fn read_file(&self, path: &str) -> Result<String, ToolError> {
        let path = self.resolve(path)?;
        let metadata = fs::metadata(&path)?;
        if !metadata.is_file() {
            return Err(ToolError::InvalidPath);
        }
        if metadata.len() > MAX_TOOL_BYTES as u64 {
            return Err(ToolError::ByteLimit);
        }
        let bytes = fs::read(path)?;
        if bytes.len() > MAX_TOOL_BYTES {
            return Err(ToolError::ByteLimit);
        }
        String::from_utf8(bytes).map_err(|_| ToolError::InvalidUtf8)
    }

    fn list_files(&self, path: &str) -> Result<String, ToolError> {
        let directory = self.resolve(path)?;
        let mut names = Vec::new();
        for item in fs::read_dir(directory)? {
            let item = item?;
            if item.file_type()?.is_symlink() {
                return Err(ToolError::Symlink);
            }
            if names.len() == MAX_TOOL_ENTRIES {
                return Err(ToolError::EntryLimit);
            }
            names.push(
                item.file_name()
                    .into_string()
                    .map_err(|_| ToolError::InvalidUtf8)?,
            );
        }
        names.sort();
        bounded_join(names)
    }

    fn search_files(&self, pattern: &str) -> Result<String, ToolError> {
        if pattern.is_empty() || pattern.len() > 128 {
            return Err(ToolError::InvalidPath);
        }
        let mut found = Vec::new();
        self.search_directory(&self.root, pattern, 0, &mut found)?;
        found.sort();
        bounded_join(found)
    }

    fn search_directory(
        &self,
        directory: &Path,
        pattern: &str,
        depth: usize,
        found: &mut Vec<String>,
    ) -> Result<(), ToolError> {
        if depth > MAX_TOOL_DEPTH {
            return Err(ToolError::DepthLimit);
        }
        for item in fs::read_dir(directory)? {
            let item = item?;
            let kind = item.file_type()?;
            if kind.is_symlink() {
                return Err(ToolError::Symlink);
            }
            let canonical = item.path().canonicalize()?;
            if !canonical.starts_with(&self.root) {
                return Err(ToolError::WorkspaceEscape);
            }
            let relative = canonical
                .strip_prefix(&self.root)
                .map_err(|_| ToolError::WorkspaceEscape)?;
            let name = relative.to_str().ok_or(ToolError::InvalidUtf8)?;
            if name.contains(pattern) {
                if found.len() == MAX_TOOL_ENTRIES {
                    return Err(ToolError::EntryLimit);
                }
                found.push(name.to_owned());
            }
            if kind.is_dir() {
                if depth == MAX_TOOL_DEPTH {
                    return Err(ToolError::DepthLimit);
                }
                self.search_directory(&canonical, pattern, depth + 1, found)?;
            }
        }
        Ok(())
    }
}

fn bounded_join(values: Vec<String>) -> Result<String, ToolError> {
    let joined = values.join("\n");
    if joined.len() > MAX_TOOL_BYTES {
        Err(ToolError::ByteLimit)
    } else {
        Ok(joined)
    }
}

fn required_field<'a>(fields: &'a [(String, String)], name: &str) -> Result<&'a str, ToolError> {
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
        .ok_or(ToolError::InvalidPath)
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
                let call_id = ToolCallId::new(id.clone())?;
                let arguments = string_object(input)?
                    .into_iter()
                    .map(|(name, value)| ToolArgument::new(name, TypedValue::String(value)))
                    .collect::<Result<Vec<_>, _>>()?;
                pending.push(PendingToolRequest::new(
                    call_id.clone(),
                    name.clone(),
                    input.clone(),
                ));
                MessageBlock::ToolCall(
                    ToolCallContent::new(call_id, name, arguments)?
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
        "Tool {} requests {}. Type `{phrase}` to approve: ",
        request.call_id.as_str(),
        request.name
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
