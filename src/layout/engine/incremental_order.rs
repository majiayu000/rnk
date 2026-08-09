//! Final-order preflight, committed-tree validation, and exact child-order commit.

use std::collections::{HashMap, HashSet};

use taffy::NodeId;

use crate::reconciler::{
    Patch, PlannedNode, PlannedNodeAction, ReconcilePlan, ReconcilePlanError, ScopedNodeIdentity,
};

use super::LayoutEngine;
use super::incremental::{ApplyPlanError, apply_error, taffy_apply_error};
#[cfg(test)]
use super::patch_error::PatchError;
use super::patch_error::{PatchFailure, PatchKind, PatchStage};

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IncrementalOrderFault {
    PreflightStyle,
    ValidateChildren,
    CommitChildren,
    PostconditionChildren,
}

#[cfg(test)]
thread_local! {
    static INCREMENTAL_ORDER_FAULT: std::cell::Cell<Option<(IncrementalOrderFault, usize)>> = const {
        std::cell::Cell::new(None)
    };
}

#[cfg(test)]
pub(super) fn set_incremental_order_fault(fault: IncrementalOrderFault) {
    set_incremental_order_fault_at(fault, 0);
}

#[cfg(test)]
pub(super) fn set_incremental_order_fault_at(fault: IncrementalOrderFault, occurrence: usize) {
    INCREMENTAL_ORDER_FAULT.with(|slot| slot.set(Some((fault, occurrence))));
}

#[cfg(test)]
fn take_incremental_order_fault(fault: IncrementalOrderFault) -> bool {
    INCREMENTAL_ORDER_FAULT.with(|slot| match slot.get() {
        Some((armed, 0)) if armed == fault => {
            slot.set(None);
            true
        }
        Some((armed, remaining)) if armed == fault => {
            slot.set(Some((armed, remaining - 1)));
            false
        }
        _ => false,
    })
}

impl LayoutEngine {
    fn preflight_style_exists(&self, node_id: NodeId) -> bool {
        #[cfg(test)]
        if take_incremental_order_fault(IncrementalOrderFault::PreflightStyle) {
            return false;
        }
        self.taffy.style(node_id).is_ok()
    }

    pub(super) fn preflight_reconcile_plan(
        &self,
        plan: &ReconcilePlan,
    ) -> Result<(), ReconcilePlanError> {
        plan.validate_final_orders()?;
        let mut old_uses = HashSet::new();
        let mut node_uses = HashMap::new();
        self.preflight_planned_node(&plan.root, &mut old_uses, &mut node_uses)?;
        for parent in &plan.parents {
            for identity in &parent.removals {
                let node_id = self.vnode_map.get(identity).copied().ok_or_else(|| {
                    ReconcilePlanError::MissingExistingNodeId {
                        identity: identity.diagnostic(),
                    }
                })?;
                if !old_uses.insert(identity.clone()) {
                    return Err(ReconcilePlanError::DuplicateExistingIdentityUse {
                        identity: identity.diagnostic(),
                    });
                }
                if let Some(first_identity) = node_uses.insert(node_id, identity.clone()) {
                    return Err(ReconcilePlanError::DuplicateExistingNodeIdUse {
                        first_identity: first_identity.diagnostic(),
                        second_identity: identity.diagnostic(),
                    });
                }
            }
        }
        Ok(())
    }

    fn preflight_planned_node(
        &self,
        planned: &PlannedNode,
        old_uses: &mut HashSet<ScopedNodeIdentity>,
        node_uses: &mut HashMap<NodeId, ScopedNodeIdentity>,
    ) -> Result<(), ReconcilePlanError> {
        let needs_existing = matches!(
            planned.action,
            PlannedNodeAction::Reuse | PlannedNodeAction::Update | PlannedNodeAction::Replace
        );
        if needs_existing {
            let old_identity = planned.old_identity.as_ref().ok_or_else(|| {
                ReconcilePlanError::MissingExistingNodeId {
                    identity: planned.identity.diagnostic(),
                }
            })?;
            if !old_uses.insert(old_identity.clone()) {
                return Err(ReconcilePlanError::DuplicateExistingIdentityUse {
                    identity: old_identity.diagnostic(),
                });
            }
            let node_id = self.vnode_map.get(old_identity).copied().ok_or_else(|| {
                ReconcilePlanError::MissingExistingNodeId {
                    identity: old_identity.diagnostic(),
                }
            })?;
            if let Some(first_identity) = node_uses.insert(node_id, old_identity.clone()) {
                return Err(ReconcilePlanError::DuplicateExistingNodeIdUse {
                    first_identity: first_identity.diagnostic(),
                    second_identity: old_identity.diagnostic(),
                });
            }
            if !self.preflight_style_exists(node_id) {
                return Err(ReconcilePlanError::MissingExistingNodeId {
                    identity: old_identity.diagnostic(),
                });
            }
        }
        for child in &planned.children {
            self.preflight_planned_node(child, old_uses, node_uses)?;
        }
        Ok(())
    }

    pub(super) fn validate_committed_plan(
        &self,
        plan: &ReconcilePlan,
    ) -> Result<(), ReconcilePlanError> {
        let mut planned_identities = HashSet::new();
        let mut planned_node_ids = HashSet::new();
        self.validate_committed_node(&plan.root, &mut planned_identities, &mut planned_node_ids)?;
        if planned_identities.len() != self.vnode_map.len()
            || planned_identities.len() != self.vnode_legacy_keys.len()
        {
            return Err(ReconcilePlanError::CommittedTreeMismatch {
                reason: "identity map does not exactly match the committed VNode",
            });
        }
        if self.taffy.total_node_count() != planned_identities.len() {
            return Err(ReconcilePlanError::CommittedTreeMismatch {
                reason: "layout tree contains unmapped or missing nodes",
            });
        }
        if planned_node_ids.len() != planned_identities.len() {
            return Err(ReconcilePlanError::CommittedTreeMismatch {
                reason: "multiple scoped identities reference one layout node",
            });
        }
        let expected_root = self.vnode_map.get(&ScopedNodeIdentity::Root).copied();
        if expected_root != self.root_node {
            return Err(ReconcilePlanError::CommittedTreeMismatch {
                reason: "root identity and root layout node disagree",
            });
        }
        Ok(())
    }

    fn validate_committed_node(
        &self,
        planned: &PlannedNode,
        identities: &mut HashSet<ScopedNodeIdentity>,
        node_ids: &mut HashSet<NodeId>,
    ) -> Result<(), ReconcilePlanError> {
        if !identities.insert(planned.identity.clone()) {
            return Err(ReconcilePlanError::CommittedTreeMismatch {
                reason: "committed VNode contains a duplicate scoped identity",
            });
        }
        let node_id = self
            .vnode_map
            .get(&planned.identity)
            .copied()
            .ok_or_else(|| ReconcilePlanError::MissingExistingNodeId {
                identity: planned.identity.diagnostic(),
            })?;
        node_ids.insert(node_id);
        if self.vnode_legacy_keys.get(&planned.identity) != Some(&planned.legacy_key) {
            return Err(ReconcilePlanError::CommittedTreeMismatch {
                reason: "legacy compatibility projection is stale",
            });
        }
        let expected_children: Option<Vec<_>> = planned
            .children
            .iter()
            .map(|child| self.vnode_map.get(&child.identity).copied())
            .collect();
        #[cfg(test)]
        let children_result =
            if take_incremental_order_fault(IncrementalOrderFault::ValidateChildren) {
                Err(())
            } else {
                Ok(self
                    .taffy
                    .children(node_id)
                    .expect("validated committed node remains readable"))
            };
        #[cfg(not(test))]
        let children_result = self.taffy.children(node_id);
        let actual_children =
            children_result.map_err(|_| ReconcilePlanError::CommittedTreeMismatch {
                reason: "mapped layout node is no longer in the Taffy tree",
            })?;
        if expected_children.as_deref() != Some(actual_children.as_slice()) {
            return Err(ReconcilePlanError::CommittedTreeMismatch {
                reason: "committed child order differs from the committed VNode",
            });
        }
        for child in &planned.children {
            self.validate_committed_node(child, identities, node_ids)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn commit_planned_children(
        &mut self,
        planned: &PlannedNode,
        target_map: &HashMap<ScopedNodeIdentity, NodeId>,
    ) -> Result<(), PatchError> {
        self.commit_planned_children_for_plan(planned, target_map, &[])
            .map_err(|error| error.patch)
    }

    pub(super) fn commit_planned_children_for_plan(
        &mut self,
        planned: &PlannedNode,
        target_map: &HashMap<ScopedNodeIdentity, NodeId>,
        patches: &[Patch],
    ) -> Result<(), ApplyPlanError> {
        self.commit_planned_children_recursive(planned, target_map, patches, None, None)
    }

    fn commit_planned_children_recursive(
        &mut self,
        planned: &PlannedNode,
        target_map: &HashMap<ScopedNodeIdentity, NodeId>,
        patches: &[Patch],
        parent_address: Option<crate::core::NodeKey>,
        enclosing_origin: Option<(usize, PatchKind, crate::core::NodeKey)>,
    ) -> Result<(), ApplyPlanError> {
        let node_origin = self
            .planned_node_origin(planned, parent_address, patches)
            .or(enclosing_origin);
        let planned_address = planned.identity.scoped_patch_address(planned.legacy_key);
        for child in &planned.children {
            self.commit_planned_children_recursive(
                child,
                target_map,
                patches,
                Some(planned_address),
                node_origin,
            )?;
        }
        let node_id = target_map[&planned.identity];
        let child_ids: Vec<_> = planned
            .children
            .iter()
            .map(|child| target_map[&child.identity])
            .collect();
        let origin = if matches!(
            planned.action,
            PlannedNodeAction::Create | PlannedNodeAction::Replace
        ) {
            node_origin
        } else {
            self.structural_child_origin(planned, patches)
        };
        let (patch_index, kind, key) =
            origin.unwrap_or((usize::MAX, PatchKind::Reorder, planned_address));
        let patch_index = (patch_index != usize::MAX).then_some(patch_index);
        let current_children = self.taffy.children(node_id).map_err(|source| {
            taffy_apply_error(
                kind,
                key,
                PatchFailure::TreeRejected,
                PatchStage::SetChildren,
                source,
                patch_index,
            )
        })?;
        if current_children == child_ids {
            return Ok(());
        }
        #[cfg(test)]
        let commit_result = if take_incremental_order_fault(IncrementalOrderFault::CommitChildren) {
            Err(taffy::TaffyError::InvalidInputNode(node_id))
        } else {
            self.taffy.set_children(node_id, &child_ids)
        };
        #[cfg(not(test))]
        let commit_result = self.taffy.set_children(node_id, &child_ids);
        commit_result.map_err(|source| {
            taffy_apply_error(
                kind,
                key,
                PatchFailure::TreeRejected,
                PatchStage::SetChildren,
                source,
                patch_index,
            )
        })
    }

    fn planned_node_origin(
        &self,
        planned: &PlannedNode,
        parent_address: Option<crate::core::NodeKey>,
        patches: &[Patch],
    ) -> Option<(usize, PatchKind, crate::core::NodeKey)> {
        match planned.action {
            PlannedNodeAction::Create => {
                patches
                    .iter()
                    .enumerate()
                    .find_map(|(patch_index, patch)| match patch {
                        Patch::Create { key, parent, .. }
                            if key.identity() == planned.legacy_key.identity()
                                && parent_address.is_some_and(|address| {
                                    address.identity() == parent.identity()
                                }) =>
                        {
                            Some((patch_index, PatchKind::Create, *parent))
                        }
                        _ => None,
                    })
            }
            PlannedNodeAction::Replace => {
                let old_identity = planned.old_identity.as_ref()?;
                let legacy_key = self.vnode_legacy_keys.get(old_identity).copied()?;
                let address = old_identity.scoped_patch_address(legacy_key);
                patches
                    .iter()
                    .enumerate()
                    .find_map(|(patch_index, patch)| match patch {
                        Patch::Replace { key, .. }
                            if key.identity() == address.identity()
                                || (!ScopedNodeIdentity::is_scoped_patch_address(*key)
                                    && key.identity() == legacy_key.identity()) =>
                        {
                            Some((patch_index, PatchKind::Replace, address))
                        }
                        _ => None,
                    })
            }
            PlannedNodeAction::Reuse | PlannedNodeAction::Update => None,
        }
    }

    fn structural_child_origin(
        &self,
        planned: &PlannedNode,
        patches: &[Patch],
    ) -> Option<(usize, PatchKind, crate::core::NodeKey)> {
        let parent_address = planned.identity.scoped_patch_address(planned.legacy_key);
        patches
            .iter()
            .enumerate()
            .find_map(|(patch_index, patch)| match patch {
                Patch::Create { parent, .. } if parent.identity() == parent_address.identity() => {
                    Some((patch_index, PatchKind::Create, *parent))
                }
                Patch::Reorder { parent, .. } if parent.identity() == parent_address.identity() => {
                    Some((patch_index, PatchKind::Reorder, *parent))
                }
                Patch::Remove { key } if self.patch_target_is_child_of(*key, &planned.identity) => {
                    Some((patch_index, PatchKind::Remove, *key))
                }
                Patch::Replace { key, .. }
                    if self.patch_target_is_child_of(*key, &planned.identity) =>
                {
                    Some((patch_index, PatchKind::Replace, *key))
                }
                _ => None,
            })
    }

    fn patch_target_is_child_of(
        &self,
        key: crate::core::NodeKey,
        parent: &ScopedNodeIdentity,
    ) -> bool {
        self.vnode_legacy_keys.iter().any(|(identity, legacy_key)| {
            identity.parent() == Some(parent)
                && (identity.scoped_patch_address(*legacy_key).identity() == key.identity()
                    || (!ScopedNodeIdentity::is_scoped_patch_address(key)
                        && legacy_key.identity() == key.identity()))
        })
    }

    #[cfg(test)]
    pub(super) fn check_reconcile_postconditions(
        &self,
        planned: &PlannedNode,
    ) -> Result<(), PatchError> {
        self.check_reconcile_postconditions_for_plan(planned, &[])
            .map_err(|error| error.patch)
    }

    pub(super) fn check_reconcile_postconditions_for_plan(
        &self,
        planned: &PlannedNode,
        patches: &[Patch],
    ) -> Result<(), ApplyPlanError> {
        let patch_key = planned.identity.scoped_patch_address(planned.legacy_key);
        let patch_index = patches.iter().position(|patch| {
            matches!(patch, Patch::Reorder { parent, .. } if parent.identity() == patch_key.identity())
        });
        let fail = || {
            apply_error(
                PatchKind::Reorder,
                patch_key,
                PatchFailure::PostconditionViolated,
                PatchStage::VerifyPostcondition,
            )
            .with_patch_index(patch_index)
        };
        let node_id = self
            .vnode_map
            .get(&planned.identity)
            .copied()
            .ok_or_else(fail)?;
        #[cfg(test)]
        let children_result =
            if take_incremental_order_fault(IncrementalOrderFault::PostconditionChildren) {
                Err(taffy::TaffyError::InvalidInputNode(node_id))
            } else {
                self.taffy.children(node_id)
            };
        #[cfg(not(test))]
        let children_result = self.taffy.children(node_id);
        let actual = children_result.map_err(|source| {
            taffy_apply_error(
                PatchKind::Reorder,
                patch_key,
                PatchFailure::PostconditionViolated,
                PatchStage::VerifyPostcondition,
                source,
                patch_index,
            )
        })?;
        let expected: Option<Vec<_>> = planned
            .children
            .iter()
            .map(|child| self.vnode_map.get(&child.identity).copied())
            .collect();
        if expected.as_deref() != Some(actual.as_slice()) {
            return Err(fail());
        }
        for child in &planned.children {
            self.check_reconcile_postconditions_for_plan(child, patches)?;
        }
        Ok(())
    }
}

#[cfg(test)]
pub(super) mod tests;
