use rnk::core::Element;
use rnk::layout::LayoutEngine;

#[test]
fn public_snapshot_read_only_accessors_compile() {
    let target = Element::text("immutable").with_key("text");
    let mut engine = LayoutEngine::new();
    engine.try_compute(&target, 20, 4).unwrap();
    let (prepared, report) = engine.try_snapshot(&target).unwrap();
    let snapshot = prepared.snapshot();
    let root = snapshot.root();
    assert_eq!(snapshot.nodes().len(), 1);
    assert_eq!(snapshot.get(root.identity()), Some(root));
    assert_eq!(root.parent(), None);
    assert_eq!(root.border_bounds().left(), 0);
    assert_eq!(report.work().nodes_visited(), 1);
}

#[test]
fn public_snapshot_mutation_surface_is_compile_fail() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/gh61_cell_rect_private_fields.rs");
    cases.compile_fail("tests/ui/gh61_snapshot_identity_private_constructor.rs");
}
