//! Modal/Dialog component for overlay content
//!
//! Provides a centered overlay that can be used for dialogs, confirmations,
//! and other modal interactions.

use crate::core::{AlignItems, BorderStyle, Color, Element, FlexDirection, JustifyContent};

/// Modal alignment options
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ModalAlign {
    /// Center the modal (default)
    #[default]
    Center,
    /// Align to top
    Top,
    /// Align to bottom
    Bottom,
}

/// Modal component for displaying overlay content
///
/// # Example
///
/// ```rust
/// use rnk::components::{Modal, Text, Box};
/// use rnk::core::BorderStyle;
///
/// let modal = Modal::new()
///     .title("Confirm")
///     .border_style(BorderStyle::Round)
///     .width(40)
///     .child(Text::new("Are you sure?").into_element())
///     .into_element();
/// ```
#[derive(Default)]
pub struct Modal {
    /// Modal title (optional)
    title: Option<String>,
    /// Child content
    children: Vec<Element>,
    /// Border style
    border_style: BorderStyle,
    /// Modal width (in characters)
    width: Option<u16>,
    /// Modal height (in lines)
    height: Option<u16>,
    /// Padding inside the modal
    padding: u16,
    /// Vertical alignment
    align: ModalAlign,
    /// Background color
    background: Option<Color>,
    /// Border color
    border_color: Option<Color>,
    /// Title color
    title_color: Option<Color>,
    /// Whether to show a backdrop/overlay
    backdrop: bool,
    /// Backdrop character
    backdrop_char: char,
}

impl Modal {
    /// Create a new modal
    pub fn new() -> Self {
        Self {
            title: None,
            children: Vec::new(),
            border_style: BorderStyle::Single,
            width: None,
            height: None,
            padding: 1,
            align: ModalAlign::Center,
            background: None,
            border_color: None,
            title_color: None,
            backdrop: false,
            backdrop_char: ' ',
        }
    }

    /// Set the modal title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Add a child element
    pub fn child(mut self, child: Element) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple children
    pub fn children(mut self, children: impl IntoIterator<Item = Element>) -> Self {
        self.children.extend(children);
        self
    }

    /// Set the border style
    pub fn border_style(mut self, style: BorderStyle) -> Self {
        self.border_style = style;
        self
    }

    /// Set the modal width
    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the modal height
    pub fn height(mut self, height: u16) -> Self {
        self.height = Some(height);
        self
    }

    /// Set the padding inside the modal
    pub fn padding(mut self, padding: u16) -> Self {
        self.padding = padding;
        self
    }

    /// Set the vertical alignment
    pub fn align(mut self, align: ModalAlign) -> Self {
        self.align = align;
        self
    }

    /// Set the background color
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Set the border color
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Set the title color
    pub fn title_color(mut self, color: Color) -> Self {
        self.title_color = Some(color);
        self
    }

    /// Enable backdrop (fills background)
    pub fn backdrop(mut self, enabled: bool) -> Self {
        self.backdrop = enabled;
        self
    }

    /// Set the backdrop character
    pub fn backdrop_char(mut self, ch: char) -> Self {
        self.backdrop_char = ch;
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        use crate::components::Box;
        use crate::components::Text;

        // Build the modal content box
        let mut content_box = Box::new()
            .flex_direction(FlexDirection::Column)
            .border_style(self.border_style)
            .padding(self.padding);

        // Apply width if specified
        if let Some(w) = self.width {
            content_box = content_box.width(w);
        }

        // Apply height if specified
        if let Some(h) = self.height {
            content_box = content_box.height(h);
        }

        // Apply background color
        if let Some(bg) = self.background {
            content_box = content_box.background(bg);
        }

        // Apply border color
        if let Some(bc) = self.border_color {
            content_box = content_box.border_color(bc);
        }

        // Add title if present
        if let Some(title) = &self.title {
            let mut title_text = Text::new(title).bold();
            if let Some(tc) = self.title_color {
                title_text = title_text.color(tc);
            }
            content_box = content_box.child(title_text.into_element());

            // Add a spacer after title
            content_box = content_box.child(Text::new("").into_element());
        }

        // Add children
        for child in self.children {
            content_box = content_box.child(child);
        }

        // Wrap in a centering container
        let justify = match self.align {
            ModalAlign::Center => JustifyContent::Center,
            ModalAlign::Top => JustifyContent::FlexStart,
            ModalAlign::Bottom => JustifyContent::FlexEnd,
        };

        let mut wrapper = Box::new()
            .flex_direction(FlexDirection::Column)
            .justify_content(justify)
            .align_items(AlignItems::Center)
            .flex_grow(1.0);

        // Add backdrop if enabled
        if self.backdrop {
            wrapper = wrapper.background(Color::Black);
        }

        wrapper.child(content_box.into_element()).into_element()
    }
}

/// Dialog component - a modal with action buttons
///
/// # Example
///
/// ```rust
/// use rnk::components::{Dialog, Text};
///
/// let dialog = Dialog::new()
///     .title("Confirm Delete")
///     .message("Are you sure you want to delete this item?")
///     .confirm_label("Delete")
///     .cancel_label("Cancel")
///     .into_element();
/// ```
#[derive(Default)]
pub struct Dialog {
    /// Dialog title
    title: Option<String>,
    /// Dialog message
    message: Option<String>,
    /// Custom content (alternative to message)
    content: Vec<Element>,
    /// Confirm button label
    confirm_label: String,
    /// Cancel button label
    cancel_label: String,
    /// Which button is focused (0 = confirm, 1 = cancel)
    focused_button: usize,
    /// Border style
    border_style: BorderStyle,
    /// Dialog width
    width: Option<u16>,
    /// Confirm button color
    confirm_color: Option<Color>,
    /// Cancel button color
    cancel_color: Option<Color>,
    /// Focus indicator color
    focus_color: Option<Color>,
}

impl Dialog {
    /// Create a new dialog
    pub fn new() -> Self {
        Self {
            title: None,
            message: None,
            content: Vec::new(),
            confirm_label: "OK".to_string(),
            cancel_label: "Cancel".to_string(),
            focused_button: 0,
            border_style: BorderStyle::Round,
            width: Some(50),
            confirm_color: Some(Color::Green),
            cancel_color: Some(Color::Red),
            focus_color: Some(Color::Cyan),
        }
    }

    /// Set the dialog title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the dialog message
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Add custom content
    pub fn content(mut self, element: Element) -> Self {
        self.content.push(element);
        self
    }

    /// Set the confirm button label
    pub fn confirm_label(mut self, label: impl Into<String>) -> Self {
        self.confirm_label = label.into();
        self
    }

    /// Set the cancel button label
    pub fn cancel_label(mut self, label: impl Into<String>) -> Self {
        self.cancel_label = label.into();
        self
    }

    /// Set which button is focused (0 = confirm, 1 = cancel)
    pub fn focused_button(mut self, index: usize) -> Self {
        self.focused_button = index.min(1);
        self
    }

    /// Set the border style
    pub fn border_style(mut self, style: BorderStyle) -> Self {
        self.border_style = style;
        self
    }

    /// Set the dialog width
    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the confirm button color
    pub fn confirm_color(mut self, color: Color) -> Self {
        self.confirm_color = Some(color);
        self
    }

    /// Set the cancel button color
    pub fn cancel_color(mut self, color: Color) -> Self {
        self.cancel_color = Some(color);
        self
    }

    /// Set the focus indicator color
    pub fn focus_color(mut self, color: Color) -> Self {
        self.focus_color = Some(color);
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        use crate::components::Box;
        use crate::components::Text;

        let mut modal = Modal::new().border_style(self.border_style);

        if let Some(w) = self.width {
            modal = modal.width(w);
        }

        if let Some(title) = &self.title {
            modal = modal.title(title);
        }

        // Add message or custom content
        if let Some(msg) = &self.message {
            modal = modal.child(Text::new(msg).into_element());
        }

        for element in self.content {
            modal = modal.child(element);
        }

        // Add spacer before buttons
        modal = modal.child(Text::new("").into_element());

        // Create button row
        let confirm_focused = self.focused_button == 0;
        let cancel_focused = self.focused_button == 1;

        let mut confirm_text = Text::new(format!("[ {} ]", self.confirm_label));
        if let Some(color) = self.confirm_color {
            confirm_text = confirm_text.color(color);
        }
        if confirm_focused {
            confirm_text = confirm_text.bold();
            if let Some(fc) = self.focus_color {
                confirm_text = confirm_text.color(fc);
            }
        }

        let mut cancel_text = Text::new(format!("[ {} ]", self.cancel_label));
        if let Some(color) = self.cancel_color {
            cancel_text = cancel_text.color(color);
        }
        if cancel_focused {
            cancel_text = cancel_text.bold();
            if let Some(fc) = self.focus_color {
                cancel_text = cancel_text.color(fc);
            }
        }

        let button_row = Box::new()
            .flex_direction(FlexDirection::Row)
            .justify_content(JustifyContent::Center)
            .gap(2.0)
            .child(confirm_text.into_element())
            .child(cancel_text.into_element())
            .into_element();

        modal = modal.child(button_row);

        modal.into_element()
    }
}

/// State for managing dialog interactions
#[derive(Debug, Clone, Default)]
pub struct DialogState {
    /// Whether the dialog is visible
    pub visible: bool,
    /// Currently focused button (0 = confirm, 1 = cancel)
    pub focused_button: usize,
}

impl DialogState {
    /// Create a new dialog state
    pub fn new() -> Self {
        Self {
            visible: false,
            focused_button: 0,
        }
    }

    /// Show the dialog
    pub fn show(&mut self) {
        self.visible = true;
        self.focused_button = 0;
    }

    /// Hide the dialog
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Focus next button
    pub fn focus_next(&mut self) {
        self.focused_button = (self.focused_button + 1) % 2;
    }

    /// Focus previous button
    pub fn focus_previous(&mut self) {
        self.focused_button = if self.focused_button == 0 { 1 } else { 0 };
    }

    /// Check if confirm button is focused
    pub fn is_confirm_focused(&self) -> bool {
        self.focused_button == 0
    }

    /// Check if cancel button is focused
    pub fn is_cancel_focused(&self) -> bool {
        self.focused_button == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modal_creation() {
        let modal = Modal::new()
            .title("Test Modal")
            .width(40)
            .padding(2)
            .border_style(BorderStyle::Round);

        // Should not panic
        let _element = modal.into_element();
    }

    #[test]
    fn test_modal_with_children() {
        use crate::components::Text;

        let modal = Modal::new()
            .title("Test")
            .child(Text::new("Line 1").into_element())
            .child(Text::new("Line 2").into_element())
            .into_element();

        // Should have children
        assert!(!modal.children.is_empty());
    }

    #[test]
    fn test_modal_alignment() {
        let top = Modal::new().align(ModalAlign::Top).into_element();
        let center = Modal::new().align(ModalAlign::Center).into_element();
        let bottom = Modal::new().align(ModalAlign::Bottom).into_element();

        // All should create valid elements
        assert!(!top.children.is_empty());
        assert!(!center.children.is_empty());
        assert!(!bottom.children.is_empty());
    }

    #[test]
    fn test_dialog_creation() {
        let dialog = Dialog::new()
            .title("Confirm")
            .message("Are you sure?")
            .confirm_label("Yes")
            .cancel_label("No");

        let _element = dialog.into_element();
    }

    #[test]
    fn test_dialog_state() {
        let mut state = DialogState::new();

        assert!(!state.visible);
        assert!(state.is_confirm_focused());

        state.show();
        assert!(state.visible);

        state.focus_next();
        assert!(state.is_cancel_focused());

        state.focus_previous();
        assert!(state.is_confirm_focused());

        state.hide();
        assert!(!state.visible);
    }

    #[test]
    fn test_dialog_focused_button() {
        let dialog = Dialog::new()
            .title("Test")
            .message("Test message")
            .focused_button(1);

        let _element = dialog.into_element();
    }
}
