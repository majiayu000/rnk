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
        Self(ELEMENT_ID_COUNTER.fetch_add(1, Ordering::SeqCst))
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

/// Text transformation function type
#[allow(dead_code)]
pub type TextTransform = Box<dyn Fn(&str) -> String + Send + Sync>;

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
    /// Horizontal scroll offset (for overflow: scroll/hidden)
    pub scroll_offset_x: Option<u16>,
    /// Vertical scroll offset (for overflow: scroll/hidden)
    pub scroll_offset_y: Option<u16>,
}

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
}

impl Default for Element {
    fn default() -> Self {
        Self::new(ElementType::Box)
    }
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
}
