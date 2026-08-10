//! Permanently-poisoned checked snapshot construction.

use std::{collections::HashMap, sync::Arc};

use crate::{core::ElementId, layout::PatchTransactionError};

use super::{
    AxisClip, CellPoint, CellRect, CellVector, FrameAliasOverlay, FrameRevision, LayoutSnapshot,
    LayoutSnapshotError, PreparedSnapshotFrame, SnapshotAttemptReport, SnapshotBuildFailure,
    SnapshotBuildReport, SnapshotBuildStrategy, SnapshotIdentity, SnapshotInvariantError,
    SnapshotNode, SnapshotNodeIndex, SnapshotTargetMismatchReason, SnapshotWorkCounterField,
    SnapshotWorkCounters, TextFlowSemanticStamp,
};

pub(crate) struct CheckedSnapshotNodeInput {
    pub(crate) element_id: ElementId,
    pub(crate) identity: SnapshotIdentity,
    pub(crate) parent: Option<SnapshotNodeIndex>,
    pub(crate) border_bounds: CellRect,
    pub(crate) content_bounds: CellRect,
    pub(crate) text_origin: CellPoint,
    pub(crate) effective_clip: AxisClip,
    pub(crate) scroll_transform: CellVector,
    pub(crate) text_flow: Option<TextFlowSemanticStamp>,
}

pub(crate) struct LayoutSnapshotBuilder {
    viewport: CellRect,
    nodes: Vec<SnapshotNode>,
    semantic_index: HashMap<SnapshotIdentity, SnapshotNodeIndex>,
    aliases: HashMap<ElementId, SnapshotNodeIndex>,
    open_path: Vec<SnapshotNodeIndex>,
    attempt_report: SnapshotAttemptReport,
    first_failure: Option<(LayoutSnapshotError, SnapshotAttemptReport)>,
}

impl LayoutSnapshotBuilder {
    pub(crate) fn new(width: u16, height: u16, operation_count: u64) -> Self {
        Self {
            viewport: CellRect::viewport(width, height),
            nodes: Vec::new(),
            semantic_index: HashMap::new(),
            aliases: HashMap::new(),
            open_path: Vec::new(),
            attempt_report: SnapshotAttemptReport::new(
                operation_count,
                SnapshotWorkCounters::zero(),
            ),
            first_failure: None,
        }
    }

    pub(crate) fn viewport(&self) -> CellRect {
        self.viewport
    }

    pub(crate) fn add_work(
        &mut self,
        delta: SnapshotWorkCounters,
    ) -> Result<(), SnapshotBuildFailure> {
        if let Some(failure) = self.poisoned_failure() {
            return Err(failure);
        }
        let next = self
            .attempt_report
            .work_counters()
            .checked_add(delta)
            .map_err(|source| self.poison(LayoutSnapshotError::WorkCounters { source }))?;
        self.attempt_report.set_work_counters(next);
        Ok(())
    }

    pub(crate) fn push_ordered(
        &mut self,
        input: CheckedSnapshotNodeInput,
    ) -> Result<SnapshotNodeIndex, SnapshotBuildFailure> {
        if let Some(failure) = self.poisoned_failure() {
            return Err(failure);
        }
        if !input.border_bounds.contains(input.content_bounds) {
            return Err(self.poison(LayoutSnapshotError::ReversedContentBounds {
                identity: input.identity,
                border_bounds: input.border_bounds,
                attempted_content_bounds: super::AttemptedContentBounds::from_raw(
                    input.content_bounds.left(),
                    input.content_bounds.top(),
                    input.content_bounds.right(),
                    input.content_bounds.bottom(),
                ),
            }));
        }
        if self.semantic_index.contains_key(&input.identity) {
            return Err(self.poison(LayoutSnapshotError::DuplicateIdentity {
                identity: input.identity,
            }));
        }
        if self.aliases.contains_key(&input.element_id) {
            return Err(self.poison(LayoutSnapshotError::InvalidTree {
                identity: Some(input.identity.clone()),
                source: SnapshotInvariantError::SnapshotTargetMismatch {
                    identity: input.identity,
                    reason: SnapshotTargetMismatchReason::MissingAlias,
                },
            }));
        }

        let index = SnapshotNodeIndex::checked(self.nodes.len());
        match (self.nodes.is_empty(), input.parent) {
            (true, None) => {}
            (true, Some(_)) => {
                return Err(self.poison(LayoutSnapshotError::InvalidTree {
                    identity: Some(input.identity.clone()),
                    source: SnapshotInvariantError::OrphanNode {
                        identity: input.identity,
                    },
                }));
            }
            (false, None) => {
                return Err(self.poison(LayoutSnapshotError::InvalidTree {
                    identity: Some(input.identity.clone()),
                    source: SnapshotInvariantError::OrphanNode {
                        identity: input.identity,
                    },
                }));
            }
            (false, Some(parent)) => {
                let Some(depth) = self
                    .open_path
                    .iter()
                    .position(|candidate| *candidate == parent)
                else {
                    let expected_parent = self
                        .open_path
                        .last()
                        .and_then(|index| self.nodes.get(index.0))
                        .map(|node| node.identity.clone())
                        .unwrap_or_else(|| input.identity.clone());
                    return Err(self.poison(LayoutSnapshotError::InvalidTree {
                        identity: Some(input.identity.clone()),
                        source: SnapshotInvariantError::MissingParent {
                            child: input.identity,
                            expected_parent,
                        },
                    }));
                };
                self.open_path.truncate(depth + 1);
                self.nodes[parent.0].children = self.nodes[parent.0]
                    .children
                    .iter()
                    .copied()
                    .chain(std::iter::once(index))
                    .collect::<Vec<_>>()
                    .into();
            }
        }

        self.semantic_index.insert(input.identity.clone(), index);
        self.aliases.insert(input.element_id, index);
        self.nodes.push(SnapshotNode {
            identity: input.identity,
            parent: input.parent,
            children: Arc::from([]),
            border_bounds: input.border_bounds,
            content_bounds: input.content_bounds,
            text_origin: input.text_origin,
            effective_clip: input.effective_clip,
            scroll_transform: input.scroll_transform,
            text_flow: input.text_flow,
        });
        self.open_path.push(index);
        Ok(index)
    }

    pub(crate) fn finish(
        mut self,
        strategy: SnapshotBuildStrategy,
        patch_count: usize,
        recovery_cause: Option<PatchTransactionError>,
        cache_hits: u64,
    ) -> Result<(PreparedSnapshotFrame, SnapshotBuildReport), SnapshotBuildFailure> {
        if let Some(failure) = self.poisoned_failure() {
            return Err(failure);
        }
        if self.nodes.is_empty() {
            let identity =
                SnapshotIdentity::from_scoped(crate::reconciler::ScopedNodeIdentity::Root);
            return Err(self.poison(LayoutSnapshotError::InvalidTree {
                identity: None,
                source: SnapshotInvariantError::SnapshotTargetMismatch {
                    identity,
                    reason: SnapshotTargetMismatchReason::MissingRoot,
                },
            }));
        }
        let node_count = u64::try_from(self.nodes.len()).map_err(|_| {
            self.poison(LayoutSnapshotError::WorkCounters {
                source: super::SnapshotCounterError::Overflow {
                    field: SnapshotWorkCounterField::SnapshotNodes,
                    lhs: 0,
                    rhs: u64::MAX,
                },
            })
        })?;
        self.add_work(SnapshotWorkCounters::from_fields(0, 0, 0, node_count, 0))?;
        let work = self.attempt_report.work_counters();
        let snapshot = Arc::new(LayoutSnapshot {
            viewport: self.viewport,
            nodes: self.nodes.into(),
            root: SnapshotNodeIndex::checked(0),
            semantic_index: Arc::new(self.semantic_index),
        });
        let prepared = PreparedSnapshotFrame {
            snapshot,
            frame_aliases: FrameAliasOverlay {
                revision: FrameRevision::next(),
                elements: self.aliases,
            },
        };
        Ok((
            prepared,
            SnapshotBuildReport {
                strategy,
                patch_count,
                recovery_cause,
                cache_hits,
                work,
            },
        ))
    }

    pub(crate) fn fail(&mut self, source: LayoutSnapshotError) -> SnapshotBuildFailure {
        self.poison(source)
    }

    fn poison(&mut self, source: LayoutSnapshotError) -> SnapshotBuildFailure {
        let (source, report) = self
            .first_failure
            .get_or_insert_with(|| (source, self.attempt_report.clone()));
        SnapshotBuildFailure::new(source.clone(), report.clone())
    }

    fn poisoned_failure(&self) -> Option<SnapshotBuildFailure> {
        self.first_failure
            .as_ref()
            .map(|(source, report)| SnapshotBuildFailure::new(source.clone(), report.clone()))
    }
}
