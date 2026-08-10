//! External-crate compile fixture for the GH-61 snapshot surface.

use rnk::core::Element;
use std::collections::BTreeSet;

use rnk::layout::*;
use rnk::renderer::{RecoveredSnapshotRenderError, SnapshotRenderError};

#[test]
fn gh61_public_snapshot_surface_is_documented_and_compiles() {
    let manifest = include_str!("fixtures/gh61_public_api.json");
    let manifest: serde_json::Value = serde_json::from_str(manifest).unwrap();
    let declared = |source: &str| {
        source
            .lines()
            .filter_map(|line| {
                let line = line.trim_start();
                line.strip_prefix("pub struct ")
                    .or_else(|| line.strip_prefix("pub enum "))
                    .and_then(|tail| {
                        tail.split(|character: char| {
                            !character.is_alphanumeric() && character != '_'
                        })
                        .next()
                    })
                    .map(str::to_owned)
            })
            .collect::<BTreeSet<_>>()
    };
    let modules = [
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
    ];
    for (module, source) in modules {
        assert!(
            source.starts_with("#![forbid(missing_docs)]"),
            "{module} must enforce public docs"
        );
        assert!(!source.contains("```ignore"));
        assert!(!source.contains("```no_run"));
    }

    let mut actual_layout = declared(include_str!("../src/layout/snapshot.rs"));
    actual_layout.extend(declared(include_str!("../src/layout/snapshot/error.rs")));
    actual_layout.insert("RecoveredSnapshotError".to_owned());
    let actual_renderer = declared(include_str!("../src/renderer/checked.rs"))
        .into_iter()
        .filter(|name| name == "SnapshotRenderError" || name == "RecoveredSnapshotRenderError")
        .collect::<BTreeSet<_>>();
    let listed = |section: &str| {
        manifest[section]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| name.as_str().unwrap().to_owned())
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(listed("layout"), actual_layout);
    assert_eq!(listed("renderer"), actual_renderer);

    let runnable_doctests = modules
        .iter()
        .map(|(_, source)| source.matches("/// ```\n").count())
        .sum::<usize>();
    assert!(runnable_doctests > 0);

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

    let _public_layout_types = (
        std::any::type_name::<ArithmeticOperation>(),
        std::any::type_name::<AttemptedContentBounds>(),
        std::any::type_name::<Axis>(),
        std::any::type_name::<AxisClip>(),
        std::any::type_name::<CellOutputError>(),
        std::any::type_name::<CellPoint>(),
        std::any::type_name::<CellRect>(),
        std::any::type_name::<CellSpan>(),
        std::any::type_name::<CellVector>(),
        std::any::type_name::<Edge>(),
        std::any::type_name::<FrameRevision>(),
        std::any::type_name::<GeometryField>(),
        std::any::type_name::<LayoutAliasError>(),
        std::any::type_name::<LayoutSnapshotError>(),
        std::any::type_name::<RecoveredSnapshotError>(),
        std::any::type_name::<SnapshotAttemptReport>(),
        std::any::type_name::<SnapshotCounterError>(),
        std::any::type_name::<SnapshotInvariantError>(),
        std::any::type_name::<SnapshotTargetMismatchReason>(),
        std::any::type_name::<SnapshotWorkCounterField>(),
    );
}
