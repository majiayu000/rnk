//! Patch provenance for node-specific layout read-back failures.

use std::collections::HashMap;

use taffy::NodeId;

use crate::core::NodeKey;
use crate::reconciler::{Patch, PlannedNode, PlannedNodeAction, ReconcilePlan, ScopedNodeIdentity};

use super::super::LayoutEngine;

#[derive(Default)]
pub(in crate::layout::engine) struct LayoutPatchOrigins {
    by_identity: HashMap<ScopedNodeIdentity, usize>,
    parents_by_patch: HashMap<usize, NodeKey>,
}

impl LayoutPatchOrigins {
    pub(in crate::layout::engine) fn for_plan(engine: &LayoutEngine, plan: &ReconcilePlan) -> Self {
        let mut origins = Self::default();
        for (patch_index, patch) in plan.patches().iter().enumerate() {
            let key = match patch {
                Patch::Update { key, .. } | Patch::Remove { key } | Patch::Replace { key, .. } => {
                    *key
                }
                Patch::Create { .. } | Patch::Reorder { .. } => continue,
            };
            let Some(identity) =
                engine
                    .vnode_legacy_keys
                    .iter()
                    .find_map(|(identity, legacy_key)| {
                        let scoped = identity.scoped_patch_address(*legacy_key);
                        (scoped.identity() == key.identity()
                            || legacy_key.identity() == key.identity())
                        .then_some(identity)
                    })
            else {
                continue;
            };
            let Some(parent_identity) = identity.parent() else {
                continue;
            };
            let Some(parent_key) = engine.vnode_legacy_keys.get(parent_identity).copied() else {
                continue;
            };
            let parent = if ScopedNodeIdentity::is_scoped_patch_address(key) {
                parent_identity.scoped_patch_address(parent_key)
            } else {
                parent_key
            };
            origins.parents_by_patch.insert(patch_index, parent);
        }
        origins.record_planned(engine, &plan.root, plan.patches(), None, None);
        origins
    }

    pub(in crate::layout::engine) fn parent_for_patch(
        &self,
        patch_index: usize,
    ) -> Option<NodeKey> {
        self.parents_by_patch.get(&patch_index).copied()
    }

    pub(in crate::layout::engine) fn record_raw_step(
        &mut self,
        plan: &ReconcilePlan,
        patch_index: usize,
    ) {
        fn visit(
            planned: &PlannedNode,
            patch_index: usize,
            origins: &mut HashMap<ScopedNodeIdentity, usize>,
        ) {
            if planned.action != PlannedNodeAction::Reuse {
                origins.insert(planned.identity.clone(), patch_index);
            }
            for child in &planned.children {
                visit(child, patch_index, origins);
            }
        }

        visit(&plan.root, patch_index, &mut self.by_identity);
    }

    pub(in crate::layout::engine) fn locate(
        &self,
        engine: &LayoutEngine,
        node_id: Option<NodeId>,
    ) -> (Option<usize>, Option<NodeKey>) {
        let Some(node_id) = node_id else {
            return (None, None);
        };
        let Some(identity) = engine
            .vnode_map
            .iter()
            .find_map(|(identity, candidate)| (*candidate == node_id).then_some(identity))
        else {
            return (None, None);
        };
        (
            self.by_identity.get(identity).copied(),
            engine.vnode_legacy_keys.get(identity).copied(),
        )
    }

    fn record_planned(
        &mut self,
        engine: &LayoutEngine,
        planned: &PlannedNode,
        patches: &[Patch],
        parent_address: Option<NodeKey>,
        enclosing_origin: Option<usize>,
    ) {
        let direct_origin = match planned.action {
            PlannedNodeAction::Reuse => None,
            PlannedNodeAction::Update => old_patch_index(engine, planned, patches, false),
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
                            Some(patch_index)
                        }
                        _ => None,
                    })
            }
            PlannedNodeAction::Replace => old_patch_index(engine, planned, patches, true),
        };
        let origin = direct_origin.or(enclosing_origin);
        if planned.action != PlannedNodeAction::Reuse
            && let Some(patch_index) = origin
        {
            self.by_identity
                .insert(planned.identity.clone(), patch_index);
        }
        let child_parent = Some(planned.identity.scoped_patch_address(planned.legacy_key));
        let child_enclosing = matches!(
            planned.action,
            PlannedNodeAction::Create | PlannedNodeAction::Replace
        )
        .then_some(origin)
        .flatten();
        for child in &planned.children {
            self.record_planned(engine, child, patches, child_parent, child_enclosing);
        }
    }
}

fn old_patch_index(
    engine: &LayoutEngine,
    planned: &PlannedNode,
    patches: &[Patch],
    replace: bool,
) -> Option<usize> {
    let identity = planned.old_identity.as_ref()?;
    let legacy_key = engine.vnode_legacy_keys.get(identity).copied()?;
    let address = identity.scoped_patch_address(legacy_key);
    patches
        .iter()
        .enumerate()
        .find_map(|(patch_index, patch)| match patch {
            Patch::Update { key, .. }
                if !replace
                    && (key.identity() == address.identity()
                        || key.identity() == legacy_key.identity()) =>
            {
                Some(patch_index)
            }
            Patch::Replace { key, .. }
                if replace
                    && (key.identity() == address.identity()
                        || key.identity() == legacy_key.identity()) =>
            {
                Some(patch_index)
            }
            _ => None,
        })
}
