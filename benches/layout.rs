//! Layout engine benchmarks

use rnk::core::{Dimension, Edges, Element, FlexDirection};
use rnk::layout::LayoutEngine;

fn main() {
    divan::main();
}

/// Create a simple element tree with given depth
fn create_nested_tree(depth: usize) -> Element {
    if depth == 0 {
        return Element::text("Leaf node");
    }

    let mut root = Element::root();
    let mut current = &mut root;

    for i in 0..depth {
        let mut child = Element::box_element();
        child.style.padding = Edges::new(1.0, 1.0, 0.0, 2.0);
        child.add_child(Element::text(format!("Level {}", i)));

        current.add_child(child);
        // Get the last child to continue building
        let last_idx = current.children.len() - 1;
        current = current.children.get_mut(last_idx).unwrap();
    }

    root
}

/// Create a wide element tree with given number of children
fn create_wide_tree(width: usize) -> Element {
    let mut root = Element::root();
    root.style.flex_direction = FlexDirection::Row;

    for i in 0..width {
        let mut child = Element::box_element();
        child.style.width = Dimension::Points(10.0);
        child.add_child(Element::text(format!("Item {}", i)));
        root.add_child(child);
    }

    root
}

/// Create a complex grid-like layout
fn create_grid_layout(rows: usize, cols: usize) -> Element {
    let mut root = Element::root();
    root.style.flex_direction = FlexDirection::Column;

    for r in 0..rows {
        let mut row = Element::box_element();
        row.style.flex_direction = FlexDirection::Row;

        for c in 0..cols {
            let mut cell = Element::box_element();
            cell.style.width = Dimension::Points(10.0);
            cell.style.height = Dimension::Points(3.0);
            cell.add_child(Element::text(format!("({},{})", r, c)));
            row.add_child(cell);
        }

        root.add_child(row);
    }

    root
}

#[divan::bench]
fn layout_engine_creation() {
    let _engine = LayoutEngine::new();
}

#[divan::bench]
fn layout_simple_text() {
    let mut engine = LayoutEngine::new();
    let element = Element::text("Hello, World!");
    engine.compute(&element, 80, 24);
}

#[divan::bench(args = [1, 5, 10, 20, 50])]
fn layout_nested_depth(depth: usize) {
    let mut engine = LayoutEngine::new();
    let tree = create_nested_tree(depth);
    engine.compute(&tree, 80, 24);
}

#[divan::bench(args = [10, 50, 100, 500])]
fn layout_wide_children(width: usize) {
    let mut engine = LayoutEngine::new();
    let tree = create_wide_tree(width);
    engine.compute(&tree, 200, 24);
}

#[divan::bench(args = [(5, 5), (10, 10), (20, 20)])]
fn layout_grid(size: (usize, usize)) {
    let mut engine = LayoutEngine::new();
    let tree = create_grid_layout(size.0, size.1);
    engine.compute(&tree, 200, 100);
}

#[divan::bench]
fn layout_with_padding_margin() {
    let mut engine = LayoutEngine::new();

    let mut root = Element::root();
    root.style.padding = Edges::new(2.0, 4.0, 2.0, 4.0);

    let mut child = Element::box_element();
    child.style.margin = Edges::new(1.0, 0.0, 1.0, 0.0);
    child.add_child(Element::text("Padded content"));

    root.add_child(child);
    engine.compute(&root, 80, 24);
}

#[divan::bench]
fn layout_flex_grow() {
    let mut engine = LayoutEngine::new();

    let mut root = Element::root();
    root.style.flex_direction = FlexDirection::Row;
    root.style.width = Dimension::Points(100.0);

    for i in 0..3 {
        let mut child = Element::box_element();
        child.style.flex_grow = 1.0;
        child.add_child(Element::text(format!("Flex {}", i)));
        root.add_child(child);
    }

    engine.compute(&root, 100, 24);
}

#[divan::bench]
fn layout_get_all_layouts() {
    let mut engine = LayoutEngine::new();
    let tree = create_grid_layout(10, 10);
    engine.compute(&tree, 200, 100);

    divan::black_box(engine.get_all_layouts());
}

#[divan::bench]
fn layout_cjk_text() {
    let mut engine = LayoutEngine::new();
    let element = Element::text("你好世界！这是一段中文测试文本。");
    engine.compute(&element, 80, 24);
}

#[divan::bench]
fn layout_mixed_text() {
    let mut engine = LayoutEngine::new();
    let element = Element::text("Hello 你好 World 世界 Mixed 混合文本");
    engine.compute(&element, 80, 24);
}

#[divan::bench(args = [10, 50, 100])]
fn layout_many_text_elements(count: usize) {
    let mut engine = LayoutEngine::new();

    let mut root = Element::root();
    root.style.flex_direction = FlexDirection::Column;

    for i in 0..count {
        root.add_child(Element::text(format!("Line number {}", i)));
    }

    engine.compute(&root, 80, 24);
}
