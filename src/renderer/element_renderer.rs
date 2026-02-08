//! Runtime element rendering entrypoint.
//!
//! Shared rendering logic lives in `tree_renderer`.

use crate::core::Element;
use crate::layout::LayoutEngine;
use crate::renderer::Output;
use crate::renderer::tree_renderer::render_element_tree;

/// Render an element tree to an output buffer
pub(crate) fn render_element(
    element: &Element,
    layout_engine: &LayoutEngine,
    output: &mut Output,
    offset_x: f32,
    offset_y: f32,
) {
    let clip_depth_before = output.clip_depth();
    render_element_tree(element, layout_engine, output, offset_x, offset_y);
    assert_eq!(
        output.clip_depth(),
        clip_depth_before,
        "render_element left an unbalanced clip stack"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Box, Text};
    use crate::core::BorderStyle;

    #[test]
    fn test_render_simple_text() {
        let element = Text::new("Hello").into_element();
        let mut engine = LayoutEngine::new();
        engine.compute(&element, 80, 24);

        let mut output = Output::new(80, 24);
        render_element(&element, &engine, &mut output, 0.0, 0.0);

        let rendered = output.render();
        assert!(rendered.contains("Hello"));
    }

    #[test]
    fn test_render_with_border() {
        let element = Box::new()
            .border_style(BorderStyle::Single)
            .child(Text::new("Test").into_element())
            .into_element();

        let mut engine = LayoutEngine::new();
        engine.compute(&element, 80, 24);

        let mut output = Output::new(80, 24);
        render_element(&element, &engine, &mut output, 0.0, 0.0);

        let rendered = output.render();
        assert!(rendered.contains("Test"));
        assert!(rendered.contains("─")); // Border character
    }
}
