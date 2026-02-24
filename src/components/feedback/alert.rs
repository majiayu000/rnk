//! Alert component for important messages
//!
//! Displays alert messages with different severity levels.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::Alert;
//!
//! fn app() -> Element {
//!     Box::new()
//!         .flex_direction(FlexDirection::Column)
//!         .gap(1.0)
//!         .children(vec![
//!             Alert::info("This is an informational message").into_element(),
//!             Alert::success("Operation completed successfully").into_element(),
//!             Alert::warning("Please review before continuing").into_element(),
//!             Alert::error("An error occurred").into_element(),
//!         ])
//!         .into_element()
//! }
//! ```

use crate::components::status::{StatusLevel, impl_status_level_from, status_style};
use crate::components::{Box as RnkBox, Text};
use crate::core::{BorderStyle, Color, Element, FlexDirection};

/// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlertLevel {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl_status_level_from!(AlertLevel);

/// An alert component
#[derive(Debug, Clone)]
pub struct Alert {
    message: String,
    level: AlertLevel,
    title: Option<String>,
    dismissible: bool,
}

impl Alert {
    /// Create a new alert
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: AlertLevel::Info,
            title: None,
            dismissible: false,
        }
    }

    /// Create an info alert
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message).level(AlertLevel::Info)
    }

    /// Create a success alert
    pub fn success(message: impl Into<String>) -> Self {
        Self::new(message).level(AlertLevel::Success)
    }

    /// Create a warning alert
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message).level(AlertLevel::Warning)
    }

    /// Create an error alert
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message).level(AlertLevel::Error)
    }

    /// Set the alert level
    pub fn level(mut self, level: AlertLevel) -> Self {
        self.level = level;
        self
    }

    /// Set a title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Make the alert dismissible
    pub fn dismissible(mut self) -> Self {
        self.dismissible = true;
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let style = status_style(self.level.into());
        let icon = style.icon;
        let color = style.fg;
        let bg = style.bg;

        let mut children = Vec::new();

        // Icon and title/message row
        let mut header_children = vec![
            Text::new(format!("{} ", icon))
                .color(color)
                .bold()
                .into_element(),
        ];

        if let Some(title) = &self.title {
            header_children.push(Text::new(title).color(color).bold().into_element());
        } else {
            header_children.push(Text::new(&self.message).color(Color::White).into_element());
        }

        if self.dismissible {
            header_children.push(Text::new(" [x]").color(Color::BrightBlack).into_element());
        }

        children.push(
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .children(header_children)
                .into_element(),
        );

        // Message (if title is set)
        if self.title.is_some() {
            children.push(
                Text::new(format!("  {}", self.message))
                    .color(Color::White)
                    .into_element(),
            );
        }

        RnkBox::new()
            .flex_direction(FlexDirection::Column)
            .padding_x(1.0)
            .padding_y(0.5)
            .background(bg)
            .border_style(BorderStyle::Round)
            .border_color(color)
            .children(children)
            .into_element()
    }
}

impl Default for Alert {
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_creation() {
        let a = Alert::new("Test message");
        assert_eq!(a.message, "Test message");
    }

    #[test]
    fn test_alert_levels() {
        let _ = Alert::info("Info").into_element();
        let _ = Alert::success("Success").into_element();
        let _ = Alert::warning("Warning").into_element();
        let _ = Alert::error("Error").into_element();
    }

    #[test]
    fn test_alert_with_title() {
        let a = Alert::error("Details").title("Error!");
        assert_eq!(a.title, Some("Error!".to_string()));
    }
}
