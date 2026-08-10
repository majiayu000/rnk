#![forbid(missing_docs)]

//! Generalized checked rendering errors and element projection entrypoints.

use std::{error::Error, fmt, io};

use crate::core::{Display, Element, ElementId, ElementType};
use crate::layout::{
    CellOutputError, IncrementalInvariantError, LayoutAliasError, LayoutEngine, LayoutLookupError,
    LayoutSnapshotError, PatchTransactionError, PreparedSnapshotFrame, SnapshotIdentity,
    TransactionalLayoutError,
};

use super::{DynamicFrameError, Output, TextRenderError, tree_renderer};

/// Failure while rendering from an immutable layout snapshot.
#[derive(Debug)]
pub enum SnapshotRenderError {
    /// Snapshot construction failed.
    Snapshot {
        /// Concrete snapshot failure.
        source: LayoutSnapshotError,
    },
    /// A frame-local element alias failed.
    Alias {
        /// Concrete alias failure.
        source: LayoutAliasError,
    },
    /// A clipped cell could not be represented by terminal output.
    Output {
        /// Semantic node identity.
        identity: SnapshotIdentity,
        /// Concrete cell conversion failure.
        source: CellOutputError,
    },
    /// Text projection failed for a semantic node.
    Text {
        /// Semantic node identity.
        identity: SnapshotIdentity,
        /// Concrete text rendering failure.
        source: TextRenderError,
    },
}

impl fmt::Display for SnapshotRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snapshot { source } => write!(formatter, "snapshot failed: {source}"),
            Self::Alias { source } => write!(formatter, "snapshot alias failed: {source}"),
            Self::Output { identity, source } => write!(
                formatter,
                "snapshot output failed for {}: {source}",
                identity.diagnostic()
            ),
            Self::Text { identity, source } => write!(
                formatter,
                "snapshot text failed for {}: {source}",
                identity.diagnostic()
            ),
        }
    }
}

impl Error for SnapshotRenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Snapshot { source } => Some(source),
            Self::Alias { source } => Some(source),
            Self::Output { source, .. } => Some(source),
            Self::Text { source, .. } => Some(source),
        }
    }
}

/// A recovered layout whose final snapshot rendering failed.
#[derive(Debug)]
pub struct RecoveredSnapshotRenderError {
    incremental: Box<PatchTransactionError>,
    render: Box<SnapshotRenderError>,
}

impl RecoveredSnapshotRenderError {
    pub(crate) fn new(incremental: PatchTransactionError, render: SnapshotRenderError) -> Self {
        Self {
            incremental: Box::new(incremental),
            render: Box::new(render),
        }
    }

    /// Original incremental transaction failure.
    pub fn incremental_failure(&self) -> &PatchTransactionError {
        &self.incremental
    }

    /// Final snapshot rendering failure.
    pub fn render_failure(&self) -> &SnapshotRenderError {
        &self.render
    }
}

impl fmt::Display for RecoveredSnapshotRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "incremental layout failed ({}); recovered snapshot render failed ({})",
            self.incremental, self.render
        )
    }
}

impl Error for RecoveredSnapshotRenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.render.as_ref())
    }
}

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
    /// Immutable snapshot construction or rendering failed.
    Snapshot(SnapshotRenderError),
    /// Recovered layout rendering failed while retaining the incremental cause.
    RecoveredSnapshot(RecoveredSnapshotRenderError),
}

impl fmt::Display for CheckedRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LayoutBuild(source) => write!(formatter, "checked layout build failed: {source}"),
            Self::Text(source) => write!(formatter, "checked text render failed: {source}"),
            Self::Layout(source) => write!(formatter, "checked layout render failed: {source}"),
            Self::Snapshot(source) => write!(formatter, "checked snapshot render failed: {source}"),
            Self::RecoveredSnapshot(source) => source.fmt(formatter),
        }
    }
}

impl Error for CheckedRenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::LayoutBuild(source) => Some(source),
            Self::Text(source) => Some(source),
            Self::Layout(source) => Some(source),
            Self::Snapshot(source) => Some(source),
            Self::RecoveredSnapshot(source) => Some(source),
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
    if element.style.display == Display::None || element.element_type == ElementType::VirtualText {
        return Ok(());
    }
    let (snapshot, _) = layout_engine.try_snapshot(element).map_err(|source| {
        legacy_snapshot_coordinate_error(element, &source).map_or_else(
            || CheckedRenderError::Snapshot(SnapshotRenderError::Snapshot { source }),
            CheckedRenderError::Text,
        )
    })?;
    try_render_element_snapshot_checked(element, &snapshot, output, offset_x, offset_y)
}

pub(crate) fn try_render_element_snapshot_checked(
    element: &Element,
    snapshot: &PreparedSnapshotFrame,
    output: &mut Output,
    offset_x: f32,
    offset_y: f32,
) -> Result<(), CheckedRenderError> {
    tree_renderer::try_render_element_snapshot(element, snapshot, output, offset_x, offset_y)
        .map_err(|error| {
            let identity = snapshot.snapshot().root().identity().clone();
            let source = match error {
                tree_renderer::ProjectionError::Snapshot(source) => {
                    SnapshotRenderError::Snapshot { source }
                }
                tree_renderer::ProjectionError::Alias(source) => {
                    SnapshotRenderError::Alias { source }
                }
                tree_renderer::ProjectionError::Output { element_id, source } => {
                    let identity = element_id
                        .and_then(|element_id| snapshot.node_for_element(element_id).ok())
                        .map(|node| node.identity().clone())
                        .unwrap_or(identity);
                    SnapshotRenderError::Output { identity, source }
                }
                other => SnapshotRenderError::Text {
                    identity,
                    source: other.into_text_render_error(element.id),
                },
            };
            CheckedRenderError::Snapshot(source)
        })
}

pub(crate) fn legacy_snapshot_coordinate_error(
    element: &Element,
    source: &LayoutSnapshotError,
) -> Option<TextRenderError> {
    if let LayoutSnapshotError::TextFlowRevision {
        identity: _,
        source,
    } = source
    {
        let element_id = element.id;
        return Some(TextRenderError::flow(element_id, source.clone()));
    }
    let (_identity, coordinate) = match source {
        LayoutSnapshotError::NonFiniteGeometry { identity, .. } => {
            (identity, super::TextCoordinateError::NonFinite)
        }
        LayoutSnapshotError::NegativeExtent { identity, .. }
        | LayoutSnapshotError::EdgeArithmeticOverflow { identity, .. }
        | LayoutSnapshotError::CellCoordinateOverflow { identity, .. }
        | LayoutSnapshotError::ReversedContentBounds { identity, .. } => {
            (identity, super::TextCoordinateError::Overflow)
        }
        _ => return None,
    };
    let element_id = legacy_coordinate_source_element(element).unwrap_or(element.id);
    Some(TextRenderError::coordinate(element_id, coordinate))
}

fn legacy_coordinate_source_element(element: &Element) -> Option<ElementId> {
    for child in &element.children {
        if let Some(element_id) = legacy_coordinate_source_element(child) {
            return Some(element_id);
        }
    }
    let padding = &element.style.padding;
    [padding.left, padding.top, padding.right, padding.bottom]
        .into_iter()
        .any(|value| !value.is_finite() || value.abs() >= i32::MAX as f32)
        .then_some(element.id)
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

#[cfg(test)]
mod tests;
