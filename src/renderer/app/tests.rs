use std::error::Error;
use std::io::{self, Write};

use super::*;
use crate::core::{Element, ElementId, ElementType};
use crate::layout::{RebuildFailure, RebuildStage, TextFlowError, TransactionalLayoutError};
use crate::renderer::TransactionalFrameError;
use crate::renderer::registry::{is_alt_screen, lock_test_registry, render_handle};

#[derive(Default)]
struct RecordingWriter {
    bytes: Vec<u8>,
    fail_write: bool,
    writes: usize,
    flushes: usize,
}

impl Write for RecordingWriter {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if self.fail_write {
            return Err(io::Error::other("injected terminal write failure"));
        }
        self.writes += 1;
        self.bytes.extend_from_slice(input);
        Ok(input.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        Ok(())
    }
}

fn dynamic_tree(text: &str) -> (Element, ElementId) {
    let mut root = Element::root();
    let child = Element::text(text).with_key("dynamic");
    let child_id = child.id;
    root.add_child(child);
    (root, child_id)
}

fn mixed_tree(dynamic_text: &str) -> (Element, ElementId) {
    let mut root = Element::root();
    let mut static_container = Element::box_element();
    static_container.style.is_static = true;
    static_container.add_child(Element::text("static-line"));
    root.add_child(static_container);
    let dynamic = Element::text(dynamic_text).with_key("dynamic");
    let dynamic_id = dynamic.id;
    root.add_child(dynamic);
    (root, dynamic_id)
}

fn expect_frame_error(
    result: Result<PreparedAppFrame, TransactionalFrameError>,
) -> TransactionalFrameError {
    match result {
        Err(error) => error,
        Ok(_) => panic!("frame preparation unexpectedly succeeded"),
    }
}

fn assert_initial_state_is_unpublished<F>(
    app: &App<F>,
    target_id: ElementId,
    terminal_before: &(Vec<String>, String, bool, usize),
) where
    F: Fn() -> Element,
{
    assert!(!app.layout_engine.has_tree());
    assert!(app.previous_vnode.is_none());
    assert!(app.layout_engine.get_layout(target_id).is_none());
    assert!(
        app.runtime_context
            .borrow()
            .get_measurement_dims(target_id)
            .is_none()
    );
    assert!(app.static_renderer.committed_lines().is_empty());
    assert_eq!(&app.terminal.committed_frame_state(), terminal_before);
}

#[test]
fn test_registry_cleanup_on_drop() {
    let _registry_guard = lock_test_registry();
    let runtime = AppRuntime::new(false);

    {
        let _guard = register_app(runtime);
        assert!(render_handle().is_some());
        assert_eq!(is_alt_screen(), Some(false));
    }

    assert!(render_handle().is_none());
    assert_eq!(is_alt_screen(), None);
}

#[test]
fn test_exit_sets_should_exit_flag() {
    let app = App::new(|| Element::text("ok"));
    assert!(!app.should_exit.load(Ordering::SeqCst));
    app.exit();
    assert!(app.should_exit.load(Ordering::SeqCst));
}

#[test]
fn test_exit_updates_runtime_context_exit_state() {
    let app = App::new(|| Element::text("ok"));
    assert!(!app.runtime_context.borrow().should_exit());
    app.exit();
    assert!(app.runtime_context.borrow().should_exit());
}

#[test]
fn app_render_candidate_preserves_typed_error_source() {
    let mut app = App::new(|| Element::text("app"));
    app.layout_engine.set_text_flow_policy(0, "…", 1);
    let failure = expect_frame_error(app.try_prepare_frame(&Element::text("app"), 20, 4));
    assert!(matches!(
        failure,
        TransactionalFrameError::Transaction(TransactionalLayoutError::InitialBuild(
            crate::layout::FullRebuildError {
                source: RebuildFailure::TextFlow(TextFlowError::InvalidTabStop),
                ..
            }
        ))
    ));
    let io_error = failure.into_io();
    let frame_error = io_error
        .get_ref()
        .and_then(|source| source.downcast_ref::<TransactionalFrameError>())
        .expect("io error must retain TransactionalFrameError");
    let mut source = frame_error.source();
    let mut found_text_flow = false;
    while let Some(current) = source {
        if matches!(
            current.downcast_ref::<TextFlowError>(),
            Some(TextFlowError::InvalidTabStop)
        ) {
            found_text_flow = true;
            break;
        }
        source = current.source();
    }
    assert!(found_text_flow, "typed TextFlow cause must remain in chain");
}

#[test]
fn duplicate_key_reaches_app_io_error_without_frame_commit() {
    let app = App::new(|| Element::text("app"));
    let mut invalid = Element::root();
    invalid.add_child(Element::text("first").with_key("duplicate"));
    invalid.add_child(Element::text("second").with_key("duplicate"));

    let failure = expect_frame_error(app.try_prepare_frame(&invalid, 20, 4));
    assert!(matches!(
        &failure,
        TransactionalFrameError::Transaction(TransactionalLayoutError::Upstream(
            crate::layout::IncrementalLayoutError::Identity(
                crate::reconciler::ReconcilePlanError::DuplicateSiblingKey { .. }
            )
        ))
    ));
    assert!(app.previous_vnode.is_none());
    assert!(!app.layout_engine.has_tree());
    assert!(
        app.runtime_context
            .borrow()
            .get_measurement_by_key_dims("duplicate")
            .is_none()
    );

    let io_error = failure.into_io();
    let frame_error = io_error
        .get_ref()
        .and_then(|source| source.downcast_ref::<TransactionalFrameError>())
        .expect("io error must retain TransactionalFrameError");
    let transactional_error = frame_error
        .source()
        .and_then(|source| source.downcast_ref::<TransactionalLayoutError>())
        .expect("frame error must retain TransactionalLayoutError");
    let incremental_error = transactional_error
        .source()
        .and_then(|source| source.downcast_ref::<crate::layout::IncrementalLayoutError>())
        .expect("transaction must retain IncrementalLayoutError");
    assert!(matches!(
        incremental_error
            .source()
            .and_then(|source| { source.downcast_ref::<crate::reconciler::ReconcilePlanError>() }),
        Some(crate::reconciler::ReconcilePlanError::DuplicateSiblingKey { .. })
    ));
}

#[test]
fn terminal_error_drops_prepared_layout_frame() {
    let mut app = App::new(|| Element::text("app"));
    let (before, before_id) = dynamic_tree("before");
    let initial = app
        .try_prepare_frame_with_mouse(&before, 20, 4, false)
        .expect("initial frame prepares");
    app.commit_prepared_frame_with_writer(initial, &mut RecordingWriter::default())
        .expect("initial frame commits");
    let terminal_before = app.terminal.committed_frame_state();
    let previous_before = format!("{:?}", app.previous_vnode);

    let (after, after_id) = dynamic_tree("after");
    let prepared = app
        .try_prepare_frame_with_mouse(&after, 20, 4, true)
        .expect("changed candidate prepares");
    let mut writer = RecordingWriter {
        fail_write: true,
        ..RecordingWriter::default()
    };
    assert!(
        app.commit_prepared_frame_with_writer(prepared, &mut writer)
            .is_err()
    );

    assert!(writer.bytes.is_empty());
    assert!(app.layout_engine.get_layout(before_id).is_some());
    assert!(app.layout_engine.get_layout(after_id).is_none());
    assert!(
        app.runtime_context
            .borrow()
            .get_measurement_dims(before_id)
            .is_some()
    );
    assert!(
        app.runtime_context
            .borrow()
            .get_measurement_dims(after_id)
            .is_none()
    );
    assert_eq!(format!("{:?}", app.previous_vnode), previous_before);
    assert_eq!(app.terminal.committed_frame_state(), terminal_before);
}

#[test]
fn snapshot_commits_only_with_prepared_app_frame() {
    let mut app = App::new(|| Element::text("app"));
    let (target, target_id) = dynamic_tree("snapshot");
    let prepared = app
        .try_prepare_frame_with_mouse(&target, 20, 4, false)
        .expect("whole frame prepares");
    let candidate_snapshot = prepared.dynamic.layout().snapshot().clone();

    assert!(app.layout_engine.get_layout(target_id).is_none());
    assert!(candidate_snapshot.root().border_bounds().width() > 0);

    app.commit_prepared_frame_with_writer(prepared, &mut RecordingWriter::default())
        .expect("whole frame commits");
    assert!(app.layout_engine.get_layout(target_id).is_some());
    assert!(
        app.runtime_context
            .borrow()
            .get_measurement_dims(target_id)
            .is_some()
    );
}

#[test]
fn stale_prepared_app_frame_fails_before_terminal_io() {
    let mut app = App::new(|| Element::text("app"));
    let (before, _) = dynamic_tree("before");
    let initial = app
        .try_prepare_frame_with_mouse(&before, 20, 4, false)
        .expect("initial frame prepares");
    app.commit_prepared_frame_with_writer(initial, &mut RecordingWriter::default())
        .expect("initial frame commits");

    let (stale_target, stale_id) = dynamic_tree("before");
    let stale = app
        .try_prepare_frame_with_mouse(&stale_target, 20, 4, true)
        .expect("alias refresh prepares");
    let (newer, newer_id) = dynamic_tree("newer");
    app.layout_engine
        .try_compute_element_incremental_transactional(&newer, app.previous_vnode.as_ref(), 20, 4)
        .expect("newer layout commits first");
    let terminal_before = app.terminal.committed_frame_state();
    let mut writer = RecordingWriter::default();

    assert!(
        app.commit_prepared_frame_with_writer(stale, &mut writer)
            .is_err()
    );
    assert!(writer.bytes.is_empty());
    assert_eq!(writer.writes, 0);
    assert_eq!(writer.flushes, 0);
    assert_eq!(app.terminal.committed_frame_state(), terminal_before);
    assert!(app.layout_engine.get_layout(newer_id).is_some());
    assert!(app.layout_engine.get_layout(stale_id).is_none());
}

#[test]
fn initial_prepared_app_frame_success_commits_once() {
    let mut app = App::new(|| Element::text("app"));
    let (root, child_id) = dynamic_tree("initial");
    let terminal_before = app.terminal.committed_frame_state();
    let prepared = app
        .try_prepare_frame_with_mouse(&root, 20, 4, false)
        .expect("initial frame prepares");
    assert_initial_state_is_unpublished(&app, child_id, &terminal_before);

    let mut writer = RecordingWriter::default();
    app.commit_prepared_frame_with_writer(prepared, &mut writer)
        .expect("initial frame commits");
    assert_eq!(writer.flushes, 1);
    assert!(app.layout_engine.has_tree());
    assert!(app.previous_vnode.is_some());
    assert!(app.layout_engine.get_layout(child_id).is_some());
    assert!(
        app.runtime_context
            .borrow()
            .get_measurement_dims(child_id)
            .is_some()
    );
    assert!(app.terminal.committed_frame_state().1.contains("initial"));
}

#[test]
fn initial_build_compute_and_postcondition_failures_write_and_publish_nothing() {
    let build_root = Element::new(ElementType::VirtualText);
    let build_id = build_root.id;
    let build_app = App::new(|| Element::text("app"));
    let build_terminal = build_app.terminal.committed_frame_state();
    let build_failure =
        expect_frame_error(build_app.try_prepare_frame_with_mouse(&build_root, 20, 4, true));
    assert!(matches!(
        build_failure,
        TransactionalFrameError::Transaction(TransactionalLayoutError::InitialBuild(
            crate::layout::FullRebuildError {
                stage: RebuildStage::BuildTarget,
                source: RebuildFailure::InvalidTargetRoot,
                ..
            }
        ))
    ));
    assert_initial_state_is_unpublished(&build_app, build_id, &build_terminal);

    let compute_app = App::new(|| Element::text("app"));
    let (compute_root, compute_id) = dynamic_tree("compute");
    let compute_terminal = compute_app.terminal.committed_frame_state();
    LayoutEngine::inject_test_compute_fault();
    let compute_failure =
        expect_frame_error(compute_app.try_prepare_frame_with_mouse(&compute_root, 20, 4, true));
    assert!(matches!(
        compute_failure,
        TransactionalFrameError::Transaction(TransactionalLayoutError::InitialBuild(
            crate::layout::FullRebuildError {
                stage: RebuildStage::ComputeLayout,
                ..
            }
        ))
    ));
    assert_initial_state_is_unpublished(&compute_app, compute_id, &compute_terminal);

    let postcondition_app = App::new(|| Element::text("app"));
    let (postcondition_root, postcondition_id) = dynamic_tree("postcondition");
    let postcondition_terminal = postcondition_app.terminal.committed_frame_state();
    LayoutEngine::inject_test_postcondition_fault();
    let postcondition_failure = expect_frame_error(postcondition_app.try_prepare_frame_with_mouse(
        &postcondition_root,
        20,
        4,
        true,
    ));
    assert!(matches!(
        postcondition_failure,
        TransactionalFrameError::Transaction(TransactionalLayoutError::InitialBuild(
            crate::layout::FullRebuildError {
                stage: RebuildStage::VerifyPostcondition,
                ..
            }
        ))
    ));
    assert_initial_state_is_unpublished(
        &postcondition_app,
        postcondition_id,
        &postcondition_terminal,
    );
}

#[test]
fn mixed_static_and_dynamic_failure_writes_no_terminal_or_static_state() {
    let app = App::new(|| Element::text("app"));
    let (mut root, dynamic_id) = mixed_tree("invalid");
    root.children
        .get_mut(1)
        .expect("dynamic child")
        .style
        .padding
        .left = f32::NAN;
    let terminal_before = app.terminal.committed_frame_state();
    let writer = RecordingWriter::default();

    assert!(
        app.try_prepare_frame_with_mouse(&root, 20, 4, true)
            .is_err()
    );
    assert!(writer.bytes.is_empty());
    assert_initial_state_is_unpublished(&app, dynamic_id, &terminal_before);
}

#[test]
fn mixed_static_and_dynamic_success_commits_once() {
    let mut app = App::new(|| Element::text("app"));
    let (root, dynamic_id) = mixed_tree("dynamic-line");
    let prepared = app
        .try_prepare_frame_with_mouse(&root, 30, 6, false)
        .expect("mixed frame prepares");
    assert!(app.static_renderer.committed_lines().is_empty());

    let mut writer = RecordingWriter::default();
    app.commit_prepared_frame_with_writer(prepared, &mut writer)
        .expect("mixed frame commits");
    let emitted = String::from_utf8(writer.bytes).expect("terminal ANSI is UTF-8");
    let static_offset = emitted.find("static-line").expect("static line emitted");
    let dynamic_offset = emitted.find("dynamic-line").expect("dynamic line emitted");
    assert!(static_offset < dynamic_offset);
    assert_eq!(writer.flushes, 1);
    assert_eq!(
        app.static_renderer.committed_lines(),
        &["static-line".to_owned()]
    );
    assert!(app.layout_engine.get_layout(dynamic_id).is_some());
    assert!(
        app.runtime_context
            .borrow()
            .get_measurement_dims(dynamic_id)
            .is_some()
    );
    assert!(app.previous_vnode.is_some());
}

#[test]
fn layout_or_render_prepare_failure_emits_no_mouse_or_frame_bytes() {
    let app = App::new(|| Element::text("app"));
    let terminal_before = app.terminal.committed_frame_state();
    let writer = RecordingWriter::default();
    let mut duplicate = Element::root();
    duplicate.add_child(Element::text("one").with_key("same"));
    duplicate.add_child(Element::text("two").with_key("same"));
    assert!(
        app.try_prepare_frame_with_mouse(&duplicate, 20, 4, true)
            .is_err()
    );

    let (mut render_failure, _) = dynamic_tree("invalid-render");
    render_failure
        .children
        .get_mut(0)
        .expect("dynamic child")
        .style
        .padding
        .left = f32::NAN;
    assert!(
        app.try_prepare_frame_with_mouse(&render_failure, 20, 4, true)
            .is_err()
    );
    assert!(writer.bytes.is_empty());
    assert_eq!(app.terminal.committed_frame_state(), terminal_before);
    assert!(!app.terminal.is_mouse_enabled());
}

#[test]
fn mouse_mode_change_is_emitted_only_during_prepared_frame_terminal_commit() {
    let mut app = App::new(|| Element::text("app"));
    let (enabled_root, _) = dynamic_tree("mouse");
    let prepared = app
        .try_prepare_frame_with_mouse(&enabled_root, 20, 4, true)
        .expect("mouse-enabled frame prepares");
    let mut enabled_writer = RecordingWriter::default();
    assert!(!app.terminal.is_mouse_enabled());
    assert!(enabled_writer.bytes.is_empty());

    app.commit_prepared_frame_with_writer(prepared, &mut enabled_writer)
        .expect("mouse-enabled frame commits");
    assert!(app.terminal.is_mouse_enabled());
    assert!(String::from_utf8_lossy(&enabled_writer.bytes).contains("?1000h"));

    let (disabled_root, _) = dynamic_tree("mouse");
    let prepared = app
        .try_prepare_frame_with_mouse(&disabled_root, 20, 4, false)
        .expect("mouse-disabled frame prepares");
    let mut disabled_writer = RecordingWriter::default();
    assert!(app.terminal.is_mouse_enabled());
    assert!(disabled_writer.bytes.is_empty());
    app.commit_prepared_frame_with_writer(prepared, &mut disabled_writer)
        .expect("mouse-disabled frame commits");
    assert!(!app.terminal.is_mouse_enabled());
    assert!(String::from_utf8_lossy(&disabled_writer.bytes).contains("?1000l"));
}
