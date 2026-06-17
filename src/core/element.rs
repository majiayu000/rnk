//! Element types for the UI tree

use crate::core::Style;
use std::sync::atomic::{AtomicU64, Ordering};

/// Global element ID counter
static ELEMENT_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Unique element identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ElementId(u64);

impl ElementId {
    /// Create a new unique element ID
    pub fn new() -> Self {
        let id = ELEMENT_ID_COUNTER
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                current.checked_add(1).filter(|next| *next != 0)
            })
            .unwrap_or_else(|_| {
                panic!("ElementId counter exhausted; cannot allocate more element IDs")
            });
        Self(id)
    }

    /// Get the root element ID
    pub const fn root() -> Self {
        Self(0)
    }

    /// Get the raw ID value
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for ElementId {
    fn default() -> Self {
        Self::new()
    }
}

/// Element type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementType {
    /// Root element
    Root,
    /// Box container element
    Box,
    /// Text element
    Text,
    /// Virtual text (nested inside Text)
    VirtualText,
}

/// Semantic role exposed to accessibility and testing consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccessibilityRole {
    /// Generic container with no stronger semantic meaning.
    #[default]
    Generic,
    /// Static text content.
    Text,
    /// Button-like action.
    Button,
    /// Single-line editable text input.
    TextInput,
    /// Multi-line editable text input.
    TextArea,
    /// Single-selection list or select control.
    Select,
    /// Multi-selection list control.
    MultiSelect,
    /// A selectable option in a list.
    Option,
    /// Dialog or modal surface.
    Dialog,
    /// Command menu or palette.
    Menu,
    /// Color picker control.
    ColorPicker,
    /// File picker control.
    FilePicker,
    /// Scrollable viewport.
    Viewport,
    /// Passive status or feedback message.
    Status,
}

/// Accessibility metadata attached to an element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibilityProps {
    /// Semantic role for the element.
    pub role: AccessibilityRole,
    /// Human-readable label.
    pub label: Option<String>,
    /// Longer description or usage hint.
    pub description: Option<String>,
    /// Whether the element is disabled.
    pub disabled: bool,
    /// Whether the element is read-only.
    pub read_only: bool,
    /// Whether the element participates in focus traversal.
    pub focusable: bool,
    /// Selection state, when applicable.
    pub selected: Option<bool>,
    /// Current value, when applicable.
    pub value: Option<String>,
}

impl Default for AccessibilityProps {
    fn default() -> Self {
        Self::new(AccessibilityRole::Generic)
    }
}

impl AccessibilityProps {
    /// Create metadata for a role.
    pub fn new(role: AccessibilityRole) -> Self {
        Self {
            role,
            label: None,
            description: None,
            disabled: false,
            read_only: false,
            focusable: false,
            selected: None,
            value: None,
        }
    }

    /// Set the human-readable label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the longer description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set disabled state.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set read-only state.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Set whether this element is focusable.
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    /// Set selection state.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    /// Set current value.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }
}

/// Children container
#[derive(Debug, Clone, Default)]
pub struct Children(Vec<Element>);

impl Children {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, element: Element) {
        self.0.push(element);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Element> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Element> {
        self.0.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&Element> {
        self.0.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Element> {
        self.0.get_mut(index)
    }
}

impl IntoIterator for Children {
    type Item = Element;
    type IntoIter = std::vec::IntoIter<Element>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Children {
    type Item = &'a Element;
    type IntoIter = std::slice::Iter<'a, Element>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl FromIterator<Element> for Children {
    fn from_iter<I: IntoIterator<Item = Element>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// Span and Line types (re-exported from components::text)
/// We use a simplified version here to avoid circular dependencies
pub use crate::components::text::Line;

/// UI Element
#[derive(Debug)]
pub struct Element {
    /// Unique identifier
    pub id: ElementId,
    /// Element type
    pub element_type: ElementType,
    /// Style properties
    pub style: Style,
    /// Child elements
    pub children: Children,
    /// Text content (for Text elements) - simple text
    pub text_content: Option<String>,
    /// Rich text spans (for Text elements with mixed styles)
    pub spans: Option<Vec<Line>>,
    /// Key for reconciliation
    pub key: Option<String>,
    /// Accessibility metadata for this element.
    pub accessibility: Option<AccessibilityProps>,
    /// Horizontal scroll offset (for overflow: scroll/hidden)
    pub scroll_offset_x: Option<u16>,
    /// Vertical scroll offset (for overflow: scroll/hidden)
    pub scroll_offset_y: Option<u16>,
}

/// Clone implementation for Element.
///
/// **Important**: Cloning an Element creates a new unique ID for the clone.
/// This is intentional because each element in the UI tree must have a unique
/// identity for proper reconciliation and layout tracking.
///
/// If you need to compare elements by content rather than identity, compare
/// their individual fields (text_content, children, style, etc.) instead of
/// using the id field.
impl Clone for Element {
    fn clone(&self) -> Self {
        Self {
            id: ElementId::new(), // Generate new ID for clone
            element_type: self.element_type,
            style: self.style.clone(),
            children: self.children.clone(),
            text_content: self.text_content.clone(),
            spans: self.spans.clone(),
            key: self.key.clone(),
            accessibility: self.accessibility.clone(),
            scroll_offset_x: self.scroll_offset_x,
            scroll_offset_y: self.scroll_offset_y,
        }
    }
}

impl Element {
    /// Create a new element
    pub fn new(element_type: ElementType) -> Self {
        Self {
            id: ElementId::new(),
            element_type,
            style: Style::new(),
            children: Children::new(),
            text_content: None,
            spans: None,
            key: None,
            accessibility: None,
            scroll_offset_x: None,
            scroll_offset_y: None,
        }
    }

    /// Create a root element
    pub fn root() -> Self {
        Self {
            id: ElementId::root(),
            element_type: ElementType::Root,
            style: Style::new(),
            children: Children::new(),
            text_content: None,
            spans: None,
            key: None,
            accessibility: None,
            scroll_offset_x: None,
            scroll_offset_y: None,
        }
    }

    /// Create a box element
    pub fn box_element() -> Self {
        Self::new(ElementType::Box)
    }

    /// Create a text element
    pub fn text(content: impl Into<String>) -> Self {
        let mut element = Self::new(ElementType::Text);
        element.text_content = Some(content.into());
        element
    }

    /// Set the element key
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Attach accessibility metadata.
    pub fn with_accessibility(mut self, props: AccessibilityProps) -> Self {
        self.accessibility = Some(props);
        self
    }

    /// Get accessibility metadata.
    pub fn accessibility(&self) -> Option<&AccessibilityProps> {
        self.accessibility.as_ref()
    }

    /// Get mutable accessibility metadata.
    pub fn accessibility_mut(&mut self) -> Option<&mut AccessibilityProps> {
        self.accessibility.as_mut()
    }

    /// Add a child element
    pub fn add_child(&mut self, child: Element) {
        self.children.push(child);
    }

    /// Check if this is a text element
    pub fn is_text(&self) -> bool {
        matches!(
            self.element_type,
            ElementType::Text | ElementType::VirtualText
        )
    }

    /// Check if this is a box element
    pub fn is_box(&self) -> bool {
        matches!(self.element_type, ElementType::Box)
    }

    /// Check if this is the root element
    pub fn is_root(&self) -> bool {
        matches!(self.element_type, ElementType::Root)
    }

    /// Get the display text (for text elements)
    pub fn get_text(&self) -> Option<&str> {
        self.text_content.as_deref()
    }

    /// Return readable fallback text from semantic metadata and descendants.
    pub fn accessible_text(&self) -> String {
        let mut parts = Vec::new();

        if let Some(accessibility) = &self.accessibility {
            push_unique_part(&mut parts, accessibility.label.as_deref());
            push_unique_part(&mut parts, accessibility.value.as_deref());
            push_unique_part(&mut parts, accessibility.description.as_deref());
        }

        if parts.is_empty() {
            push_unique_part(&mut parts, self.text_content.as_deref());
        }

        for child in &self.children {
            let text = child.accessible_text();
            push_unique_part(&mut parts, (!text.is_empty()).then_some(text.as_str()));
        }

        parts.join(" ")
    }
}

fn push_unique_part(parts: &mut Vec<String>, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    let trimmed = value.trim();
    if trimmed.is_empty() || parts.iter().any(|part| part == trimmed) {
        return;
    }
    parts.push(trimmed.to_string());
}

impl Default for Element {
    fn default() -> Self {
        Self::new(ElementType::Box)
    }
}

/// Implement `From<T> for Element` for types that have `into_element(self) -> Element`.
#[macro_export]
macro_rules! impl_into_element {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for $crate::core::Element {
                fn from(c: $ty) -> Self {
                    c.into_element()
                }
            }
        )*
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_id_unique() {
        let id1 = ElementId::new();
        let id2 = ElementId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_root_element_id() {
        let root = ElementId::root();
        assert_eq!(root.as_u64(), 0);
    }

    #[test]
    fn test_element_creation() {
        let element = Element::box_element();
        assert_eq!(element.element_type, ElementType::Box);
        assert!(element.children.is_empty());
    }

    #[test]
    fn test_text_element() {
        let element = Element::text("Hello");
        assert_eq!(element.element_type, ElementType::Text);
        assert_eq!(element.get_text(), Some("Hello"));
    }

    #[test]
    fn test_add_child() {
        let mut parent = Element::box_element();
        let child = Element::text("Child");
        parent.add_child(child);
        assert_eq!(parent.children.len(), 1);
    }

    #[test]
    fn test_children_iterator() {
        let mut parent = Element::box_element();
        parent.add_child(Element::text("A"));
        parent.add_child(Element::text("B"));

        let texts: Vec<_> = parent
            .children
            .iter()
            .filter_map(|e| e.get_text())
            .collect();
        assert_eq!(texts, vec!["A", "B"]);
    }

    #[test]
    fn test_clone_creates_new_id() {
        let original = Element::text("Hello");
        let cloned = original.clone();

        // Clone should have different ID
        assert_ne!(original.id, cloned.id);

        // But same content
        assert_eq!(original.get_text(), cloned.get_text());
        assert_eq!(original.element_type, cloned.element_type);
    }

    #[test]
    fn test_accessibility_metadata_and_fallback_text() {
        let mut element = Element::box_element().with_accessibility(
            AccessibilityProps::new(AccessibilityRole::Button)
                .label("Submit")
                .description("Saves the form")
                .focusable(true),
        );
        element.add_child(Element::text("Ctrl+S"));

        let Some(props) = element.accessibility() else {
            panic!("accessibility metadata missing");
        };
        assert_eq!(props.role, AccessibilityRole::Button);
        assert!(props.focusable);
        assert_eq!(element.accessible_text(), "Submit Saves the form Ctrl+S");
    }

    #[test]
    fn test_accessible_text_uses_descendants_when_unlabelled() {
        let mut element = Element::box_element();
        element.add_child(Element::text("First"));
        element.add_child(Element::text("Second"));

        assert_eq!(element.accessible_text(), "First Second");
    }
}
