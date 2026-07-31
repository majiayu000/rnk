//! Final-order preflight, committed-tree validation, and exact child-order commit.

use std::collections::{HashMap, HashSet};

use taffy::NodeId;

use crate::reconciler::{
    PlannedNode, PlannedNodeAction, ReconcilePlan, ReconcilePlanError, ScopedNodeIdentity,
};

use super::LayoutEngine;
use super::patch_error::{PatchError, PatchFailure, PatchKind};

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IncrementalOrderFault {
    PreflightStyle,
    ValidateChildren,
    CommitChildren,
    PostconditionChildren,
}

#[cfg(test)]
thread_local! {
    static INCREMENTAL_ORDER_FAULT: std::cell::Cell<Option<IncrementalOrderFault>> = const {
        std::cell::Cell::new(None)
    };
}

#[cfg(test)]
fn set_incremental_order_fault(fault: IncrementalOrderFault) {
    INCREMENTAL_ORDER_FAULT.with(|slot| slot.set(Some(fault)));
}

#[cfg(test)]
fn take_incremental_order_fault(fault: IncrementalOrderFault) -> bool {
    INCREMENTAL_ORDER_FAULT.with(|slot| {
        if slot.get() == Some(fault) {
            slot.set(None);
            true
        } else {
            false
        }
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

    pub(super) fn commit_planned_children(
        &mut self,
        planned: &PlannedNode,
        target_map: &HashMap<ScopedNodeIdentity, NodeId>,
    ) -> Result<(), PatchError> {
        for child in &planned.children {
            self.commit_planned_children(child, target_map)?;
        }
        let node_id = target_map[&planned.identity];
        let child_ids: Vec<_> = planned
            .children
            .iter()
            .map(|child| target_map[&child.identity])
            .collect();
        #[cfg(test)]
        let commit_result = if take_incremental_order_fault(IncrementalOrderFault::CommitChildren) {
            Err(())
        } else {
            self.taffy
                .set_children(node_id, &child_ids)
                .expect("validated target nodes remain writable");
            Ok(())
        };
        #[cfg(not(test))]
        let commit_result = self.taffy.set_children(node_id, &child_ids);
        commit_result.map_err(|_| {
            PatchError::new(
                PatchKind::Reorder,
                planned.legacy_key,
                PatchFailure::TreeRejected,
            )
        })
    }

    pub(super) fn check_reconcile_postconditions(
        &self,
        planned: &PlannedNode,
    ) -> Result<(), PatchError> {
        let fail = || {
            PatchError::new(
                PatchKind::Reorder,
                planned.legacy_key,
                PatchFailure::PostconditionViolated,
            )
        };
        let node_id = self
            .vnode_map
            .get(&planned.identity)
            .copied()
            .ok_or_else(fail)?;
        #[cfg(test)]
        let children_result =
            if take_incremental_order_fault(IncrementalOrderFault::PostconditionChildren) {
                Err(())
            } else {
                Ok(self
                    .taffy
                    .children(node_id)
                    .expect("committed target node remains readable"))
            };
        #[cfg(not(test))]
        let children_result = self.taffy.children(node_id);
        let actual = children_result.map_err(|_| fail())?;
        let expected: Option<Vec<_>> = planned
            .children
            .iter()
            .map(|child| self.vnode_map.get(&child.identity).copied())
            .collect();
        if expected.as_deref() != Some(actual.as_slice()) {
            return Err(fail());
        }
        for child in &planned.children {
            self.check_reconcile_postconditions(child)?;
        }
        Ok(())
    }
}

#[cfg(test)]
pub(super) mod tests;
