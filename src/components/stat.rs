//! Stat component for displaying statistics
//!
//! Displays key metrics with labels and optional trends.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::Stat;
//!
//! fn dashboard() -> Element {
//!     Box::new()
//!         .flex_direction(FlexDirection::Row)
//!         .gap(2.0)
//!         .children(vec![
//!             Stat::new("Users", "1,234").trend_up("12%").into_element(),
//!             Stat::new("Revenue", "$5,678").trend_down("3%").into_element(),
//!             Stat::new("Orders", "89").into_element(),
//!         ])
//!         .into_element()
//! }
//! ```

use crate::components::{Box as RnkBox, Text};
use crate::core::{Color, Element, FlexDirection};

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    Up,
    Down,
    Neutral,
}

/// A stat component for displaying metrics
#[derive(Debug, Clone)]
pub struct Stat {
    label: String,
    value: String,
    trend: Option<(Trend, String)>,
    help_text: Option<String>,
}

impl Stat {
    /// Create a new stat
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            trend: None,
            help_text: None,
        }
    }

    /// Add an upward trend
    pub fn trend_up(mut self, change: impl Into<String>) -> Self {
        self.trend = Some((Trend::Up, change.into()));
        self
    }

    /// Add a downward trend
    pub fn trend_down(mut self, change: impl Into<String>) -> Self {
        self.trend = Some((Trend::Down, change.into()));
        self
    }

    /// Add a neutral trend
    pub fn trend_neutral(mut self, change: impl Into<String>) -> Self {
        self.trend = Some((Trend::Neutral, change.into()));
        self
    }

    /// Add help text
    pub fn help(mut self, text: impl Into<String>) -> Self {
        self.help_text = Some(text.into());
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let mut children = Vec::new();

        // Label
        children.push(
            Text::new(&self.label)
                .color(Color::BrightBlack)
                .into_element(),
        );

        // Value with optional trend
        let mut value_children = vec![
            Text::new(&self.value)
                .color(Color::White)
                .bold()
                .into_element(),
        ];

        if let Some((trend, change)) = &self.trend {
            let (icon, color) = match trend {
                Trend::Up => ("↑", Color::Green),
                Trend::Down => ("↓", Color::Red),
                Trend::Neutral => ("→", Color::Yellow),
            };
            value_children.push(
                Text::new(format!(" {} {}", icon, change))
                    .color(color)
                    .into_element(),
            );
        }

        children.push(
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .children(value_children)
                .into_element(),
        );

        // Help text
        if let Some(help) = &self.help_text {
            children.push(
                Text::new(help)
                    .color(Color::BrightBlack)
                    .dim()
                    .into_element(),
            );
        }

        RnkBox::new()
            .flex_direction(FlexDirection::Column)
            .children(children)
            .into_element()
    }
}

impl Default for Stat {
    fn default() -> Self {
        Self::new("", "0")
    }
}

/// Create a simple stat
pub fn stat(label: impl Into<String>, value: impl Into<String>) -> Element {
    Stat::new(label, value).into_element()
}

/// Create a stat with upward trend
pub fn stat_up(label: impl Into<String>, value: impl Into<String>, change: impl Into<String>) -> Element {
    Stat::new(label, value).trend_up(change).into_element()
}

/// Create a stat with downward trend
pub fn stat_down(label: impl Into<String>, value: impl Into<String>, change: impl Into<String>) -> Element {
    Stat::new(label, value).trend_down(change).into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_creation() {
        let s = Stat::new("Users", "100");
        assert_eq!(s.label, "Users");
        assert_eq!(s.value, "100");
    }

    #[test]
    fn test_stat_with_trend() {
        let s = Stat::new("Sales", "$500").trend_up("10%");
        assert!(matches!(s.trend, Some((Trend::Up, _))));
    }

    #[test]
    fn test_stat_into_element() {
        let _ = Stat::new("Test", "123").into_element();
        let _ = Stat::new("Test", "123").trend_up("5%").into_element();
        let _ = Stat::new("Test", "123").trend_down("3%").into_element();
    }

    #[test]
    fn test_stat_helpers() {
        let _ = stat("Label", "Value");
        let _ = stat_up("Sales", "$100", "10%");
        let _ = stat_down("Costs", "$50", "5%");
    }
}
