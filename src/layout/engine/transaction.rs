#![forbid(missing_docs)]

//! Clone-staged target-aware layout transaction and recovery.

use crate::core::{Element, VNode};
use crate::reconciler::{
    ReconcilePlan, ReconcilePlanError, ScopedIdentityArena, plan_diff_in, plan_initial_tree_in,
    semantically_equal_vnode_in,
};

use super::{
    CheckedIncrementalLayoutReport, FullRebuildError, IncrementalLayoutError,
    InvalidLayoutTargetError, LayoutEngine, PatchStage, PatchTransactionCause,
    PatchTransactionError, RebuildFailure, RebuildStage, RecoveredSnapshotError,
    TransactionalLayoutError,
    context_sync::{ContextSyncError, LayoutRunError},
    incremental::ElementVNodeSnapshot,
    patching,
    postcondition::{TargetAliasExpectation, TargetValidationError},
    snapshot::{
        SnapshotAttemptEvidence, SnapshotProducerEvidence, attempt_evidence_since,
        cache_hits_since, mutations_since, recomputes_since,
    },
};
use crate::layout::{LayoutSnapshot, PreparedSnapshotFrame, SnapshotBuildReport};

/// A fully validated layout frame that has not changed the committed engine.
///
/// Rendering code can read the candidate through crate-internal adapters and
/// defer [`commit`](Self::commit) until every fallible output operation has
/// succeeded. Dropping this value leaves the source engine unchanged.
///
/// ```
/// use rnk::{core::Element, layout::{CheckedIncrementalLayoutReport, LayoutEngine, PreparedLayoutFrame}};
/// let mut engine = LayoutEngine::new();
/// let frame: PreparedLayoutFrame = engine.prepare_element_incremental(&Element::root(), None, 20, 4).expect("prepared frame");
/// assert!(matches!(frame.report(), CheckedIncrementalLayoutReport::InitialFullBuild));
/// frame.commit(&mut engine);
/// ```
pub struct PreparedLayoutFrame {
    state: PreparedLayoutState,
    current_vnode: VNode,
    report: CheckedIncrementalLayoutReport,
    source_epoch: std::sync::Arc<()>,
}

struct CandidateAttemptError {
    source: Box<PatchTransactionError>,
    evidence: SnapshotAttemptEvidence,
}

enum PreparedLayoutState {
    Replacement(LayoutEngine),
    AliasOverlay(LayoutEngine),
}

#[derive(Debug)]
pub(crate) struct PreparedLayoutCommitError;

impl std::fmt::Display for PreparedLayoutCommitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("prepared layout frame no longer matches its source engine")
    }
}

impl std::error::Error for PreparedLayoutCommitError {}

pub(crate) struct BoundPreparedLayoutFrame<'a> {
    prepared: PreparedLayoutFrame,
    committed: &'a mut LayoutEngine,
}

impl PreparedLayoutFrame {
    /// Returns the target VNode represented by this prepared frame.
    pub fn current_vnode(&self) -> &VNode {
        &self.current_vnode
    }

    /// Returns the checked incremental/recovery classification.
    pub fn report(&self) -> &CheckedIncrementalLayoutReport {
        &self.report
    }

    /// Returns the immutable terminal-cell snapshot for this candidate.
    pub fn snapshot(&self) -> &LayoutSnapshot {
        self.engine().prepared_snapshot().snapshot()
    }

    /// Returns non-semantic work evidence for snapshot construction.
    pub fn snapshot_report(&self) -> &SnapshotBuildReport {
        self.engine().prepared_snapshot_report()
    }

    /// Returns the semantic snapshot together with this frame's exact aliases.
    pub fn prepared_snapshot(&self) -> &PreparedSnapshotFrame {
        self.engine().prepared_snapshot()
    }

    pub(crate) fn engine(&self) -> &LayoutEngine {
        match &self.state {
            PreparedLayoutState::Replacement(candidate)
            | PreparedLayoutState::AliasOverlay(candidate) => candidate,
        }
    }

    /// Atomically replaces `committed` with this already-validated candidate.
    ///
    /// Once the source engine remains current, this operation only moves owned
    /// values and cannot fail.
    ///
    /// # Panics
    ///
    /// Panics before mutation if `committed` is not the engine this frame was
    /// prepared from, or if that engine has published a newer state.
    pub fn commit(self, committed: &mut LayoutEngine) -> (VNode, CheckedIncrementalLayoutReport) {
        assert!(
            self.is_fresh_for(committed),
            "prepared layout frame no longer matches its source engine"
        );
        self.commit_unchecked(committed)
    }

    pub(crate) fn bind(
        self,
        committed: &mut LayoutEngine,
    ) -> Result<BoundPreparedLayoutFrame<'_>, PreparedLayoutCommitError> {
        if !self.is_fresh_for(committed) {
            return Err(PreparedLayoutCommitError);
        }
        Ok(BoundPreparedLayoutFrame {
            prepared: self,
            committed,
        })
    }

    fn is_fresh_for(&self, committed: &LayoutEngine) -> bool {
        std::sync::Arc::ptr_eq(&self.source_epoch, &committed.commit_epoch)
    }

    fn commit_unchecked(
        self,
        committed: &mut LayoutEngine,
    ) -> (VNode, CheckedIncrementalLayoutReport) {
        match self.state {
            PreparedLayoutState::Replacement(candidate) => *committed = candidate,
            PreparedLayoutState::AliasOverlay(overlay) => {
                committed.node_map = overlay.node_map;
                committed.element_keys = overlay.element_keys;
                committed.element_scopes = overlay.element_scopes;
                committed.current_text_flows = overlay.current_text_flows;
                committed.published_snapshot = overlay.published_snapshot;
                committed.published_snapshot_report = overlay.published_snapshot_report;
                committed.successful_mutations = overlay.successful_mutations;
            }
        }
        committed.rotate_commit_epoch();
        (self.current_vnode, self.report)
    }
}

impl BoundPreparedLayoutFrame<'_> {
    pub(crate) fn commit(self) -> (VNode, CheckedIncrementalLayoutReport) {
        self.prepared.commit_unchecked(self.committed)
    }
}

impl LayoutEngine {
    /// Prepare an Element layout frame without publishing it.
    ///
    /// The returned candidate includes target-exact ElementId aliases and has
    /// passed layout, TextFlow, and structural postconditions. It can be
    /// rendered and then committed, or simply dropped on any later failure.
    ///
    /// # Errors
    ///
    /// Returns [`TransactionalLayoutError::Upstream`] for preflight failures,
    /// [`TransactionalLayoutError::InitialBuild`] for an initial-frame failure,
    /// [`TransactionalLayoutError::Snapshot`] when the checked snapshot builder
    /// rejects an initial or ordinary candidate while preserving its partial
    /// attempt report,
    /// [`TransactionalLayoutError::InvalidTarget`] when a committed engine is
    /// given a target that cannot form a layout tree,
    /// [`TransactionalLayoutError::RecoveredSnapshot`] when the one permitted
    /// recovery candidate reaches snapshot construction but fails there,
    /// or [`TransactionalLayoutError::RecoveryFailed`] when both layout
    /// candidates fail before snapshot construction.
    ///
    /// ```
    /// use rnk::core::Element;
    /// use rnk::layout::{CheckedIncrementalLayoutReport, LayoutEngine};
    ///
    /// let root = Element::root();
    /// let mut engine = LayoutEngine::new();
    /// let prepared = engine
    ///     .prepare_element_incremental(&root, None, 20, 4)
    ///     .expect("valid initial frame");
    /// assert!(matches!(
    ///     prepared.report(),
    ///     CheckedIncrementalLayoutReport::InitialFullBuild
    /// ));
    /// let (_vnode, report) = prepared.commit(&mut engine);
    /// assert!(matches!(report, CheckedIncrementalLayoutReport::InitialFullBuild));
    /// assert!(engine.has_tree());
    /// ```
    pub fn prepare_element_incremental(
        &self,
        root: &Element,
        previous_vnode: Option<&VNode>,
        width: u16,
        height: u16,
    ) -> Result<PreparedLayoutFrame, TransactionalLayoutError> {
        self.prepare_element_incremental_inner(root, previous_vnode, width, height)
    }

    /// Compute and publish an Element frame with typed transaction recovery.
    ///
    /// Invalid reconciliation input fails before candidate construction. Once
    /// preflight succeeds, apply, context synchronization, layout, read-back,
    /// and publication form one clone-staged transaction. A failed incremental
    /// candidate gets exactly one fresh target rebuild; if that also fails the
    /// returned error retains both causes and this engine remains unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`TransactionalLayoutError::Upstream`] for preflight failures,
    /// [`TransactionalLayoutError::InitialBuild`] for an initial-frame failure,
    /// [`TransactionalLayoutError::Snapshot`] when the checked snapshot builder
    /// rejects an initial or ordinary candidate while preserving its partial
    /// attempt report,
    /// [`TransactionalLayoutError::InvalidTarget`] when a committed engine is
    /// given a target that cannot form a layout tree,
    /// [`TransactionalLayoutError::RecoveredSnapshot`] when the one permitted
    /// recovery candidate reaches snapshot construction but fails there,
    /// or [`TransactionalLayoutError::RecoveryFailed`] when both layout
    /// candidates fail before snapshot construction.
    ///
    /// ```
    /// use rnk::{core::{Element, ElementType}, layout::{LayoutEngine, TransactionalLayoutError}};
    /// let mut engine = LayoutEngine::new();
    /// let root = Element::new(ElementType::VirtualText);
    /// let result = engine.try_compute_element_incremental_transactional(&root, None, 20, 4);
    /// assert!(matches!(result, Err(TransactionalLayoutError::InitialBuild(_))));
    /// ```
    pub fn try_compute_element_incremental_transactional(
        &mut self,
        root: &Element,
        previous_vnode: Option<&VNode>,
        width: u16,
        height: u16,
    ) -> Result<(VNode, CheckedIncrementalLayoutReport), TransactionalLayoutError> {
        let prepared = self.prepare_element_incremental(root, previous_vnode, width, height)?;
        Ok(prepared.commit(self))
    }

    fn prepare_element_incremental_inner(
        &self,
        root: &Element,
        previous_vnode: Option<&VNode>,
        width: u16,
        height: u16,
    ) -> Result<PreparedLayoutFrame, TransactionalLayoutError> {
        let mut identity_arena = ScopedIdentityArena::seeded(self.vnode_map.keys());
        let snapshot = ElementVNodeSnapshot::from_element(root, &mut identity_arena)
            .map_err(IncrementalLayoutError::from)?;
        let initial_frame = previous_vnode.is_none() || !self.has_tree();
        if !snapshot.has_layout_root {
            let rebuild = FullRebuildError {
                stage: RebuildStage::BuildTarget,
                key: None,
                source: RebuildFailure::InvalidTargetRoot,
            };
            if initial_frame {
                return Err(TransactionalLayoutError::InitialBuild(rebuild));
            }
            return Err(TransactionalLayoutError::InvalidTarget(
                InvalidLayoutTargetError {
                    key: rebuild.key,
                    source: rebuild.source,
                },
            ));
        }
        let current_vnode = snapshot.vnode.clone();
        let initial_plan = plan_initial_tree_in(&current_vnode, &mut identity_arena)
            .map_err(IncrementalLayoutError::from)?;
        if initial_frame {
            let mut candidate = self
                .try_rebuild_snapshot_fresh(&snapshot, &current_vnode, width, height)
                .map_err(TransactionalLayoutError::InitialBuild)?;
            let evidence = SnapshotProducerEvidence::initial(
                mutations_since(&candidate, self),
                recomputes_since(&candidate, self),
                cache_hits_since(&candidate, self),
            );
            let (prepared_snapshot, snapshot_report) = candidate
                .try_build_snapshot_for(root, &snapshot, &initial_plan, &evidence)
                .map_err(TransactionalLayoutError::Snapshot)?;
            candidate.stage_prepared_snapshot(prepared_snapshot, snapshot_report);
            return Ok(PreparedLayoutFrame {
                state: PreparedLayoutState::Replacement(candidate),
                current_vnode,
                report: CheckedIncrementalLayoutReport::InitialFullBuild,
                source_epoch: self.commit_epoch.clone(),
            });
        }

        let previous = previous_vnode.expect("incremental precondition checked");
        let committed = self.committed_vnode.as_ref().ok_or_else(|| {
            IncrementalLayoutError::from(ReconcilePlanError::PreviousTreeMismatch)
        })?;
        if !semantically_equal_vnode_in(committed, previous, &mut identity_arena)
            .map_err(IncrementalLayoutError::from)?
        {
            return Err(
                IncrementalLayoutError::from(ReconcilePlanError::PreviousTreeMismatch).into(),
            );
        }
        let committed_plan = plan_diff_in(committed, committed, &mut identity_arena)
            .map_err(IncrementalLayoutError::from)?;
        self.validate_committed_plan(&committed_plan)
            .map_err(IncrementalLayoutError::from)?;
        let plan = plan_diff_in(committed, &current_vnode, &mut identity_arena)
            .map_err(IncrementalLayoutError::from)?;
        self.preflight_reconcile_plan(&plan)
            .map_err(IncrementalLayoutError::from)?;

        let patch_count = plan.patches().len();
        let viewport_changed = self.last_width != width || self.last_height != height;
        let unchanged_frame = patch_count == 0
            && !viewport_changed
            && self.text_contexts_match_scoped(&snapshot.text_inputs);
        let attempt: Result<Self, CandidateAttemptError> = if unchanged_frame {
            self.prepare_unchanged_element_candidate(
                &snapshot,
                &current_vnode,
                &plan,
                width,
                height,
            )
            .map_err(|source| CandidateAttemptError {
                source: Box::new(source),
                evidence: SnapshotAttemptEvidence {
                    mutations: Some(0),
                    text_flow_recomputes: Some(0),
                    cache_hits: Some(0),
                },
            })
        } else {
            self.prepare_changed_element_candidate_with_evidence(
                &snapshot,
                &current_vnode,
                &plan,
                width,
                height,
            )
        };

        match attempt {
            Ok(mut candidate) => {
                let report = if patch_count == 0 {
                    if viewport_changed {
                        CheckedIncrementalLayoutReport::RecomputedViewport
                    } else {
                        CheckedIncrementalLayoutReport::NoChange
                    }
                } else {
                    CheckedIncrementalLayoutReport::Incremental { patch_count }
                };
                let evidence = SnapshotProducerEvidence::incremental(
                    patch_count,
                    mutations_since(&candidate, self),
                    recomputes_since(&candidate, self),
                    cache_hits_since(&candidate, self),
                );
                let (prepared_snapshot, snapshot_report) = candidate
                    .try_build_snapshot_for(root, &snapshot, &plan, &evidence)
                    .map_err(TransactionalLayoutError::Snapshot)?;
                candidate.stage_prepared_snapshot(prepared_snapshot, snapshot_report);
                Ok(PreparedLayoutFrame {
                    state: if unchanged_frame {
                        PreparedLayoutState::AliasOverlay(candidate)
                    } else {
                        PreparedLayoutState::Replacement(candidate)
                    },
                    current_vnode,
                    report,
                    source_epoch: self.commit_epoch.clone(),
                })
            }
            Err(attempt_failure) => {
                let incremental_failure = *attempt_failure.source;
                match self.try_rebuild_snapshot_fresh(&snapshot, &current_vnode, width, height) {
                    Ok(mut rebuilt) => {
                        let evidence = SnapshotProducerEvidence::recovered(
                            patch_count,
                            incremental_failure.clone(),
                            attempt_failure.evidence,
                            attempt_evidence_since(&rebuilt, self),
                        );
                        let (prepared_snapshot, snapshot_report) = rebuilt
                            .try_build_snapshot_for(root, &snapshot, &plan, &evidence)
                            .map_err(|snapshot| {
                                TransactionalLayoutError::RecoveredSnapshot(
                                    RecoveredSnapshotError::new(
                                        incremental_failure.clone(),
                                        snapshot,
                                    ),
                                )
                            })?;
                        rebuilt.stage_prepared_snapshot(prepared_snapshot, snapshot_report);
                        Ok(PreparedLayoutFrame {
                            state: PreparedLayoutState::Replacement(rebuilt),
                            current_vnode,
                            report: CheckedIncrementalLayoutReport::RecoveredFullRebuild {
                                patch_count,
                                incremental_failure,
                            },
                            source_epoch: self.commit_epoch.clone(),
                        })
                    }
                    Err(rebuild) => Err(TransactionalLayoutError::RecoveryFailed {
                        incremental: Box::new(incremental_failure),
                        rebuild: Box::new(rebuild),
                    }),
                }
            }
        }
    }

    fn prepare_unchanged_element_candidate(
        &self,
        snapshot: &ElementVNodeSnapshot,
        target: &VNode,
        plan: &ReconcilePlan,
        width: u16,
        height: u16,
    ) -> Result<Self, PatchTransactionError> {
        let mut candidate = Self {
            taffy: self.taffy.clone(),
            node_map: std::collections::HashMap::new(),
            element_keys: std::collections::HashMap::new(),
            element_scopes: std::collections::HashMap::new(),
            vnode_map: self.vnode_map.clone(),
            vnode_legacy_keys: self.vnode_legacy_keys.clone(),
            root_node: self.root_node,
            last_width: self.last_width,
            last_height: self.last_height,
            flow_cache: self.flow_cache.clone(),
            text_flow_policy: self.text_flow_policy.clone(),
            current_text_flows: std::collections::HashMap::new(),
            current_vnode_flows: self.current_vnode_flows.clone(),
            committed_vnode: self.committed_vnode.clone(),
            commit_epoch: self.commit_epoch.clone(),
            published_snapshot: self.published_snapshot.clone(),
            published_snapshot_report: self.published_snapshot_report.clone(),
            successful_mutations: self.successful_mutations,
        };
        candidate
            .try_publish_noop_element_aliases(snapshot)
            .map_err(|source| {
                patching::transaction_stage_error(
                    plan,
                    PatchStage::VerifyPostcondition,
                    PatchTransactionCause::Invariant(source),
                )
            })?;
        candidate
            .validate_target_exact(
                plan,
                TargetAliasExpectation::Element(snapshot),
                target,
                width,
                height,
            )
            .map_err(|error| postcondition_error(plan, None, error))?;
        Ok(candidate)
    }

    #[cfg(test)]
    fn prepare_changed_element_candidate(
        &self,
        snapshot: &ElementVNodeSnapshot,
        target: &VNode,
        plan: &ReconcilePlan,
        width: u16,
        height: u16,
    ) -> Result<Self, PatchTransactionError> {
        self.prepare_changed_element_candidate_with_evidence(snapshot, target, plan, width, height)
            .map_err(|failure| *failure.source)
    }

    fn prepare_changed_element_candidate_with_evidence(
        &self,
        snapshot: &ElementVNodeSnapshot,
        target: &VNode,
        plan: &ReconcilePlan,
        width: u16,
        height: u16,
    ) -> Result<Self, CandidateAttemptError> {
        let mut candidate = self.staged_clone();
        candidate.last_width = width;
        candidate.last_height = height;
        let layout_origins = patching::LayoutPatchOrigins::for_plan(&candidate, plan);
        let attempt = (|| {
            candidate.apply_reconcile_plan(plan).map_err(|source| {
                patching::transaction_error_for_plan(plan, &layout_origins, source)
            })?;
            candidate
                .try_sync_text_contexts(&snapshot.text_inputs)
                .map_err(|source| context_sync_error(plan, &candidate, &layout_origins, source))?;
            candidate
                .try_sync_element_node_map_scoped(snapshot)
                .map_err(|source| {
                    patching::transaction_stage_error(
                        plan,
                        PatchStage::VerifyPostcondition,
                        PatchTransactionCause::Invariant(source),
                    )
                })?;
            candidate
                .run_layout_and_publish_checked(&mut || false)
                .map_err(|source| layout_run_error(plan, &candidate, &layout_origins, source))?;
            candidate.committed_vnode = super::Shared::new(Some(target.clone()));
            candidate
                .validate_target_exact(
                    plan,
                    TargetAliasExpectation::Element(snapshot),
                    target,
                    width,
                    height,
                )
                .map_err(|error| postcondition_error(plan, Some(&layout_origins), error))?;
            Ok::<(), PatchTransactionError>(())
        })();
        match attempt {
            Ok(()) => Ok(candidate),
            Err(source) => Err(CandidateAttemptError {
                source: Box::new(source),
                evidence: attempt_evidence_since(&candidate, self),
            }),
        }
    }
}

fn context_sync_error(
    plan: &ReconcilePlan,
    candidate: &LayoutEngine,
    origins: &patching::LayoutPatchOrigins,
    source: ContextSyncError,
) -> PatchTransactionError {
    let fallback_key = source.key();
    let (patch_index, key) = origins.locate(candidate, source.node_id());
    let cause = patching::context_sync_cause(source);
    if let Some(patch_index) = patch_index {
        return patching::direct_transaction_error_at_with_parent(
            plan.patches(),
            patch_index,
            origins.parent_for_patch(patch_index),
            PatchStage::SetContext,
            cause,
        );
    }
    patching::transaction_stage_error_for_key(
        plan,
        Some(origins),
        key.or(fallback_key),
        PatchStage::SetContext,
        cause,
    )
}

fn layout_run_error(
    plan: &ReconcilePlan,
    candidate: &LayoutEngine,
    origins: &patching::LayoutPatchOrigins,
    source: LayoutRunError,
) -> PatchTransactionError {
    let (patch_index, key) = origins.locate(candidate, source.node_id());
    let (stage, cause) = patching::layout_run_error_parts(source);
    if let Some(patch_index) = patch_index {
        return patching::direct_transaction_error_at_with_parent(
            plan.patches(),
            patch_index,
            origins.parent_for_patch(patch_index),
            stage,
            cause,
        );
    }
    patching::batch_transaction_error_with_key(key, stage, cause)
}

fn postcondition_error(
    plan: &ReconcilePlan,
    origins: Option<&patching::LayoutPatchOrigins>,
    error: TargetValidationError,
) -> PatchTransactionError {
    let key = error.key;
    let cause = patching::target_validation_cause(error.source);
    patching::transaction_stage_error_for_key(
        plan,
        origins,
        key,
        PatchStage::VerifyPostcondition,
        cause,
    )
}

#[cfg(test)]
pub(super) mod tests;
