//! Layout system using Taffy

mod engine;
pub mod measure;
mod snapshot;
pub mod text_flow;
mod utils;

pub(crate) use engine::{
    BoundPreparedLayoutFrame, CheckedLayoutSnapshot, LegacyLayoutSnapshotError,
    PreparedLayoutCommitError,
};
pub use engine::{
    CheckedIncrementalLayoutReport, DirectPatchApplyReport, DirectPatchError,
    DirectPatchPreflightCause, DirectPatchPreflightError, FullRebuildError,
    IncrementalInvariantError, IncrementalLayoutError, IncrementalLayoutOutcome,
    IncrementalPatchKind, InvalidLayoutTargetError, Layout, LayoutEngine, LayoutLookupError,
    PatchError, PatchFailure, PatchKind, PatchStage, PatchTransactionCause, PatchTransactionError,
    PreparedLayoutFrame, RebuildFailure, RebuildStage, RecoveredSnapshotError,
    TransactionalLayoutError,
};
pub use measure::{
    TextAlign, display_width, measure_text, measure_text_width, pad_text, truncate_middle,
    truncate_start, truncate_text, wrap_text,
};
pub use snapshot::{
    ArithmeticOperation, AttemptedContentBounds, Axis, AxisClip, CellOutputError, CellPoint,
    CellRect, CellSpan, CellVector, Edge, FrameRevision, GeometryField, LayoutAliasError,
    LayoutSnapshot, LayoutSnapshotError, PreparedSnapshotFrame, SnapshotAttemptReport,
    SnapshotBuildError, SnapshotBuildReport, SnapshotBuildStrategy, SnapshotCounterError,
    SnapshotIdentity, SnapshotInvariantError, SnapshotNode, SnapshotNodeIndex,
    SnapshotTargetMismatchReason, SnapshotWorkCounterField, SnapshotWorkCounters,
    TextFlowSemanticStamp,
};
pub use text_flow::{
    StyledTextRange, TextFlow, TextFlowCache, TextFlowDiagnostic, TextFlowError, TextFlowInput,
    TextFlowOptions, TextFlowPlacement, TextFlowPositionMapEntry, TextFlowRow, TextFlowRun,
    TextFlowSource, TextFlowSourceKind, TextFlowToken, UnicodeWidthPolicy,
};
pub use utils::{
    Position, center, center_horizontal, center_vertical, h_gap, h_spacer, join_horizontal,
    join_vertical, pad_to_width, place, place_horizontal, place_vertical, space_around,
    space_between, space_evenly, v_gap, v_spacer,
};
