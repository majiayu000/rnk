#![forbid(missing_docs)]

use std::error::Error;
use std::fmt;
use std::io;

use crate::core::ElementId;
use crate::layout::{IncrementalLayoutError, LayoutLookupError, TextFlowError};

/// A typed projection failure that does not expose frame contents or source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextProjectionError {
    /// A visible element had no current-frame layout.
    MissingLayout,
    /// The staged writer and projected source map disagreed.
    WriterOutcomeMismatch,
    /// Rendering completed with an unmatched clip operation.
    UnbalancedClipStack,
    /// A deterministic test-only projection fault was injected.
    InjectedFailure,
}

impl fmt::Display for TextProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingLayout => write!(formatter, "missing current element layout"),
            Self::WriterOutcomeMismatch => write!(formatter, "staged writer outcome mismatch"),
            Self::UnbalancedClipStack => write!(formatter, "unbalanced staged clip stack"),
            Self::InjectedFailure => write!(formatter, "injected staged projection failure"),
        }
    }
}

impl Error for TextProjectionError {}

/// A typed coordinate failure produced while projecting a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextCoordinateError {
    /// A projected coordinate was NaN or infinite.
    NonFinite,
    /// A finite coordinate exceeded the terminal coordinate range.
    Overflow,
}

impl fmt::Display for TextCoordinateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => write!(formatter, "non-finite render coordinate"),
            Self::Overflow => write!(formatter, "render coordinate overflow"),
        }
    }
}

impl Error for TextCoordinateError {}

/// End-to-end text layout and rendering failure.
#[derive(Debug)]
pub enum TextRenderError {
    /// TextFlow construction failed for an element.
    Flow {
        /// Element whose flow failed.
        element_id: ElementId,
        /// Concrete TextFlow failure.
        source: TextFlowError,
    },
    /// A text element had no flow published for the current frame.
    MissingCurrentFlow {
        /// Element missing its current flow.
        element_id: ElementId,
    },
    /// Projected output did not cover the complete source text.
    IncompleteSourceMap {
        /// Element with incomplete source coverage.
        element_id: ElementId,
    },
    /// Staging or source projection failed.
    Projection {
        /// Element whose projection failed.
        element_id: ElementId,
        /// Concrete projection failure.
        source: TextProjectionError,
    },
    /// A render coordinate could not be represented safely.
    Coordinate {
        /// Element whose coordinate failed.
        element_id: ElementId,
        /// Concrete coordinate failure.
        source: TextCoordinateError,
    },
    /// A terminal output operation failed.
    Io {
        /// Static description of the failed operation.
        operation: &'static str,
        /// Underlying I/O failure.
        source: io::Error,
    },
}

impl TextRenderError {
    pub(crate) fn flow(element_id: ElementId, source: TextFlowError) -> Self {
        Self::Flow { element_id, source }
    }

    pub(crate) fn projection(element_id: ElementId, source: TextProjectionError) -> Self {
        Self::Projection { element_id, source }
    }

    pub(crate) fn coordinate(element_id: ElementId, source: TextCoordinateError) -> Self {
        Self::Coordinate { element_id, source }
    }

    /// Builds a terminal I/O rendering failure without discarding its source.
    pub fn io(operation: &'static str, source: io::Error) -> Self {
        Self::Io { operation, source }
    }

    pub(crate) fn into_io(self) -> io::Error {
        io::Error::other(self)
    }
}

impl fmt::Display for TextRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flow { element_id, source } => {
                write!(
                    formatter,
                    "text flow failed for element {element_id:?}: {source}"
                )
            }
            Self::MissingCurrentFlow { element_id } => {
                write!(
                    formatter,
                    "missing current TextFlow for element {element_id:?}"
                )
            }
            Self::IncompleteSourceMap { element_id } => {
                write!(
                    formatter,
                    "incomplete source projection for element {element_id:?}"
                )
            }
            Self::Projection { element_id, source } => {
                write!(
                    formatter,
                    "text projection failed for element {element_id:?}: {source}"
                )
            }
            Self::Coordinate { element_id, source } => {
                write!(
                    formatter,
                    "text coordinate failed for element {element_id:?}: {source}"
                )
            }
            Self::Io { operation, source } => {
                write!(formatter, "terminal I/O failed while {operation}: {source}")
            }
        }
    }
}

impl Error for TextRenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Flow { source, .. } => Some(source),
            Self::Projection { source, .. } => Some(source),
            Self::Coordinate { source, .. } => Some(source),
            Self::Io { source, .. } => Some(source),
            Self::MissingCurrentFlow { .. } | Self::IncompleteSourceMap { .. } => None,
        }
    }
}

/// Checked dynamic-frame failure.
///
/// Reconciliation identity and legacy lookup failures remain separate from
/// TextFlow/text projection errors so invalid targets cannot enter a rebuild
/// fallback or appear as a successful frame.
#[derive(Debug)]
pub enum DynamicFrameError {
    /// Reconciliation planning or incremental layout failed.
    Incremental(IncrementalLayoutError),
    /// Text projection or output staging failed.
    Text(TextRenderError),
    /// A legacy compatibility layout lookup was ambiguous.
    LegacyLookup(LayoutLookupError),
}

impl fmt::Display for DynamicFrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incremental(source) => write!(formatter, "dynamic layout failed: {source}"),
            Self::Text(source) => write!(formatter, "dynamic text render failed: {source}"),
            Self::LegacyLookup(source) => {
                write!(formatter, "dynamic layout lookup failed: {source}")
            }
        }
    }
}

impl Error for DynamicFrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Incremental(source) => Some(source),
            Self::Text(source) => Some(source),
            Self::LegacyLookup(source) => Some(source),
        }
    }
}

impl From<IncrementalLayoutError> for DynamicFrameError {
    fn from(source: IncrementalLayoutError) -> Self {
        Self::Incremental(source)
    }
}

impl From<TextRenderError> for DynamicFrameError {
    fn from(source: TextRenderError) -> Self {
        Self::Text(source)
    }
}

impl From<LayoutLookupError> for DynamicFrameError {
    fn from(source: LayoutLookupError) -> Self {
        Self::LegacyLookup(source)
    }
}
