//! Checked Element conversion and scoped reconciliation-plan application.

use std::collections::{HashMap, HashSet};

use taffy::NodeId;

use crate::core::{Element, ElementId, ElementType, NodeKey, Props, VNode, VNodeType};
use crate::layout::TextFlowInput;
use crate::reconciler::{
    Patch, PlannedNode, PlannedNodeAction, ReconcilePlan, ReconcilePlanError, ScopedIdentityArena,
    ScopedNodeIdentity, compatibility_token_for_exact, resolve_child_identity,
};

use super::patch_error::{PatchError, PatchFailure, PatchKind, PatchStage, PatchTransactionCause};
use super::text_flow_bridge::{
    NodeContext, compatibility_text, input_from_element, input_from_vnode,
};
use super::{IncrementalInvariantError, LayoutEngine, normalized_taffy_style};

type PatchOrigin = (PatchKind, NodeKey, usize);

#[derive(Clone, Copy, Default)]
struct MaterializeContext {
    parent_address: Option<NodeKey>,
    enclosing_origin: Option<PatchOrigin>,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IncrementalFault {
    CreateText,
    CreateBox,
    CreateBoxContext,
    UpdateStyle,
    UpdateTextContext,
    Remove,
}

#[cfg(test)]
thread_local! {
    static INCREMENTAL_FAULT: std::cell::Cell<Option<(IncrementalFault, usize)>> = const {
        std::cell::Cell::new(None)
    };
}

#[cfg(test)]
pub(super) fn set_incremental_fault(fault: IncrementalFault) {
    set_incremental_fault_at(fault, 0);
}

#[cfg(test)]
pub(super) fn set_incremental_fault_at(fault: IncrementalFault, occurrence: usize) {
    INCREMENTAL_FAULT.with(|slot| slot.set(Some((fault, occurrence))));
}

#[cfg(test)]
fn take_incremental_fault(fault: IncrementalFault) -> bool {
    INCREMENTAL_FAULT.with(|slot| match slot.get() {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ApplyPlanError {
    pub(super) patch: PatchError,
    pub(super) stage: PatchStage,
    pub(super) source: PatchTransactionCause,
    pub(super) patch_index: Option<usize>,
}

impl std::fmt::Display for ApplyPlanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} at {:?}", self.patch, self.stage)
    }
}

pub(super) fn apply_error(
    kind: PatchKind,
    key: NodeKey,
    failure: PatchFailure,
    stage: PatchStage,
) -> ApplyPlanError {
    ApplyPlanError {
        patch: PatchError::new(kind, key, failure),
        stage,
        source: PatchTransactionCause::Patch(failure),
        patch_index: None,
    }
}

fn patches_index_for_key(kind: PatchKind, key: NodeKey, patches: &[Patch]) -> Option<usize> {
    patches.iter().position(|patch| {
        let candidate = match (kind, patch) {
            (PatchKind::Create, Patch::Create { key, .. })
            | (PatchKind::Update, Patch::Update { key, .. })
            | (PatchKind::Remove, Patch::Remove { key })
            | (PatchKind::Replace, Patch::Replace { key, .. }) => Some(*key),
            (PatchKind::Reorder, Patch::Reorder { parent, .. }) => Some(*parent),
            _ => None,
        };
        candidate.is_some_and(|candidate| candidate.identity() == key.identity())
    })
}

pub(super) fn taffy_apply_error(
    kind: PatchKind,
    key: NodeKey,
    failure: PatchFailure,
    stage: PatchStage,
    source: taffy::TaffyError,
    patch_index: Option<usize>,
) -> ApplyPlanError {
    ApplyPlanError {
        patch: PatchError::new(kind, key, failure),
        stage,
        source: PatchTransactionCause::Taffy(source),
        patch_index,
    }
}

impl ApplyPlanError {
    pub(super) fn with_patch_index(mut self, patch_index: Option<usize>) -> Self {
        self.patch_index = patch_index;
        self
    }
}

pub(super) struct ElementVNodeSnapshot {
    pub(super) vnode: VNode,
    pub(super) has_layout_root: bool,
    pub(super) element_scopes: HashMap<ElementId, ScopedNodeIdentity>,
    pub(super) element_keys: HashMap<ElementId, NodeKey>,
    pub(super) text_inputs: HashMap<ScopedNodeIdentity, TextFlowInput>,
}

impl ElementVNodeSnapshot {
    pub(super) fn from_element(
        root: &Element,
        arena: &mut ScopedIdentityArena,
    ) -> Result<Self, ReconcilePlanError> {
        let mut snapshot = Self {
            vnode: VNode::root(),
            has_layout_root: false,
            element_scopes: HashMap::new(),
            element_keys: HashMap::new(),
            text_inputs: HashMap::new(),
        };
        if let Some(vnode) = build_element_vnode(
            root,
            &ScopedNodeIdentity::Root,
            0,
            true,
            &mut snapshot,
            arena,
        )? {
            snapshot.vnode = vnode;
            snapshot.has_layout_root = true;
        }
        Ok(snapshot)
    }
}

fn build_element_vnode(
    element: &Element,
    parent: &ScopedNodeIdentity,
    actual_index: usize,
    is_root: bool,
    snapshot: &mut ElementVNodeSnapshot,
    arena: &mut ScopedIdentityArena,
) -> Result<Option<VNode>, ReconcilePlanError> {
    let node_type = match element.element_type {
        ElementType::Root => VNodeType::Root,
        ElementType::Box => VNodeType::Box,
        ElementType::Text => VNodeType::Text(compatibility_text(element)),
        ElementType::VirtualText => return Ok(None),
    };
    let mut props = Props::with_style(element.style.clone());
    props.key = element.key.clone();
    props.scroll_offset_x = element.scroll_offset_x;
    props.scroll_offset_y = element.scroll_offset_y;

    let type_id = node_type.type_id();
    let key = match &element.key {
        Some(exact) => NodeKey::with_key(exact.as_str(), type_id, actual_index),
        None => NodeKey::new(type_id, actual_index),
    };
    let mut vnode = VNode::new(node_type, props);
    vnode.key = key;

    let resolved_identity: Result<_, ReconcilePlanError> = if is_root {
        Ok((ScopedNodeIdentity::Root, key))
    } else {
        resolve_child_identity(
            &vnode,
            actual_index,
            parent,
            &compatibility_token_for_exact,
            arena,
        )
        .map(|resolved| (resolved.scoped, resolved.legacy_key))
    };
    resolved_identity.and_then(|(scoped, legacy_key)| {
        snapshot.element_scopes.insert(element.id, scoped.clone());
        snapshot.element_keys.insert(element.id, legacy_key);
        if let Some(input) = input_from_element(element) {
            snapshot.text_inputs.insert(scoped.clone(), input);
        }

        let mut child_index = 0usize;
        for child in &element.children {
            if let Some(child_vnode) =
                build_element_vnode(child, &scoped, child_index, false, snapshot, arena)?
            {
                vnode.children.push(child_vnode);
                child_index += 1;
            }
        }
        Ok(Some(vnode))
    })
}

impl LayoutEngine {
    fn set_update_style(&mut self, node_id: NodeId, style: taffy::Style) -> taffy::TaffyResult<()> {
        #[cfg(test)]
        if take_incremental_fault(IncrementalFault::UpdateStyle) {
            return Err(taffy::TaffyError::InvalidInputNode(node_id));
        }
        self.taffy.set_style(node_id, style)
    }

    fn set_update_text_context(
        &mut self,
        node_id: NodeId,
        context: NodeContext,
    ) -> taffy::TaffyResult<()> {
        #[cfg(test)]
        if take_incremental_fault(IncrementalFault::UpdateTextContext) {
            return Err(taffy::TaffyError::InvalidInputNode(node_id));
        }
        self.taffy.set_node_context(node_id, Some(context))
    }

    pub(super) fn reset_scoped_vnode_tree(&mut self) {
        self.taffy.clear();
        self.node_map.clear();
        self.element_keys.clear();
        self.element_scopes.clear();
        self.vnode_map.clear();
        self.vnode_legacy_keys.clear();
        self.root_node = None;
        self.current_text_flows.clear();
        self.current_vnode_flows.clear();
        self.committed_vnode = super::Shared::default();
        self.published_snapshot = None;
        self.published_snapshot_report = None;
    }

    pub(super) fn apply_reconcile_plan(
        &mut self,
        plan: &ReconcilePlan,
    ) -> Result<(), ApplyPlanError> {
        let old_map = self.vnode_map.clone();
        let old_legacy_keys = self.vnode_legacy_keys.clone();
        let mut target_map = HashMap::new();
        let mut target_legacy_keys = HashMap::new();
        self.materialize_planned_node(
            &plan.root,
            &old_map,
            &mut target_map,
            &mut target_legacy_keys,
            plan.patches(),
            MaterializeContext::default(),
        )?;
        self.commit_planned_children_for_plan(&plan.root, &target_map, plan.patches())?;

        let target_node_ids: HashSet<_> = target_map.values().copied().collect();
        let mut replacements = Vec::new();
        collect_replaced_old_identities(&plan.root, &mut replacements);
        let removals: Vec<_> = plan
            .parents
            .iter()
            .flat_map(|parent| parent.removals.iter().cloned())
            .collect();
        let mut obsolete: Vec<_> = old_map
            .iter()
            .filter(|(_, node_id)| !target_node_ids.contains(node_id))
            .map(|(identity, node_id)| (identity.clone(), *node_id))
            .collect();
        obsolete.sort_by_key(|(identity, _)| {
            let replacement = replacements
                .iter()
                .find(|root| scoped_identity_is_within(identity, root));
            let removal = removals
                .iter()
                .find(|root| scoped_identity_is_within(identity, root));
            let (kind, locator_identity) = replacement
                .map(|root| (PatchKind::Replace, root))
                .or_else(|| removal.map(|root| (PatchKind::Remove, root)))
                .unwrap_or((PatchKind::Remove, identity));
            let patch_index = old_legacy_keys
                .get(locator_identity)
                .and_then(|key| {
                    patches_index_for_key(
                        kind,
                        locator_identity.scoped_patch_address(*key),
                        plan.patches(),
                    )
                })
                .unwrap_or(usize::MAX);
            (patch_index, identity.diagnostic())
        });
        for (identity, node_id) in obsolete {
            let replacement = replacements
                .iter()
                .find(|root| scoped_identity_is_within(&identity, root));
            let removal = removals
                .iter()
                .find(|root| scoped_identity_is_within(&identity, root));
            let (kind, locator_identity) = replacement
                .map(|root| (PatchKind::Replace, root))
                .or_else(|| removal.map(|root| (PatchKind::Remove, root)))
                .unwrap_or((PatchKind::Remove, &identity));
            let legacy_key = old_legacy_keys
                .get(locator_identity)
                .copied()
                .ok_or_else(|| {
                    apply_error(
                        kind,
                        plan.root.legacy_key,
                        PatchFailure::PostconditionViolated,
                        PatchStage::VerifyPostcondition,
                    )
                })?;
            let patch_key = locator_identity.scoped_patch_address(legacy_key);
            let patch_index = patches_index_for_key(kind, patch_key, plan.patches());
            #[cfg(test)]
            let remove_result = if take_incremental_fault(IncrementalFault::Remove) {
                Err(taffy::TaffyError::InvalidInputNode(node_id))
            } else {
                self.taffy.remove(node_id).map(|_| ())
            };
            #[cfg(not(test))]
            let remove_result = self.taffy.remove(node_id).map(|_| ());
            remove_result.map_err(|source| {
                taffy_apply_error(
                    kind,
                    patch_key,
                    PatchFailure::TreeRejected,
                    PatchStage::RemoveNode,
                    source,
                    patch_index,
                )
            })?;
        }

        self.root_node = target_map.get(&ScopedNodeIdentity::Root).copied();
        self.vnode_map = target_map.into();
        self.vnode_legacy_keys = target_legacy_keys.into();
        self.node_map.clear();
        self.element_keys.clear();
        self.element_scopes.clear();
        self.current_text_flows.clear();
        self.current_vnode_flows.clear();
        self.check_reconcile_postconditions_for_plan(&plan.root, plan.patches())?;
        if self.taffy.total_node_count() != self.vnode_map.len() {
            return Err(apply_error(
                PatchKind::Remove,
                plan.root.legacy_key,
                PatchFailure::PostconditionViolated,
                PatchStage::VerifyPostcondition,
            ));
        }
        Ok(())
    }

    fn materialize_planned_node(
        &mut self,
        planned: &PlannedNode,
        old_map: &HashMap<ScopedNodeIdentity, NodeId>,
        target_map: &mut HashMap<ScopedNodeIdentity, NodeId>,
        target_legacy_keys: &mut HashMap<ScopedNodeIdentity, NodeKey>,
        patches: &[Patch],
        context: MaterializeContext,
    ) -> Result<(), ApplyPlanError> {
        let mut child_origin = context.enclosing_origin;
        let node_id = match planned.action {
            PlannedNodeAction::Reuse | PlannedNodeAction::Update => {
                let old_identity = planned.old_identity.as_ref().ok_or_else(|| {
                    apply_error(
                        PatchKind::Update,
                        planned.legacy_key,
                        PatchFailure::UnknownNode,
                        PatchStage::ResolveTarget,
                    )
                })?;
                old_map.get(old_identity).copied().ok_or_else(|| {
                    apply_error(
                        PatchKind::Update,
                        planned.legacy_key,
                        PatchFailure::UnknownNode,
                        PatchStage::ResolveTarget,
                    )
                })?
            }
            PlannedNodeAction::Create | PlannedNodeAction::Replace => {
                let (kind, key, patch_index) = self.planned_create_locator(
                    planned,
                    context.parent_address,
                    patches,
                    context.enclosing_origin,
                )?;
                child_origin = patch_index.map(|index| (kind, key, index));
                self.create_detached_planned_node(planned, kind, key, patch_index)?
            }
        };

        if planned.action == PlannedNodeAction::Update && planned.mutations.style {
            let patch_index = self.planned_update_patch_index(planned, patches);
            let style = normalized_taffy_style(&planned.vnode.props.style, planned.vnode.is_text());
            let current_style = self.taffy.style(node_id).map_err(|source| {
                taffy_apply_error(
                    PatchKind::Update,
                    planned.legacy_key,
                    PatchFailure::TreeRejected,
                    PatchStage::SetStyle,
                    source,
                    patch_index,
                )
            })?;
            if current_style != &style {
                self.set_update_style(node_id, style).map_err(|source| {
                    taffy_apply_error(
                        PatchKind::Update,
                        planned.legacy_key,
                        PatchFailure::TreeRejected,
                        PatchStage::SetStyle,
                        source,
                        patch_index,
                    )
                })?;
            }
        }
        if planned.action == PlannedNodeAction::Update && planned.mutations.text_context {
            let patch_index = self.planned_update_patch_index(planned, patches);
            let input = input_from_vnode(&planned.vnode);
            let context_matches = input.as_ref().is_some_and(|input| {
                self.taffy
                    .get_node_context(node_id)
                    .is_some_and(|context| context.matches(input, &self.text_flow_policy))
            });
            if !context_matches {
                let context = NodeContext::new(input, &self.text_flow_policy);
                self.set_update_text_context(node_id, context)
                    .map_err(|source| {
                        taffy_apply_error(
                            PatchKind::Update,
                            planned.legacy_key,
                            PatchFailure::TreeRejected,
                            PatchStage::SetContext,
                            source,
                            patch_index,
                        )
                    })?;
            }
        }
        if target_map
            .insert(planned.identity.clone(), node_id)
            .is_some()
        {
            return Err(apply_error(
                PatchKind::Create,
                planned.legacy_key,
                PatchFailure::PostconditionViolated,
                PatchStage::VerifyPostcondition,
            ));
        }
        target_legacy_keys.insert(planned.identity.clone(), planned.legacy_key);
        let child_parent_address = planned.identity.scoped_patch_address(planned.legacy_key);
        for child in &planned.children {
            self.materialize_planned_node(
                child,
                old_map,
                target_map,
                target_legacy_keys,
                patches,
                MaterializeContext {
                    parent_address: Some(child_parent_address),
                    enclosing_origin: child_origin,
                },
            )?;
        }
        Ok(())
    }

    fn planned_create_locator(
        &self,
        planned: &PlannedNode,
        parent_address: Option<NodeKey>,
        patches: &[Patch],
        enclosing_origin: Option<PatchOrigin>,
    ) -> Result<(PatchKind, NodeKey, Option<usize>), ApplyPlanError> {
        match planned.action {
            PlannedNodeAction::Replace => {
                let old_identity = planned.old_identity.as_ref().ok_or_else(|| {
                    apply_error(
                        PatchKind::Replace,
                        planned.legacy_key,
                        PatchFailure::UnknownNode,
                        PatchStage::ResolveTarget,
                    )
                })?;
                let old_key = self
                    .vnode_legacy_keys
                    .get(old_identity)
                    .copied()
                    .ok_or_else(|| {
                        apply_error(
                            PatchKind::Replace,
                            planned.legacy_key,
                            PatchFailure::UnknownNode,
                            PatchStage::ResolveTarget,
                        )
                    })?;
                let key = old_identity.scoped_patch_address(old_key);
                Ok((
                    PatchKind::Replace,
                    key,
                    patches_index_for_key(PatchKind::Replace, key, patches),
                ))
            }
            PlannedNodeAction::Create => {
                let located =
                    patches
                        .iter()
                        .enumerate()
                        .find_map(|(patch_index, patch)| match patch {
                            Patch::Create {
                                key,
                                parent: patch_parent,
                                ..
                            } if key.identity() == planned.legacy_key.identity()
                                && parent_address.is_some_and(|parent| {
                                    parent.identity() == patch_parent.identity()
                                }) =>
                            {
                                Some((patch_index, *patch_parent))
                            }
                            _ => None,
                        });
                if let Some((patch_index, parent)) = located {
                    Ok((PatchKind::Create, parent, Some(patch_index)))
                } else if let Some((kind, key, patch_index)) = enclosing_origin {
                    Ok((kind, key, Some(patch_index)))
                } else {
                    Ok((PatchKind::Create, planned.legacy_key, None))
                }
            }
            PlannedNodeAction::Reuse | PlannedNodeAction::Update => Err(apply_error(
                PatchKind::Create,
                planned.legacy_key,
                PatchFailure::PostconditionViolated,
                PatchStage::ResolveTarget,
            )),
        }
    }

    fn planned_update_patch_index(
        &self,
        planned: &PlannedNode,
        patches: &[Patch],
    ) -> Option<usize> {
        let old_identity = planned.old_identity.as_ref()?;
        let old_key = self.vnode_legacy_keys.get(old_identity).copied()?;
        patches_index_for_key(
            PatchKind::Update,
            old_identity.scoped_patch_address(old_key),
            patches,
        )
    }

    fn create_detached_planned_node(
        &mut self,
        planned: &PlannedNode,
        kind: PatchKind,
        key: NodeKey,
        patch_index: Option<usize>,
    ) -> Result<NodeId, ApplyPlanError> {
        let style = normalized_taffy_style(&planned.vnode.props.style, planned.vnode.is_text());
        let context = NodeContext::new(input_from_vnode(&planned.vnode), &self.text_flow_policy);
        if planned.vnode.is_text() {
            #[cfg(test)]
            let create_result = if take_incremental_fault(IncrementalFault::CreateText) {
                Err(taffy::TaffyError::InvalidInputNode(NodeId::new(u64::MAX)))
            } else {
                self.taffy.new_leaf_with_context(style, context)
            };
            #[cfg(not(test))]
            let create_result = self.taffy.new_leaf_with_context(style, context);
            create_result.map_err(|source| {
                taffy_apply_error(
                    kind,
                    key,
                    PatchFailure::BuildFailed,
                    PatchStage::CreateNode,
                    source,
                    patch_index,
                )
            })
        } else {
            #[cfg(test)]
            let create_result = if take_incremental_fault(IncrementalFault::CreateBox) {
                Err(taffy::TaffyError::InvalidInputNode(NodeId::new(u64::MAX)))
            } else {
                self.taffy.new_leaf(style)
            };
            #[cfg(not(test))]
            let create_result = self.taffy.new_leaf(style);
            let node_id = create_result.map_err(|source| {
                taffy_apply_error(
                    kind,
                    key,
                    PatchFailure::BuildFailed,
                    PatchStage::CreateNode,
                    source,
                    patch_index,
                )
            })?;
            #[cfg(test)]
            let context_result = if take_incremental_fault(IncrementalFault::CreateBoxContext) {
                Err(taffy::TaffyError::InvalidInputNode(node_id))
            } else {
                self.taffy.set_node_context(node_id, Some(context))
            };
            #[cfg(not(test))]
            let context_result = self.taffy.set_node_context(node_id, Some(context));
            context_result.map_err(|source| {
                taffy_apply_error(
                    kind,
                    key,
                    PatchFailure::BuildFailed,
                    PatchStage::SetContext,
                    source,
                    patch_index,
                )
            })?;
            Ok(node_id)
        }
    }

    #[cfg(test)]
    pub(super) fn sync_element_node_map_scoped(&mut self, snapshot: &ElementVNodeSnapshot) {
        self.try_sync_element_node_map_scoped(snapshot)
            .unwrap_or_else(|reason| panic!("element layout map synchronization failed: {reason}"));
    }

    pub(super) fn try_sync_element_node_map_scoped(
        &mut self,
        snapshot: &ElementVNodeSnapshot,
    ) -> Result<(), IncrementalInvariantError> {
        let mut node_map = HashMap::with_capacity(snapshot.element_scopes.len());
        for (element_id, identity) in &snapshot.element_scopes {
            let Some(node_id) = self.vnode_map.get(identity).copied() else {
                return Err(IncrementalInvariantError::ElementMapMismatch);
            };
            node_map.insert(*element_id, node_id);
        }
        self.node_map = node_map;
        self.element_keys = snapshot.element_keys.clone();
        self.element_scopes = snapshot.element_scopes.clone();
        Ok(())
    }

    pub(super) fn try_publish_noop_element_aliases(
        &mut self,
        snapshot: &ElementVNodeSnapshot,
    ) -> Result<(), IncrementalInvariantError> {
        let mut node_map = HashMap::with_capacity(snapshot.element_scopes.len());
        for (element_id, identity) in &snapshot.element_scopes {
            let Some(node_id) = self.vnode_map.get(identity).copied() else {
                return Err(IncrementalInvariantError::ElementMapMismatch);
            };
            node_map.insert(*element_id, node_id);
        }
        let mut current_text_flows = HashMap::with_capacity(snapshot.text_inputs.len());
        for (element_id, identity) in &snapshot.element_scopes {
            if !snapshot.text_inputs.contains_key(identity) {
                continue;
            }
            let flow = self
                .current_vnode_flows
                .get(identity)
                .cloned()
                .ok_or(IncrementalInvariantError::CurrentFrameContextMismatch)?;
            current_text_flows.insert(*element_id, flow);
        }
        self.node_map = node_map;
        self.element_keys = snapshot.element_keys.clone();
        self.element_scopes = snapshot.element_scopes.clone();
        self.current_text_flows = current_text_flows;
        Ok(())
    }
}

fn collect_replaced_old_identities(
    planned: &PlannedNode,
    replacements: &mut Vec<ScopedNodeIdentity>,
) {
    if planned.action == PlannedNodeAction::Replace
        && let Some(identity) = &planned.old_identity
    {
        replacements.push(identity.clone());
    }
    for child in &planned.children {
        collect_replaced_old_identities(child, replacements);
    }
}

fn scoped_identity_is_within(candidate: &ScopedNodeIdentity, root: &ScopedNodeIdentity) -> bool {
    let mut current = Some(candidate);
    while let Some(identity) = current {
        if identity == root {
            return true;
        }
        current = identity.parent();
    }
    false
}

#[cfg(test)]
pub(super) mod tests;
