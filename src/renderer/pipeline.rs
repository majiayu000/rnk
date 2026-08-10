//! Render pipeline extraction for dynamic frame rendering.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::{Element, VNode};
use crate::layout::{IncrementalLayoutError, LayoutEngine, PreparedSnapshotFrame};
use crate::renderer::{DynamicFrameError, Output, TextRenderError};
use crate::runtime::RuntimeContext;

mod prepared;
pub(crate) use prepared::PreparedDynamicFrame;

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
        match Self::try_render_dynamic_frame_checked(
            dynamic_root,
            width,
            height,
            layout_engine,
            runtime_context,
            previous_vnode,
        ) {
            Ok(rendered) => Ok(rendered),
            Err(DynamicFrameError::Text(source)) => Err(source),
            Err(DynamicFrameError::Incremental(IncrementalLayoutError::TextFlow(source))) => {
                Err(TextRenderError::flow(dynamic_root.id, source))
            }
            Err(DynamicFrameError::Incremental(IncrementalLayoutError::Identity(source))) => {
                panic!("dynamic identity validation failed: {source}")
            }
            Err(DynamicFrameError::LegacyLookup(source)) => {
                panic!("dynamic layout lookup failed: {source}")
            }
        }
    }

    pub(crate) fn try_render_dynamic_frame_checked(
        dynamic_root: &Element,
        width: u16,
        height: u16,
        layout_engine: &mut LayoutEngine,
        runtime_context: &Rc<RefCell<RuntimeContext>>,
        previous_vnode: &mut Option<VNode>,
    ) -> Result<String, DynamicFrameError> {
        let prepared = Self::prepare_dynamic_frame(
            dynamic_root,
            width,
            height,
            layout_engine,
            previous_vnode.as_ref(),
        )
        .map_err(|source| prepared::legacy_dynamic_error(source, dynamic_root.id))?;
        Ok(prepared.commit(layout_engine, runtime_context, previous_vnode))
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn try_render_dynamic_frame_with_renderer(
        dynamic_root: &Element,
        width: u16,
        height: u16,
        layout_engine: &mut LayoutEngine,
        runtime_context: &Rc<RefCell<RuntimeContext>>,
        previous_vnode: &mut Option<VNode>,
        renderer: impl FnOnce(
            &Element,
            &PreparedSnapshotFrame,
            &mut Output,
            f32,
            f32,
        ) -> Result<(), TextRenderError>,
    ) -> Result<String, TextRenderError> {
        match Self::try_render_dynamic_frame_with_renderer_checked(
            dynamic_root,
            width,
            height,
            layout_engine,
            runtime_context,
            previous_vnode,
            renderer,
        ) {
            Ok(rendered) => Ok(rendered),
            Err(DynamicFrameError::Text(source)) => Err(source),
            Err(DynamicFrameError::Incremental(IncrementalLayoutError::TextFlow(source))) => {
                Err(TextRenderError::flow(dynamic_root.id, source))
            }
            Err(DynamicFrameError::Incremental(IncrementalLayoutError::Identity(source))) => {
                panic!("dynamic identity validation failed: {source}")
            }
            Err(DynamicFrameError::LegacyLookup(source)) => {
                panic!("dynamic layout lookup failed: {source}")
            }
        }
    }

    fn try_render_dynamic_frame_with_renderer_checked(
        dynamic_root: &Element,
        width: u16,
        height: u16,
        layout_engine: &mut LayoutEngine,
        runtime_context: &Rc<RefCell<RuntimeContext>>,
        previous_vnode: &mut Option<VNode>,
        renderer: impl FnOnce(
            &Element,
            &PreparedSnapshotFrame,
            &mut Output,
            f32,
            f32,
        ) -> Result<(), TextRenderError>,
    ) -> Result<String, DynamicFrameError> {
        let prepared = Self::prepare_dynamic_frame_with_renderer(
            dynamic_root,
            width,
            height,
            layout_engine,
            previous_vnode.as_ref(),
            |element, engine, output, x, y| {
                renderer(element, engine, output, x, y)
                    .map_err(crate::renderer::CheckedRenderError::from)
            },
        )
        .map_err(|source| prepared::legacy_dynamic_error(source, dynamic_root.id))?;
        Ok(prepared.commit(layout_engine, runtime_context, previous_vnode))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Box, Text};
    use crate::layout::{IncrementalLayoutError, LayoutLookupError};
    use crate::reconciler::ReconcilePlanError;
    use crate::renderer::DynamicFrameError;
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

        let attempt = RenderPipeline::prepare_dynamic_frame(
            &Element::text("failing"),
            20,
            4,
            &layout_engine,
            previous_vnode.as_ref(),
        );

        assert!(matches!(
            attempt,
            Err(crate::renderer::TransactionalFrameError::Transaction(
                crate::layout::TransactionalLayoutError::RecoveryFailed { .. }
            ))
        ));
        assert_eq!(previous_vnode, before);
        assert_eq!(layout_engine.get_all_layouts().len(), 1);
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
        assert_eq!(layout_engine.get_all_layouts().len(), 2);
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
        let flow_failure = RenderPipeline::prepare_dynamic_frame(
            &flow_candidate,
            20,
            4,
            &layout_engine,
            previous_vnode.as_ref(),
        );
        assert!(matches!(
            flow_failure,
            Err(crate::renderer::TransactionalFrameError::Transaction(
                crate::layout::TransactionalLayoutError::RecoveryFailed { .. }
            ))
        ));
        assert_eq!(layout_engine.get_all_layouts().len(), 2);
        assert!(layout_engine.has_tree());
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
        assert_eq!(layout_engine.get_all_layouts().len(), 2);
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

    #[test]
    fn identity_error_commits_no_frame_or_previous_vnode() {
        let runtime_context = Rc::new(RefCell::new(RuntimeContext::new()));
        let mut layout_engine = LayoutEngine::new();
        let mut previous_vnode = None;
        let stable = keyed_root("stable");
        RenderPipeline::try_render_dynamic_frame_checked(
            &stable,
            20,
            4,
            &mut layout_engine,
            &runtime_context,
            &mut previous_vnode,
        )
        .unwrap();
        let before_vnode = previous_vnode.clone();
        let before_measurement = runtime_context
            .borrow()
            .get_measurement_by_key_dims("child");
        let stable_root_id = stable.id;

        let invalid = Box::new()
            .child(Text::new("first").key("duplicate"))
            .child(Text::new("second").key("duplicate"))
            .into_element();
        let failure = RenderPipeline::try_render_dynamic_frame_checked(
            &invalid,
            20,
            4,
            &mut layout_engine,
            &runtime_context,
            &mut previous_vnode,
        )
        .expect_err("duplicate target must reach checked pipeline");

        assert!(matches!(
            failure,
            DynamicFrameError::Incremental(IncrementalLayoutError::Identity(
                ReconcilePlanError::DuplicateSiblingKey { .. }
            ))
        ));
        assert_eq!(previous_vnode, before_vnode);
        assert!(layout_engine.get_layout(stable_root_id).is_some());
        assert_eq!(
            runtime_context
                .borrow()
                .get_measurement_by_key_dims("child"),
            before_measurement
        );
    }

    #[test]
    fn repeated_raw_measurement_key_is_typed_ambiguity() {
        let runtime_context = Rc::new(RefCell::new(RuntimeContext::new()));
        let mut layout_engine = LayoutEngine::new();
        let mut previous_vnode = None;
        let tree = Box::new()
            .child(
                Box::new()
                    .key("left")
                    .child(Box::new().key("shared").width(3.0)),
            )
            .child(
                Box::new()
                    .key("right")
                    .child(Box::new().key("shared").width(7.0)),
            )
            .into_element();

        RenderPipeline::try_render_dynamic_frame_checked(
            &tree,
            20,
            4,
            &mut layout_engine,
            &runtime_context,
            &mut previous_vnode,
        )
        .unwrap();

        assert!(matches!(
            runtime_context
                .borrow()
                .try_get_measurement_by_key_dims("shared"),
            Err(LayoutLookupError::AmbiguousMeasurementKey {
                scoped_match_count: 2,
                ..
            })
        ));
    }

    fn measurement_frame(branches: &[(&str, f32)]) -> (Element, Vec<crate::core::ElementId>) {
        let mut root = Box::new().width(30.0).height(4.0);
        let mut shared_ids = Vec::new();
        for (branch, width) in branches {
            let shared = Box::new()
                .key("shared")
                .width(*width)
                .height(1.0)
                .into_element();
            shared_ids.push(shared.id);
            root = root.child(Box::new().key(*branch).child(shared));
        }
        (root.into_element(), shared_ids)
    }

    #[test]
    fn measurement_candidates_transition_unique_ambiguous_unique() {
        use std::panic::{AssertUnwindSafe, catch_unwind};

        let runtime_context = Rc::new(RefCell::new(RuntimeContext::new()));
        let mut layout_engine = LayoutEngine::new();
        let mut previous_vnode = None;

        let (first, first_ids) = measurement_frame(&[("left", 3.0)]);
        RenderPipeline::try_render_dynamic_frame_checked(
            &first,
            30,
            4,
            &mut layout_engine,
            &runtime_context,
            &mut previous_vnode,
        )
        .expect("unique frame renders");
        let first_raw = layout_engine
            .node_key_for_element(first_ids[0])
            .expect("current element has a raw key")
            .identity();
        assert_eq!(
            runtime_context.borrow().get_measurement(first_ids[0]),
            Some((3, 1))
        );
        assert_eq!(
            runtime_context
                .borrow()
                .try_get_measurement_by_key_dims("shared"),
            Ok(Some((3.0, 1.0)))
        );
        assert_eq!(
            runtime_context
                .borrow()
                .try_get_measurement_by_node_key_dims(first_raw),
            Ok(Some((3.0, 1.0)))
        );
        let first_composite = runtime_context
            .borrow()
            .try_resolve_measurement_key_alias("shared")
            .expect("unique alias is not ambiguous")
            .expect("unique alias has a composite projection");
        assert_ne!(first_raw, first_composite);
        assert_eq!(
            runtime_context
                .borrow()
                .try_get_measurement_by_node_key_dims(first_composite),
            Ok(Some((3.0, 1.0)))
        );

        let (second, second_ids) = measurement_frame(&[("left", 4.0), ("right", 7.0)]);
        RenderPipeline::try_render_dynamic_frame_checked(
            &second,
            30,
            4,
            &mut layout_engine,
            &runtime_context,
            &mut previous_vnode,
        )
        .expect("ambiguous frame still renders");
        let second_raw = layout_engine
            .node_key_for_element(second_ids[0])
            .expect("current element has a raw key")
            .identity();
        assert_eq!(runtime_context.borrow().get_measurement(first_ids[0]), None);
        assert_eq!(
            second_ids
                .iter()
                .map(|id| runtime_context.borrow().get_measurement(*id))
                .collect::<Vec<_>>(),
            vec![Some((4, 1)), Some((7, 1))]
        );
        assert!(matches!(
            runtime_context
                .borrow()
                .try_get_measurement_by_key_dims("shared"),
            Err(LayoutLookupError::AmbiguousMeasurementKey {
                scoped_match_count: 2,
                ..
            })
        ));
        assert!(matches!(
            runtime_context
                .borrow()
                .try_get_measurement_by_node_key_dims(second_raw),
            Err(LayoutLookupError::AmbiguousMeasurementNodeIdentity {
                scoped_match_count: 2,
                ..
            })
        ));
        assert_eq!(
            runtime_context
                .borrow()
                .try_resolve_measurement_key_alias("shared"),
            Err(LayoutLookupError::AmbiguousMeasurementKey {
                key_token: crate::core::NodeKey::compatibility_token("shared"),
                scoped_match_count: 2,
            })
        );
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                runtime_context
                    .borrow()
                    .get_measurement_by_node_key_dims(second_raw)
            }))
            .is_err()
        );
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                runtime_context
                    .borrow()
                    .resolve_measurement_key_alias("shared")
            }))
            .is_err()
        );
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                runtime_context
                    .borrow()
                    .get_measurement_by_key_dims("shared")
            }))
            .is_err()
        );

        let (third, third_ids) = measurement_frame(&[("right", 9.0)]);
        RenderPipeline::try_render_dynamic_frame_checked(
            &third,
            30,
            4,
            &mut layout_engine,
            &runtime_context,
            &mut previous_vnode,
        )
        .expect("unique frame renders after ambiguity");
        let third_raw = layout_engine
            .node_key_for_element(third_ids[0])
            .expect("current element has a raw key")
            .identity();
        assert_eq!(
            second_ids
                .iter()
                .map(|id| runtime_context.borrow().get_measurement(*id))
                .collect::<Vec<_>>(),
            vec![None, None]
        );
        assert_eq!(
            runtime_context.borrow().get_measurement(third_ids[0]),
            Some((9, 1))
        );
        assert_eq!(
            runtime_context
                .borrow()
                .try_get_measurement_by_key_dims("shared"),
            Ok(Some((9.0, 1.0)))
        );
        assert_eq!(
            runtime_context
                .borrow()
                .try_get_measurement_by_node_key_dims(third_raw),
            Ok(Some((9.0, 1.0)))
        );
        assert_eq!(
            runtime_context
                .borrow()
                .try_get_measurement_by_node_key_dims(first_composite),
            Ok(None),
            "the removed first-frame composite identity must not retain a stale measurement"
        );
        let unknown = crate::core::NodeKey::with_key(
            "unknown",
            layout_engine
                .node_key_for_element(third_ids[0])
                .expect("current raw key")
                .type_id,
            0,
        )
        .identity();
        assert_eq!(
            runtime_context
                .borrow()
                .try_get_measurement_by_node_key_dims(unknown),
            Ok(None)
        );
    }

    #[test]
    fn failure_commits_no_engine_previous_measurement_or_frame() {
        super::prepared::tests::failure_commits_no_engine_previous_measurement_or_frame();
    }

    #[test]
    fn cancelled_candidate_cannot_interleave_with_next_batch() {
        super::prepared::tests::cancelled_candidate_cannot_interleave_with_next_batch();
    }

    #[test]
    fn unchanged_frame_new_element_ids_render_and_commit_aliases() {
        super::prepared::tests::unchanged_frame_new_element_ids_render_and_commit_aliases();
    }

    #[test]
    fn failed_unchanged_frame_keeps_previous_aliases() {
        super::prepared::tests::failed_unchanged_frame_keeps_previous_aliases();
    }
}
