#![forbid(missing_docs)]

//! Applying a batch of reconciler patches to the Taffy tree.
mod layout_origin;
#[cfg(test)]
mod rebuild_counter;
#[cfg(test)]
pub(super) use rebuild_counter::take_attempts as take_fresh_rebuild_attempts;

use super::IncrementalInvariantError;
use super::LayoutEngine;
use super::context_sync::LayoutRunError;
use super::incremental::{ApplyPlanError, ElementVNodeSnapshot};
use super::patch_error::{
    DirectPatchApplyReport, DirectPatchError, DirectPatchPreflightCause, DirectPatchPreflightError,
    FullRebuildError, IncrementalPatchKind, LayoutLookupError, PatchError, PatchFailure,
    PatchStage, PatchTransactionCause, PatchTransactionError, RebuildFailure, RebuildStage,
    TransactionalLayoutError,
};
use crate::core::{NodeKey, VNode};
use crate::reconciler::{
    Patch, ReconcilePlan, ReconcilePlanError, ScopedIdentityArena, plan_initial_tree_in,
};
pub(super) use layout_origin::LayoutPatchOrigins;

fn direct_patch_locator(
    patch_index: usize,
    patch: &Patch,
) -> (
    usize,
    IncrementalPatchKind,
    Option<NodeKey>,
    Option<NodeKey>,
) {
    match patch {
        Patch::Create { key, parent, .. } => (
            patch_index,
            IncrementalPatchKind::Create,
            Some(*key),
            Some(*parent),
        ),
        Patch::Update { key, .. } => (patch_index, IncrementalPatchKind::Update, Some(*key), None),
        Patch::Remove { key } => (patch_index, IncrementalPatchKind::Remove, Some(*key), None),
        Patch::Replace { key, .. } => {
            (patch_index, IncrementalPatchKind::Replace, Some(*key), None)
        }
        Patch::Reorder { parent, .. } => (
            patch_index,
            IncrementalPatchKind::Reorder,
            None,
            Some(*parent),
        ),
    }
}

pub(super) fn batch_transaction_error(
    stage: PatchStage,
    source: PatchTransactionCause,
) -> PatchTransactionError {
    batch_transaction_error_with_key(None, stage, source)
}

pub(super) fn batch_transaction_error_with_key(
    key: Option<NodeKey>,
    stage: PatchStage,
    source: PatchTransactionCause,
) -> PatchTransactionError {
    PatchTransactionError {
        patch_index: None,
        kind: IncrementalPatchKind::Recompute,
        key,
        parent: None,
        stage,
        source: Box::new(source),
    }
}

pub(super) fn layout_run_error_parts(
    source: LayoutRunError,
) -> (PatchStage, PatchTransactionCause) {
    match source {
        LayoutRunError::Taffy { source, .. } => (
            PatchStage::ComputeLayout,
            PatchTransactionCause::Taffy(source),
        ),
        LayoutRunError::TextFlow { source, .. } => (
            PatchStage::ComputeLayout,
            PatchTransactionCause::TextFlow(source),
        ),
        LayoutRunError::ReadBackTaffy { source, .. } => {
            (PatchStage::ReadBack, PatchTransactionCause::Taffy(source))
        }
        LayoutRunError::ReadBackTextFlow { source, .. } => (
            PatchStage::ReadBack,
            PatchTransactionCause::TextFlow(source),
        ),
        LayoutRunError::Invariant { source, .. } => (
            PatchStage::ReadBack,
            PatchTransactionCause::Invariant(source),
        ),
    }
}

pub(super) fn direct_transaction_error_at(
    patches: &[Patch],
    patch_index: usize,
    stage: PatchStage,
    source: PatchTransactionCause,
) -> PatchTransactionError {
    let (_, _, key, parent) = direct_patch_locator(patch_index, &patches[patch_index]);
    direct_transaction_error_with_locator(patches, patch_index, key, parent, stage, source)
}

pub(super) fn direct_transaction_error_at_with_parent(
    patches: &[Patch],
    patch_index: usize,
    parent: Option<NodeKey>,
    stage: PatchStage,
    source: PatchTransactionCause,
) -> PatchTransactionError {
    if parent.is_none() {
        return direct_transaction_error_at(patches, patch_index, stage, source);
    }
    let (_, _, key, patch_parent) = direct_patch_locator(patch_index, &patches[patch_index]);
    direct_transaction_error_with_locator(
        patches,
        patch_index,
        key,
        parent.or(patch_parent),
        stage,
        source,
    )
}

fn direct_transaction_error_with_locator(
    patches: &[Patch],
    patch_index: usize,
    key: Option<NodeKey>,
    parent: Option<NodeKey>,
    stage: PatchStage,
    source: PatchTransactionCause,
) -> PatchTransactionError {
    let (_, kind, _, _) = direct_patch_locator(patch_index, &patches[patch_index]);
    PatchTransactionError {
        patch_index: Some(patch_index),
        kind,
        key,
        parent,
        stage,
        source: Box::new(source),
    }
}

pub(super) fn transaction_error_for_plan(
    plan: &ReconcilePlan,
    origins: &LayoutPatchOrigins,
    error: ApplyPlanError,
) -> PatchTransactionError {
    if let Some(patch_index) = error.patch_index {
        return direct_transaction_error_at_with_parent(
            plan.patches(),
            patch_index,
            origins.parent_for_patch(patch_index),
            error.stage,
            error.source,
        );
    }
    PatchTransactionError {
        patch_index: None,
        kind: IncrementalPatchKind::Recompute,
        key: Some(error.patch.key),
        parent: None,
        stage: error.stage,
        source: Box::new(error.source),
    }
}

pub(super) fn transaction_stage_error(
    _plan: &ReconcilePlan,
    stage: PatchStage,
    source: PatchTransactionCause,
) -> PatchTransactionError {
    batch_transaction_error(stage, source)
}

pub(super) fn transaction_stage_error_for_key(
    plan: &ReconcilePlan,
    origins: Option<&LayoutPatchOrigins>,
    key: Option<NodeKey>,
    stage: PatchStage,
    source: PatchTransactionCause,
) -> PatchTransactionError {
    let mut matching = Vec::new();
    if let Some(key) = key {
        for (patch_index, patch) in plan.patches().iter().enumerate() {
            let (_, _, patch_key, parent) = direct_patch_locator(patch_index, patch);
            if patch_key
                .or(parent)
                .is_some_and(|candidate| candidate.identity() == key.identity())
            {
                matching.push(patch_index);
            }
        }
    }
    if let [patch_index] = matching.as_slice() {
        return direct_transaction_error_at_with_parent(
            plan.patches(),
            *patch_index,
            origins.and_then(|origins| origins.parent_for_patch(*patch_index)),
            stage,
            source,
        );
    }
    batch_transaction_error_with_key(key, stage, source)
}

fn legacy_preflight_error(error: DirectPatchPreflightError) -> DirectPatchError {
    let key = if error.kind == IncrementalPatchKind::Create {
        error.parent.or(error.key)
    } else {
        error.key.or(error.parent)
    }
    .unwrap_or_else(NodeKey::root);
    match *error.source {
        DirectPatchPreflightCause::Identity(source) => DirectPatchError::Identity(source),
        DirectPatchPreflightCause::AmbiguousTarget { match_count }
        | DirectPatchPreflightCause::AmbiguousParent { match_count } => {
            DirectPatchError::Lookup(LayoutLookupError::AmbiguousLegacyNodeKey {
                key,
                scoped_match_count: match_count,
            })
        }
        DirectPatchPreflightCause::AlreadyExists => {
            DirectPatchError::Identity(ReconcilePlanError::DuplicatePlannedIdentity {
                identity: format!("{:?}", key.identity()),
            })
        }
        source => {
            let failure = match source {
                DirectPatchPreflightCause::MissingTarget
                | DirectPatchPreflightCause::MissingParent
                | DirectPatchPreflightCause::DependencyRemoved { .. }
                | DirectPatchPreflightCause::DependencyReplaced { .. } => PatchFailure::UnknownNode,
                DirectPatchPreflightCause::RootMutation => PatchFailure::MissingParent,
                _ => PatchFailure::PostconditionViolated,
            };
            DirectPatchError::Patch(PatchError::new(error.kind.legacy(), key, failure))
        }
    }
}

fn validate_legacy_patch_payloads(patches: &[Patch]) -> Result<(), DirectPatchError> {
    for patch in patches {
        let subtree = match patch {
            Patch::Create { node, .. } | Patch::Replace { node, .. } => Some(node),
            Patch::Update { .. } | Patch::Remove { .. } | Patch::Reorder { .. } => None,
        };
        if let Some(subtree) = subtree {
            let mut arena = ScopedIdentityArena::default();
            plan_initial_tree_in(subtree, &mut arena).map_err(DirectPatchError::Identity)?;
        }
    }
    Ok(())
}

pub(super) fn legacy_direct_transaction_error(error: TransactionalLayoutError) -> DirectPatchError {
    match error {
        TransactionalLayoutError::DirectPatch(DirectPatchError::Preflight(source)) => {
            legacy_preflight_error(source)
        }
        TransactionalLayoutError::DirectPatch(DirectPatchError::Transaction(source)) => {
            DirectPatchError::Patch(source.legacy())
        }
        TransactionalLayoutError::DirectPatch(source) => source,
        other => panic!("targetless patching returned an impossible error: {other}"),
    }
}
impl LayoutEngine {
    /// Apply a batch of patches, or none of them.
    ///
    /// Returns whether the tree changed. A rejected batch leaves this engine
    /// exactly as it was, so the caller can fall back to a full rebuild from
    /// the current tree; see
    /// [`try_apply_patches_checked`](Self::try_apply_patches_checked) for every
    /// typed rejection cause.
    ///
    /// # Panics
    ///
    /// Panics with the final typed cause when the transaction is rejected.
    /// Use [`try_apply_patches_transactional`](Self::try_apply_patches_transactional)
    /// when the caller must recover without unwinding.
    ///
    /// ```
    /// use rnk::layout::LayoutEngine;
    ///
    /// let mut engine = LayoutEngine::new();
    /// assert!(!engine.apply_patches(&[]));
    /// ```
    pub fn apply_patches(&mut self, patches: &[Patch]) -> bool {
        self.try_apply_patches(patches)
            .unwrap_or_else(|error| panic!("patch transaction failed: {error}"))
    }

    /// Legacy transactional adapter preserving the pre-GH59 error signature.
    ///
    /// # Errors
    ///
    /// Returns [`PatchError`] for the six legacy patch-application failures.
    ///
    /// # Panics
    ///
    /// Panics with the typed cause when canonical identity validation or a
    /// scoped compatibility lookup fails. Use
    /// [`try_apply_patches_checked`](Self::try_apply_patches_checked) to handle
    /// those causes explicitly.
    ///
    /// ```
    /// use rnk::layout::LayoutEngine;
    ///
    /// let mut engine = LayoutEngine::new();
    /// assert!(!engine.try_apply_patches(&[]).expect("empty batch"));
    /// ```
    pub fn try_apply_patches(&mut self, patches: &[Patch]) -> Result<bool, PatchError> {
        match self.try_apply_patches_checked(patches) {
            Ok(changed) => Ok(changed),
            Err(DirectPatchError::Patch(source)) => Err(source),
            Err(error) => panic!("patch identity validation failed: {error}"),
        }
    }

    /// Apply a public raw patch batch transactionally with independent errors.
    ///
    /// # Errors
    ///
    /// Returns a typed identity, preflight, or patch transaction failure.
    ///
    /// ```
    /// use rnk::layout::LayoutEngine;
    ///
    /// let mut engine = LayoutEngine::new();
    /// assert!(!engine.try_apply_patches_checked(&[]).expect("empty batch"));
    /// ```
    pub fn try_apply_patches_checked(
        &mut self,
        patches: &[Patch],
    ) -> Result<bool, DirectPatchError> {
        validate_legacy_patch_payloads(patches)?;
        self.try_apply_patches_transactional(patches)
            .map(|report| matches!(report, DirectPatchApplyReport::Applied { .. }))
            .map_err(legacy_direct_transaction_error)
    }

    /// Apply a targetless raw patch batch as one checked transaction.
    ///
    /// The entire batch is first simulated against the private canonical
    /// VNode. Candidate mutation, layout, and target verification then happen
    /// on a clone, so every error leaves the committed engine unchanged. This
    /// targetless entrypoint never attempts a full rebuild.
    ///
    /// # Errors
    ///
    /// Returns [`TransactionalLayoutError::DirectPatch`] with the original
    /// patch ordinal and a typed preflight or transaction cause.
    ///
    /// ```
    /// use rnk::{core::{Props, VNode}, layout::{DirectPatchError, DirectPatchPreflightCause, LayoutEngine, TransactionalLayoutError}, reconciler::Patch};
    /// let child = VNode::box_node().with_key("same");
    /// let key = child.key;
    /// let tree = VNode::root().children([
    ///     VNode::box_node().with_key("left").child(child.clone()),
    ///     VNode::box_node().with_key("right").child(child),
    /// ]);
    /// let mut engine = LayoutEngine::new();
    /// engine.compute_vnode(&tree, 20, 4);
    /// let error = engine.try_apply_patches_transactional(&[Patch::update(key, Props::new(), Props::new())]).expect_err("ambiguous raw target");
    /// assert!(matches!(error, TransactionalLayoutError::DirectPatch(DirectPatchError::Preflight(source)) if matches!(*source.source, DirectPatchPreflightCause::AmbiguousTarget { .. })));
    /// ```
    pub fn try_apply_patches_transactional(
        &mut self,
        patches: &[Patch],
    ) -> Result<DirectPatchApplyReport, TransactionalLayoutError> {
        if patches.is_empty() {
            return Ok(DirectPatchApplyReport::NoChange);
        }
        let resolved = self
            .preflight_direct_patch_batch(patches)
            .map_err(|source| {
                TransactionalLayoutError::DirectPatch(DirectPatchError::Preflight(source))
            })?;
        let mut candidate = self.staged_clone();
        let mut layout_origins = LayoutPatchOrigins::default();
        for (patch_index, plan, error_origins) in &resolved.steps {
            candidate.apply_reconcile_plan(plan).map_err(|source| {
                let patch_index = error_origins
                    .get(&source.patch.key.identity())
                    .copied()
                    .unwrap_or(*patch_index);
                let locator = resolved.locators[patch_index];
                TransactionalLayoutError::DirectPatch(DirectPatchError::Transaction(
                    direct_transaction_error_with_locator(
                        patches,
                        patch_index,
                        locator.key,
                        locator.parent,
                        source.stage,
                        source.source,
                    ),
                ))
            })?;
            layout_origins.record_raw_step(plan, *patch_index);
        }
        candidate
            .run_layout_and_publish_checked(&mut || false)
            .map_err(|source| {
                let (patch_index, key) = layout_origins.locate(&candidate, source.node_id());
                let (stage, cause) = layout_run_error_parts(source);
                let error = if let Some(patch_index) = patch_index {
                    let locator = resolved.locators[patch_index];
                    direct_transaction_error_with_locator(
                        patches,
                        patch_index,
                        locator.key,
                        locator.parent,
                        stage,
                        cause,
                    )
                } else {
                    batch_transaction_error_with_key(key, stage, cause)
                };
                TransactionalLayoutError::DirectPatch(DirectPatchError::Transaction(error))
            })?;
        let target = resolved.target;
        candidate.committed_vnode = super::Shared::new(Some(target.clone()));
        candidate
            .validate_target_exact(
                &resolved.plan,
                super::postcondition::TargetAliasExpectation::RawVNode,
                &target,
                candidate.last_width,
                candidate.last_height,
            )
            .map_err(|error| {
                let key = error.key;
                TransactionalLayoutError::DirectPatch(DirectPatchError::Transaction(
                    batch_transaction_error_with_key(
                        key,
                        PatchStage::VerifyPostcondition,
                        match error.source {
                            super::postcondition::TargetValidationCause::Taffy(source) => {
                                PatchTransactionCause::Taffy(source)
                            }
                            super::postcondition::TargetValidationCause::Invariant(source) => {
                                PatchTransactionCause::Invariant(source)
                            }
                        },
                    ),
                ))
            })?;
        candidate.rotate_commit_epoch();
        *self = candidate;
        Ok(DirectPatchApplyReport::Applied {
            patch_count: patches.len(),
        })
    }

    pub(super) fn try_rebuild_snapshot_fresh(
        &self,
        snapshot: &ElementVNodeSnapshot,
        current_vnode: &VNode,
        width: u16,
        height: u16,
    ) -> Result<Self, FullRebuildError> {
        #[cfg(test)]
        rebuild_counter::record_attempt();
        let mut fresh = Self::new();
        fresh.flow_cache = self.flow_cache.clone();
        fresh.text_flow_policy = self.text_flow_policy.clone();
        fresh.last_width = width;
        fresh.last_height = height;
        let mut arena = ScopedIdentityArena::default();
        let plan =
            plan_initial_tree_in(current_vnode, &mut arena).map_err(|_| FullRebuildError {
                stage: RebuildStage::BuildTarget,
                key: Some(current_vnode.key),
                source: RebuildFailure::InvalidTargetRoot,
            })?;
        fresh.apply_reconcile_plan(&plan).map_err(|source| {
            let stage = match source.stage {
                PatchStage::SetContext => RebuildStage::SetContext,
                PatchStage::VerifyPostcondition => RebuildStage::VerifyPostcondition,
                _ => RebuildStage::BuildTarget,
            };
            FullRebuildError {
                stage,
                key: Some(source.patch.key),
                source: rebuild_failure(source.source),
            }
        })?;
        fresh
            .try_sync_text_contexts(&snapshot.text_inputs)
            .map_err(|source| {
                let key = source.key().or(Some(current_vnode.key));
                FullRebuildError {
                    stage: RebuildStage::SetContext,
                    key,
                    source: match source {
                        super::context_sync::ContextSyncError::Taffy { source, .. } => {
                            RebuildFailure::Taffy(source)
                        }
                        super::context_sync::ContextSyncError::Invariant { source, .. } => {
                            RebuildFailure::Invariant(source)
                        }
                    },
                }
            })?;
        fresh
            .try_sync_element_node_map_scoped(snapshot)
            .map_err(|reason| FullRebuildError {
                stage: RebuildStage::VerifyPostcondition,
                key: Some(current_vnode.key),
                source: RebuildFailure::Invariant(reason),
            })?;
        fresh
            .run_layout_and_publish_checked(&mut || false)
            .map_err(|source| {
                let key = LayoutPatchOrigins::default()
                    .locate(&fresh, source.node_id())
                    .1
                    .or(Some(current_vnode.key));
                FullRebuildError {
                    stage: match source {
                        LayoutRunError::Taffy { .. } | LayoutRunError::TextFlow { .. } => {
                            RebuildStage::ComputeLayout
                        }
                        LayoutRunError::ReadBackTaffy { .. }
                        | LayoutRunError::ReadBackTextFlow { .. }
                        | LayoutRunError::Invariant { .. } => RebuildStage::VerifyPostcondition,
                    },
                    key,
                    source: match source {
                        LayoutRunError::Taffy { source, .. }
                        | LayoutRunError::ReadBackTaffy { source, .. } => {
                            RebuildFailure::Taffy(source)
                        }
                        LayoutRunError::TextFlow { source, .. }
                        | LayoutRunError::ReadBackTextFlow { source, .. } => {
                            RebuildFailure::TextFlow(source)
                        }
                        LayoutRunError::Invariant { source, .. } => {
                            RebuildFailure::Invariant(source)
                        }
                    },
                }
            })?;
        fresh.committed_vnode = super::Shared::new(Some(current_vnode.clone()));
        fresh
            .validate_target_exact(
                &plan,
                super::postcondition::TargetAliasExpectation::Element(snapshot),
                current_vnode,
                width,
                height,
            )
            .map_err(|error| FullRebuildError {
                stage: RebuildStage::VerifyPostcondition,
                key: error.key.or(Some(current_vnode.key)),
                source: match error.source {
                    super::postcondition::TargetValidationCause::Taffy(source) => {
                        RebuildFailure::Taffy(source)
                    }
                    super::postcondition::TargetValidationCause::Invariant(source) => {
                        RebuildFailure::Invariant(source)
                    }
                },
            })?;
        if fresh.root_node.is_none() {
            return Err(FullRebuildError {
                stage: RebuildStage::VerifyPostcondition,
                key: Some(current_vnode.key),
                source: RebuildFailure::InvalidTargetRoot,
            });
        }
        Ok(fresh)
    }
}

fn rebuild_failure(source: PatchTransactionCause) -> RebuildFailure {
    match source {
        PatchTransactionCause::Taffy(source) => RebuildFailure::Taffy(source),
        PatchTransactionCause::TextFlow(source) => RebuildFailure::TextFlow(source),
        PatchTransactionCause::Invariant(reason) => RebuildFailure::Invariant(reason),
        PatchTransactionCause::Patch(PatchFailure::UnknownNode) => {
            RebuildFailure::Invariant(IncrementalInvariantError::ScopedMapMismatch)
        }
        PatchTransactionCause::Patch(PatchFailure::MissingParent) => {
            RebuildFailure::Invariant(IncrementalInvariantError::MissingRoot)
        }
        PatchTransactionCause::Patch(PatchFailure::BuildFailed) => {
            RebuildFailure::Invariant(IncrementalInvariantError::InvalidMappedNode)
        }
        PatchTransactionCause::Patch(PatchFailure::TreeRejected) => {
            RebuildFailure::Invariant(IncrementalInvariantError::InvalidMappedNode)
        }
        PatchTransactionCause::Patch(PatchFailure::LayoutFailed) => {
            RebuildFailure::Invariant(IncrementalInvariantError::MissingComputedLayout)
        }
        PatchTransactionCause::Patch(PatchFailure::PostconditionViolated) => {
            RebuildFailure::Invariant(IncrementalInvariantError::CurrentFrameContextMismatch)
        }
    }
}

#[cfg(test)]
pub(super) mod contract_tests {
    use super::super::LayoutEngine;
    use crate::components::Text;
    use crate::core::{Dimension, Element, TextWrap, VNode};
    use crate::layout::{IncrementalLayoutError, LayoutLookupError, TextFlowError};
    use crate::reconciler::ReconcilePlanError;

    fn fixed_width_parent(child: Element) -> Element {
        let mut parent = Element::box_element();
        parent.style.width = Dimension::Points(4.0);
        parent.add_child(child);
        parent
    }

    pub(crate) fn incremental_wrap_modes_refresh_context_bidirectionally() {
        for truncate_mode in [
            TextWrap::Truncate,
            TextWrap::TruncateStart,
            TextWrap::TruncateMiddle,
            TextWrap::TruncateEnd,
        ] {
            let mut engine = LayoutEngine::new();
            let initial_text = Text::new("abcdefgh")
                .key("wrap-context")
                .wrap(TextWrap::Wrap)
                .into_element();
            let initial_id = initial_text.id;
            let initial = fixed_width_parent(initial_text);
            let (wrapped, first_outcome) =
                engine.compute_element_incremental(&initial, None, 80, 10);
            assert!(!first_outcome.used_reconciler);
            let initial_layout = engine.get_layout(initial_id).expect("wrapped layout");
            assert_eq!((initial_layout.width, initial_layout.height), (4.0, 2.0));

            let truncated_text = Text::new("abcdefgh")
                .key("wrap-context")
                .wrap(truncate_mode)
                .into_element();
            let truncated_id = truncated_text.id;
            let truncated = fixed_width_parent(truncated_text);
            let (truncated_vnode, outcome) =
                engine.compute_element_incremental(&truncated, Some(&wrapped), 80, 10);
            assert!(outcome.used_reconciler);
            assert_eq!(outcome.patch_count, 1);
            assert!(!outcome.fallback_full_rebuild);
            let incremental = engine.get_layout(truncated_id).expect("truncated layout");
            let mut rebuilt = LayoutEngine::new();
            rebuilt.compute_element_incremental(&truncated, None, 80, 10);
            let full = rebuilt.get_layout(truncated_id).expect("rebuilt layout");
            assert_eq!(
                (incremental.width, incremental.height),
                (full.width, full.height)
            );
            assert_eq!((incremental.width, incremental.height), (4.0, 1.0));

            let wrapped_text = Text::new("abcdefgh")
                .key("wrap-context")
                .wrap(TextWrap::Wrap)
                .into_element();
            let wrapped_id = wrapped_text.id;
            let wrapped_again = fixed_width_parent(wrapped_text);
            let (_, outcome) =
                engine.compute_element_incremental(&wrapped_again, Some(&truncated_vnode), 80, 10);
            assert!(outcome.used_reconciler);
            assert_eq!(outcome.patch_count, 1);
            assert!(!outcome.fallback_full_rebuild);
            let incremental = engine.get_layout(wrapped_id).expect("wrapped layout");
            let mut rebuilt = LayoutEngine::new();
            rebuilt.compute_element_incremental(&wrapped_again, None, 80, 10);
            let full = rebuilt
                .get_layout(wrapped_id)
                .expect("rebuilt wrapped layout");
            assert_eq!(
                (incremental.width, incremental.height),
                (full.width, full.height)
            );
            assert_eq!((incremental.width, incremental.height), (4.0, 2.0));
        }
    }

    pub(crate) fn duplicate_sibling_key_fails_before_mutation() {
        let mut engine = LayoutEngine::new();
        let stable = Element::box_element();
        let (previous, _) = engine.compute_element_incremental(&stable, None, 20, 4);
        let before_root = engine.root_node;
        let before_count = engine.node_count();
        let mut invalid = Element::box_element();
        invalid.add_child(Element::box_element().with_key("duplicate"));
        invalid.add_child(Element::box_element().with_key("duplicate"));
        let failure = engine
            .try_compute_element_incremental_checked(&invalid, Some(&previous), 20, 4)
            .expect_err("duplicate target is rejected");
        assert!(matches!(
            failure,
            IncrementalLayoutError::Identity(ReconcilePlanError::DuplicateSiblingKey { .. })
        ));
        assert_eq!(engine.root_node, before_root);
        assert_eq!(engine.node_count(), before_count);
    }

    pub(crate) fn raw_legacy_lookup_reports_typed_ambiguity() {
        let tree = VNode::box_node().children([
            VNode::box_node()
                .with_key("left")
                .child(VNode::text("a").with_key("shared")),
            VNode::box_node()
                .with_key("right")
                .child(VNode::text("b").with_key("shared")),
        ]);
        let raw = tree.children[0].children[0].key;
        let mut engine = LayoutEngine::new();
        engine.compute_vnode(&tree, 20, 4);
        assert!(matches!(
            engine.try_get_vnode_layout(raw),
            Err(LayoutLookupError::AmbiguousLegacyNodeKey {
                scoped_match_count: 2,
                ..
            })
        ));
    }

    pub(crate) fn textflow_and_identity_causes_remain_distinct() {
        let identity = IncrementalLayoutError::from(ReconcilePlanError::PreviousTreeMismatch);
        let text = IncrementalLayoutError::from(TextFlowError::InvalidTabStop);
        assert!(matches!(identity, IncrementalLayoutError::Identity(_)));
        assert!(matches!(text, IncrementalLayoutError::TextFlow(_)));
    }

    pub(crate) fn checked_layout_accepts_public_box_text_component_roots() {
        struct Component;
        for root in [
            VNode::box_node(),
            VNode::text("root"),
            VNode::component::<Component>(),
        ] {
            let mut engine = LayoutEngine::new();
            engine.compute_vnode(&root, 20, 4);
            assert!(engine.get_vnode_layout(root.key).is_some());
        }
    }
}

#[cfg(test)]
pub(super) mod tests;
