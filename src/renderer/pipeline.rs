//! Render pipeline extraction for dynamic frame rendering.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::core::{Element, VNode};
use crate::layout::LayoutEngine;
use crate::reconciler::SiblingIdentity;
use crate::renderer::element_renderer::try_render_element;
use crate::renderer::{Output, TextRenderError};
use crate::runtime::RuntimeContext;

/// Dynamic render pipeline for the `App` runner.
pub(crate) struct RenderPipeline;

impl RenderPipeline {
    #[allow(dead_code)]
    pub(crate) fn render_dynamic_frame(
        dynamic_root: &Element,
        width: u16,
        height: u16,
        layout_engine: &mut LayoutEngine,
        runtime_context: &Rc<RefCell<RuntimeContext>>,
        previous_vnode: &mut Option<VNode>,
    ) -> String {
        Self::try_render_dynamic_frame(
            dynamic_root,
            width,
            height,
            layout_engine,
            runtime_context,
            previous_vnode,
        )
        .unwrap_or_else(|error| panic!("dynamic text render failed: {error}"))
    }

    pub(crate) fn try_render_dynamic_frame(
        dynamic_root: &Element,
        width: u16,
        height: u16,
        layout_engine: &mut LayoutEngine,
        runtime_context: &Rc<RefCell<RuntimeContext>>,
        previous_vnode: &mut Option<VNode>,
    ) -> Result<String, TextRenderError> {
        Self::try_render_dynamic_frame_with_renderer(
            dynamic_root,
            width,
            height,
            layout_engine,
            runtime_context,
            previous_vnode,
            try_render_element,
        )
    }

    fn try_render_dynamic_frame_with_renderer(
        dynamic_root: &Element,
        width: u16,
        height: u16,
        layout_engine: &mut LayoutEngine,
        runtime_context: &Rc<RefCell<RuntimeContext>>,
        previous_vnode: &mut Option<VNode>,
        renderer: impl FnOnce(
            &Element,
            &LayoutEngine,
            &mut Output,
            f32,
            f32,
        ) -> Result<(), TextRenderError>,
    ) -> Result<String, TextRenderError> {
        let (current_vnode, _layout_outcome) = match layout_engine.try_compute_element_incremental(
            dynamic_root,
            previous_vnode.as_ref(),
            width,
            height,
        ) {
            Ok(candidate) => candidate,
            Err(source) => {
                *layout_engine = LayoutEngine::new();
                return Err(TextRenderError::flow(dynamic_root.id, source));
            }
        };

        let mut key_aliases = HashMap::new();
        Self::collect_key_aliases(dynamic_root, layout_engine, &mut key_aliases);
        let layouts = layout_engine.get_all_layouts();
        let vnode_layouts = layout_engine.get_all_vnode_layouts();

        let Some(root_layout) = layout_engine.get_layout(dynamic_root.id) else {
            *layout_engine = LayoutEngine::new();
            return Err(TextRenderError::IncompleteSourceMap {
                element_id: dynamic_root.id,
            });
        };
        let content_width = (root_layout.width as u16).max(1).min(width);
        let render_height = (root_layout.height as u16).max(1).min(height);

        let mut output = Output::new(content_width, render_height);
        if let Err(error) = renderer(dynamic_root, layout_engine, &mut output, 0.0, 0.0) {
            *layout_engine = LayoutEngine::new();
            return Err(error);
        }
        let rendered = output.render();

        runtime_context
            .borrow_mut()
            .set_measure_layouts_with_node_keys(layouts, vnode_layouts, key_aliases);
        *previous_vnode = Some(current_vnode);
        Ok(rendered)
    }

    fn collect_key_aliases(
        element: &Element,
        layout_engine: &LayoutEngine,
        out: &mut HashMap<String, SiblingIdentity>,
    ) {
        if let Some(key) = &element.key
            && let Some(node_key) = layout_engine.node_key_for_element(element.id)
        {
            out.insert(key.clone(), node_key.identity());
        }

        for child in &element.children {
            Self::collect_key_aliases(child, layout_engine, out);
        }
    }
}

#[cfg(test)]
mod typed_error_tests {
    use super::*;
    use crate::components::{Box, Text};
    use crate::layout::TextFlowError;
    use crate::renderer::TextCoordinateError;

    #[test]
    fn text_flow_error_keeps_previous_vnode() {
        let runtime_context = Rc::new(RefCell::new(RuntimeContext::new()));
        let mut layout_engine = LayoutEngine::new();
        let mut previous_vnode = None;
        let stable = Element::text("stable");
        RenderPipeline::render_dynamic_frame(
            &stable,
            20,
            4,
            &mut layout_engine,
            &runtime_context,
            &mut previous_vnode,
        );
        let before = previous_vnode.clone();
        layout_engine.set_text_flow_policy(0, "…", 1);

        let attempt = RenderPipeline::try_render_dynamic_frame(
            &Element::text("failing"),
            20,
            4,
            &mut layout_engine,
            &runtime_context,
            &mut previous_vnode,
        );

        assert!(matches!(
            attempt,
            Err(TextRenderError::Flow {
                source: TextFlowError::InvalidTabStop,
                ..
            })
        ));
        assert_eq!(previous_vnode, before);
        assert!(layout_engine.get_all_layouts().is_empty());
    }

    fn keyed_root(text: &str) -> Element {
        Box::new()
            .width(12)
            .height(2)
            .child(Text::new(text).into_element().with_key("child"))
            .into_element()
            .with_key("root")
    }

    #[test]
    fn incremental_failure_retries_from_clean_layout_tree() {
        let runtime_context = Rc::new(RefCell::new(RuntimeContext::new()));
        let mut layout_engine = LayoutEngine::new();
        let mut previous_vnode = None;
        let stable = keyed_root("stable");
        RenderPipeline::try_render_dynamic_frame(
            &stable,
            20,
            4,
            &mut layout_engine,
            &runtime_context,
            &mut previous_vnode,
        )
        .unwrap();
        let stable_vnode = previous_vnode.clone();
        let stable_measurement = runtime_context
            .borrow()
            .get_measurement_by_key_dims("child");

        let mut invalid_layout = keyed_root("invalid");
        invalid_layout
            .children
            .get_mut(0)
            .expect("test root has one child")
            .style
            .padding
            .left = f32::NAN;
        let layout_failure = RenderPipeline::try_render_dynamic_frame(
            &invalid_layout,
            20,
            4,
            &mut layout_engine,
            &runtime_context,
            &mut previous_vnode,
        );
        assert!(matches!(
            layout_failure,
            Err(TextRenderError::Coordinate {
                source: TextCoordinateError::NonFinite,
                ..
            })
        ));
        assert!(layout_engine.get_all_layouts().is_empty());
        assert_eq!(previous_vnode, stable_vnode);
        assert_eq!(
            runtime_context
                .borrow()
                .get_measurement_by_key_dims("child"),
            stable_measurement
        );

        let corrected = keyed_root("corrected");
        let corrected_output = RenderPipeline::try_render_dynamic_frame(
            &corrected,
            20,
            4,
            &mut layout_engine,
            &runtime_context,
            &mut previous_vnode,
        )
        .unwrap();
        assert!(corrected_output.contains("corrected"));
        assert_eq!(layout_engine.get_all_layouts().len(), 2);
        assert_ne!(previous_vnode, stable_vnode);
        let corrected_vnode = previous_vnode.clone();
        let corrected_measurement = runtime_context
            .borrow()
            .get_measurement_by_key_dims("child");

        let flow_candidate = keyed_root("flow retry");
        layout_engine.set_text_flow_policy(0, "…", 2);
        let flow_failure = RenderPipeline::try_render_dynamic_frame(
            &flow_candidate,
            20,
            4,
            &mut layout_engine,
            &runtime_context,
            &mut previous_vnode,
        );
        assert!(matches!(
            flow_failure,
            Err(TextRenderError::Flow {
                source: TextFlowError::InvalidTabStop,
                ..
            })
        ));
        assert!(layout_engine.get_all_layouts().is_empty());
        assert!(!layout_engine.has_tree());
        assert_eq!(previous_vnode, corrected_vnode);
        assert_eq!(
            runtime_context
                .borrow()
                .get_measurement_by_key_dims("child"),
            corrected_measurement
        );

        layout_engine.set_text_flow_policy(4, "…", 3);
        let flow_retry_output = RenderPipeline::try_render_dynamic_frame(
            &flow_candidate,
            20,
            4,
            &mut layout_engine,
            &runtime_context,
            &mut previous_vnode,
        )
        .unwrap();
        assert!(flow_retry_output.contains("flow retry"));
        assert_eq!(layout_engine.get_all_layouts().len(), 2);
        assert_eq!(layout_engine.get_all_vnode_layouts().len(), 2);
        assert_eq!(layout_engine.node_count(), 4);
        assert_ne!(previous_vnode, corrected_vnode);
        let flow_vnode = previous_vnode.clone();
        let flow_measurement = runtime_context
            .borrow()
            .get_measurement_by_key_dims("child");

        let missing_id = flow_candidate
            .children
            .get(0)
            .expect("test root has one child")
            .id;
        let projection_failure = RenderPipeline::try_render_dynamic_frame_with_renderer(
            &flow_candidate,
            20,
            4,
            &mut layout_engine,
            &runtime_context,
            &mut previous_vnode,
            |_, _, _, _, _| {
                Err(TextRenderError::MissingCurrentFlow {
                    element_id: missing_id,
                })
            },
        );
        assert!(matches!(
            projection_failure,
            Err(TextRenderError::MissingCurrentFlow { element_id })
                if element_id == missing_id
        ));
        assert!(layout_engine.get_all_layouts().is_empty());
        assert_eq!(previous_vnode, flow_vnode);
        assert_eq!(
            runtime_context
                .borrow()
                .get_measurement_by_key_dims("child"),
            flow_measurement
        );

        let retry_output = RenderPipeline::try_render_dynamic_frame(
            &flow_candidate,
            20,
            4,
            &mut layout_engine,
            &runtime_context,
            &mut previous_vnode,
        )
        .unwrap();
        assert!(retry_output.contains("flow retry"));
        assert_eq!(layout_engine.get_all_layouts().len(), 2);
        assert_eq!(layout_engine.get_all_vnode_layouts().len(), 2);
        assert_eq!(layout_engine.node_count(), 4);
    }
}
