//! ScrollableBox component - Container with overflow clipping and virtual scrolling
//!
//! This component provides a scrollable area that:
//! - Clips content that overflows the viewport
//! - Supports virtual scrolling (only renders visible items)
//! - Integrates with use_scroll hook for scroll state management

use crate::components::Box;
use crate::core::{BorderStyle, Color, Element, FlexDirection, Overflow};

/// A scrollable container with overflow clipping
///
/// # Example
///
/// ```ignore
/// use rnk::prelude::*;
///
/// fn app() -> Element {
///     let scroll = use_scroll();
///
///     // Set content size based on total items
///     scroll.set_content_size(80, items.len());
///     scroll.set_viewport_size(80, 10);
///
///     use_input(move |_input, key| {
///         if key.up_arrow { scroll.scroll_up(1); }
///         if key.down_arrow { scroll.scroll_down(1); }
///     });
///
///     ScrollableBox::new()
///         .height(10)
///         .scroll_offset_y(scroll.offset_y() as u16)
///         .children(items.iter().map(|item| {
///             Text::new(item).into_element()
///         }))
///         .into_element()
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct ScrollableBox {
    inner: Box,
    /// Show scrollbar indicator
    show_scrollbar: bool,
    /// Scrollbar color
    scrollbar_color: Option<Color>,
}

impl ScrollableBox {
    /// Create a new ScrollableBox
    pub fn new() -> Self {
        let inner = Box::new()
            .overflow_y(Overflow::Hidden)
            .flex_direction(FlexDirection::Column);

        Self {
            inner,
            show_scrollbar: false,
            scrollbar_color: None,
        }
    }

    /// Set the height of the scrollable area
    pub fn height(mut self, height: impl Into<crate::core::Dimension>) -> Self {
        self.inner = self.inner.height(height);
        self
    }

    /// Set the width of the scrollable area
    pub fn width(mut self, width: impl Into<crate::core::Dimension>) -> Self {
        self.inner = self.inner.width(width);
        self
    }

    /// Set the vertical scroll offset
    pub fn scroll_offset_y(mut self, offset: u16) -> Self {
        self.inner = self.inner.scroll_offset_y(offset);
        self
    }

    /// Set the horizontal scroll offset
    pub fn scroll_offset_x(mut self, offset: u16) -> Self {
        self.inner = self.inner.scroll_offset_x(offset);
        self
    }

    /// Set flex grow
    pub fn flex_grow(mut self, grow: f32) -> Self {
        self.inner = self.inner.flex_grow(grow);
        self
    }

    /// Set flex direction for children
    pub fn flex_direction(mut self, direction: FlexDirection) -> Self {
        self.inner = self.inner.flex_direction(direction);
        self
    }

    /// Set background color
    pub fn background(mut self, color: Color) -> Self {
        self.inner = self.inner.background(color);
        self
    }

    /// Set border style
    pub fn border_style(mut self, style: BorderStyle) -> Self {
        self.inner = self.inner.border_style(style);
        self
    }

    /// Set border color
    pub fn border_color(mut self, color: Color) -> Self {
        self.inner = self.inner.border_color(color);
        self
    }

    /// Set padding
    pub fn padding(mut self, padding: impl Into<crate::core::Edges>) -> Self {
        self.inner = self.inner.padding(padding);
        self
    }

    /// Show scrollbar indicator
    pub fn scrollbar(mut self, show: bool) -> Self {
        self.show_scrollbar = show;
        self
    }

    /// Set scrollbar color
    pub fn scrollbar_color(mut self, color: Color) -> Self {
        self.scrollbar_color = Some(color);
        self
    }

    /// Add a child element
    pub fn child(mut self, element: Element) -> Self {
        self.inner = self.inner.child(element);
        self
    }

    /// Add multiple children
    pub fn children(mut self, elements: impl IntoIterator<Item = Element>) -> Self {
        self.inner = self.inner.children(elements);
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        self.inner.into_element()
    }
}

/// Helper to create a virtual scroll view that only renders visible items
///
/// This is useful for large lists where rendering all items would be expensive.
///
/// # Example
///
/// ```ignore
/// use rnk::prelude::*;
///
/// fn app() -> Element {
///     let scroll = use_scroll();
///     let items: Vec<String> = (0..1000).map(|i| format!("Item {}", i)).collect();
///
///     scroll.set_content_size(80, items.len());
///     scroll.set_viewport_size(80, 20);
///
///     virtual_scroll_view(
///         &items,
///         scroll.offset_y(),
///         20,  // viewport height
///         |item, index| {
///             Text::new(format!("[{}] {}", index, item)).into_element()
///         }
///     )
/// }
/// ```
pub fn virtual_scroll_view<T, F>(
    items: &[T],
    scroll_offset: usize,
    viewport_height: usize,
    render_item: F,
) -> Element
where
    F: Fn(&T, usize) -> Element,
{
    // Calculate visible range
    let start = scroll_offset.min(items.len());
    let end = (scroll_offset + viewport_height).min(items.len());

    // Create container
    let mut container = Box::new()
        .flex_direction(FlexDirection::Column)
        .overflow_y(Overflow::Hidden)
        .height(viewport_height as i32);

    // Only render visible items
    for global_idx in start..end {
        if let Some(item) = items.get(global_idx) {
            let child = render_item(item, global_idx);
            // Position items relative to their visible position
            // The scroll offset is handled by the parent
            container = container.child(child);
        }
    }

    container.into_element()
}

/// Create a fixed-bottom layout with scrollable content area
///
/// This creates the classic chat/terminal layout:
/// ```text
/// ┌────────────────────────────────┐
/// │ [Scrollable content area]      │ ← flex_grow: 1
/// │                                │
/// ├────────────────────────────────┤
/// │ [Fixed bottom area]            │ ← fixed height
/// └────────────────────────────────┘
/// ```
///
/// # Example
///
/// ```ignore
/// use rnk::prelude::*;
///
/// fn app() -> Element {
///     let scroll = use_scroll();
///
///     fixed_bottom_layout(
///         // Content area
///         ScrollableBox::new()
///             .scroll_offset_y(scroll.offset_y() as u16)
///             .children(messages.iter().map(render_message))
///             .into_element(),
///         // Bottom area (input + status)
///         Box::new()
///             .flex_direction(FlexDirection::Column)
///             .child(Text::new("─".repeat(80)).dim().into_element())
///             .child(Text::new("> input here").into_element())
///             .child(Text::new("status bar").dim().into_element())
///             .into_element(),
///     )
/// }
/// ```
pub fn fixed_bottom_layout(content: Element, bottom: Element) -> Element {
    Box::new()
        .flex_direction(FlexDirection::Column)
        .height(crate::core::Dimension::Percent(100.0))
        .child(
            // Scrollable content area takes remaining space
            Box::new()
                .flex_grow(1.0)
                .overflow_y(Overflow::Hidden)
                .child(content)
                .into_element(),
        )
        .child(bottom)
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Text;

    #[test]
    fn test_scrollable_box_creation() {
        let element = ScrollableBox::new()
            .height(10)
            .scroll_offset_y(5)
            .child(Text::new("Hello").into_element())
            .into_element();

        assert_eq!(element.scroll_offset_y, Some(5));
        assert_eq!(element.children.len(), 1);
    }

    #[test]
    fn test_virtual_scroll_view() {
        let items: Vec<String> = (0..100).map(|i| format!("Item {}", i)).collect();

        let element = virtual_scroll_view(
            &items,
            10, // scroll offset
            5,  // viewport height
            |item, _idx| Text::new(item.clone()).into_element(),
        );

        // Should only have 5 children (viewport height)
        assert_eq!(element.children.len(), 5);
    }

    #[test]
    fn test_virtual_scroll_view_at_end() {
        let items: Vec<String> = (0..10).map(|i| format!("Item {}", i)).collect();

        let element = virtual_scroll_view(
            &items,
            8, // scroll offset near end
            5, // viewport height
            |item, _idx| Text::new(item.clone()).into_element(),
        );

        // Should only have 2 children (items 8 and 9)
        assert_eq!(element.children.len(), 2);
    }

    #[test]
    fn test_fixed_bottom_layout() {
        let content = Text::new("Content").into_element();
        let bottom = Text::new("Bottom").into_element();

        let element = fixed_bottom_layout(content, bottom);

        // Should have 2 children: content wrapper and bottom
        assert_eq!(element.children.len(), 2);
    }
}
