//! Render pipeline extraction for dynamic frame rendering.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::core::{Element, VNode};
use crate::layout::{Layout, LayoutEngine};
use crate::renderer::Output;
use crate::renderer::element_renderer::render_element;
use crate::runtime::RuntimeContext;

/// Dynamic render pipeline for the `App` runner.
pub(crate) struct RenderPipeline;

impl RenderPipeline {
    pub(crate) fn render_dynamic_frame(
        dynamic_root: &Element,
        width: u16,
        height: u16,
        layout_engine: &mut LayoutEngine,
        runtime_context: &Rc<RefCell<RuntimeContext>>,
        previous_vnode: &mut Option<VNode>,
    ) -> String {
        // Compute layout with reconciler diff/patch when possible.
        let (current_vnode, _layout_outcome) = layout_engine.compute_element_incremental(
            dynamic_root,
            previous_vnode.as_ref(),
            width,
            height,
        );
        *previous_vnode = Some(current_vnode);

        // Build keyed layout map for stable cross-frame measurement lookups.
        let mut keyed_layouts = HashMap::new();
        Self::collect_keyed_layouts(dynamic_root, layout_engine, &mut keyed_layouts);

        // Update measure context with latest layouts.
        runtime_context
            .borrow_mut()
            .set_measure_layouts_with_keys(layout_engine.get_all_layouts(), keyed_layouts);

        // Get content size from root layout.
        let root_layout = layout_engine
            .get_layout(dynamic_root.id)
            .unwrap_or_default();
        let content_width = (root_layout.width as u16).max(1).min(width);
        let render_height = (root_layout.height as u16).max(1).min(height);

        // Render to output buffer.
        let mut output = Output::new(content_width, render_height);
        render_element(dynamic_root, layout_engine, &mut output, 0.0, 0.0);
        output.render()
    }

    fn collect_keyed_layouts(
        element: &Element,
        layout_engine: &LayoutEngine,
        out: &mut HashMap<String, Layout>,
    ) {
        if let Some(key) = &element.key
            && let Some(layout) = layout_engine.get_layout(element.id)
        {
            out.insert(key.clone(), layout);
        }

        for child in &element.children {
            Self::collect_keyed_layouts(child, layout_engine, out);
        }
    }
}
