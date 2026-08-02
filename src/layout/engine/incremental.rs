//! Checked Element conversion and scoped reconciliation-plan application.

use std::collections::{HashMap, HashSet};

use taffy::NodeId;

use crate::core::{Element, ElementId, ElementType, NodeKey, Props, VNode, VNodeType};
use crate::layout::TextFlowInput;
use crate::reconciler::{
    PlannedNode, PlannedNodeAction, ReconcilePlan, ReconcilePlanError, ScopedIdentityArena,
    ScopedNodeIdentity, compatibility_token_for_exact, resolve_child_identity,
};

use super::patch_error::{PatchError, PatchFailure, PatchKind};
use super::text_flow_bridge::{
    NodeContext, compatibility_text, input_from_element, input_from_vnode,
};
use super::{LayoutEngine, normalized_taffy_style};

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IncrementalFault {
    CreateText,
    CreateBox,
    CreateBoxContext,
    UpdateStyle,
    UpdateTextContext,
    Remove,
}

#[cfg(test)]
thread_local! {
    static INCREMENTAL_FAULT: std::cell::Cell<Option<IncrementalFault>> = const {
        std::cell::Cell::new(None)
    };
}

#[cfg(test)]
fn set_incremental_fault(fault: IncrementalFault) {
    INCREMENTAL_FAULT.with(|slot| slot.set(Some(fault)));
}

#[cfg(test)]
fn take_incremental_fault(fault: IncrementalFault) -> bool {
    INCREMENTAL_FAULT.with(|slot| {
        if slot.get() == Some(fault) {
            slot.set(None);
            true
        } else {
            false
        }
    })
}

pub(super) struct ElementVNodeSnapshot {
    pub(super) vnode: VNode,
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
        self.committed_vnode = None;
    }

    pub(super) fn apply_reconcile_plan(&mut self, plan: &ReconcilePlan) -> Result<(), PatchError> {
        let old_map = self.vnode_map.clone();
        let mut target_map = HashMap::new();
        let mut target_legacy_keys = HashMap::new();
        self.materialize_planned_node(
            &plan.root,
            &old_map,
            &mut target_map,
            &mut target_legacy_keys,
        )?;
        self.commit_planned_children(&plan.root, &target_map)?;

        let target_node_ids: HashSet<_> = target_map.values().copied().collect();
        let obsolete: Vec<_> = old_map
            .values()
            .copied()
            .filter(|node_id| !target_node_ids.contains(node_id))
            .collect();
        for node_id in obsolete {
            #[cfg(test)]
            let remove_result = if take_incremental_fault(IncrementalFault::Remove) {
                Err(())
            } else {
                self.taffy
                    .remove(node_id)
                    .expect("validated obsolete node remains removable");
                Ok(())
            };
            #[cfg(not(test))]
            let remove_result = self.taffy.remove(node_id).map(|_| ());
            remove_result.map_err(|_| {
                PatchError::new(
                    PatchKind::Remove,
                    plan.root.legacy_key,
                    PatchFailure::TreeRejected,
                )
            })?;
        }

        self.root_node = target_map.get(&ScopedNodeIdentity::Root).copied();
        self.vnode_map = target_map;
        self.vnode_legacy_keys = target_legacy_keys;
        self.node_map.clear();
        self.element_keys.clear();
        self.element_scopes.clear();
        self.current_text_flows.clear();
        self.current_vnode_flows.clear();
        self.check_reconcile_postconditions(&plan.root)?;
        if self.taffy.total_node_count() != self.vnode_map.len() {
            return Err(PatchError::new(
                PatchKind::Remove,
                plan.root.legacy_key,
                PatchFailure::PostconditionViolated,
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
    ) -> Result<(), PatchError> {
        let node_id = match planned.action {
            PlannedNodeAction::Reuse | PlannedNodeAction::Update => {
                let old_identity = planned.old_identity.as_ref().ok_or_else(|| {
                    PatchError::new(
                        PatchKind::Update,
                        planned.legacy_key,
                        PatchFailure::UnknownNode,
                    )
                })?;
                old_map.get(old_identity).copied().ok_or_else(|| {
                    PatchError::new(
                        PatchKind::Update,
                        planned.legacy_key,
                        PatchFailure::UnknownNode,
                    )
                })?
            }
            PlannedNodeAction::Create | PlannedNodeAction::Replace => {
                self.create_detached_planned_node(planned)?
            }
        };

        if planned.action == PlannedNodeAction::Update && planned.mutations.style {
            let style = normalized_taffy_style(&planned.vnode.props.style, planned.vnode.is_text());
            if self.taffy.style(node_id).ok() != Some(&style) {
                self.set_update_style(node_id, style).map_err(|_| {
                    PatchError::new(
                        PatchKind::Update,
                        planned.legacy_key,
                        PatchFailure::TreeRejected,
                    )
                })?;
            }
        }
        if planned.action == PlannedNodeAction::Update && planned.mutations.text_context {
            let input = input_from_vnode(&planned.vnode);
            let context_matches = input.as_ref().is_some_and(|input| {
                self.taffy
                    .get_node_context(node_id)
                    .is_some_and(|context| context.matches(input, &self.text_flow_policy))
            });
            if !context_matches {
                let context = NodeContext::new(input, &self.text_flow_policy);
                self.set_update_text_context(node_id, context)
                    .map_err(|_| {
                        PatchError::new(
                            PatchKind::Update,
                            planned.legacy_key,
                            PatchFailure::TreeRejected,
                        )
                    })?;
            }
        }
        if target_map
            .insert(planned.identity.clone(), node_id)
            .is_some()
        {
            return Err(PatchError::new(
                PatchKind::Create,
                planned.legacy_key,
                PatchFailure::PostconditionViolated,
            ));
        }
        target_legacy_keys.insert(planned.identity.clone(), planned.legacy_key);
        for child in &planned.children {
            self.materialize_planned_node(child, old_map, target_map, target_legacy_keys)?;
        }
        Ok(())
    }

    fn create_detached_planned_node(
        &mut self,
        planned: &PlannedNode,
    ) -> Result<NodeId, PatchError> {
        let style = normalized_taffy_style(&planned.vnode.props.style, planned.vnode.is_text());
        let context = NodeContext::new(input_from_vnode(&planned.vnode), &self.text_flow_policy);
        if planned.vnode.is_text() {
            #[cfg(test)]
            let create_result = if take_incremental_fault(IncrementalFault::CreateText) {
                Err(())
            } else {
                Ok(self
                    .taffy
                    .new_leaf_with_context(style, context)
                    .expect("Taffy leaf allocation is infallible for validated input"))
            };
            #[cfg(not(test))]
            let create_result = self.taffy.new_leaf_with_context(style, context);
            create_result.map_err(|_| {
                PatchError::new(
                    PatchKind::Create,
                    planned.legacy_key,
                    PatchFailure::BuildFailed,
                )
            })
        } else {
            #[cfg(test)]
            let create_result = if take_incremental_fault(IncrementalFault::CreateBox) {
                Err(())
            } else {
                Ok(self
                    .taffy
                    .new_leaf(style)
                    .expect("Taffy leaf allocation is infallible for validated input"))
            };
            #[cfg(not(test))]
            let create_result = self.taffy.new_leaf(style);
            let node_id = create_result.map_err(|_| {
                PatchError::new(
                    PatchKind::Create,
                    planned.legacy_key,
                    PatchFailure::BuildFailed,
                )
            })?;
            #[cfg(test)]
            let context_result = if take_incremental_fault(IncrementalFault::CreateBoxContext) {
                Err(())
            } else {
                self.taffy
                    .set_node_context(node_id, Some(context))
                    .expect("newly allocated node context remains writable");
                Ok(())
            };
            #[cfg(not(test))]
            let context_result = self.taffy.set_node_context(node_id, Some(context));
            context_result.map_err(|_| {
                PatchError::new(
                    PatchKind::Create,
                    planned.legacy_key,
                    PatchFailure::BuildFailed,
                )
            })?;
            Ok(node_id)
        }
    }

    pub(super) fn sync_element_node_map_scoped(&mut self, snapshot: &ElementVNodeSnapshot) {
        self.node_map.clear();
        self.element_keys = snapshot.element_keys.clone();
        self.element_scopes = snapshot.element_scopes.clone();
        for (element_id, identity) in &snapshot.element_scopes {
            let node_id = self.vnode_map.get(identity).copied().unwrap_or_else(|| {
                panic!(
                    "current element scope {} has no layout node",
                    identity.diagnostic()
                )
            });
            self.node_map.insert(*element_id, node_id);
        }
    }
}

#[cfg(test)]
pub(super) mod tests;
