#![forbid(missing_docs)]

//! Generalized checked entrypoints for [`TestRenderer`].

use crate::core::{Display, Element, ElementType};
use crate::layout::LayoutEngine;
use crate::renderer::{CheckedRenderError, Output, try_render_element_tree_checked};

use super::renderer::{TestRenderer, strip_ansi_codes};

impl TestRenderer {
    /// Render an Element tree with generalized checked layout and render errors.
    ///
    /// The returned String is created only after the complete tree has passed
    /// checked layout preparation and every required-layout lookup.
    ///
    /// # Errors
    ///
    /// Returns [`CheckedRenderError::LayoutBuild`] for initial layout failures,
    /// [`CheckedRenderError::Layout`] for missing required layouts, or
    /// [`CheckedRenderError::Text`] for text projection failures.
    ///
    /// ```
    /// use rnk::core::Element;
    /// use rnk::testing::TestRenderer;
    ///
    /// let output = TestRenderer::new(20, 4)
    ///     .try_render_to_ansi_checked(&Element::text("checked"))
    ///     .expect("checked test render");
    /// assert!(output.contains("checked"));
    /// ```
    pub fn try_render_to_ansi_checked(
        &self,
        element: &Element,
    ) -> Result<String, CheckedRenderError> {
        let committed = LayoutEngine::new();
        if element.style.display == Display::None
            || element.element_type == ElementType::VirtualText
        {
            return render_candidate(self, element, &committed);
        }
        let prepared = committed
            .prepare_element_incremental(element, None, self.width(), self.height())
            .map_err(CheckedRenderError::LayoutBuild)?;
        render_candidate(self, element, prepared.engine())
    }

    /// Render checked test output and strip ANSI styling after complete success.
    ///
    /// # Errors
    ///
    /// Returns the same generalized failure as
    /// [`try_render_to_ansi_checked`](Self::try_render_to_ansi_checked), without
    /// returning partial plain text.
    ///
    /// ```
    /// use rnk::core::Element;
    /// use rnk::testing::TestRenderer;
    ///
    /// let output = TestRenderer::new(20, 4)
    ///     .try_render_to_plain_checked(&Element::text("plain"))
    ///     .expect("checked plain render");
    /// assert!(output.contains("plain"));
    /// ```
    pub fn try_render_to_plain_checked(
        &self,
        element: &Element,
    ) -> Result<String, CheckedRenderError> {
        self.try_render_to_ansi_checked(element)
            .map(|ansi| strip_ansi_codes(&ansi))
    }
}

fn render_candidate(
    renderer: &TestRenderer,
    element: &Element,
    engine: &LayoutEngine,
) -> Result<String, CheckedRenderError> {
    let mut output = Output::new(renderer.width(), renderer.height());
    let clip_depth_before = output.clip_depth();
    try_render_element_tree_checked(element, engine, &mut output, 0.0, 0.0)?;
    debug_assert_eq!(
        output.clip_depth(),
        clip_depth_before,
        "checked test renderer left an unbalanced clip stack"
    );
    Ok(output.render())
}
