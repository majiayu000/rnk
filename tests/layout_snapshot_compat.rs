use rnk::core::Element;
use rnk::layout::{Layout, LayoutEngine, PreparedLayoutFrame};
use rnk::renderer::{Output, try_render_element_checked};
use rnk::runtime::RuntimeContext;
use rnk::testing::TestRenderer;
use std::collections::HashMap;

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

#[test]
fn legacy_runtime_measurement_setters_compile_and_fail_loudly() {
    let element = Element::text("measurement compatibility");
    let valid = Layout {
        width: 20.0,
        height: 4.0,
        ..Layout::default()
    };
    let mut runtime = RuntimeContext::new();
    runtime.set_measure_layouts(HashMap::from([(element.id, valid)]));
    assert_eq!(runtime.get_measurement(element.id), Some((20, 4)));
    runtime.set_measure_layouts_with_keys(
        HashMap::from([(element.id, valid)]),
        HashMap::from([("measurement".to_owned(), valid)]),
    );
    assert_eq!(
        runtime.get_measurement_by_key_dims("measurement"),
        Some((20.0, 4.0))
    );
    runtime.set_measure_layouts_with_node_keys(HashMap::new(), HashMap::new(), HashMap::new());

    runtime.set_measurement(element.id, 7, 3);
    let invalid = Layout {
        width: 70_000.0,
        height: 1.0,
        ..Layout::default()
    };
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.set_measure_layouts(HashMap::from([(element.id, invalid)]));
    }));
    assert!(failure.is_err(), "legacy void setter must fail loudly");
    assert_eq!(runtime.get_measurement(element.id), Some((7, 3)));
}
