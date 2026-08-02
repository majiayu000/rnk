//! Target-exact candidate validation.

use std::{collections::HashSet, sync::Arc};

use taffy::NodeId;

use crate::core::{NodeKey, Props, VNode};
use crate::reconciler::{PlannedNode, ReconcilePlan, ScopedNodeIdentity};

use super::{
    IncrementalInvariantError, LayoutEngine,
    incremental::ElementVNodeSnapshot,
    normalized_taffy_style,
    text_flow_bridge::{input_from_vnode, same_text_flow_input},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TargetValidationCause {
    Taffy(taffy::TaffyError),
    Invariant(IncrementalInvariantError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TargetValidationError {
    pub(super) key: Option<NodeKey>,
    pub(super) source: TargetValidationCause,
}

impl TargetValidationError {
    fn invariant(key: Option<NodeKey>, source: IncrementalInvariantError) -> Self {
        Self {
            key,
            source: TargetValidationCause::Invariant(source),
        }
    }

    fn taffy(key: Option<NodeKey>, source: taffy::TaffyError) -> Self {
        Self {
            key,
            source: TargetValidationCause::Taffy(source),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum TargetAliasExpectation<'a> {
    RawVNode,
    Element(&'a ElementVNodeSnapshot),
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PostconditionFault {
    MissingRoot,
    InvalidRoot,
    ScopedMapMismatch,
    MissingComputedLayout,
    CurrentFrameContextMismatch,
}

#[cfg(test)]
thread_local! {
    static POSTCONDITION_FAULT: std::cell::Cell<Option<(PostconditionFault, usize)>> =
        const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(super) fn set_postcondition_fault(fault: PostconditionFault) {
    set_postcondition_fault_at(fault, 0);
}

#[cfg(test)]
pub(super) fn set_postcondition_fault_at(fault: PostconditionFault, occurrence: usize) {
    POSTCONDITION_FAULT.with(|slot| slot.set(Some((fault, occurrence))));
}

#[cfg(test)]
fn take_postcondition_fault() -> Option<IncrementalInvariantError> {
    POSTCONDITION_FAULT.with(|slot| match slot.get() {
        Some((fault, 0)) => {
            slot.set(None);
            Some(match fault {
                PostconditionFault::MissingRoot => IncrementalInvariantError::MissingRoot,
                PostconditionFault::InvalidRoot => IncrementalInvariantError::InvalidRoot,
                PostconditionFault::ScopedMapMismatch => {
                    IncrementalInvariantError::ScopedMapMismatch
                }
                PostconditionFault::MissingComputedLayout => {
                    IncrementalInvariantError::MissingComputedLayout
                }
                PostconditionFault::CurrentFrameContextMismatch => {
                    IncrementalInvariantError::CurrentFrameContextMismatch
                }
            })
        }
        Some((fault, remaining)) => {
            slot.set(Some((fault, remaining - 1)));
            None
        }
        None => None,
    })
}

impl LayoutEngine {
    pub(super) fn validate_target_exact(
        &self,
        plan: &ReconcilePlan,
        aliases: TargetAliasExpectation<'_>,
        target: &VNode,
        width: u16,
        height: u16,
    ) -> Result<(), TargetValidationError> {
        #[cfg(test)]
        if let Some(source) = take_postcondition_fault() {
            return Err(TargetValidationError::invariant(Some(target.key), source));
        }

        if !planned_tree_matches_target(&plan.root, target) {
            return Err(TargetValidationError::invariant(
                Some(target.key),
                IncrementalInvariantError::ScopedMapMismatch,
            ));
        }
        if self.committed_vnode.is_none() {
            return Err(TargetValidationError::invariant(
                Some(target.key),
                IncrementalInvariantError::CurrentFrameContextMismatch,
            ));
        }
        if self.last_width != width || self.last_height != height {
            return Err(TargetValidationError::invariant(
                Some(target.key),
                IncrementalInvariantError::CurrentFrameContextMismatch,
            ));
        }

        let mut identities = HashSet::new();
        let mut mapped_nodes = HashSet::new();
        let mut planned_nodes = Vec::new();
        collect_planned_nodes(&plan.root, &mut planned_nodes);
        if self.vnode_map.len() != planned_nodes.len()
            || self.vnode_legacy_keys.len() != planned_nodes.len()
        {
            return Err(TargetValidationError::invariant(
                Some(target.key),
                IncrementalInvariantError::ScopedMapMismatch,
            ));
        }

        for planned in &planned_nodes {
            if !identities.insert(planned.identity.clone()) {
                return Err(TargetValidationError::invariant(
                    Some(planned.legacy_key),
                    IncrementalInvariantError::ScopedMapMismatch,
                ));
            }
            let node_id = self
                .vnode_map
                .get(&planned.identity)
                .copied()
                .ok_or_else(|| {
                    TargetValidationError::invariant(
                        Some(planned.legacy_key),
                        IncrementalInvariantError::ScopedMapMismatch,
                    )
                })?;
            if self.taffy.get_node_context(node_id).is_none() || !mapped_nodes.insert(node_id) {
                return Err(TargetValidationError::invariant(
                    Some(planned.legacy_key),
                    IncrementalInvariantError::InvalidMappedNode,
                ));
            }
            if self.vnode_legacy_keys.get(&planned.identity) != Some(&planned.legacy_key) {
                return Err(TargetValidationError::invariant(
                    Some(planned.legacy_key),
                    IncrementalInvariantError::CompatibilityMapMismatch,
                ));
            }
        }
        let root = self.root_node.ok_or_else(|| {
            TargetValidationError::invariant(
                Some(target.key),
                IncrementalInvariantError::MissingRoot,
            )
        })?;
        if self.vnode_map.get(&ScopedNodeIdentity::Root).copied() != Some(root)
            || self.taffy.parent(root).is_some()
        {
            return Err(TargetValidationError::invariant(
                Some(target.key),
                IncrementalInvariantError::InvalidRoot,
            ));
        }

        let reachable = self.reachable_target_nodes(root, target.key)?;
        if reachable != mapped_nodes {
            return Err(TargetValidationError::invariant(
                Some(target.key),
                IncrementalInvariantError::ReachableNodeSetMismatch,
            ));
        }
        if self.taffy.total_node_count() != planned_nodes.len() {
            return Err(TargetValidationError::invariant(
                Some(target.key),
                IncrementalInvariantError::NodeCountMismatch,
            ));
        }

        let mut compatibility_projections = HashSet::new();
        let mut expected_text_identities = HashSet::new();
        for planned in planned_nodes {
            let node_id = self.vnode_map[&planned.identity];
            self.validate_planned_node_exact(
                planned,
                node_id,
                aliases,
                &mut compatibility_projections,
                &mut expected_text_identities,
            )?;
        }
        if self.current_vnode_flows.len() != expected_text_identities.len() {
            return Err(TargetValidationError::invariant(
                Some(target.key),
                IncrementalInvariantError::CurrentFrameContextMismatch,
            ));
        }
        self.validate_aliases_exact(aliases, &expected_text_identities, target.key)
    }

    fn reachable_target_nodes(
        &self,
        root: NodeId,
        root_key: NodeKey,
    ) -> Result<HashSet<NodeId>, TargetValidationError> {
        let mut reachable = HashSet::new();
        let mut pending = vec![root];
        while let Some(node_id) = pending.pop() {
            if !reachable.insert(node_id) {
                return Err(TargetValidationError::invariant(
                    Some(root_key),
                    IncrementalInvariantError::ReachableNodeCycle,
                ));
            }
            if self.taffy.get_node_context(node_id).is_none() {
                return Err(TargetValidationError::invariant(
                    Some(root_key),
                    IncrementalInvariantError::InvalidMappedNode,
                ));
            }
            let children = self
                .taffy
                .children(node_id)
                .map_err(|source| TargetValidationError::taffy(Some(root_key), source))?;
            pending.extend(children);
        }
        Ok(reachable)
    }

    fn validate_planned_node_exact(
        &self,
        planned: &PlannedNode,
        node_id: NodeId,
        aliases: TargetAliasExpectation<'_>,
        compatibility_projections: &mut HashSet<crate::reconciler::SiblingIdentity>,
        expected_text_identities: &mut HashSet<ScopedNodeIdentity>,
    ) -> Result<(), TargetValidationError> {
        let key = Some(planned.legacy_key);
        let expected_style =
            normalized_taffy_style(&planned.vnode.props.style, planned.vnode.is_text());
        let style = self
            .taffy
            .style(node_id)
            .map_err(|source| TargetValidationError::taffy(key, source))?;
        if !taffy_styles_match(style, &expected_style) {
            return Err(TargetValidationError::invariant(
                key,
                IncrementalInvariantError::CurrentFrameContextMismatch,
            ));
        }
        if self
            .taffy
            .dirty(node_id)
            .map_err(|source| TargetValidationError::taffy(key, source))?
        {
            return Err(TargetValidationError::invariant(
                key,
                IncrementalInvariantError::MissingComputedLayout,
            ));
        }
        let layout = self
            .taffy
            .layout(node_id)
            .map_err(|source| TargetValidationError::taffy(key, source))?;
        let expected_children: Vec<_> = planned
            .children
            .iter()
            .map(|child| self.vnode_map[&child.identity])
            .collect();
        let actual_children = self
            .taffy
            .children(node_id)
            .map_err(|source| TargetValidationError::taffy(key, source))?;
        if actual_children != expected_children {
            return Err(TargetValidationError::invariant(
                key,
                IncrementalInvariantError::ChildOrderMismatch,
            ));
        }

        let projection = planned.identity.composite_identity(planned.legacy_key);
        if !compatibility_projections.insert(projection) {
            return Err(TargetValidationError::invariant(
                key,
                IncrementalInvariantError::CompatibilityMapMismatch,
            ));
        }

        let expected_input = match aliases {
            TargetAliasExpectation::RawVNode => input_from_vnode(&planned.vnode),
            TargetAliasExpectation::Element(snapshot) => {
                snapshot.text_inputs.get(&planned.identity).cloned()
            }
        };
        let context = self.taffy.get_node_context(node_id).ok_or_else(|| {
            TargetValidationError::invariant(
                key,
                IncrementalInvariantError::CurrentFrameContextMismatch,
            )
        })?;
        match expected_input {
            Some(input) => {
                expected_text_identities.insert(planned.identity.clone());
                if !context.matches(&input, &self.text_flow_policy) {
                    return Err(TargetValidationError::invariant(
                        key,
                        IncrementalInvariantError::CurrentFrameContextMismatch,
                    ));
                }
                let horizontal_inset = layout.padding.left
                    + layout.padding.right
                    + layout.border.left
                    + layout.border.right;
                let text_width = (layout.size.width - horizontal_inset).max(0.0).floor() as usize;
                let options = self.text_flow_policy.options(&input, text_width);
                let active = context.active_flow().ok_or_else(|| {
                    TargetValidationError::invariant(
                        key,
                        IncrementalInvariantError::CurrentFrameContextMismatch,
                    )
                })?;
                if !same_text_flow_input(&active.cache_identity().input, &input)
                    || active.cache_identity().options != options
                    || !self
                        .current_vnode_flows
                        .get(&planned.identity)
                        .is_some_and(|published| Arc::ptr_eq(active, published))
                {
                    return Err(TargetValidationError::invariant(
                        key,
                        IncrementalInvariantError::CurrentFrameContextMismatch,
                    ));
                }
            }
            None => {
                if context.input().is_some()
                    || context.active_flow().is_some()
                    || self.current_vnode_flows.contains_key(&planned.identity)
                {
                    return Err(TargetValidationError::invariant(
                        key,
                        IncrementalInvariantError::CurrentFrameContextMismatch,
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_aliases_exact(
        &self,
        aliases: TargetAliasExpectation<'_>,
        expected_text_identities: &HashSet<ScopedNodeIdentity>,
        root_key: NodeKey,
    ) -> Result<(), TargetValidationError> {
        let fail = |source| TargetValidationError::invariant(Some(root_key), source);
        match aliases {
            TargetAliasExpectation::RawVNode => {
                if !self.node_map.is_empty()
                    || !self.element_keys.is_empty()
                    || !self.element_scopes.is_empty()
                    || !self.current_text_flows.is_empty()
                {
                    return Err(fail(IncrementalInvariantError::ElementMapMismatch));
                }
            }
            TargetAliasExpectation::Element(snapshot) => {
                if self.element_scopes != snapshot.element_scopes
                    || self.element_keys != snapshot.element_keys
                    || self.node_map.len() != snapshot.element_scopes.len()
                    || snapshot.element_scopes.len() != self.vnode_map.len()
                {
                    return Err(fail(IncrementalInvariantError::ElementMapMismatch));
                }
                for (element_id, identity) in &snapshot.element_scopes {
                    let expected_node = self
                        .vnode_map
                        .get(identity)
                        .copied()
                        .ok_or_else(|| fail(IncrementalInvariantError::ElementMapMismatch))?;
                    if self.node_map.get(element_id).copied() != Some(expected_node) {
                        return Err(fail(IncrementalInvariantError::ElementMapMismatch));
                    }
                }
                if self.current_text_flows.len() != snapshot.text_inputs.len()
                    || snapshot.text_inputs.len() != expected_text_identities.len()
                {
                    return Err(fail(IncrementalInvariantError::CurrentFrameContextMismatch));
                }
                for (element_id, identity) in &snapshot.element_scopes {
                    let is_text = snapshot.text_inputs.contains_key(identity);
                    match (
                        is_text,
                        self.current_text_flows.get(element_id),
                        self.current_vnode_flows.get(identity),
                    ) {
                        (true, Some(element), Some(vnode)) if Arc::ptr_eq(element, vnode) => {}
                        (false, None, None) => {}
                        _ => {
                            return Err(fail(
                                IncrementalInvariantError::CurrentFrameContextMismatch,
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn collect_planned_nodes<'a>(planned: &'a PlannedNode, output: &mut Vec<&'a PlannedNode>) {
    output.push(planned);
    for child in &planned.children {
        collect_planned_nodes(child, output);
    }
}

fn planned_tree_matches_target(planned: &PlannedNode, target: &VNode) -> bool {
    planned.vnode.key == target.key
        && planned.vnode.node_type == target.node_type
        && props_snapshots_match(&planned.vnode.props, &target.props)
        && planned.children.len() == target.children.len()
        && planned
            .children
            .iter()
            .zip(&target.children)
            .all(|(planned, target)| planned_tree_matches_target(planned, target))
}

fn props_snapshots_match(left: &Props, right: &Props) -> bool {
    left.semantically_eq(right)
}

#[cfg(test)]
fn style_snapshots_match(left: &crate::core::Style, right: &crate::core::Style) -> bool {
    left.semantically_eq(right)
}

fn taffy_styles_match(actual: &taffy::Style, expected: &taffy::Style) -> bool {
    actual == expected || format!("{actual:?}") == format!("{expected:?}")
}

#[cfg(test)]
mod tests;
