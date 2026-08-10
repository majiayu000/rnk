use rnk::core::{Dimension, Element, FlexDirection};
use rnk::layout::LayoutEngine;
use rnk::renderer::try_render_to_string_checked;
use rnk::testing::TestRenderer;

#[test]
fn nested_fractional_edges_need_one_cell_snapshot() {
    let mut root = Element::box_element().with_key("root");
    root.style.width = Dimension::Points(10.0);
    root.style.flex_direction = FlexDirection::Row;
    for key in ["a", "b", "c"] {
        let mut child = Element::box_element().with_key(key);
        child.style.width = Dimension::Percent(100.0 / 3.0);
        root.add_child(child);
    }

    let prepared = LayoutEngine::new()
        .prepare_element_incremental(&root, None, 10, 2)
        .unwrap();
    let snapshot = prepared.snapshot();
    let children = snapshot.root().children();
    for pair in children.windows(2) {
        let left = snapshot.nodes().nth(pair[0].as_usize()).unwrap();
        let right = snapshot.nodes().nth(pair[1].as_usize()).unwrap();
        assert!(left.border_bounds().right() <= right.border_bounds().left());
    }
}

#[test]
fn render_entrypoints_must_share_snapshot_contract() {
    let element = Element::text("shared 世界🙂");
    let checked = try_render_to_string_checked(&element, 20).unwrap();
    let testing = TestRenderer::new(20, 4)
        .try_render_to_plain_checked(&element)
        .unwrap();
    assert_eq!(checked, testing);
}
