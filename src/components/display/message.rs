//! Message component for chat-style interfaces
//!
//! Provides styled message components for different roles (user, assistant, system, tool).

use crate::components::{Box, Text};
use crate::core::{Color, Element, FlexDirection};

/// Message role in a chat interface
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    /// User message (typically with > prefix, yellow)
    User,
    /// Assistant/AI message (typically with ● prefix, white)
    Assistant,
    /// System message (typically with ● prefix, cyan)
    System,
    /// Tool call message (typically with ● prefix, magenta)
    Tool,
    /// Tool result message (typically with ⎿ prefix, gray)
    ToolResult,
    /// Error message (typically with ● prefix, red)
    Error,
}

/// A styled message component for chat interfaces
///
/// # Example
///
/// ```ignore
/// use rnk::components::{Message, MessageRole};
///
/// // User message
/// let msg = Message::new(MessageRole::User, "Hello, world!");
/// rnk::println(msg.into_element());
///
/// // Assistant message
/// let msg = Message::assistant("Hi! How can I help?");
/// rnk::println(msg.into_element());
/// ```
pub struct Message {
    role: MessageRole,
    content: String,
    prefix: Option<String>,
}

impl Message {
    /// Create a new message with the specified role and content
    pub fn new(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            prefix: None,
        }
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(MessageRole::User, content)
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Assistant, content)
    }

    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(MessageRole::System, content)
    }

    /// Create a tool call message
    pub fn tool(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Tool, content)
    }

    /// Create a tool result message
    pub fn tool_result(content: impl Into<String>) -> Self {
        Self::new(MessageRole::ToolResult, content)
    }

    /// Create an error message
    pub fn error(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Error, content)
    }

    /// Set a custom prefix (overrides default role prefix)
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Get the default prefix for this role
    fn default_prefix(&self) -> &str {
        match self.role {
            MessageRole::User => "> ",
            MessageRole::Assistant => "● ",
            MessageRole::System => "● ",
            MessageRole::Tool => "● ",
            MessageRole::ToolResult => "  ⎿ ",
            MessageRole::Error => "● ",
        }
    }

    /// Get the color for this role
    fn color(&self) -> Color {
        match self.role {
            MessageRole::User => Color::Yellow,
            MessageRole::Assistant => Color::BrightWhite,
            MessageRole::System => Color::Cyan,
            MessageRole::Tool => Color::Magenta,
            MessageRole::ToolResult => Color::Ansi256(245),
            MessageRole::Error => Color::Red,
        }
    }

    /// Get the prefix color for this role
    fn prefix_color(&self) -> Color {
        match self.role {
            MessageRole::User => Color::Yellow,
            MessageRole::Assistant => Color::BrightWhite,
            MessageRole::System => Color::Cyan,
            MessageRole::Tool => Color::Magenta,
            MessageRole::ToolResult => Color::Ansi256(245),
            MessageRole::Error => Color::Red,
        }
    }

    /// Convert to an Element
    pub fn into_element(self) -> Element {
        let prefix = self
            .prefix
            .as_deref()
            .unwrap_or_else(|| self.default_prefix());
        let prefix_color = self.prefix_color();
        let content_color = self.color();

        let mut container = Box::new().flex_direction(FlexDirection::Row);

        // Add prefix
        if !prefix.is_empty() {
            container =
                container.child(Text::new(prefix).color(prefix_color).bold().into_element());
        }

        // Add content
        container = container.child(Text::new(&self.content).color(content_color).into_element());

        container.into_element()
    }
}

/// Tool call message with name and arguments
///
/// # Example
///
/// ```ignore
/// use rnk::components::ToolCall;
///
/// let tool = ToolCall::new("read_file", "path=/tmp/test.txt");
/// rnk::println(tool.into_element());
/// ```
pub struct ToolCall {
    name: String,
    args: String,
}

impl ToolCall {
    /// Create a new tool call message
    pub fn new(name: impl Into<String>, args: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            args: args.into(),
        }
    }

    /// Convert to an Element
    pub fn into_element(self) -> Element {
        Box::new()
            .flex_direction(FlexDirection::Row)
            .child(Text::new("● ").color(Color::Magenta).into_element())
            .child(
                Text::new(&self.name)
                    .color(Color::Magenta)
                    .bold()
                    .into_element(),
            )
            .child(
                Text::new(format!("({})", self.args))
                    .color(Color::Magenta)
                    .into_element(),
            )
            .into_element()
    }
}

/// Thinking block for displaying AI reasoning
///
/// # Example
///
/// ```ignore
/// use rnk::components::ThinkingBlock;
///
/// let thinking = ThinkingBlock::new("Analyzing the problem...\nConsidering options...");
/// rnk::println(thinking.into_element());
/// ```
pub struct ThinkingBlock {
    content: String,
    max_lines: usize,
}

impl ThinkingBlock {
    /// Create a new thinking block
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            max_lines: 5,
        }
    }

    /// Set the maximum number of lines to display
    pub fn max_lines(mut self, max_lines: usize) -> Self {
        self.max_lines = max_lines;
        self
    }

    /// Convert to an Element
    pub fn into_element(self) -> Element {
        let lines: Vec<&str> = self.content.lines().take(self.max_lines).collect();
        let has_more = self.content.lines().count() > self.max_lines;

        let mut container = Box::new().flex_direction(FlexDirection::Column).child(
            Text::new("● Thinking...")
                .color(Color::Magenta)
                .into_element(),
        );

        for line in lines {
            container = container.child(
                Box::new()
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello");
        let element = msg.into_element();
        assert!(!element.children.is_empty());
    }

    #[test]
    fn test_message_roles() {
        let _user = Message::user("test");
        let _assistant = Message::assistant("test");
        let _system = Message::system("test");
        let _tool = Message::tool("test");
        let _error = Message::error("test");
    }

    #[test]
    fn test_tool_call() {
        let tool = ToolCall::new("read_file", "path=/tmp/test.txt");
        let element = tool.into_element();
        assert!(!element.children.is_empty());
    }

    #[test]
    fn test_thinking_block() {
        let thinking = ThinkingBlock::new("Line 1\nLine 2\nLine 3");
        let element = thinking.into_element();
        assert!(!element.children.is_empty());
    }
}
