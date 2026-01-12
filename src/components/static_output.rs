//! Static component - renders content once and persists it

use crate::core::{Element, ElementType, Style};
use crate::hooks::use_signal;

/// Static component that renders items only once.
///
/// Content rendered by Static is "committed" to the terminal and won't be
/// re-rendered, even when the rest of the UI updates. This is useful for
/// logs, completed tasks, or any output that should persist.
///
/// # Example
///
/// ```ignore
/// let logs = use_signal(|| vec!["Starting...", "Loading..."]);
///
/// Static::new(logs.get(), |item, _index| {
///     Text::new(item).into_element()
/// })
/// ```
pub struct Static<T, F>
where
    T: Clone + 'static,
    F: Fn(&T, usize) -> Element,
{
    items: Vec<T>,
    render_fn: F,
    style: Style,
}

impl<T, F> Static<T, F>
where
    T: Clone + 'static,
    F: Fn(&T, usize) -> Element,
{
    /// Create a new Static component with items and render function
    pub fn new(items: Vec<T>, render_fn: F) -> Self {
        Self {
            items,
            render_fn,
            style: Style::default(),
        }
    }

    /// Set custom style for the Static container
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Convert to Element, tracking which items have been rendered
    pub fn into_element(self) -> Element {
        // Track how many items have been rendered
        let rendered_count = use_signal(|| 0usize);

        let current_count = rendered_count.get();
        let total_items = self.items.len();

        // Only render items that haven't been rendered yet
        let new_items: Vec<Element> = self
            .items
            .iter()
            .enumerate()
            .skip(current_count)
            .map(|(index, item)| (self.render_fn)(item, index))
            .collect();

        // Update the rendered count
        if total_items > current_count {
            rendered_count.set(total_items);
        }

        // Create container element
        let mut element = Element::new(ElementType::Box);
        element.style = self.style.clone();
        element.style.is_static = true; // Mark as static for the renderer

        // Add only the new items as children
        for child in new_items {
            element.add_child(child);
        }

        element
    }
}

/// Convenience function for creating Static components
pub fn static_output<T, F>(items: Vec<T>, render_fn: F) -> Element
where
    T: Clone + 'static,
    F: Fn(&T, usize) -> Element,
{
    Static::new(items, render_fn).into_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Text;
    use crate::hooks::context::{HookContext, with_hooks};
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_static_renders_items() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        let element = with_hooks(ctx.clone(), || {
            Static::new(vec!["a", "b", "c"], |item, _| {
                Text::new(*item).into_element()
            })
            .into_element()
        });

        assert_eq!(element.children.len(), 3);
    }

    #[test]
    fn test_static_incremental_render() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        // First render with 2 items
        let element1 = with_hooks(ctx.clone(), || {
            Static::new(vec!["a", "b"], |item, _| Text::new(*item).into_element()).into_element()
        });
        assert_eq!(element1.children.len(), 2);

        // Second render with 4 items - should only render 2 new ones
        // with_hooks automatically resets hook_index via begin_render
        let element2 = with_hooks(ctx.clone(), || {
            Static::new(vec!["a", "b", "c", "d"], |item, _| {
                Text::new(*item).into_element()
            })
            .into_element()
        });

        // Only new items should be rendered
        assert_eq!(element2.children.len(), 2);
    }
}
