#![forbid(missing_docs)]

//! Generalized checked rendering errors and element projection entrypoints.

use std::{error::Error, fmt, io};

use crate::core::{Display, Element, ElementId, ElementType};
use crate::layout::{
    IncrementalInvariantError, LayoutEngine, LayoutLookupError, TransactionalLayoutError,
};

use super::{DynamicFrameError, Output, TextRenderError, tree_renderer};

/// A required layout was absent or its compatibility projection was invalid.
///
/// ```
/// use rnk::{core::Element, layout::LayoutEngine, renderer::{CheckedRenderError, Output, try_render_element_checked}};
/// let element = Element::text("missing");
/// let error = try_render_element_checked(&element, &LayoutEngine::new(), &mut Output::new(20, 4), 0.0, 0.0).expect_err("layout is required");
/// assert!(matches!(error, CheckedRenderError::Layout(_)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutRenderError {
    /// A visible non-root Element had no layout in the prepared frame.
    MissingElementLayout {
        /// Missing Element identifier.
        element_id: ElementId,
    },
    /// The visible render root had no layout in the prepared frame.
    MissingRootLayout {
        /// Missing root Element identifier.
        element_id: ElementId,
    },
    /// A legacy or composite layout lookup was ambiguous.
    LayoutLookup(LayoutLookupError),
    /// A prepared target-exact layout snapshot became internally inconsistent.
    Invariant(IncrementalInvariantError),
}

impl fmt::Display for LayoutRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingElementLayout { element_id } => {
                write!(
                    formatter,
                    "missing required layout for element {element_id:?}"
                )
            }
            Self::MissingRootLayout { element_id } => {
                write!(
                    formatter,
                    "missing required root layout for element {element_id:?}"
                )
            }
            Self::LayoutLookup(source) => source.fmt(formatter),
            Self::Invariant(source) => source.fmt(formatter),
        }
    }
}

impl Error for LayoutRenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::LayoutLookup(source) => Some(source),
            Self::Invariant(source) => Some(source),
            Self::MissingElementLayout { .. } | Self::MissingRootLayout { .. } => None,
        }
    }
}

impl From<LayoutLookupError> for LayoutRenderError {
    fn from(source: LayoutLookupError) -> Self {
        Self::LayoutLookup(source)
    }
}

/// Failure from checked layout construction or element rendering.
///
/// ```
/// use rnk::{core::Element, renderer::{CheckedRenderError, try_render_to_string_checked}};
/// let mut root = Element::root();
/// root.add_child(Element::box_element().with_key("duplicate"));
/// root.add_child(Element::box_element().with_key("duplicate"));
/// let error = try_render_to_string_checked(&root, 20).expect_err("duplicate target key");
/// assert!(matches!(error, CheckedRenderError::LayoutBuild(_)));
/// ```
#[derive(Debug)]
pub enum CheckedRenderError {
    /// Initial or incremental layout preparation failed.
    LayoutBuild(TransactionalLayoutError),
    /// TextFlow or staged text projection failed.
    Text(TextRenderError),
    /// A required layout lookup failed.
    Layout(LayoutRenderError),
}

impl fmt::Display for CheckedRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LayoutBuild(source) => write!(formatter, "checked layout build failed: {source}"),
            Self::Text(source) => write!(formatter, "checked text render failed: {source}"),
            Self::Layout(source) => write!(formatter, "checked layout render failed: {source}"),
        }
    }
}

impl Error for CheckedRenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::LayoutBuild(source) => Some(source),
            Self::Text(source) => Some(source),
            Self::Layout(source) => Some(source),
        }
    }
}

impl From<TransactionalLayoutError> for CheckedRenderError {
    fn from(source: TransactionalLayoutError) -> Self {
        Self::LayoutBuild(source)
    }
}

impl From<TextRenderError> for CheckedRenderError {
    fn from(source: TextRenderError) -> Self {
        Self::Text(source)
    }
}

impl From<LayoutRenderError> for CheckedRenderError {
    fn from(source: LayoutRenderError) -> Self {
        Self::Layout(source)
    }
}

/// Whole-frame error that keeps GH59 dynamic errors separate from GH60 work.
///
/// ```
/// use rnk::renderer::TransactionalFrameError;
/// fn category(error: &TransactionalFrameError) -> &'static str {
///     match error { TransactionalFrameError::Transaction(_) => "layout", _ => "other" }
/// }
/// ```
#[non_exhaustive]
#[derive(Debug)]
pub enum TransactionalFrameError {
    /// A legacy GH59 dynamic-frame boundary failed.
    Upstream(DynamicFrameError),
    /// Delayed layout preparation or recovery failed.
    Transaction(TransactionalLayoutError),
    /// Required-layout or generalized rendering failed.
    Render(CheckedRenderError),
}

impl TransactionalFrameError {
    pub(crate) fn into_io(self) -> io::Error {
        io::Error::other(self)
    }
}

impl fmt::Display for TransactionalFrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Upstream(source) => write!(formatter, "dynamic frame failed: {source}"),
            Self::Transaction(source) => write!(formatter, "layout transaction failed: {source}"),
            Self::Render(source) => write!(formatter, "frame render failed: {source}"),
        }
    }
}

impl Error for TransactionalFrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Upstream(source) => Some(source),
            Self::Transaction(source) => Some(source),
            Self::Render(source) => Some(source),
        }
    }
}

impl From<DynamicFrameError> for TransactionalFrameError {
    fn from(source: DynamicFrameError) -> Self {
        Self::Upstream(source)
    }
}

impl From<TransactionalLayoutError> for TransactionalFrameError {
    fn from(source: TransactionalLayoutError) -> Self {
        Self::Transaction(source)
    }
}

impl From<CheckedRenderError> for TransactionalFrameError {
    fn from(source: CheckedRenderError) -> Self {
        Self::Render(source)
    }
}

/// Render a visible Element tree after validating every required layout.
///
/// `Display::None` and `VirtualText` subtrees are filtered before lookup.
/// The underlying renderer stages its output, so an error leaves `output`
/// unchanged.
///
/// # Errors
///
/// Returns [`CheckedRenderError::Layout`] for a missing required layout and
/// [`CheckedRenderError::Text`] for TextFlow or projection failures.
///
/// ```
/// use rnk::core::Element;
/// use rnk::layout::LayoutEngine;
/// use rnk::renderer::{Output, try_render_element_tree_checked};
///
/// let element = Element::text("checked");
/// let mut engine = LayoutEngine::new();
/// engine.try_compute(&element, 20, 4).expect("layout");
/// let mut output = Output::new(20, 4);
/// try_render_element_tree_checked(&element, &engine, &mut output, 0.0, 0.0)
///     .expect("render");
/// assert!(output.render().contains("checked"));
/// ```
pub fn try_render_element_tree_checked(
    element: &Element,
    layout_engine: &LayoutEngine,
    output: &mut Output,
    offset_x: f32,
    offset_y: f32,
) -> Result<(), CheckedRenderError> {
    validate_required_layouts(element, layout_engine, true)?;
    tree_renderer::try_render_element_tree(element, layout_engine, output, offset_x, offset_y)?;
    Ok(())
}

/// Render one Element entrypoint with generalized checked errors.
///
/// This is equivalent to [`try_render_element_tree_checked`] and is provided
/// for callers that use the runtime element-renderer naming.
///
/// # Errors
///
/// Returns a typed layout or text-render failure without partial output.
///
/// ```
/// use rnk::{core::Element, layout::LayoutEngine, renderer::{Output, try_render_element_checked}};
/// let element = Element::text("checked");
/// let mut engine = LayoutEngine::new();
/// engine.try_compute(&element, 20, 4).expect("layout");
/// let mut output = Output::new(20, 4);
/// try_render_element_checked(&element, &engine, &mut output, 0.0, 0.0).expect("render");
/// assert!(output.render().contains("checked"));
/// ```
pub fn try_render_element_checked(
    element: &Element,
    layout_engine: &LayoutEngine,
    output: &mut Output,
    offset_x: f32,
    offset_y: f32,
) -> Result<(), CheckedRenderError> {
    try_render_element_tree_checked(element, layout_engine, output, offset_x, offset_y)
}

/// Render an Element tree to a String through the generalized checked layout
/// and required-layout boundary.
///
/// Unlike the TextFlow-only compatibility helpers, this entrypoint preserves
/// initial layout construction, postcondition, required-layout, and text
/// rendering failures as [`CheckedRenderError`]. It never returns a partial
/// String.
///
/// # Errors
///
/// Returns [`CheckedRenderError::LayoutBuild`] when checked layout preparation
/// fails, [`CheckedRenderError::Layout`] when a required layout is unavailable,
/// or [`CheckedRenderError::Text`] when text projection fails.
///
/// ```
/// use rnk::core::Element;
/// use rnk::renderer::try_render_to_string_checked;
///
/// let rendered = try_render_to_string_checked(&Element::text("checked"), 20)
///     .expect("checked string render");
/// assert!(rendered.contains("checked"));
/// ```
pub fn try_render_to_string_checked(
    element: &Element,
    width: u16,
) -> Result<String, CheckedRenderError> {
    super::render_to_string::try_render_to_string_checked_core(
        element,
        width,
        &super::render_to_string::RenderOptions::default(),
        4,
    )
}

fn validate_required_layouts(
    element: &Element,
    layout_engine: &LayoutEngine,
    is_root: bool,
) -> Result<(), LayoutRenderError> {
    if element.style.display == Display::None || element.element_type == ElementType::VirtualText {
        return Ok(());
    }
    if layout_engine
        .try_get_required_layout(element.id)
        .map_err(LayoutRenderError::Invariant)?
        .is_none()
    {
        return Err(if is_root {
            LayoutRenderError::MissingRootLayout {
                element_id: element.id,
            }
        } else {
            LayoutRenderError::MissingElementLayout {
                element_id: element.id,
            }
        });
    }
    for child in &element.children {
        validate_required_layouts(child, layout_engine, false)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
