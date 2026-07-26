use std::error::Error;
use std::fmt;
use std::io;

use crate::core::ElementId;
use crate::layout::TextFlowError;

/// A typed projection failure that does not expose frame contents or source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextProjectionError {
    MissingLayout,
    WriterOutcomeMismatch,
    UnbalancedClipStack,
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
    NonFinite,
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
    Flow {
        element_id: ElementId,
        source: TextFlowError,
    },
    MissingCurrentFlow {
        element_id: ElementId,
    },
    IncompleteSourceMap {
        element_id: ElementId,
    },
    Projection {
        element_id: ElementId,
        source: TextProjectionError,
    },
    Coordinate {
        element_id: ElementId,
        source: TextCoordinateError,
    },
    Io {
        operation: &'static str,
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
