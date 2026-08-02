//! GH-60 generalized renderer error and compatibility acceptance ledger.

use std::error::Error;
use std::panic::{AssertUnwindSafe, catch_unwind};

use rnk::core::{Display, Element, ElementType, NodeKey, Props, Style, VNode, VNodeType};
use rnk::layout::{
    IncrementalLayoutError, IncrementalLayoutOutcome, Layout, LayoutEngine, LayoutLookupError,
    RebuildFailure, TextFlowError, TransactionalLayoutError,
};
use rnk::reconciler::{Patch, ReconcilePlanError};
use rnk::renderer::{
    CheckedRenderError, DynamicFrameError, LayoutRenderError, Output, TextRenderError,
    try_render_element_checked, try_render_element_tree_checked, try_render_to_string_checked,
};
use rnk::testing::TestRenderer;

fn duplicate_key_tree() -> Element {
    let mut root = Element::root();
    root.add_child(Element::text("first").with_key("duplicate"));
    root.add_child(Element::text("second").with_key("duplicate"));
    root
}

fn duplicate_element_id_tree() -> Element {
    let mut root = Element::root();
    let first = Element::text("first").with_key("first");
    let duplicate_id = first.id;
    let mut second = Element::text("second").with_key("second");
    second.id = duplicate_id;
    root.add_child(first);
    root.add_child(second);
    root
}

fn checked_initial_failure(error: CheckedRenderError) -> rnk::layout::FullRebuildError {
    match error {
        CheckedRenderError::LayoutBuild(TransactionalLayoutError::InitialBuild(source)) => source,
        other => panic!("expected checked initial-build cause, got {other}"),
    }
}

fn dual_transaction_failure() -> TransactionalLayoutError {
    let mut engine = LayoutEngine::new();
    let mut before = Element::root();
    before.add_child(Element::text("before").with_key("first"));
    let (previous, _) = engine
        .try_compute_element_incremental_transactional(&before, None, 20, 4)
        .expect("initial frame");
    engine
        .try_compute_element_incremental_transactional(
            &duplicate_element_id_tree(),
            Some(&previous),
            20,
            4,
        )
        .expect_err("candidate and rebuild must reject duplicate ElementId aliases")
}

#[test]
fn text_identity_transaction_and_rebuild_causes_stay_distinct() {
    let text = rnk::try_render_to_string_with_tab_stop(&Element::text("text"), 20, 0)
        .expect_err("invalid tab stop is a TextRenderError");
    assert!(matches!(
        text,
        TextRenderError::Flow {
            source: TextFlowError::InvalidTabStop,
            ..
        }
    ));

    let identity = try_render_to_string_checked(&duplicate_key_tree(), 20)
        .expect_err("duplicate keys are identity failures");
    assert!(matches!(
        identity,
        CheckedRenderError::LayoutBuild(TransactionalLayoutError::Upstream(
            IncrementalLayoutError::Identity(ReconcilePlanError::DuplicateSiblingKey { .. })
        ))
    ));

    let transaction = dual_transaction_failure();
    let (incremental, rebuild) = match transaction {
        TransactionalLayoutError::RecoveryFailed {
            incremental,
            rebuild,
        } => (incremental, rebuild),
        other => panic!("expected transaction plus rebuild causes, got {other}"),
    };
    assert!(matches!(
        incremental.source.as_ref(),
        rnk::layout::PatchTransactionCause::Invariant(_)
    ));
    assert!(matches!(rebuild.source, RebuildFailure::Invariant(_)));
}

fn exhaust_incremental(error: IncrementalLayoutError) -> &'static str {
    match error {
        IncrementalLayoutError::Identity(_) => "identity",
        IncrementalLayoutError::TextFlow(_) => "text",
    }
}

fn exhaust_dynamic(error: DynamicFrameError) -> &'static str {
    match error {
        DynamicFrameError::Incremental(_) => "incremental",
        DynamicFrameError::Text(_) => "text",
        DynamicFrameError::LegacyLookup(_) => "lookup",
    }
}

#[test]
fn gh59_exhaustive_error_matches_still_compile_with_gh60_wrappers() {
    assert_eq!(
        exhaust_incremental(IncrementalLayoutError::Identity(
            ReconcilePlanError::PreviousTreeMismatch
        )),
        "identity"
    );
    assert_eq!(
        exhaust_dynamic(DynamicFrameError::LegacyLookup(
            LayoutLookupError::AmbiguousLegacyNodeKey {
                key: NodeKey::root(),
                scoped_match_count: 2,
            }
        )),
        "lookup"
    );
}

#[test]
fn missing_layout_reaches_all_checked_render_entrypoints() {
    let element = Element::text("missing");
    let engine = LayoutEngine::new();
    let mut tree_output = Output::new(20, 4);
    let tree_failure =
        try_render_element_tree_checked(&element, &engine, &mut tree_output, 0.0, 0.0)
            .expect_err("tree renderer must require root layout");
    assert!(matches!(
        tree_failure,
        CheckedRenderError::Layout(LayoutRenderError::MissingRootLayout { element_id })
            if element_id == element.id
    ));

    let mut element_output = Output::new(20, 4);
    let element_failure =
        try_render_element_checked(&element, &engine, &mut element_output, 0.0, 0.0)
            .expect_err("element renderer must require root layout");
    assert!(matches!(
        element_failure,
        CheckedRenderError::Layout(LayoutRenderError::MissingRootLayout { element_id })
            if element_id == element.id
    ));

    assert!(matches!(
        try_render_to_string_checked(&duplicate_key_tree(), 20),
        Err(CheckedRenderError::LayoutBuild(_))
    ));
    assert!(matches!(
        TestRenderer::new(20, 4).try_render_to_ansi_checked(&duplicate_key_tree()),
        Err(CheckedRenderError::LayoutBuild(_))
    ));
}

#[test]
fn virtual_text_is_filtered_before_required_layout_lookup() {
    let virtual_text = Element::new(ElementType::VirtualText);
    let hidden = {
        let mut element = Element::text("hidden");
        element.style.display = Display::None;
        element
    };
    let engine = LayoutEngine::new();
    let mut output = Output::new(20, 4);
    assert!(try_render_element_tree_checked(&virtual_text, &engine, &mut output, 0.0, 0.0).is_ok());
    assert!(try_render_element_checked(&hidden, &engine, &mut output, 0.0, 0.0).is_ok());
    assert_eq!(
        try_render_to_string_checked(&virtual_text, 20).expect("VirtualText is filtered"),
        ""
    );
    assert!(
        TestRenderer::new(20, 4)
            .try_render_to_plain_checked(&virtual_text)
            .is_ok()
    );
}

#[test]
fn cloned_nan_text_style_remains_renderable() {
    let mut element = Element::text("nan projection");
    element.style.flex_grow = f32::NAN;

    assert_eq!(
        TestRenderer::new(20, 4)
            .try_render_to_plain_checked(&element)
            .expect("cloned token and run styles remain semantically equal"),
        "nan projection"
    );
}

#[test]
fn hidden_test_root_is_filtered_before_layout_preparation() {
    let mut hidden = duplicate_key_tree();
    hidden.style.display = Display::None;
    let renderer = TestRenderer::new(20, 4);

    assert_eq!(
        renderer
            .try_render_to_ansi_checked(&hidden)
            .expect("hidden checked ANSI root is filtered"),
        ""
    );
    assert_eq!(
        renderer
            .try_render_to_plain_checked(&hidden)
            .expect("hidden checked plain root is filtered"),
        ""
    );
    assert_eq!(
        renderer
            .try_render_to_ansi(&hidden)
            .expect("hidden legacy ANSI root is filtered"),
        ""
    );
    assert_eq!(renderer.render_to_plain(&hidden), "");
}

#[test]
fn static_and_string_layout_failure_returns_no_partial_output() {
    let invalid = duplicate_element_id_tree();
    let string_failure = try_render_to_string_checked(&invalid, 20)
        .expect_err("invalid layout cannot return a partial String");
    assert!(matches!(
        string_failure,
        CheckedRenderError::LayoutBuild(TransactionalLayoutError::InitialBuild(_))
    ));
    let testing_failure = TestRenderer::new(20, 4)
        .try_render_to_plain_checked(&invalid)
        .expect_err("testing renderer cannot return partial text");
    assert!(matches!(
        testing_failure,
        CheckedRenderError::LayoutBuild(TransactionalLayoutError::InitialBuild(_))
    ));
}

#[test]
fn checked_renderers_preserve_initial_layout_build_cause() {
    let string_rebuild = checked_initial_failure(
        try_render_to_string_checked(&duplicate_element_id_tree(), 20)
            .expect_err("string checked build fails"),
    );
    let testing_rebuild = checked_initial_failure(
        TestRenderer::new(20, 4)
            .try_render_to_ansi_checked(&duplicate_element_id_tree())
            .expect_err("testing checked build fails"),
    );
    assert!(matches!(
        string_rebuild.source,
        RebuildFailure::Invariant(_)
    ));
    assert!(matches!(
        testing_rebuild.source,
        RebuildFailure::Invariant(_)
    ));
    assert!(string_rebuild.source.source().is_some());
    assert!(testing_rebuild.source.source().is_some());
}

#[test]
fn every_required_layout_failure_is_observed_without_fallback() {
    let mut committed_element = Element::root();
    committed_element.add_child(Element::text("committed"));
    let mut engine = LayoutEngine::new();
    engine
        .prepare_element_incremental(&committed_element, None, 20, 4)
        .expect("committed layout")
        .commit(&mut engine);

    let mut changed = Element::root();
    let missing = Element::text("missing child");
    let missing_id = missing.id;
    changed.add_child(missing);
    let mut output = Output::new(20, 4);
    let before = output.render();
    let failure = try_render_element_tree_checked(&changed, &engine, &mut output, 0.0, 0.0)
        .expect_err("fresh child must not use a stale/default layout");
    assert!(matches!(
        failure,
        CheckedRenderError::Layout(LayoutRenderError::MissingElementLayout { element_id })
            if element_id == missing_id
    ));
    assert_eq!(output.render(), before);

    let mut filtered = Element::root();
    let mut hidden = Element::text("hidden");
    hidden.style.display = Display::None;
    filtered.add_child(hidden);
    filtered.add_child(Element::new(ElementType::VirtualText));
    assert!(try_render_element_tree_checked(&filtered, &engine, &mut output, 0.0, 0.0).is_ok());
}

#[test]
fn legacy_wrappers_compile_and_fail_loudly_on_final_error() {
    let valid = Element::text("valid");
    assert!(rnk::render_to_string(&valid, 20).contains("valid"));
    assert!(
        TestRenderer::new(20, 4)
            .render_to_plain(&valid)
            .contains("valid")
    );

    let string_try = catch_unwind(AssertUnwindSafe(|| {
        let _ = rnk::try_render_to_string(&duplicate_key_tree(), 20);
    }));
    let string_non_try = catch_unwind(AssertUnwindSafe(|| {
        let _ = rnk::render_to_string(&duplicate_key_tree(), 20);
    }));
    let testing_try = catch_unwind(AssertUnwindSafe(|| {
        let _ = TestRenderer::new(20, 4).try_render_to_ansi(&duplicate_key_tree());
    }));
    let testing_non_try = catch_unwind(AssertUnwindSafe(|| {
        let _ = TestRenderer::new(20, 4).render_to_ansi(&duplicate_key_tree());
    }));
    assert!(string_try.is_err());
    assert!(string_non_try.is_err());
    assert!(testing_try.is_err());
    assert!(testing_non_try.is_err());

    let mut engine = LayoutEngine::new();
    let committed = Element::text("committed");
    let committed_id = committed.id;
    engine.compute(&committed, 20, 4);
    let committed_layout = engine
        .get_layout(committed_id)
        .expect("valid legacy compute publishes layout");
    let committed_node_count = engine.node_count();
    let invalid = Element::new(ElementType::VirtualText);

    let engine_try = catch_unwind(AssertUnwindSafe(|| {
        let _ = engine.try_compute(&invalid, 20, 4);
    }));
    assert!(engine_try.is_err());
    let after_try = engine
        .get_layout(committed_id)
        .expect("failed legacy try keeps committed layout");
    assert_eq!(
        (after_try.x, after_try.y, after_try.width, after_try.height),
        (
            committed_layout.x,
            committed_layout.y,
            committed_layout.width,
            committed_layout.height,
        )
    );
    assert_eq!(engine.node_count(), committed_node_count);

    let engine_non_try = catch_unwind(AssertUnwindSafe(|| {
        engine.compute(&invalid, 20, 4);
    }));
    assert!(engine_non_try.is_err());
    let after_non_try = engine
        .get_layout(committed_id)
        .expect("failed legacy compute keeps committed layout");
    assert_eq!(
        (
            after_non_try.x,
            after_non_try.y,
            after_non_try.width,
            after_non_try.height,
        ),
        (
            committed_layout.x,
            committed_layout.y,
            committed_layout.width,
            committed_layout.height,
        )
    );
    assert_eq!(engine.node_count(), committed_node_count);
}

#[test]
fn public_layout_vnode_patch_outcome_literals_compile() {
    let props = Props {
        style: Style::new(),
        key: None,
        scroll_offset_x: None,
        scroll_offset_y: None,
    };
    let vnode = VNode {
        key: NodeKey::root(),
        node_type: VNodeType::Root,
        props: props.clone(),
        children: Vec::new(),
    };
    let patch = Patch::Update {
        key: vnode.key,
        old_props: props.clone(),
        new_props: props,
    };
    let outcome = IncrementalLayoutOutcome {
        used_reconciler: true,
        patch_count: 1,
        fallback_full_rebuild: false,
        patch_error: None,
    };
    let layout = Layout {
        x: 0.0,
        y: 0.0,
        width: 20.0,
        height: 4.0,
    };
    assert!(matches!(patch, Patch::Update { key, .. } if key == vnode.key));
    assert_eq!(outcome.patch_count, 1);
    assert_eq!(layout.width, 20.0);
}
