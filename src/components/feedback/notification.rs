//! Notification/Toast component for displaying temporary messages
//!
//! Provides toast-style notifications with auto-dismiss and various styles.

use crate::components::status::{StatusLevel, impl_status_level_from, status_style};
use crate::components::{Box, Text};
use crate::core::{AlignItems, Color, Element, FlexDirection, JustifyContent};

/// Notification level/type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotificationLevel {
    /// Informational message
    #[default]
    Info,
    /// Success message
    Success,
    /// Warning message
    Warning,
    /// Error message
    Error,
}

impl NotificationLevel {
    /// Get the default color for this level
    pub fn color(&self) -> Color {
        status_style((*self).into()).fg
    }

    /// Get the default icon for this level
    pub fn icon(&self) -> &'static str {
        status_style((*self).into()).icon
    }

    /// Get the label for this level
    pub fn label(&self) -> &'static str {
        status_style((*self).into()).label
    }
}

impl_status_level_from!(NotificationLevel);

/// Position for notification display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotificationPosition {
    /// Top of screen
    Top,
    /// Top right corner
    #[default]
    TopRight,
    /// Top left corner
    TopLeft,
    /// Bottom of screen
    Bottom,
    /// Bottom right corner
    BottomRight,
    /// Bottom left corner
    BottomLeft,
}

impl NotificationPosition {
    /// Get the justify_content and align_items values for this position
    pub fn to_flex_alignment(&self) -> (JustifyContent, AlignItems) {
        match self {
            NotificationPosition::Top => (JustifyContent::FlexStart, AlignItems::Center),
            NotificationPosition::TopRight => (JustifyContent::FlexStart, AlignItems::FlexEnd),
            NotificationPosition::TopLeft => (JustifyContent::FlexStart, AlignItems::FlexStart),
            NotificationPosition::Bottom => (JustifyContent::FlexEnd, AlignItems::Center),
            NotificationPosition::BottomRight => (JustifyContent::FlexEnd, AlignItems::FlexEnd),
            NotificationPosition::BottomLeft => (JustifyContent::FlexEnd, AlignItems::FlexStart),
        }
    }
}

/// Notification style configuration
#[derive(Debug, Clone)]
pub struct NotificationStyle {
    /// Show icon
    pub show_icon: bool,
    /// Show level label
    pub show_label: bool,
    /// Border style
    pub border: NotificationBorder,
    /// Custom icon override
    pub custom_icon: Option<String>,
    /// Custom color override
    pub custom_color: Option<Color>,
    /// Padding
    pub padding: usize,
    /// Minimum width
    pub min_width: Option<usize>,
    /// Maximum width
    pub max_width: Option<usize>,
}

impl Default for NotificationStyle {
    fn default() -> Self {
        Self {
            show_icon: true,
            show_label: false,
            border: NotificationBorder::Rounded,
            custom_icon: None,
            custom_color: None,
            padding: 1,
            min_width: None,
            max_width: Some(60),
        }
    }
}

impl NotificationStyle {
    /// Create a new notification style
    pub fn new() -> Self {
        Self::default()
    }

    /// Show or hide icon
    pub fn show_icon(mut self, show: bool) -> Self {
        self.show_icon = show;
        self
    }

    /// Show or hide level label
    pub fn show_label(mut self, show: bool) -> Self {
        self.show_label = show;
        self
    }

    /// Set border style
    pub fn border(mut self, border: NotificationBorder) -> Self {
        self.border = border;
        self
    }

    /// Set custom icon
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.custom_icon = Some(icon.into());
        self
    }

    /// Set custom color
    pub fn color(mut self, color: Color) -> Self {
        self.custom_color = Some(color);
        self
    }

    /// Set padding
    pub fn padding(mut self, padding: usize) -> Self {
        self.padding = padding;
        self
    }

    /// Set minimum width
    pub fn min_width(mut self, width: usize) -> Self {
        self.min_width = Some(width);
        self
    }

    /// Set maximum width
    pub fn max_width(mut self, width: usize) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Minimal style (no border, just icon and text)
    pub fn minimal() -> Self {
        Self::new()
            .border(NotificationBorder::None)
            .show_label(false)
            .padding(0)
    }

    /// Compact style (single line with border)
    pub fn compact() -> Self {
        Self::new()
            .border(NotificationBorder::Single)
            .show_label(false)
            .padding(1)
    }

    /// Detailed style (with label and border)
    pub fn detailed() -> Self {
        Self::new()
            .border(NotificationBorder::Rounded)
            .show_label(true)
            .padding(1)
    }
}

/// Border style for notifications
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotificationBorder {
    /// No border
    None,
    /// Single line border
    Single,
    /// Double line border
    Double,
    /// Rounded corners
    #[default]
    Rounded,
    /// Heavy/bold border
    Heavy,
}

impl NotificationBorder {
    /// Get border characters (top-left, top-right, bottom-left, bottom-right, horizontal, vertical)
    pub fn chars(
        &self,
    ) -> Option<(
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    )> {
        match self {
            NotificationBorder::None => None,
            NotificationBorder::Single => Some(("┌", "┐", "└", "┘", "─", "│")),
            NotificationBorder::Double => Some(("╔", "╗", "╚", "╝", "═", "║")),
            NotificationBorder::Rounded => Some(("╭", "╮", "╰", "╯", "─", "│")),
            NotificationBorder::Heavy => Some(("┏", "┓", "┗", "┛", "━", "┃")),
        }
    }
}

/// A single notification item
#[derive(Debug, Clone)]
pub struct NotificationItem {
    /// Unique ID
    pub id: String,
    /// Message content
    pub message: String,
    /// Notification level
    pub level: NotificationLevel,
    /// Title (optional)
    pub title: Option<String>,
    /// Creation timestamp (ms)
    pub created_at: u64,
    /// Duration before auto-dismiss (ms), None for persistent
    pub duration_ms: Option<u64>,
    /// Whether notification is dismissible
    pub dismissible: bool,
}

impl NotificationItem {
    /// Create a new notification
    pub fn new(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            message: message.into(),
            level: NotificationLevel::Info,
            title: None,
            created_at: 0,
            duration_ms: Some(3000),
            dismissible: true,
        }
    }

    /// Set notification level
    pub fn level(mut self, level: NotificationLevel) -> Self {
        self.level = level;
        self
    }

    /// Set title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set creation timestamp
    pub fn created_at(mut self, timestamp: u64) -> Self {
        self.created_at = timestamp;
        self
    }

    /// Set duration (None for persistent)
    pub fn duration(mut self, ms: Option<u64>) -> Self {
        self.duration_ms = ms;
        self
    }

    /// Set whether dismissible
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }

    /// Create an info notification
    pub fn info(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, message).level(NotificationLevel::Info)
    }

    /// Create a success notification
    pub fn success(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, message).level(NotificationLevel::Success)
    }

    /// Create a warning notification
    pub fn warning(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, message).level(NotificationLevel::Warning)
    }

    /// Create an error notification
    pub fn error(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, message).level(NotificationLevel::Error)
    }

    /// Check if notification should be dismissed
    pub fn should_dismiss(&self, current_time: u64) -> bool {
        if let Some(duration) = self.duration_ms {
            current_time.saturating_sub(self.created_at) >= duration
        } else {
            false
        }
    }
}

/// State for managing multiple notifications
#[derive(Debug, Clone, Default)]
pub struct NotificationState {
    /// Active notifications
    notifications: Vec<NotificationItem>,
    /// Next notification ID
    next_id: u64,
    /// Maximum number of visible notifications
    max_visible: usize,
    /// Position for notifications
    position: NotificationPosition,
}

impl NotificationState {
    /// Create a new notification state
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
            next_id: 1,
            max_visible: 5,
            position: NotificationPosition::TopRight,
        }
    }

    /// Set maximum visible notifications
    pub fn max_visible(mut self, max: usize) -> Self {
        self.max_visible = max;
        self
    }

    /// Set notification position
    pub fn position(mut self, position: NotificationPosition) -> Self {
        self.position = position;
        self
    }

    /// Add a notification
    pub fn push(&mut self, mut item: NotificationItem, current_time: u64) -> String {
        if item.id.is_empty() {
            item.id = format!("notification-{}", self.next_id);
            self.next_id += 1;
        }
        item.created_at = current_time;
        let id = item.id.clone();
        self.notifications.push(item);
        id
    }

    /// Add an info notification
    pub fn info(&mut self, message: impl Into<String>, current_time: u64) -> String {
        let item = NotificationItem::info("", message);
        self.push(item, current_time)
    }

    /// Add a success notification
    pub fn success(&mut self, message: impl Into<String>, current_time: u64) -> String {
        let item = NotificationItem::success("", message);
        self.push(item, current_time)
    }

    /// Add a warning notification
    pub fn warning(&mut self, message: impl Into<String>, current_time: u64) -> String {
        let item = NotificationItem::warning("", message);
        self.push(item, current_time)
    }

    /// Add an error notification
    pub fn error(&mut self, message: impl Into<String>, current_time: u64) -> String {
        let item = NotificationItem::error("", message);
        self.push(item, current_time)
    }

    /// Dismiss a notification by ID
    pub fn dismiss(&mut self, id: &str) -> bool {
        if let Some(pos) = self.notifications.iter().position(|n| n.id == id) {
            self.notifications.remove(pos);
            true
        } else {
            false
        }
    }

    /// Dismiss all notifications
    pub fn dismiss_all(&mut self) {
        self.notifications.clear();
    }

    /// Update state, removing expired notifications
    pub fn update(&mut self, current_time: u64) {
        self.notifications
            .retain(|n| !n.should_dismiss(current_time));
    }

    /// Get visible notifications
    pub fn visible(&self) -> &[NotificationItem] {
        let len = self.notifications.len();
        let start = len.saturating_sub(self.max_visible);
        &self.notifications[start..]
    }

    /// Get all notifications
    pub fn all(&self) -> &[NotificationItem] {
        &self.notifications
    }

    /// Check if there are any notifications
    pub fn is_empty(&self) -> bool {
        self.notifications.is_empty()
    }

    /// Get notification count
    pub fn count(&self) -> usize {
        self.notifications.len()
    }

    /// Get position
    pub fn get_position(&self) -> NotificationPosition {
        self.position
    }
}

/// Toast component for rendering a single notification
#[derive(Debug, Clone)]
pub struct Toast<'a> {
    item: &'a NotificationItem,
    style: NotificationStyle,
}

impl<'a> Toast<'a> {
    /// Create a new toast
    pub fn new(item: &'a NotificationItem) -> Self {
        Self {
            item,
            style: NotificationStyle::default(),
        }
    }

    /// Set style
    pub fn style(mut self, style: NotificationStyle) -> Self {
        self.style = style;
        self
    }

    /// Render the toast to a string
    pub fn render(&self) -> String {
        let color = self
            .style
            .custom_color
            .unwrap_or_else(|| self.item.level.color());
        let icon = self
            .style
            .custom_icon
            .as_deref()
            .unwrap_or_else(|| self.item.level.icon());

        let mut content = String::new();

        // Build content line
        if self.style.show_icon {
            content.push_str(icon);
            content.push(' ');
        }

        if self.style.show_label {
            content.push_str(self.item.level.label());
            content.push_str(": ");
        }

        if let Some(title) = &self.item.title {
            content.push_str(title);
            content.push_str(" - ");
        }

        content.push_str(&self.item.message);

        // Apply max width
        if let Some(max_width) = self.style.max_width {
            if content.len() > max_width {
                content.truncate(max_width - 3);
                content.push_str("...");
            }
        }

        // Apply min width
        if let Some(min_width) = self.style.min_width {
            while content.len() < min_width {
                content.push(' ');
            }
        }

        // Render with border
        if let Some((tl, tr, bl, br, h, v)) = self.style.border.chars() {
            let padding = " ".repeat(self.style.padding);
            let inner_width = content.len() + self.style.padding * 2;
            let top = format!("{}{}{}", tl, h.repeat(inner_width), tr);
            let middle = format!("{}{}{}{}", v, padding, content, padding);
            let middle = format!("{}{}", middle, v);
            let bottom = format!("{}{}{}", bl, h.repeat(inner_width), br);

            format!(
                "{}{}\n{}\n{}\x1b[0m",
                color.to_ansi_fg(),
                top,
                middle,
                bottom
            )
        } else {
            // No border
            let padding = " ".repeat(self.style.padding);
            format!(
                "{}{}{}{}\x1b[0m",
                color.to_ansi_fg(),
                padding,
                content,
                padding
            )
        }
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        Text::new(self.render()).into_element()
    }
}

/// Notification container for rendering multiple notifications
#[derive(Debug)]
pub struct Notification<'a> {
    state: &'a NotificationState,
    style: NotificationStyle,
}

impl<'a> Notification<'a> {
    /// Create a new notification container
    pub fn new(state: &'a NotificationState) -> Self {
        Self {
            state,
            style: NotificationStyle::default(),
        }
    }

    /// Set style for all notifications
    pub fn style(mut self, style: NotificationStyle) -> Self {
        self.style = style;
        self
    }

    /// Render all visible notifications
    pub fn render(&self) -> String {
        let notifications = self.state.visible();
        if notifications.is_empty() {
            return String::new();
        }

        notifications
            .iter()
            .map(|item| Toast::new(item).style(self.style.clone()).render())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Convert to Element with proper positioning
    ///
    /// Uses Box layout to position notifications according to the state's position setting.
    /// The returned element should be placed in a full-screen container for proper positioning.
    pub fn into_element(self) -> Element {
        let notifications = self.state.visible();
        if notifications.is_empty() {
            return Box::new().into_element();
        }

        // Get alignment based on position
        let (justify, align) = self.state.get_position().to_flex_alignment();

        // Create notification container with vertical stacking
        let mut container = Box::new().flex_direction(FlexDirection::Column);

        for item in notifications {
            container = container.child(
                Text::new(Toast::new(item).style(self.style.clone()).render()).into_element(),
            );
        }

        // Wrap in positioning container
        Box::new()
            .flex_direction(FlexDirection::Column)
            .justify_content(justify)
            .align_items(align)
            .flex_grow(1.0)
            .child(container.into_element())
            .into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_level() {
        assert_eq!(NotificationLevel::Info.icon(), "ℹ");
        assert_eq!(NotificationLevel::Success.icon(), "✓");
        assert_eq!(NotificationLevel::Warning.icon(), "⚠");
        assert_eq!(NotificationLevel::Error.icon(), "✗");
    }

    #[test]
    fn test_notification_level_color() {
        assert_eq!(NotificationLevel::Info.color(), Color::Cyan);
        assert_eq!(NotificationLevel::Success.color(), Color::Green);
        assert_eq!(NotificationLevel::Warning.color(), Color::Yellow);
        assert_eq!(NotificationLevel::Error.color(), Color::Red);
    }

    #[test]
    fn test_notification_item_creation() {
        let item = NotificationItem::new("test-1", "Hello world");
        assert_eq!(item.id, "test-1");
        assert_eq!(item.message, "Hello world");
        assert_eq!(item.level, NotificationLevel::Info);
    }

    #[test]
    fn test_notification_item_builders() {
        let info = NotificationItem::info("1", "Info message");
        assert_eq!(info.level, NotificationLevel::Info);

        let success = NotificationItem::success("2", "Success message");
        assert_eq!(success.level, NotificationLevel::Success);

        let warning = NotificationItem::warning("3", "Warning message");
        assert_eq!(warning.level, NotificationLevel::Warning);

        let error = NotificationItem::error("4", "Error message");
        assert_eq!(error.level, NotificationLevel::Error);
    }

    #[test]
    fn test_notification_item_should_dismiss() {
        let item = NotificationItem::new("1", "Test")
            .created_at(1000)
            .duration(Some(3000));

        assert!(!item.should_dismiss(2000));
        assert!(!item.should_dismiss(3999));
        assert!(item.should_dismiss(4000));
        assert!(item.should_dismiss(5000));
    }

    #[test]
    fn test_notification_item_persistent() {
        let item = NotificationItem::new("1", "Test")
            .created_at(1000)
            .duration(None);

        assert!(!item.should_dismiss(10000));
        assert!(!item.should_dismiss(100000));
    }

    #[test]
    fn test_notification_state() {
        let mut state = NotificationState::new();
        assert!(state.is_empty());

        let id = state.info("Test message", 1000);
        assert!(!state.is_empty());
        assert_eq!(state.count(), 1);
        assert!(!id.is_empty());
    }

    #[test]
    fn test_notification_state_dismiss() {
        let mut state = NotificationState::new();
        let id = state.info("Test", 1000);

        assert!(state.dismiss(&id));
        assert!(state.is_empty());
        assert!(!state.dismiss(&id)); // Already dismissed
    }

    #[test]
    fn test_notification_state_update() {
        let mut state = NotificationState::new();
        state.info("Test 1", 1000);
        state.info("Test 2", 2000);

        assert_eq!(state.count(), 2);

        // At t=3500, neither should expire yet (first expires at 4000)
        state.update(3500);
        assert_eq!(state.count(), 2);

        // At t=4500, first notification should expire (created at 1000, duration 3000)
        state.update(4500);
        assert_eq!(state.count(), 1);

        // At t=5500, second notification should expire (created at 2000, duration 3000)
        state.update(5500);
        assert!(state.is_empty());
    }

    #[test]
    fn test_notification_state_max_visible() {
        let mut state = NotificationState::new().max_visible(2);

        state.info("Test 1", 1000);
        state.info("Test 2", 1000);
        state.info("Test 3", 1000);

        assert_eq!(state.count(), 3);
        assert_eq!(state.visible().len(), 2);
    }

    #[test]
    fn test_toast_render() {
        let item = NotificationItem::info("1", "Hello world");
        let toast = Toast::new(&item);
        let rendered = toast.render();

        assert!(rendered.contains("ℹ"));
        assert!(rendered.contains("Hello world"));
    }

    #[test]
    fn test_toast_render_minimal() {
        let item = NotificationItem::success("1", "Done!");
        let toast = Toast::new(&item).style(NotificationStyle::minimal());
        let rendered = toast.render();

        assert!(rendered.contains("✓"));
        assert!(rendered.contains("Done!"));
        assert!(!rendered.contains("─")); // No border
    }

    #[test]
    fn test_notification_border_chars() {
        assert!(NotificationBorder::None.chars().is_none());
        assert!(NotificationBorder::Single.chars().is_some());
        assert!(NotificationBorder::Double.chars().is_some());
        assert!(NotificationBorder::Rounded.chars().is_some());
        assert!(NotificationBorder::Heavy.chars().is_some());
    }

    #[test]
    fn test_notification_style_builder() {
        let style = NotificationStyle::new()
            .show_icon(false)
            .show_label(true)
            .border(NotificationBorder::Double)
            .padding(2);

        assert!(!style.show_icon);
        assert!(style.show_label);
        assert_eq!(style.border, NotificationBorder::Double);
        assert_eq!(style.padding, 2);
    }

    #[test]
    fn test_notification_container() {
        let mut state = NotificationState::new();
        state.info("Message 1", 1000);
        state.success("Message 2", 1000);

        let notification = Notification::new(&state);
        let rendered = notification.render();

        assert!(rendered.contains("Message 1"));
        assert!(rendered.contains("Message 2"));
    }

    #[test]
    fn test_notification_position_alignment() {
        use crate::core::{AlignItems, JustifyContent};

        assert_eq!(
            NotificationPosition::Top.to_flex_alignment(),
            (JustifyContent::FlexStart, AlignItems::Center)
        );
        assert_eq!(
            NotificationPosition::TopRight.to_flex_alignment(),
            (JustifyContent::FlexStart, AlignItems::FlexEnd)
        );
        assert_eq!(
            NotificationPosition::TopLeft.to_flex_alignment(),
            (JustifyContent::FlexStart, AlignItems::FlexStart)
        );
        assert_eq!(
            NotificationPosition::Bottom.to_flex_alignment(),
            (JustifyContent::FlexEnd, AlignItems::Center)
        );
        assert_eq!(
            NotificationPosition::BottomRight.to_flex_alignment(),
            (JustifyContent::FlexEnd, AlignItems::FlexEnd)
        );
        assert_eq!(
            NotificationPosition::BottomLeft.to_flex_alignment(),
            (JustifyContent::FlexEnd, AlignItems::FlexStart)
        );
    }
}
