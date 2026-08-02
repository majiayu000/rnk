//! Scoped identity indexes and checked compatibility projections.

mod direct_helpers;
mod resolution;

use std::collections::{HashMap, HashSet};

use crate::core::{NodeKey, VNode};
use crate::reconciler::{
    Patch, PlannedNode, PlannedNodeAction, ReconcilePlan, ReconcilePlanError, ScopedIdentityArena,
    ScopedNodeIdentity, SiblingIdentity, insert_composite_projection, plan_diff_in,
    plan_initial_tree_in,
};

use super::LayoutEngine;
use super::patch_error::{
    DirectPatchPreflightCause, DirectPatchPreflightError, IncrementalPatchKind,
};
use direct_helpers::*;
use resolution::*;

pub(super) struct ResolvedDirectPatchBatch {
    pub(super) target: VNode,
    pub(super) plan: ReconcilePlan,
    pub(super) steps: Vec<(usize, ReconcilePlan, HashMap<SiblingIdentity, usize>)>,
    pub(super) locators: Vec<ResolvedPatchLocator>,
}

#[derive(Clone, Copy)]
pub(super) struct ResolvedPatchLocator {
    pub(super) key: Option<NodeKey>,
    pub(super) parent: Option<NodeKey>,
}

#[derive(Clone)]
struct VirtualNodeEntry {
    identity: ScopedNodeIdentity,
    legacy_key: NodeKey,
    raw_key: NodeKey,
    path: Vec<usize>,
}

fn patch_locator(patch: &Patch) -> (IncrementalPatchKind, Option<NodeKey>, Option<NodeKey>) {
    match patch {
        Patch::Create { key, parent, .. } => {
            (IncrementalPatchKind::Create, Some(*key), Some(*parent))
        }
        Patch::Update { key, .. } => (IncrementalPatchKind::Update, Some(*key), None),
        Patch::Remove { key } => (IncrementalPatchKind::Remove, Some(*key), None),
        Patch::Replace { key, .. } => (IncrementalPatchKind::Replace, Some(*key), None),
        Patch::Reorder { parent, .. } => (IncrementalPatchKind::Reorder, None, Some(*parent)),
    }
}

fn preflight_error(
    patch_index: usize,
    patch: &Patch,
    source: DirectPatchPreflightCause,
) -> DirectPatchPreflightError {
    let (kind, key, parent) = patch_locator(patch);
    preflight_error_with_locator(patch_index, kind, key, parent, source)
}

fn preflight_error_with_locator(
    patch_index: usize,
    kind: IncrementalPatchKind,
    key: Option<NodeKey>,
    parent: Option<NodeKey>,
    source: DirectPatchPreflightCause,
) -> DirectPatchPreflightError {
    DirectPatchPreflightError {
        patch_index,
        kind,
        key,
        parent,
        source: Box::new(source),
    }
}

fn resolution_error(
    patch_index: usize,
    patch: &Patch,
    error: ResolutionFailure,
) -> DirectPatchPreflightError {
    let (kind, key, parent) = patch_locator(patch);
    preflight_error_with_locator(
        patch_index,
        kind,
        key,
        error.parent.or(parent),
        error.source,
    )
}

fn resolved_parent_locator(
    index: &[VirtualNodeEntry],
    entry: &VirtualNodeEntry,
) -> Option<NodeKey> {
    let parent = entry.identity.parent()?;
    index
        .iter()
        .find(|candidate| &candidate.identity == parent)
        .map(|parent| parent.identity.scoped_patch_address(parent.legacy_key))
}

fn resolved_entry_locator(
    index: &[VirtualNodeEntry],
    patch: &Patch,
    entry: &VirtualNodeEntry,
) -> ResolvedPatchLocator {
    let (_, key, parent) = patch_locator(patch);
    ResolvedPatchLocator {
        key,
        parent: resolved_parent_locator(index, entry).or(parent),
    }
}

fn virtual_index(target: &VNode) -> Result<Vec<VirtualNodeEntry>, ReconcilePlanError> {
    let mut arena = ScopedIdentityArena::default();
    let plan = plan_initial_tree_in(target, &mut arena)?;
    let mut entries = Vec::with_capacity(target.node_count());
    collect_virtual_entries(&plan.root, &mut Vec::new(), &mut entries);
    let mut projections = HashMap::with_capacity(entries.len());
    for entry in &entries {
        insert_composite_projection(&mut projections, &entry.identity, entry.legacy_key).map_err(
            |(identity, first_scope)| ReconcilePlanError::CompositeIdentityCollision {
                identity,
                first_scope: first_scope.diagnostic(),
                second_scope: entry.identity.diagnostic(),
            },
        )?;
    }
    Ok(entries)
}

fn collect_virtual_entries(
    planned: &PlannedNode,
    path: &mut Vec<usize>,
    entries: &mut Vec<VirtualNodeEntry>,
) {
    entries.push(VirtualNodeEntry {
        identity: planned.identity.clone(),
        legacy_key: planned.legacy_key,
        raw_key: planned.vnode.key,
        path: path.clone(),
    });
    for (index, child) in planned.children.iter().enumerate() {
        path.push(index);
        collect_virtual_entries(child, path, entries);
        path.pop();
    }
}

fn record_step_preflight_origins(
    plan: &ReconcilePlan,
    patch_index: usize,
    origins: &mut HashMap<String, usize>,
) {
    fn visit(planned: &PlannedNode, patch_index: usize, origins: &mut HashMap<String, usize>) {
        if planned.action != PlannedNodeAction::Reuse {
            let identity = planned.old_identity.as_ref().unwrap_or(&planned.identity);
            origins.insert(identity.diagnostic(), patch_index);
        }
        for child in &planned.children {
            visit(child, patch_index, origins);
        }
    }

    visit(&plan.root, patch_index, origins);
    for parent in &plan.parents {
        for identity in &parent.removals {
            origins.insert(identity.diagnostic(), patch_index);
        }
    }
}

fn final_preflight_origin(
    source: &ReconcilePlanError,
    origins: &HashMap<String, usize>,
) -> Option<usize> {
    match source {
        ReconcilePlanError::MissingExistingNodeId { identity }
        | ReconcilePlanError::DuplicateExistingIdentityUse { identity } => {
            origins.get(identity).copied()
        }
        ReconcilePlanError::DuplicateExistingNodeIdUse {
            first_identity,
            second_identity,
        } => origins
            .get(first_identity)
            .into_iter()
            .chain(origins.get(second_identity))
            .copied()
            .min(),
        _ => None,
    }
}

impl LayoutEngine {
    pub(super) fn preflight_direct_patch_batch(
        &self,
        patches: &[Patch],
    ) -> Result<ResolvedDirectPatchBatch, DirectPatchPreflightError> {
        let Some(committed) = self.committed_vnode.as_ref() else {
            let (kind, key, parent) = patch_locator(&patches[0]);
            return Err(DirectPatchPreflightError {
                patch_index: 0,
                kind,
                key,
                parent,
                source: Box::new(if parent.is_some() {
                    DirectPatchPreflightCause::MissingParent
                } else {
                    DirectPatchPreflightCause::MissingTarget
                }),
            });
        };
        let mut arena = ScopedIdentityArena::seeded(self.vnode_map.keys());
        let committed_plan = plan_diff_in(committed, committed, &mut arena).map_err(|source| {
            preflight_error(0, &patches[0], DirectPatchPreflightCause::Identity(source))
        })?;
        self.validate_committed_plan(&committed_plan)
            .map_err(|source| {
                preflight_error(0, &patches[0], DirectPatchPreflightCause::Identity(source))
            })?;

        let mut target = committed.clone();
        let initial_index = virtual_index(&target).map_err(|source| {
            preflight_error(0, &patches[0], DirectPatchPreflightCause::Identity(source))
        })?;
        let mut aliases = VirtualAliases::from_index(&initial_index);
        let mut tombstones = Vec::new();
        let mut forced_replacements = Vec::new();
        let mut steps = Vec::with_capacity(patches.len());
        let mut locators = Vec::with_capacity(patches.len());
        let mut final_plan_error_origin = None;
        let mut preflight_origins = HashMap::new();
        for (patch_index, patch) in patches.iter().enumerate() {
            let index = virtual_index(&target).map_err(|source| {
                preflight_error(
                    patch_index,
                    patch,
                    DirectPatchPreflightCause::Identity(source),
                )
            })?;
            let previous_target = target.clone();
            let mut replacement_path = None;
            let mut new_subtree = None;
            let mut error_origins = HashMap::new();
            let (_, default_key, default_parent) = patch_locator(patch);
            let mut locator = ResolvedPatchLocator {
                key: default_key,
                parent: default_parent,
            };
            match patch {
                Patch::Create {
                    key,
                    parent,
                    props,
                    node,
                } => {
                    if props != &node.props {
                        return Err(preflight_error(
                            patch_index,
                            patch,
                            DirectPatchPreflightCause::PayloadMismatch,
                        ));
                    }
                    if let Some(conflicting_key) = first_subtree_duplicate(node) {
                        return Err(preflight_error(
                            patch_index,
                            patch,
                            DirectPatchPreflightCause::SubtreeCollision { conflicting_key },
                        ));
                    }
                    let parent_entry =
                        resolve_virtual(&index, *parent, LookupRole::Parent, &tombstones, &aliases)
                            .map_err(|error| resolution_error(patch_index, patch, error))?;
                    locator.parent = Some(
                        parent_entry
                            .identity
                            .scoped_patch_address(parent_entry.legacy_key),
                    );
                    let parent_node = vnode_at(&target, &parent_entry.path);
                    let canonical_key = props
                        .key
                        .as_deref()
                        .map(|exact| NodeKey::with_key(exact, node.node_type.type_id(), key.index));
                    let slot = if canonical_key
                        .is_some_and(|canonical| canonical.identity() != key.identity())
                    {
                        parent_node.children.len()
                    } else {
                        key.index
                    };
                    if slot > parent_node.children.len() {
                        return Err(preflight_error(
                            patch_index,
                            patch,
                            DirectPatchPreflightCause::PositionalIdentityShift,
                        ));
                    }
                    let (payload_identity, payload_key) =
                        canonical_payload_identity(&index, node, slot, &parent_entry.identity)
                            .map_err(|source| {
                                preflight_error(
                                    patch_index,
                                    patch,
                                    DirectPatchPreflightCause::Identity(source),
                                )
                            })?;
                    if key.identity() != payload_key.identity()
                        && key.identity() != node.key.identity()
                    {
                        return Err(preflight_error(
                            patch_index,
                            patch,
                            DirectPatchPreflightCause::PayloadMismatch,
                        ));
                    }
                    let prospective_match = index
                        .iter()
                        .find(|entry| entry.identity == payload_identity);
                    if sibling_collision(&index, &parent_entry.identity, None, payload_key)
                        .is_some()
                        || prospective_match.is_some()
                    {
                        return Err(preflight_error(
                            patch_index,
                            patch,
                            DirectPatchPreflightCause::AlreadyExists,
                        ));
                    }
                    aliases.insert_child(&parent_entry.path, slot);
                    vnode_at_mut(&mut target, &parent_entry.path)
                        .children
                        .insert(slot, node.clone());
                    let mut created_path = parent_entry.path.clone();
                    created_path.push(slot);
                    new_subtree = Some((created_path, node));
                    normalize_child_indices(&mut target);
                }
                Patch::Update {
                    key,
                    old_props,
                    new_props,
                } => {
                    let entry =
                        resolve_virtual(&index, *key, LookupRole::Target, &tombstones, &aliases)
                            .map_err(|error| resolution_error(patch_index, patch, error))?;
                    locator = resolved_entry_locator(&index, patch, &entry);
                    let target_node = vnode_at_mut(&mut target, &entry.path);
                    if &target_node.props != old_props {
                        return Err(preflight_error_with_locator(
                            patch_index,
                            IncrementalPatchKind::Update,
                            locator.key,
                            locator.parent,
                            DirectPatchPreflightCause::StaleProps,
                        ));
                    }
                    if !entry.path.is_empty() && old_props.key != new_props.key {
                        return Err(preflight_error_with_locator(
                            patch_index,
                            IncrementalPatchKind::Update,
                            locator.key,
                            locator.parent,
                            DirectPatchPreflightCause::PayloadMismatch,
                        ));
                    }
                    target_node.props = new_props.clone();
                }
                Patch::Remove { key } => {
                    let entry =
                        resolve_virtual(&index, *key, LookupRole::Target, &tombstones, &aliases)
                            .map_err(|error| resolution_error(patch_index, patch, error))?;
                    locator = resolved_entry_locator(&index, patch, &entry);
                    let Some((&child_index, parent_path)) = entry.path.split_last() else {
                        return Err(preflight_error(
                            patch_index,
                            patch,
                            DirectPatchPreflightCause::RootMutation,
                        ));
                    };
                    record_tombstones(
                        &index,
                        &entry.path,
                        patch_index,
                        TombstoneKind::Removed,
                        &mut tombstones,
                        &aliases,
                    );
                    record_error_origins(&index, &entry.path, patch_index, &mut error_origins);
                    aliases.remove_subtree(&entry.path);
                    vnode_at_mut(&mut target, parent_path)
                        .children
                        .remove(child_index);
                    normalize_child_indices(&mut target);
                }
                Patch::Replace {
                    key,
                    new_props,
                    node,
                } => {
                    let entry =
                        resolve_virtual(&index, *key, LookupRole::Target, &tombstones, &aliases)
                            .map_err(|error| resolution_error(patch_index, patch, error))?;
                    locator = resolved_entry_locator(&index, patch, &entry);
                    if new_props != &node.props {
                        return Err(preflight_error_with_locator(
                            patch_index,
                            IncrementalPatchKind::Replace,
                            locator.key,
                            locator.parent,
                            DirectPatchPreflightCause::PayloadMismatch,
                        ));
                    }
                    if let Some(conflicting_key) = first_subtree_duplicate(node) {
                        return Err(preflight_error_with_locator(
                            patch_index,
                            IncrementalPatchKind::Replace,
                            locator.key,
                            locator.parent,
                            DirectPatchPreflightCause::SubtreeCollision { conflicting_key },
                        ));
                    }
                    record_tombstones(
                        &index,
                        &entry.path,
                        patch_index,
                        TombstoneKind::Replaced,
                        &mut tombstones,
                        &aliases,
                    );
                    aliases.replace_subtree(&entry.path);
                    replacement_path = Some(entry.path.clone());
                    if entry.path.is_empty() {
                        target = node.clone();
                    } else {
                        let (&child_index, parent_path) = entry
                            .path
                            .split_last()
                            .expect("non-root path has a final index");
                        let parent_entry = index
                            .iter()
                            .find(|candidate| candidate.path == parent_path)
                            .expect("a non-root target has an indexed parent");
                        let (_payload_identity, payload_key) = canonical_payload_identity(
                            &index,
                            node,
                            child_index,
                            &parent_entry.identity,
                        )
                        .map_err(|source| {
                            preflight_error_with_locator(
                                patch_index,
                                IncrementalPatchKind::Replace,
                                locator.key,
                                locator.parent,
                                DirectPatchPreflightCause::Identity(source),
                            )
                        })?;
                        if let Some(conflicting_key) = sibling_collision(
                            &index,
                            &parent_entry.identity,
                            Some(&entry.path),
                            payload_key,
                        ) {
                            return Err(preflight_error_with_locator(
                                patch_index,
                                IncrementalPatchKind::Replace,
                                locator.key,
                                locator.parent,
                                DirectPatchPreflightCause::SubtreeCollision { conflicting_key },
                            ));
                        }
                        vnode_at_mut(&mut target, parent_path).children[child_index] = node.clone();
                    }
                    new_subtree = Some((entry.path.clone(), node));
                    normalize_child_indices(&mut target);
                }
                Patch::Reorder { parent, order } => {
                    let parent_entry =
                        resolve_virtual(&index, *parent, LookupRole::Parent, &tombstones, &aliases)
                            .map_err(|error| resolution_error(patch_index, patch, error))?;
                    locator.parent = Some(
                        parent_entry
                            .identity
                            .scoped_patch_address(parent_entry.legacy_key),
                    );
                    let current = vnode_at(&target, &parent_entry.path);
                    let child_count = current.children.len();
                    let mut requested = Vec::with_capacity(order.len());
                    let mut unique = HashSet::with_capacity(order.len());
                    for (to, key) in order.iter().enumerate() {
                        if to >= child_count || (key.user_key.is_none() && key.index >= child_count)
                        {
                            return Err(preflight_error_with_locator(
                                patch_index,
                                IncrementalPatchKind::Reorder,
                                Some(*key),
                                locator.parent,
                                DirectPatchPreflightCause::InvalidReorderMove {
                                    from: key.index,
                                    to,
                                    child_count,
                                },
                            ));
                        }
                        let child = resolve_virtual_child(
                            &index,
                            &parent_entry,
                            *key,
                            &tombstones,
                            &aliases,
                        )
                        .map_err(|source| {
                            preflight_error_with_locator(
                                patch_index,
                                IncrementalPatchKind::Reorder,
                                Some(*key),
                                locator.parent,
                                source,
                            )
                        })?;
                        let child_index = *child
                            .path
                            .last()
                            .expect("direct child path has a final index");
                        if !unique.insert(child_index) {
                            return Err(preflight_error_with_locator(
                                patch_index,
                                IncrementalPatchKind::Reorder,
                                Some(*key),
                                locator.parent,
                                DirectPatchPreflightCause::InvalidReorderMove {
                                    from: child_index,
                                    to,
                                    child_count,
                                },
                            ));
                        }
                        let positional_target = if ScopedNodeIdentity::is_scoped_patch_address(*key)
                        {
                            aliases.positional_target_index(&child.path)
                        } else if key.user_key.is_none() && child.legacy_key.user_key.is_none() {
                            Some(key.index)
                        } else {
                            None
                        };
                        requested.push((child_index, positional_target));
                    }
                    if order.len() != child_count {
                        let from = (0..child_count)
                            .find(|index| !unique.contains(index))
                            .unwrap_or(order.len());
                        return Err(preflight_error_with_locator(
                            patch_index,
                            IncrementalPatchKind::Reorder,
                            current.children.get(from).map(|child| child.key),
                            locator.parent,
                            DirectPatchPreflightCause::InvalidReorderMove {
                                from,
                                to: order.len(),
                                child_count,
                            },
                        ));
                    }
                    if let Some((offending_index, _)) =
                        requested
                            .iter()
                            .enumerate()
                            .find(|(new_index, (_, positional_target))| {
                                positional_target.is_some_and(|target| *new_index != target)
                            })
                    {
                        return Err(preflight_error_with_locator(
                            patch_index,
                            IncrementalPatchKind::Reorder,
                            Some(order[offending_index]),
                            locator.parent,
                            DirectPatchPreflightCause::InvalidReorderMove {
                                from: requested[offending_index].0,
                                to: offending_index,
                                child_count,
                            },
                        ));
                    }
                    let already_ordered = requested
                        .iter()
                        .map(|(child_index, _)| *child_index)
                        .eq(0..requested.len());
                    if !already_ordered {
                        let old_children = current.children.clone();
                        let old_order: Vec<_> = requested
                            .iter()
                            .map(|(child_index, _)| *child_index)
                            .collect();
                        aliases.reorder_children(&parent_entry.path, &old_order);
                        vnode_at_mut(&mut target, &parent_entry.path).children = requested
                            .into_iter()
                            .map(|(index, _)| old_children[index].clone())
                            .collect();
                        normalize_child_indices(&mut target);
                    }
                }
            }
            let validated_index = virtual_index(&target).map_err(|source| {
                preflight_error_with_locator(
                    patch_index,
                    patch_locator(patch).0,
                    locator.key,
                    locator.parent,
                    DirectPatchPreflightCause::Identity(source),
                )
            })?;
            if let Some((path, payload)) = &new_subtree {
                aliases.register_new_subtree(&validated_index, path, payload);
            }
            aliases.register_current(&validated_index);
            let replacement = if let Some(path) = replacement_path {
                let Some(replacement) = validated_index.iter().find(|entry| entry.path == path)
                else {
                    return Err(preflight_error(
                        patch_index,
                        patch,
                        DirectPatchPreflightCause::MissingTarget,
                    ));
                };
                forced_replacements.push(replacement.identity.clone());
                Some(replacement.identity.clone())
            } else {
                None
            };
            let mut arena = ScopedIdentityArena::seeded(self.vnode_map.keys());
            let mut step =
                plan_diff_in(&previous_target, &target, &mut arena).map_err(|source| {
                    preflight_error(
                        patch_index,
                        patch,
                        DirectPatchPreflightCause::Identity(source),
                    )
                })?;
            if let Some(replacement) = replacement {
                force_replace_subtree(&mut step.root, &replacement);
            }
            record_step_preflight_origins(&step, patch_index, &mut preflight_origins);
            let mut prefix_arena = ScopedIdentityArena::seeded(self.vnode_map.keys());
            if plan_diff_in(committed, &target, &mut prefix_arena).is_ok() {
                final_plan_error_origin = None;
            } else if final_plan_error_origin.is_none() {
                final_plan_error_origin = Some(patch_index);
            }
            steps.push((patch_index, step, error_origins));
            locators.push(locator);
        }

        let mut arena = ScopedIdentityArena::seeded(self.vnode_map.keys());
        let mut plan = plan_diff_in(committed, &target, &mut arena).map_err(|source| {
            let patch_index = final_plan_error_origin.unwrap_or(0);
            preflight_error(
                patch_index,
                &patches[patch_index],
                DirectPatchPreflightCause::Identity(source),
            )
        })?;
        for replacement in &forced_replacements {
            force_replace_subtree(&mut plan.root, replacement);
        }
        self.preflight_reconcile_plan(&plan).map_err(|source| {
            let patch_index = final_preflight_origin(&source, &preflight_origins).unwrap_or(0);
            preflight_error(
                patch_index,
                &patches[patch_index],
                DirectPatchPreflightCause::Identity(source),
            )
        })?;
        Ok(ResolvedDirectPatchBatch {
            target,
            plan,
            steps,
            locators,
        })
    }
}
