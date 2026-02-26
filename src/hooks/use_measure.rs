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

// Measure context is now stored in RuntimeContext.
// The legacy thread-local MEASURE_CONTEXT has been removed.

/// Measure an element by its ID
///
/// Returns the dimensions (width, height) of the element after layout.
/// This must be called during or after the render phase when layout
/// has been computed.
pub fn measure_element(element_id: ElementId) -> Option<Dimensions> {
    if let Some(ctx) = crate::runtime::current_runtime() {
        ctx.borrow()
            .get_measurement_dims(element_id)
            .map(|(w, h)| Dimensions {
                width: w,
                height: h,
            })
    } else {
        None
    }
}

/// Measure an element by its user-provided key.
pub fn measure_element_by_key(key: &str) -> Option<Dimensions> {
    if let Some(ctx) = crate::runtime::current_runtime() {
        ctx.borrow()
            .get_measurement_by_key_dims(key)
            .map(|(w, h)| Dimensions {
                width: w,
                height: h,
            })
    } else {
        None
    }
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
    let element_key = use_signal(|| None::<String>);
    let element_id_clone = element_id.clone();
    let element_key_clone = element_key.clone();

    let measure_ref = MeasureRef {
        element_id,
        element_key,
    };
    let get_dimensions = move || {
        if let Some(id) = element_id_clone.get()
            && let Some(dims) = measure_element(id)
        {
            return Some(dims);
        }
        element_key_clone
            .get()
            .as_deref()
            .and_then(measure_element_by_key)
    };

    (measure_ref, get_dimensions)
}

/// Reference for tracking an element to measure
#[derive(Clone)]
pub struct MeasureRef {
    element_id: crate::hooks::Signal<Option<ElementId>>,
    element_key: crate::hooks::Signal<Option<String>>,
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

    /// Set the user key to measure.
    ///
    /// This provides a stable lookup path across frames when element IDs change.
    pub fn set_key(&self, key: impl Into<String>) {
        self.element_key.set(Some(key.into()));
    }

    /// Get the current user key.
    pub fn get_key(&self) -> Option<String> {
        self.element_key.get()
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

    #[test]
    fn test_measure_element_by_key_with_runtime() {
        use crate::runtime::{RuntimeContext, set_current_runtime};
        use std::cell::RefCell;
        use std::collections::HashMap;
        use std::rc::Rc;

        let ctx = Rc::new(RefCell::new(RuntimeContext::new()));
        let mut keyed = HashMap::new();
        keyed.insert(
            "sidebar".to_string(),
            Layout {
                x: 0.0,
                y: 0.0,
                width: 30.0,
                height: 10.0,
            },
        );

        ctx.borrow_mut()
            .set_measure_layouts_with_keys(HashMap::new(), keyed);
        set_current_runtime(Some(ctx));

        let dims = measure_element_by_key("sidebar");
        assert_eq!(dims.map(|d| (d.width, d.height)), Some((30.0, 10.0)));

        set_current_runtime(None);
    }
}
