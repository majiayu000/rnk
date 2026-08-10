use std::{cell::RefCell, rc::Rc};

use crate::components::{Box, Text};
use crate::core::{Dimension, Element, Position};
use crate::layout::{Axis, CellOutputError, LayoutEngine};
use crate::renderer::{
    CheckedRenderError, Output, SnapshotRenderError, TextRenderError, TransactionalFrameError,
    try_render_element_tree_checked, try_render_to_string_checked,
};
use crate::runtime::RuntimeContext;
use crate::testing::TestRenderer;

use crate::renderer::static_content::StaticRenderer;

use super::super::{PreparedDynamicFrame, RenderPipeline};

fn keyed_root(text: &str) -> Element {
    Box::new()
        .width(12)
        .height(2)
        .child(Text::new(text).into_element().with_key("child"))
        .into_element()
        .with_key("root")
}

fn commit_initial(
    root: &Element,
    engine: &mut LayoutEngine,
    runtime: &Rc<RefCell<RuntimeContext>>,
    previous: &mut Option<crate::core::VNode>,
) {
    let prepared = RenderPipeline::prepare_dynamic_frame(root, 20, 4, engine, previous.as_ref())
        .expect("initial prepared frame");
    prepared.commit(engine, runtime, previous);
}

pub(crate) fn failure_commits_no_engine_previous_measurement_or_frame() {
    let runtime = Rc::new(RefCell::new(RuntimeContext::new()));
    let mut engine = LayoutEngine::new();
    let mut previous = None;
    let stable = keyed_root("stable");
    let stable_id = stable.children.iter().next().expect("stable child").id;
    commit_initial(&stable, &mut engine, &runtime, &mut previous);
    let previous_before = previous.clone();
    let measurement_before = runtime.borrow().get_measurement_by_key_dims("child");
    let node_count_before = engine.node_count();
    let failing = keyed_root("failing");

    let result = RenderPipeline::prepare_dynamic_frame_with_renderer(
        &failing,
        20,
        4,
        &engine,
        previous.as_ref(),
        |element, _, output, _, _| {
            output.write(0, 0, "partial", &element.style);
            Err(CheckedRenderError::Text(
                TextRenderError::MissingCurrentFlow {
                    element_id: element.id,
                },
            ))
        },
    );
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("render failure must drop the prepared layout and output"),
    };

    assert!(matches!(
        error,
        crate::renderer::TransactionalFrameError::Render(CheckedRenderError::Text(_))
    ));
    assert_eq!(engine.node_count(), node_count_before);
    assert!(engine.get_layout(stable_id).is_some());
    assert_eq!(previous, previous_before);
    assert_eq!(
        runtime.borrow().get_measurement_by_key_dims("child"),
        measurement_before
    );
}

pub(crate) fn cancelled_candidate_cannot_interleave_with_next_batch() {
    let runtime = Rc::new(RefCell::new(RuntimeContext::new()));
    let mut engine = LayoutEngine::new();
    let mut previous = None;
    let stable = keyed_root("stable");
    commit_initial(&stable, &mut engine, &runtime, &mut previous);

    let cancelled = keyed_root("cancelled");
    let cancelled_id = cancelled
        .children
        .iter()
        .next()
        .expect("cancelled child")
        .id;
    let prepared =
        RenderPipeline::prepare_dynamic_frame(&cancelled, 20, 4, &engine, previous.as_ref())
            .expect("cancelled candidate prepares");
    drop(prepared);
    let stale =
        RenderPipeline::prepare_dynamic_frame(&cancelled, 20, 4, &engine, previous.as_ref())
            .expect("second candidate remains pending");

    let next = keyed_root("next");
    let next_id = next.children.iter().next().expect("next child").id;
    let prepared = RenderPipeline::prepare_dynamic_frame(&next, 20, 4, &engine, previous.as_ref())
        .expect("next candidate prepares from committed state");
    assert!(prepared.rendered().contains("next"));
    prepared.commit(&mut engine, &runtime, &mut previous);

    assert!(engine.get_layout(cancelled_id).is_none());
    assert!(engine.get_layout(next_id).is_some());
    let previous_after_next = previous.clone();
    let stale_commit = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        stale.commit(&mut engine, &runtime, &mut previous)
    }));
    assert!(stale_commit.is_err(), "stale candidate must not interleave");
    assert_eq!(previous, previous_after_next);
    assert!(engine.get_layout(cancelled_id).is_none());
    assert!(engine.get_layout(next_id).is_some());
}

pub(crate) fn unchanged_frame_new_element_ids_render_and_commit_aliases() {
    let runtime = Rc::new(RefCell::new(RuntimeContext::new()));
    let mut engine = LayoutEngine::new();
    let mut previous = None;
    let first = keyed_root("same");
    let old_id = first.children.iter().next().expect("first child").id;
    commit_initial(&first, &mut engine, &runtime, &mut previous);

    let current = keyed_root("same");
    let current_id = current.children.iter().next().expect("current child").id;
    let prepared =
        RenderPipeline::prepare_dynamic_frame(&current, 20, 4, &engine, previous.as_ref())
            .expect("unchanged candidate refreshes aliases");
    assert!(prepared.layout().engine().get_layout(current_id).is_some());
    assert!(engine.get_layout(current_id).is_none());
    prepared.commit(&mut engine, &runtime, &mut previous);

    assert!(engine.get_layout(current_id).is_some());
    assert!(engine.get_layout(old_id).is_none());
    assert!(runtime.borrow().get_measurement(current_id).is_some());
}

pub(crate) fn failed_unchanged_frame_keeps_previous_aliases() {
    let runtime = Rc::new(RefCell::new(RuntimeContext::new()));
    let mut engine = LayoutEngine::new();
    let mut previous = None;
    let first = keyed_root("same");
    let old_id = first.children.iter().next().expect("first child").id;
    commit_initial(&first, &mut engine, &runtime, &mut previous);
    let previous_before = previous.clone();
    let current = keyed_root("same");
    let current_id = current.children.iter().next().expect("current child").id;

    let result: Result<PreparedDynamicFrame, _> =
        RenderPipeline::prepare_dynamic_frame_with_renderer(
            &current,
            20,
            4,
            &engine,
            previous.as_ref(),
            |element, _, _, _, _| {
                Err(CheckedRenderError::Text(
                    TextRenderError::MissingCurrentFlow {
                        element_id: element.id,
                    },
                ))
            },
        );
    assert!(result.is_err());
    assert!(engine.get_layout(old_id).is_some());
    assert!(engine.get_layout(current_id).is_none());
    assert_eq!(previous, previous_before);
    assert!(runtime.borrow().get_measurement(old_id).is_some());
    assert!(runtime.borrow().get_measurement(current_id).is_none());
}

pub(crate) fn oversized_measurement_fails_before_atomic_publication() {
    let runtime = Rc::new(RefCell::new(RuntimeContext::new()));
    let mut engine = LayoutEngine::new();
    let mut previous = None;
    let stable = keyed_root("stable");
    commit_initial(&stable, &mut engine, &runtime, &mut previous);
    let previous_before = previous.clone();
    let measurement_before = runtime.borrow().get_measurement_by_key_dims("child");
    let (snapshot_before, report_before) = engine.try_snapshot(&stable).unwrap();

    let mut oversized_child = Element::text("oversized").with_key("child");
    let oversized_child_id = oversized_child.id;
    oversized_child.style.position = Position::Absolute;
    oversized_child.style.width = Dimension::Points(70_000.0);
    let mut oversized = Element::box_element().with_key("root");
    oversized.add_child(oversized_child);
    let error = match RenderPipeline::prepare_dynamic_frame(
        &oversized,
        u16::MAX,
        4,
        &engine,
        previous.as_ref(),
    ) {
        Err(error) => error,
        Ok(_) => {
            panic!("runtime measurement must reject an oversized snapshot child before commit")
        }
    };
    assert!(matches!(
        error,
        TransactionalFrameError::Render(CheckedRenderError::Snapshot(
            SnapshotRenderError::Output {
                source: CellOutputError::ExtentOutOfRange {
                    axis: Axis::X,
                    start: 0,
                    end: 70_000,
                },
                ..
            }
        ))
    ));
    assert_eq!(previous, previous_before);
    assert_eq!(
        runtime.borrow().get_measurement_by_key_dims("child"),
        measurement_before
    );
    let (snapshot_after, report_after) = engine.try_snapshot(&stable).unwrap();
    assert_eq!(snapshot_before.snapshot(), snapshot_after.snapshot());
    assert_eq!(report_before, report_after);
    assert!(engine.get_layout(oversized_child_id).is_none());
}

pub(crate) fn all_correctness_consumers_use_authoritative_snapshot() {
    let runtime = Rc::new(RefCell::new(RuntimeContext::new()));
    let mut engine = LayoutEngine::new();
    let mut previous = None;
    let target = Element::text("consumer parity").with_key("child");
    let prepared = RenderPipeline::prepare_dynamic_frame(&target, 20, 4, &engine, None)
        .expect("dynamic consumer prepares from the authoritative snapshot");
    assert!(prepared.rendered().contains("consumer parity"));
    prepared.commit(&mut engine, &runtime, &mut previous);
    let (published, _) = engine.try_snapshot(&target).unwrap();
    let expected_bounds = published.snapshot().root().border_bounds();
    assert_eq!(
        runtime.borrow().get_measurement_by_key_dims("child"),
        Some((
            expected_bounds.width() as f32,
            expected_bounds.height() as f32
        ))
    );

    let mut checked_output = Output::new(20, 4);
    try_render_element_tree_checked(&target, &engine, &mut checked_output, 0.0, 0.0)
        .expect("public checked helper consumes the published exact snapshot");
    assert!(checked_output.render().contains("consumer parity"));
    assert!(
        try_render_to_string_checked(&target, 20)
            .unwrap()
            .contains("consumer parity")
    );
    assert!(
        TestRenderer::new(20, 4)
            .try_render_to_plain_checked(&target)
            .unwrap()
            .contains("consumer parity")
    );

    let mut static_root = Element::box_element();
    static_root.style.is_static = true;
    static_root.add_child(Element::text("consumer parity"));
    let static_lines = StaticRenderer::new()
        .try_extract_static_content_checked(&static_root, 20)
        .expect("static consumer uses its prepared authoritative snapshot");
    assert!(static_lines.join("\n").contains("consumer parity"));
}
