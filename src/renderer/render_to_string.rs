//! Element to string rendering
//!
//! This module provides utilities for rendering elements to strings
//! outside of the main application runtime.

use crate::core::{Display, Element, ElementType};
use crate::layout::{
    FullRebuildError, IncrementalLayoutError, LayoutEngine, PreparedLayoutFrame, RebuildFailure,
    TransactionalLayoutError,
};
use crate::renderer::{
    CheckedRenderError, Output, Terminal, TextRenderError, legacy_snapshot_coordinate_error,
    try_render_element_snapshot_checked,
};

const DEFAULT_TEXT_FLOW_TAB_STOP: usize = 4;

/// Options for controlling render-to-string behavior.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Whether to trim trailing whitespace from each line (default: true)
    pub trim: bool,
    /// Whether to normalize CRLF to LF (default: true).
    /// Set to false for raw terminal mode where CRLF is needed.
    pub normalize_line_endings: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            trim: true,
            normalize_line_endings: true,
        }
    }
}

/// Render an element to a string with full control over options.
pub fn render_to_string_with_options(
    element: &Element,
    width: u16,
    options: &RenderOptions,
) -> String {
    try_render_to_string_with_options(element, width, options)
        .unwrap_or_else(|error| panic!("render_to_string failed: {error}"))
}

pub fn try_render_to_string_with_options(
    element: &Element,
    width: u16,
    options: &RenderOptions,
) -> Result<String, TextRenderError> {
    try_render_to_string_with_options_and_tab_stop(
        element,
        width,
        options,
        DEFAULT_TEXT_FLOW_TAB_STOP,
    )
}

/// Render an element with an explicit tab-stop policy.
///
/// A tab stop of zero or one larger than the supported TextFlow expansion
/// returns the corresponding [`TextRenderError::Flow`] source.
pub fn try_render_to_string_with_tab_stop(
    element: &Element,
    width: u16,
    tab_stop: usize,
) -> Result<String, TextRenderError> {
    try_render_to_string_with_options_and_tab_stop(
        element,
        width,
        &RenderOptions::default(),
        tab_stop,
    )
}

fn try_render_to_string_with_options_and_tab_stop(
    element: &Element,
    width: u16,
    options: &RenderOptions,
    tab_stop: usize,
) -> Result<String, TextRenderError> {
    try_render_to_string_checked_core(element, width, options, tab_stop)
        .map_err(|error| legacy_string_error(element, error))
}

pub(super) fn try_render_to_string_checked_core(
    element: &Element,
    width: u16,
    options: &RenderOptions,
    tab_stop: usize,
) -> Result<String, CheckedRenderError> {
    let raw = RenderHelper.try_render_to_output_checked(element, width, tab_stop)?;

    if !options.normalize_line_endings {
        return Ok(raw);
    }

    let normalized = raw.replace("\r\n", "\n");

    if options.trim {
        Ok(normalized
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n"))
    } else {
        Ok(normalized)
    }
}

fn legacy_string_error(element: &Element, error: CheckedRenderError) -> TextRenderError {
    match error {
        CheckedRenderError::Text(source) => source,
        CheckedRenderError::LayoutBuild(TransactionalLayoutError::Upstream(
            IncrementalLayoutError::TextFlow(source),
        ))
        | CheckedRenderError::LayoutBuild(TransactionalLayoutError::InitialBuild(
            FullRebuildError {
                source: RebuildFailure::TextFlow(source),
                ..
            },
        )) => TextRenderError::flow(element.id, source),
        CheckedRenderError::LayoutBuild(TransactionalLayoutError::Snapshot(source)) => {
            legacy_snapshot_coordinate_error(element, &source).unwrap_or_else(|| {
                panic!("legacy string renderer cannot represent snapshot error: {source}")
            })
        }
        CheckedRenderError::LayoutBuild(TransactionalLayoutError::SnapshotBuild(source)) => {
            legacy_snapshot_coordinate_error(element, source.source_error()).unwrap_or_else(|| {
                panic!("legacy string renderer cannot represent snapshot error: {source}")
            })
        }
        CheckedRenderError::LayoutBuild(TransactionalLayoutError::RecoveredSnapshot(source)) => {
            legacy_snapshot_coordinate_error(element, source.snapshot_failure()).unwrap_or_else(
                || panic!("legacy string renderer cannot represent snapshot error: {source}"),
            )
        }
        other => panic!("legacy string renderer cannot represent checked error: {other}"),
    }
}

/// Render an element to a string with specified width.
///
/// Trims trailing whitespace and normalizes line endings to LF.
///
/// # Example
///
/// ```
/// use rnk::core::Element;
///
/// let element = Element::text("Hello!");
/// let output = rnk::render_to_string(&element, 80);
/// assert!(output.contains("Hello!"));
/// ```
pub fn render_to_string(element: &Element, width: u16) -> String {
    try_render_to_string(element, width)
        .unwrap_or_else(|error| panic!("render_to_string failed: {error}"))
}

pub fn try_render_to_string(element: &Element, width: u16) -> Result<String, TextRenderError> {
    try_render_to_string_with_options(element, width, &RenderOptions::default())
}

/// Render an element to a string without trimming trailing spaces.
pub fn render_to_string_no_trim(element: &Element, width: u16) -> String {
    try_render_to_string_no_trim(element, width)
        .unwrap_or_else(|error| panic!("render_to_string_no_trim failed: {error}"))
}

pub fn try_render_to_string_no_trim(
    element: &Element,
    width: u16,
) -> Result<String, TextRenderError> {
    try_render_to_string_with_options(
        element,
        width,
        &RenderOptions {
            trim: false,
            ..Default::default()
        },
    )
}

/// Render an element to a string with CRLF line endings for raw mode.
///
/// Use this when writing to a terminal in raw mode, where `\n` alone
/// does not perform a carriage return.
pub fn render_to_string_raw(element: &Element, width: u16) -> String {
    try_render_to_string_raw(element, width)
        .unwrap_or_else(|error| panic!("render_to_string_raw failed: {error}"))
}

pub fn try_render_to_string_raw(element: &Element, width: u16) -> Result<String, TextRenderError> {
    try_render_to_string_with_options(
        element,
        width,
        &RenderOptions {
            trim: false,
            normalize_line_endings: false,
        },
    )
}

/// Render an element to a string with automatic width detection.
///
/// # Example
///
/// ```
/// use rnk::core::Element;
///
/// // Keep the example independent of whether rustdoc owns a terminal.
/// let render: fn(&Element) -> String = rnk::render_to_string_auto;
/// let _ = render;
/// ```
pub fn render_to_string_auto(element: &Element) -> String {
    try_render_to_string_auto(element)
        .unwrap_or_else(|error| panic!("render_to_string_auto failed: {error}"))
}

pub fn try_render_to_string_auto(element: &Element) -> Result<String, TextRenderError> {
    try_render_to_string_auto_with_size_provider(element, Terminal::size)
}

fn try_render_to_string_auto_with_size_provider(
    element: &Element,
    size_provider: impl FnOnce() -> std::io::Result<(u16, u16)>,
) -> Result<String, TextRenderError> {
    let (width, _) =
        size_provider().map_err(|source| TextRenderError::io("querying terminal size", source))?;
    try_render_to_string(element, width)
}

/// Helper struct for rendering elements outside the app runtime
struct RenderHelper;

impl RenderHelper {
    fn try_render_to_output_checked(
        &self,
        element: &Element,
        width: u16,
        tab_stop: usize,
    ) -> Result<String, CheckedRenderError> {
        if element.element_type == ElementType::VirtualText
            || element.style.display == Display::None
        {
            return Ok(String::new());
        }
        let mut engine = LayoutEngine::new();
        engine.set_text_flow_policy(tab_stop, "…", 1);
        let layout_width = width;
        let prepared = self.try_resolve_render_height_checked(element, layout_width, &engine)?;
        let content_height =
            u16::try_from(prepared.snapshot().root().border_bounds().height().max(1))
                .unwrap_or(u16::MAX);
        let render_width = layout_width;

        let mut output = Output::new(render_width, content_height);
        let clip_depth_before = output.clip_depth();
        try_render_element_snapshot_checked(
            element,
            prepared.prepared_snapshot(),
            &mut output,
            0.0,
            0.0,
        )?;
        debug_assert_eq!(
            output.clip_depth(),
            clip_depth_before,
            "render_to_string left an unbalanced clip stack"
        );

        Ok(output.render())
    }

    fn try_resolve_render_height_checked(
        &self,
        element: &Element,
        width: u16,
        engine: &LayoutEngine,
    ) -> Result<PreparedLayoutFrame, CheckedRenderError> {
        let mut probe_height = 64;
        let mut measured_height = 1;

        for _ in 0..6 {
            let prepared = engine
                .prepare_element_incremental(element, None, width, probe_height)
                .map_err(CheckedRenderError::LayoutBuild)?;
            measured_height =
                u16::try_from(prepared.snapshot().root().border_bounds().height().max(1))
                    .unwrap_or(u16::MAX);

            // We have headroom; current probe height is enough.
            if measured_height.saturating_add(1) < probe_height {
                break;
            }

            if probe_height == u16::MAX {
                break;
            }

            probe_height = probe_height
                .saturating_mul(2)
                .max(probe_height.saturating_add(1));
        }

        let resolved_height = measured_height.max(1);
        engine
            .prepare_element_incremental(element, None, width, resolved_height)
            .map_err(CheckedRenderError::LayoutBuild)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::io;

    use super::*;
    use crate::components::{Box, Text};
    use crate::core::{BorderStyle, Display};

    #[test]
    fn test_render_to_string_simple() {
        let element = Text::new("Hello").into_element();
        let output = render_to_string(&element, 80);
        assert!(output.contains("Hello"));
    }

    #[test]
    fn test_render_to_string_with_border() {
        let element = Box::new()
            .border_style(BorderStyle::Single)
            .child(Text::new("Test").into_element())
            .into_element();
        let output = render_to_string(&element, 80);
        assert!(output.contains("Test"));
        assert!(output.contains("─")); // Border character
    }

    #[test]
    fn test_render_to_string_no_trim() {
        let element = Text::new("Hi").into_element();
        let trimmed = render_to_string(&element, 80);
        let not_trimmed = render_to_string_no_trim(&element, 80);
        // Both should contain the text
        assert!(trimmed.contains("Hi"));
        assert!(not_trimmed.contains("Hi"));
    }

    #[test]
    fn test_render_to_string_applies_scroll_offset() {
        let element = Box::new()
            .padding_left(4.0)
            .scroll_offset_x(2)
            .child(Text::new("X").into_element())
            .into_element();

        let output = render_to_string(&element, 20);
        let first_line = output.lines().next().unwrap_or_default();
        let x_pos = first_line.find('X').unwrap_or(usize::MAX);

        assert_eq!(x_pos, 2);
    }

    #[test]
    fn test_render_to_string_handles_tall_content() {
        let mut container = Box::new().flex_direction(crate::core::FlexDirection::Column);
        for i in 0..1100 {
            container = container.child(Text::new(format!("line-{i}")).into_element());
        }

        let element = container.into_element();
        let output = render_to_string(&element, 40);

        assert!(output.contains("line-0"));
        assert!(output.contains("line-1099"));
    }

    #[test]
    fn hidden_root_is_filtered_before_layout_probe() {
        let mut element = Box::new()
            .children([
                Text::new("hidden-a").key("duplicate").into_element(),
                Text::new("hidden-b").key("duplicate").into_element(),
            ])
            .into_element();
        element.style.display = Display::None;

        assert!(
            try_render_to_string_checked_core(
                &element,
                20,
                &RenderOptions::default(),
                DEFAULT_TEXT_FLOW_TAB_STOP,
            )
            .expect("hidden root is filtered")
            .is_empty()
        );
    }

    #[test]
    fn auto_size_failure_preserves_terminal_io_source() {
        let element = Element::text("never rendered");
        let error = try_render_to_string_auto_with_size_provider(&element, || {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "terminal size provider closed",
            ))
        })
        .unwrap_err();

        assert!(matches!(
            &error,
            TextRenderError::Io {
                operation: "querying terminal size",
                ..
            }
        ));
        assert_eq!(
            error
                .source()
                .and_then(|source| source.downcast_ref::<io::Error>())
                .map(io::Error::kind),
            Some(io::ErrorKind::BrokenPipe)
        );
    }
}
