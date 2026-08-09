//! Deterministic same-parent structural patch scheduling.

use std::collections::HashMap;

use crate::core::NodeKey;
use crate::reconciler::{Patch, ReconcilePlanError, ScopedNodeIdentity};

use super::{PlannedNode, PlannedNodeAction, ValidatedNode};

pub(super) fn reorder_keys(
    target: &[ValidatedNode<'_>],
    planned: &[PlannedNode],
    scoped_patch_addresses: bool,
) -> Vec<NodeKey> {
    target
        .iter()
        .zip(planned)
        .map(|(target, planned)| {
            let identity = &target.identity;
            if scoped_patch_addresses
                && identity.legacy_key.user_key.is_none()
                && matches!(
                    planned.action,
                    PlannedNodeAction::Reuse | PlannedNodeAction::Update
                )
            {
                identity.scoped.scoped_patch_address(identity.legacy_key)
            } else {
                identity.legacy_key
            }
        })
        .collect()
}

pub(super) fn schedule_structural_patches(
    parent: &ScopedNodeIdentity,
    old_children: &[ValidatedNode<'_>],
    planned_children: &[PlannedNode],
    require_exact_order: bool,
    creates: Vec<Patch>,
    removals: Vec<(usize, Patch)>,
) -> Result<Vec<Patch>, ReconcilePlanError> {
    let old_count = old_children.len();
    let mut generation_keys: Vec<_> = old_children
        .iter()
        .map(|child| child.identity.legacy_key)
        .collect();
    generation_keys.extend(creates.iter().map(create_key));
    let target_generations = target_generations(old_children, planned_children, creates.len());
    let mut pending_creates: Vec<Option<Patch>> = creates.into_iter().map(Some).collect();
    let mut pending_removals: HashMap<_, _> = removals.into_iter().collect();
    let mut current: Vec<_> = (0..old_count).collect();
    let mut scheduled = Vec::with_capacity(pending_creates.len() + pending_removals.len());

    while pending_creates.iter().any(Option::is_some) || !pending_removals.is_empty() {
        let ready_create = pending_creates
            .iter()
            .enumerate()
            .filter_map(|(create_index, patch)| {
                let key = create_key(patch.as_ref()?);
                create_is_ready(key, &current, &generation_keys)
                    .then_some((key.index, create_index))
            })
            .max();
        if let Some((_, create_index)) = ready_create {
            let generation = old_count + create_index;
            current.insert(generation_keys[generation].index, generation);
            scheduled.push(
                pending_creates[create_index]
                    .take()
                    .expect("ready create remains pending"),
            );
            continue;
        }
        let Some(old_index) = current
            .iter()
            .copied()
            .find(|generation| pending_removals.contains_key(generation))
        else {
            return Err(schedule_error(parent));
        };
        let current_index = current
            .iter()
            .position(|generation| *generation == old_index)
            .expect("selected removal remains current");
        current.remove(current_index);
        scheduled.push(
            pending_removals
                .remove(&old_index)
                .expect("selected removal remains pending"),
        );
    }
    if require_exact_order && current != target_generations {
        return Err(schedule_error(parent));
    }
    Ok(scheduled)
}

fn target_generations(
    old_children: &[ValidatedNode<'_>],
    planned_children: &[PlannedNode],
    create_count: usize,
) -> Vec<usize> {
    let old_by_identity: HashMap<_, _> = old_children
        .iter()
        .enumerate()
        .map(|(index, child)| (&child.identity.scoped, index))
        .collect();
    let mut next_create = 0usize;
    let generations = planned_children
        .iter()
        .map(|child| {
            child
                .old_identity
                .as_ref()
                .and_then(|identity| old_by_identity.get(identity).copied())
                .unwrap_or_else(|| {
                    let generation = old_children.len() + next_create;
                    next_create += 1;
                    generation
                })
        })
        .collect();
    debug_assert_eq!(next_create, create_count);
    generations
}

fn schedule_error(parent: &ScopedNodeIdentity) -> ReconcilePlanError {
    ReconcilePlanError::PlannedChildrenMismatch {
        parent_scope: parent.diagnostic(),
    }
}

fn create_key(patch: &Patch) -> NodeKey {
    let Patch::Create { key, .. } = patch else {
        unreachable!("the create queue only stores create patches");
    };
    *key
}

fn create_is_ready(key: NodeKey, current: &[usize], generation_keys: &[NodeKey]) -> bool {
    if key.index > current.len() {
        return false;
    }
    if key.user_key.is_some() {
        return current
            .iter()
            .all(|generation| generation_keys[*generation].identity() != key.identity());
    }
    current.get(key.index).is_none_or(|generation| {
        let occupant = generation_keys[*generation];
        occupant.user_key.is_some() || occupant.type_id != key.type_id
    })
}
