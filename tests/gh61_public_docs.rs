//! External-crate compile fixture for the GH-61 snapshot surface.

use rnk::core::Element;
use rnk::layout::{
    AxisClip, CellPoint, CellRect, CellSpan, CellVector, FrameRevision, LayoutEngine,
    LayoutSnapshot, PreparedSnapshotFrame, SnapshotBuildReport, SnapshotBuildStrategy,
    SnapshotIdentity, SnapshotNode, SnapshotNodeIndex, SnapshotWorkCounters, TextFlowSemanticStamp,
};
use rnk::renderer::{RecoveredSnapshotRenderError, SnapshotRenderError};

#[test]
fn gh61_public_snapshot_surface_is_documented_and_compiles() {
    let manifest = include_str!("fixtures/gh61_public_api.json");
    for (module, source) in [
        ("snapshot", include_str!("../src/layout/snapshot.rs")),
        (
            "snapshot errors",
            include_str!("../src/layout/snapshot/error.rs"),
        ),
        (
            "snapshot quantizer",
            include_str!("../src/layout/snapshot/quantize.rs"),
        ),
        (
            "snapshot renderer",
            include_str!("../src/renderer/checked.rs"),
        ),
    ] {
        assert!(
            source.starts_with("#![forbid(missing_docs)]"),
            "{module} must enforce public docs"
        );
        assert!(!source.contains("```ignore"));
        assert!(!source.contains("```no_run"));
    }

    let element = Element::text("docs");
    let prepared = LayoutEngine::new()
        .prepare_element_incremental(&element, None, 20, 4)
        .unwrap();
    let snapshot: &LayoutSnapshot = prepared.snapshot();
    let node: &SnapshotNode = snapshot.root();
    let _: &SnapshotIdentity = node.identity();
    let _: CellRect = node.border_bounds();
    let _: CellPoint = node.border_bounds().origin();
    let _: CellVector = node.scroll_transform();
    let clip: AxisClip = node.effective_clip();
    let _: CellSpan = clip.x();
    let _: Option<SnapshotNodeIndex> = node.parent();
    let _: Option<&TextFlowSemanticStamp> = node.text_flow();
    let frame: &PreparedSnapshotFrame = prepared.prepared_snapshot();
    let _: FrameRevision = frame.frame_revision();
    let report: &SnapshotBuildReport = prepared.snapshot_report();
    let _: SnapshotBuildStrategy = report.strategy();
    let _: SnapshotWorkCounters = report.work();
    let _: Option<SnapshotRenderError> = None;
    let _: Option<RecoveredSnapshotRenderError> = None;

    for public_name in [
        "LayoutSnapshot",
        "PreparedSnapshotFrame",
        "SnapshotIdentity",
        "SnapshotRenderError",
        "RecoveredSnapshotRenderError",
    ] {
        assert!(manifest.contains(&format!("\"{public_name}\"")));
    }
}
