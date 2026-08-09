//! External-crate compile fixture for the documented GH-60 checked surface.

use rnk::core::{Element, ElementType, VNode};
use rnk::layout::{
    CheckedIncrementalLayoutReport, DirectPatchApplyReport, DirectPatchError,
    DirectPatchPreflightCause, DirectPatchPreflightError, FullRebuildError, IncrementalLayoutError,
    IncrementalPatchKind, InvalidLayoutTargetError, LayoutEngine, PatchStage,
    PatchTransactionCause, PatchTransactionError, PreparedLayoutFrame, RebuildFailure,
    RebuildStage, TransactionalLayoutError,
};
use rnk::renderer::{
    CheckedRenderError, DynamicFrameError, LayoutRenderError, Output, TransactionalFrameError,
};
use rnk::testing::TestRenderer;

fn checked_report_name(report: &CheckedIncrementalLayoutReport) -> &'static str {
    match report {
        CheckedIncrementalLayoutReport::InitialFullBuild => "initial",
        CheckedIncrementalLayoutReport::NoChange => "no-change",
        CheckedIncrementalLayoutReport::Incremental { .. } => "incremental",
        CheckedIncrementalLayoutReport::RecomputedViewport => "viewport",
        CheckedIncrementalLayoutReport::RecoveredFullRebuild { .. } => "recovered",
        _ => "future",
    }
}

fn direct_report_name(report: DirectPatchApplyReport) -> &'static str {
    match report {
        DirectPatchApplyReport::NoChange => "no-change",
        DirectPatchApplyReport::Applied { .. } => "applied",
        _ => "future",
    }
}

fn transaction_name(error: &TransactionalLayoutError) -> &'static str {
    match error {
        TransactionalLayoutError::Upstream(_) => "upstream",
        TransactionalLayoutError::DirectPatch(_) => "direct",
        TransactionalLayoutError::InitialBuild(_) => "initial",
        TransactionalLayoutError::InvalidTarget(_) => "invalid-target",
        TransactionalLayoutError::RecoveryFailed { .. } => "recovery",
        _ => "future",
    }
}

fn frame_name(error: &TransactionalFrameError) -> &'static str {
    match error {
        TransactionalFrameError::Upstream(_) => "upstream",
        TransactionalFrameError::Transaction(_) => "transaction",
        TransactionalFrameError::Render(_) => "render",
        _ => "future",
    }
}

fn gh59_incremental_name(error: IncrementalLayoutError) -> &'static str {
    match error {
        IncrementalLayoutError::Identity(_) => "identity",
        IncrementalLayoutError::TextFlow(_) => "text",
    }
}

fn gh59_dynamic_name(error: DynamicFrameError) -> &'static str {
    match error {
        DynamicFrameError::Incremental(_) => "incremental",
        DynamicFrameError::Text(_) => "text",
        DynamicFrameError::LegacyLookup(_) => "lookup",
    }
}

#[test]
fn gh60_public_checked_surface_is_documented_and_compiles() {
    for (module, source) in [
        (
            "layout transaction",
            include_str!("../src/layout/engine/transaction.rs"),
        ),
        (
            "layout transaction errors",
            include_str!("../src/layout/engine/patch_error.rs"),
        ),
        (
            "layout preflight errors",
            include_str!("../src/layout/engine/patch_error/preflight.rs"),
        ),
        (
            "layout invariant errors",
            include_str!("../src/layout/engine/invariant_error.rs"),
        ),
        (
            "raw patch transaction",
            include_str!("../src/layout/engine/patching.rs"),
        ),
        (
            "checked renderer",
            include_str!("../src/renderer/checked.rs"),
        ),
        ("renderer errors", include_str!("../src/renderer/error.rs")),
        (
            "checked test renderer",
            include_str!("../src/testing/checked_renderer.rs"),
        ),
    ] {
        assert_eq!(
            source.lines().next(),
            Some("#![forbid(missing_docs)]"),
            "{module} must enforce public documentation"
        );
        for rejected_fence in ["```ignore", "```no_run", "```compile_fail"] {
            assert!(
                !source.contains(rejected_fence),
                "{module} contains a non-runnable rustdoc fence: {rejected_fence}"
            );
        }
    }

    let element = Element::text("documented");
    let mut engine = LayoutEngine::new();
    let prepared: PreparedLayoutFrame = engine
        .prepare_element_incremental(&element, None, 20, 4)
        .expect("documented prepare");
    assert_eq!(checked_report_name(prepared.report()), "initial");
    let (previous, report) = prepared.commit(&mut engine);
    assert_eq!(checked_report_name(&report), "initial");

    let mut output = Output::new(20, 4);
    rnk::try_render_element_tree_checked(&element, &engine, &mut output, 0.0, 0.0)
        .expect("root re-exported checked tree renderer");
    rnk::try_render_element_checked(&element, &engine, &mut output, 0.0, 0.0)
        .expect("root re-exported checked element renderer");
    assert!(
        rnk::try_render_to_string_checked(&element, 20)
            .expect("root re-exported checked String renderer")
            .contains("documented")
    );
    assert!(
        TestRenderer::new(20, 4)
            .try_render_to_plain_checked(&element)
            .expect("documented checked TestRenderer")
            .contains("documented")
    );

    assert_eq!(
        direct_report_name(
            engine
                .try_apply_patches_transactional(&[])
                .expect("documented direct transaction"),
        ),
        "no-change"
    );

    let invalid_root = Element::new(ElementType::VirtualText);
    let invalid_target =
        match engine.prepare_element_incremental(&invalid_root, Some(&previous), 20, 4) {
            Err(error) => error,
            Ok(_) => panic!("invalid incremental target unexpectedly prepared"),
        };
    assert_eq!(transaction_name(&invalid_target), "invalid-target");
    assert!(invalid_target.incremental_failure().is_none());
    assert!(invalid_target.rebuild_failure().is_none());

    let fresh_engine = LayoutEngine::new();
    let initial = match fresh_engine.prepare_element_incremental(&invalid_root, None, 20, 4) {
        Err(error) => error,
        Ok(_) => panic!("invalid root unexpectedly prepared"),
    };
    assert_eq!(transaction_name(&initial), "initial");
    assert!(initial.incremental_failure().is_none());
    assert!(matches!(
        initial.rebuild_failure(),
        Some(FullRebuildError {
            stage: RebuildStage::BuildTarget,
            source: RebuildFailure::InvalidTargetRoot,
            ..
        })
    ));

    let frame = TransactionalFrameError::Transaction(initial);
    assert_eq!(frame_name(&frame), "transaction");
    assert_eq!(
        gh59_incremental_name(IncrementalLayoutError::Identity(
            rnk::reconciler::ReconcilePlanError::PreviousTreeMismatch
        )),
        "identity"
    );
    assert_eq!(
        gh59_dynamic_name(DynamicFrameError::Incremental(
            IncrementalLayoutError::Identity(
                rnk::reconciler::ReconcilePlanError::PreviousTreeMismatch
            )
        )),
        "incremental"
    );

    let _: Option<CheckedRenderError> = None;
    let _: Option<LayoutRenderError> = None;
    let _: Option<DirectPatchError> = None;
    let _: Option<DirectPatchPreflightCause> = None;
    let _: Option<DirectPatchPreflightError> = None;
    let _: Option<IncrementalPatchKind> = None;
    let _: Option<InvalidLayoutTargetError> = None;
    let _: Option<PatchStage> = None;
    let _: Option<PatchTransactionCause> = None;
    let _: Option<PatchTransactionError> = None;
    let _: Option<VNode> = None;
}
