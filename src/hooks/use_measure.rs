//! Element measurement hook

use crate::core::ElementId;
use crate::layout::Layout;
use std::collections::HashMap;

/// Measurement result for an element
#[derive(Debug, Clone, Copy, Default)]
pub struct Dimensions {
    pub width: f32,
    pub height: f32,
}

impl From<Layout> for Dimensions {
    fn from(layout: Layout) -> Self {
        Self {
            width: layout.width,
            height: layout.height,
        }
    }
}

/// Context for measuring elements
///
/// This is passed to components that need to measure their children or themselves.
#[derive(Clone, Default)]
pub struct MeasureContext {
    layouts: HashMap<ElementId, Layout>,
}

impl MeasureContext {
    /// Create a new measure context
    pub fn new() -> Self {
        Self {
            layouts: HashMap::new(),
        }
    }

    /// Set layouts from a layout engine (called internally by the renderer)
    pub fn set_layouts(&mut self, layouts: HashMap<ElementId, Layout>) {
        self.layouts = layouts;
    }

    /// Measure an element by its ID
    pub fn measure(&self, element_id: ElementId) -> Option<Dimensions> {
        self.layouts
            .get(&element_id)
            .map(|layout| Dimensions::from(*layout))
    }
}

// Thread-local storage for the measure context
thread_local! {
    static MEASURE_CONTEXT: std::cell::RefCell<Option<MeasureContext>> = const { std::cell::RefCell::new(None) };
}

/// Set the current measure context (called by App during render)
pub fn set_measure_context(ctx: Option<MeasureContext>) {
    MEASURE_CONTEXT.with(|c| {
        *c.borrow_mut() = ctx;
    });
}

/// Get the current measure context
pub fn get_measure_context() -> Option<MeasureContext> {
    MEASURE_CONTEXT.with(|c| c.borrow().clone())
}

/// Measure an element by its ID
///
/// Returns the dimensions (width, height) of the element after layout.
/// This must be called during or after the render phase when layout
/// has been computed.
///
/// # Example
///
/// ```ignore
/// let element = Box::new()
///     .width(20)
///     .height(10)
///     .into_element();
///
/// // After rendering...
/// if let Some(dims) = measure_element(element.id) {
///     println!("Size: {}x{}", dims.width, dims.height);
/// }
/// ```
pub fn measure_element(element_id: ElementId) -> Option<Dimensions> {
    get_measure_context().and_then(|ctx| ctx.measure(element_id))
}

/// Hook to create a ref-like pattern for measuring elements
///
/// Returns a callback that can be used to measure the element after render.
///
/// # Example
///
/// ```ignore
/// let (measure_ref, get_dimensions) = use_measure();
///
/// // Later, after layout:
/// if let Some(dims) = get_dimensions() {
///     // Use dimensions
/// }
/// ```
pub fn use_measure() -> (MeasureRef, impl Fn() -> Option<Dimensions>) {
    use crate::hooks::use_signal;

    let element_id = use_signal(|| None::<ElementId>);
    let element_id_clone = element_id.clone();

    let measure_ref = MeasureRef { element_id };
    let get_dimensions = move || element_id_clone.get().and_then(measure_element);

    (measure_ref, get_dimensions)
}

/// Reference for tracking an element to measure
#[derive(Clone)]
pub struct MeasureRef {
    element_id: crate::hooks::Signal<Option<ElementId>>,
}

impl MeasureRef {
    /// Set the element ID to measure
    pub fn set(&self, id: ElementId) {
        self.element_id.set(Some(id));
    }

    /// Get the current element ID
    pub fn get(&self) -> Option<ElementId> {
        self.element_id.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Element, ElementType};
    use crate::layout::LayoutEngine;

    #[test]
    fn test_dimensions_from_layout() {
        let layout = Layout {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 50.0,
        };

        let dims = Dimensions::from(layout);
        assert_eq!(dims.width, 100.0);
        assert_eq!(dims.height, 50.0);
    }

    #[test]
    fn test_measure_context() {
        let mut element = Element::new(ElementType::Box);
        element.style.width = crate::core::Dimension::Points(80.0);
        element.style.height = crate::core::Dimension::Points(24.0);

        let mut engine = LayoutEngine::new();
        engine.compute(&element, 100, 100);

        let mut ctx = MeasureContext::new();
        ctx.set_layouts(engine.get_all_layouts());

        let dims = ctx.measure(element.id);
        assert!(dims.is_some());
        let dims = dims.unwrap();
        assert_eq!(dims.width, 80.0);
        assert_eq!(dims.height, 24.0);
    }
}
