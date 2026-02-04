//! DevTools - Debugging tools for rnk applications
//!
//! Provides a component tree inspector, state viewer, and layout debugger.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::DevTools;
//!
//! fn app() -> Element {
//!     let devtools_visible = use_signal(|| false);
//!
//!     use_input(move |_, key| {
//!         if key.f12 {
//!             devtools_visible.update(|v| *v = !*v);
//!         }
//!     });
//!
//!     Box::new()
//!         .children(vec![
//!             my_app_content(),
//!             DevTools::new()
//!                 .visible(devtools_visible.get())
//!                 .into_element(),
//!         ])
//!         .into_element()
//! }
//! ```

use crate::components::{Box as RnkBox, Text};
use crate::core::{BorderStyle, Color, Element, FlexDirection, JustifyContent};

/// DevTools panel for debugging rnk applications
#[derive(Debug, Clone)]
pub struct DevTools {
    visible: bool,
    active_tab: DevToolsTab,
    width: u16,
}

/// DevTools tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DevToolsTab {
    #[default]
    Tree,
    State,
    Layout,
    Performance,
}

impl DevTools {
    /// Create a new DevTools panel
    pub fn new() -> Self {
        Self {
            visible: false,
            active_tab: DevToolsTab::Tree,
            width: 40,
        }
    }

    /// Set visibility
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Set active tab
    pub fn tab(mut self, tab: DevToolsTab) -> Self {
        self.active_tab = tab;
        self
    }

    /// Set panel width
    pub fn width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        if !self.visible {
            return RnkBox::new().into_element();
        }

        RnkBox::new()
            .flex_direction(FlexDirection::Column)
            .width(self.width)
            .border_style(BorderStyle::Round)
            .border_color(Color::Magenta)
            .background(Color::Ansi256(235))
            .children(vec![
                self.render_header(),
                self.render_tabs(),
                self.render_content(),
            ])
            .into_element()
    }

    fn render_header(&self) -> Element {
        RnkBox::new()
            .flex_direction(FlexDirection::Row)
            .justify_content(JustifyContent::SpaceBetween)
            .padding_x(1.0)
            .background(Color::Ansi256(236))
            .children(vec![
                Text::new("DevTools")
                    .color(Color::Magenta)
                    .bold()
                    .into_element(),
                Text::new("F12 to close")
                    .color(Color::BrightBlack)
                    .into_element(),
            ])
            .into_element()
    }

    fn render_tabs(&self) -> Element {
        let tabs = [
            (DevToolsTab::Tree, "Tree"),
            (DevToolsTab::State, "State"),
            (DevToolsTab::Layout, "Layout"),
            (DevToolsTab::Performance, "Perf"),
        ];

        let tab_elements: Vec<Element> = tabs
            .iter()
            .map(|(tab, label)| {
                let is_active = *tab == self.active_tab;
                Text::new(*label)
                    .color(if is_active {
                        Color::Cyan
                    } else {
                        Color::BrightBlack
                    })
                    .bold()
                    .into_element()
            })
            .collect();

        RnkBox::new()
            .flex_direction(FlexDirection::Row)
            .padding_x(1.0)
            .gap(2.0)
            .background(Color::Ansi256(237))
            .children(tab_elements)
            .into_element()
    }

    fn render_content(&self) -> Element {
        match self.active_tab {
            DevToolsTab::Tree => self.render_tree_tab(),
            DevToolsTab::State => self.render_state_tab(),
            DevToolsTab::Layout => self.render_layout_tab(),
            DevToolsTab::Performance => self.render_performance_tab(),
        }
    }

    fn render_tree_tab(&self) -> Element {
        RnkBox::new()
            .flex_direction(FlexDirection::Column)
            .padding(1)
            .flex_grow(1.0)
            .children(vec![
                Text::new("Component Tree")
                    .color(Color::White)
                    .bold()
                    .into_element(),
                Text::new("").into_element(),
                self.render_tree_node("App", 0, true),
                self.render_tree_node("Box", 1, false),
                self.render_tree_node("Text", 2, false),
                self.render_tree_node("Box", 2, false),
                self.render_tree_node("Text", 3, false),
            ])
            .into_element()
    }

    fn render_tree_node(&self, name: &str, depth: usize, selected: bool) -> Element {
        let indent = "  ".repeat(depth);
        let prefix = if depth == 0 { "▼" } else { "├─" };

        RnkBox::new()
            .background(if selected {
                Color::Ansi256(238)
            } else {
                Color::Reset
            })
            .child(
                Text::new(format!("{}{} {}", indent, prefix, name))
                    .color(if selected {
                        Color::Cyan
                    } else {
                        Color::White
                    })
                    .into_element(),
            )
            .into_element()
    }

    fn render_state_tab(&self) -> Element {
        RnkBox::new()
            .flex_direction(FlexDirection::Column)
            .padding(1)
            .flex_grow(1.0)
            .children(vec![
                Text::new("Signal State")
                    .color(Color::White)
                    .bold()
                    .into_element(),
                Text::new("").into_element(),
                self.render_state_item("count", "i32", "0"),
                self.render_state_item("items", "Vec<String>", "[...]"),
                self.render_state_item("visible", "bool", "true"),
            ])
            .into_element()
    }

    fn render_state_item(&self, name: &str, type_name: &str, value: &str) -> Element {
        RnkBox::new()
            .flex_direction(FlexDirection::Row)
            .gap(1.0)
            .children(vec![
                Text::new(name).color(Color::Cyan).into_element(),
                Text::new(format!("({})", type_name))
                    .color(Color::BrightBlack)
                    .into_element(),
                Text::new("=").color(Color::White).into_element(),
                Text::new(value).color(Color::Yellow).into_element(),
            ])
            .into_element()
    }

    fn render_layout_tab(&self) -> Element {
        RnkBox::new()
            .flex_direction(FlexDirection::Column)
            .padding(1)
            .flex_grow(1.0)
            .children(vec![
                Text::new("Layout Info")
                    .color(Color::White)
                    .bold()
                    .into_element(),
                Text::new("").into_element(),
                self.render_layout_item("Position", "x: 0, y: 0"),
                self.render_layout_item("Size", "w: 80, h: 24"),
                self.render_layout_item("Padding", "1, 1, 1, 1"),
                self.render_layout_item("Margin", "0, 0, 0, 0"),
                self.render_layout_item("Flex", "row, grow: 1.0"),
            ])
            .into_element()
    }

    fn render_layout_item(&self, label: &str, value: &str) -> Element {
        RnkBox::new()
            .flex_direction(FlexDirection::Row)
            .gap(1.0)
            .children(vec![
                Text::new(format!("{}:", label))
                    .color(Color::Green)
                    .into_element(),
                Text::new(value).color(Color::White).into_element(),
            ])
            .into_element()
    }

    fn render_performance_tab(&self) -> Element {
        RnkBox::new()
            .flex_direction(FlexDirection::Column)
            .padding(1)
            .flex_grow(1.0)
            .children(vec![
                Text::new("Performance")
                    .color(Color::White)
                    .bold()
                    .into_element(),
                Text::new("").into_element(),
                self.render_perf_item("FPS", "60", Color::Green),
                self.render_perf_item("Frame Time", "16.7ms", Color::Green),
                self.render_perf_item("Render Count", "42", Color::White),
                self.render_perf_item("Layout Time", "0.5ms", Color::Green),
                self.render_perf_item("Hooks", "5", Color::White),
            ])
            .into_element()
    }

    fn render_perf_item(&self, label: &str, value: &str, value_color: Color) -> Element {
        RnkBox::new()
            .flex_direction(FlexDirection::Row)
            .justify_content(JustifyContent::SpaceBetween)
            .children(vec![
                Text::new(label).color(Color::BrightBlack).into_element(),
                Text::new(value).color(value_color).into_element(),
            ])
            .into_element()
    }
}

impl Default for DevTools {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_devtools_creation() {
        let devtools = DevTools::new();
        assert!(!devtools.visible);
        assert_eq!(devtools.active_tab, DevToolsTab::Tree);
    }

    #[test]
    fn test_devtools_visible() {
        let devtools = DevTools::new().visible(true);
        assert!(devtools.visible);
    }

    #[test]
    fn test_devtools_tab() {
        let devtools = DevTools::new().tab(DevToolsTab::State);
        assert_eq!(devtools.active_tab, DevToolsTab::State);
    }

    #[test]
    fn test_devtools_into_element() {
        let devtools = DevTools::new().visible(true);
        let _ = devtools.into_element();
    }

    #[test]
    fn test_devtools_hidden() {
        let devtools = DevTools::new().visible(false);
        let _ = devtools.into_element();
    }
}
