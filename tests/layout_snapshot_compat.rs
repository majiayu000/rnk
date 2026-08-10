use rnk::core::Element;
use rnk::layout::{Layout, LayoutEngine, PreparedLayoutFrame};
use rnk::renderer::{Output, try_render_element_checked};
use rnk::testing::TestRenderer;

#[test]
fn existing_layout_engine_renderer_and_testing_surface_compiles() {
    let element = Element::text("compat");
    let mut engine = LayoutEngine::new();
    engine.try_compute(&element, 20, 4).unwrap();
    let _: Option<Layout> = engine.get_layout(element.id);
    let _: std::collections::HashMap<_, Layout> = engine.get_all_layouts();

    let prepared: PreparedLayoutFrame = LayoutEngine::new()
        .prepare_element_incremental(&element, None, 20, 4)
        .unwrap();
    assert!(prepared.snapshot().root().border_bounds().width() > 0);

    let mut output = Output::new(20, 4);
    try_render_element_checked(&element, &engine, &mut output, 0.0, 0.0).unwrap();
    assert_eq!(TestRenderer::new(20, 4).render_to_plain(&element), "compat");
}
