//! Layout engine using Taffy

use crate::core::{
    Dimension, Element, ElementId, ElementType, NodeKey, Props, Style, VNode, VNodeType,
};
use crate::layout::{TextFlow, TextFlowError, TextFlowInput};
use crate::reconciler::{SiblingIdentity, diff};
use std::{collections::HashMap, sync::Arc};
use taffy::{NodeId, TaffyTree};

/// Computed layout for an element
#[derive(Debug, Clone, Copy, Default)]
pub struct Layout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Outcome of an incremental layout computation.
#[derive(Debug, Clone, Copy, Default)]
pub struct IncrementalLayoutOutcome {
    /// Whether diff/patch path was used.
    pub used_reconciler: bool,
    /// Number of generated patches for this frame.
    pub patch_count: usize,
    /// Whether incremental path failed and full rebuild was used.
    pub fallback_full_rebuild: bool,
    /// Why the patch batch was rejected, when one was.
    ///
    /// A fallback rebuild produces the right layout either way, so without
    /// this the rejection is invisible and a persistent patching fault looks
    /// like normal operation.
    pub patch_error: Option<PatchError>,
}

/// Layout engine that computes element positions
pub struct LayoutEngine {
    taffy: TaffyTree<NodeContext>,
    node_map: HashMap<ElementId, NodeId>,
    element_keys: HashMap<ElementId, NodeKey>,
    /// Map from a node's cross-frame identity to its Taffy NodeId.
    ///
    /// Keyed by `SiblingIdentity`, not by `NodeKey`: `NodeKey`'s derived `Eq`
    /// includes the current index, so a keyed child that moved would no longer
    /// find its own node here.
    vnode_map: HashMap<SiblingIdentity, NodeId>,
    /// Root node ID for incremental updates
    root_node: Option<NodeId>,
    /// Last computed width
    last_width: u16,
    /// Last computed height
    last_height: u16,
    flow_cache: FlowCache,
    text_flow_policy: TextFlowPolicy,
    current_text_flows: HashMap<ElementId, Arc<TextFlow>>,
    current_vnode_flows: HashMap<SiblingIdentity, Arc<TextFlow>>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
            node_map: HashMap::new(),
            element_keys: HashMap::new(),
            vnode_map: HashMap::new(),
            root_node: None,
            last_width: 0,
            last_height: 0,
            flow_cache: FlowCache::default(),
            text_flow_policy: TextFlowPolicy::default(),
            current_text_flows: HashMap::new(),
            current_vnode_flows: HashMap::new(),
        }
    }

    /// Build layout tree from element tree
    pub fn build_tree(&mut self, element: &Element) -> Option<NodeId> {
        self.taffy.clear();
        self.node_map.clear();
        self.element_keys.clear();
        self.vnode_map.clear();
        self.root_node = None;
        self.current_text_flows.clear();
        self.current_vnode_flows.clear();
        self.build_node(element)
    }

    fn build_node(&mut self, element: &Element) -> Option<NodeId> {
        // Skip virtual text nodes (they don't have layout)
        if element.element_type == ElementType::VirtualText {
            return None;
        }

        let taffy_style = normalized_taffy_style(&element.style, element.is_text());

        // Build children first
        let child_nodes: Vec<NodeId> = element
            .children
            .iter()
            .filter_map(|child| self.build_node(child))
            .collect();

        let context = NodeContext::new(input_from_element(element), &self.text_flow_policy);

        // Create node with measure function for text
        let node_id = if element.is_text() {
            self.taffy
                .new_leaf_with_context(taffy_style, context)
                .ok()?
        } else {
            let node = self
                .taffy
                .new_with_children(taffy_style, &child_nodes)
                .ok()?;
            // Set context for non-text nodes too
            let _ = self.taffy.set_node_context(node, Some(context));
            node
        };

        self.node_map.insert(element.id, node_id);
        Some(node_id)
    }

    /// Compute layout for the tree
    pub fn compute(&mut self, root: &Element, width: u16, height: u16) {
        self.try_compute(root, width, height)
            .unwrap_or_else(|error| panic!("text flow layout failed: {error}"));
    }

    pub fn try_compute(
        &mut self,
        root: &Element,
        width: u16,
        height: u16,
    ) -> Result<(), TextFlowError> {
        self.try_compute_interruptible(root, width, height, || false)
    }

    pub(crate) fn try_compute_interruptible(
        &mut self,
        root: &Element,
        width: u16,
        height: u16,
        mut interrupted: impl FnMut() -> bool,
    ) -> Result<(), TextFlowError> {
        let mut candidate = self.staged_clone();
        if let Some(root_node) = candidate.build_tree(root) {
            candidate.root_node = Some(root_node);
            candidate.last_width = width;
            candidate.last_height = height;
            candidate.run_layout_and_publish(&mut interrupted)?;
        }
        *self = candidate;
        Ok(())
    }

    /// Compute layout from an `Element` tree using reconciler diff/patch when possible.
    ///
    /// Returns the current frame VNode snapshot plus incremental execution metadata.
    pub fn compute_element_incremental(
        &mut self,
        root: &Element,
        previous_vnode: Option<&VNode>,
        width: u16,
        height: u16,
    ) -> (VNode, IncrementalLayoutOutcome) {
        self.try_compute_element_incremental(root, previous_vnode, width, height)
            .unwrap_or_else(|error| panic!("incremental text flow layout failed: {error}"))
    }

    pub fn try_compute_element_incremental(
        &mut self,
        root: &Element,
        previous_vnode: Option<&VNode>,
        width: u16,
        height: u16,
    ) -> Result<(VNode, IncrementalLayoutOutcome), TextFlowError> {
        let mut candidate = self.staged_clone();
        let mut element_key_map = HashMap::new();
        let mut text_inputs = HashMap::new();
        let current_vnode = candidate
            .element_to_vnode(root, "root", 0, &mut element_key_map, &mut text_inputs)
            .unwrap_or_else(VNode::root);

        candidate.last_width = width;
        candidate.last_height = height;

        let mut outcome = IncrementalLayoutOutcome::default();

        let can_use_incremental = previous_vnode.is_some() && candidate.has_tree();
        if can_use_incremental {
            let prev = previous_vnode.expect("checked is_some");
            let patches = diff(prev, &current_vnode);
            outcome.patch_count = patches.len();
            outcome.used_reconciler = true;

            if patches.is_empty() {
                candidate.sync_text_contexts(&text_inputs);
                candidate.sync_element_node_map(&element_key_map);
                candidate.run_layout_and_publish(&mut || false)?;
                *self = candidate;
                return Ok((current_vnode, outcome));
            }

            match candidate
                .apply_patches_only(&patches)
                .and_then(|()| candidate.check_batch_postconditions(&patches))
            {
                Ok(()) => {
                    candidate.sync_text_contexts(&text_inputs);
                    candidate.sync_element_node_map(&element_key_map);
                    candidate.run_layout_and_publish(&mut || false)?;
                    *self = candidate;
                    return Ok((current_vnode, outcome));
                }
                Err(error) => {
                    // The batch is rejected whole. `candidate` may be partly
                    // patched, but the rebuild below clears it first, so what
                    // is committed comes only from `current_vnode`.
                    outcome.patch_error = Some(error);
                }
            }
        }

        // Fallback path: no previous tree or incremental update failed.
        candidate.build_vnode_tree(&current_vnode);
        candidate.sync_text_contexts(&text_inputs);
        candidate.sync_element_node_map(&element_key_map);
        candidate.run_layout_and_publish(&mut || false)?;
        outcome.fallback_full_rebuild = can_use_incremental;
        *self = candidate;
        Ok((current_vnode, outcome))
    }

    // ==================== VNode-based Layout ====================

    /// Build layout tree from VNode tree
    pub fn build_vnode_tree(&mut self, vnode: &VNode) -> Option<NodeId> {
        self.taffy.clear();
        self.node_map.clear();
        self.element_keys.clear();
        self.vnode_map.clear();
        self.current_text_flows.clear();
        self.current_vnode_flows.clear();
        self.root_node = self.build_vnode(vnode);
        self.root_node
    }

    fn build_vnode(&mut self, vnode: &VNode) -> Option<NodeId> {
        let taffy_style = normalized_taffy_style(&vnode.props.style, vnode.is_text());

        // Build children first
        let child_nodes: Vec<NodeId> = vnode
            .children
            .iter()
            .filter_map(|child| self.build_vnode(child))
            .collect();

        let context = NodeContext::new(input_from_vnode(vnode), &self.text_flow_policy);

        // Create node
        let node_id = if vnode.is_text() {
            self.taffy
                .new_leaf_with_context(taffy_style, context)
                .ok()?
        } else {
            let node = self
                .taffy
                .new_with_children(taffy_style, &child_nodes)
                .ok()?;
            let _ = self.taffy.set_node_context(node, Some(context));
            node
        };

        self.vnode_map.insert(vnode.key.identity(), node_id);
        Some(node_id)
    }

    fn element_to_vnode(
        &self,
        element: &Element,
        parent_path: &str,
        index: usize,
        element_key_map: &mut HashMap<ElementId, NodeKey>,
        text_inputs: &mut HashMap<NodeKey, TextFlowInput>,
    ) -> Option<VNode> {
        if element.element_type == ElementType::VirtualText {
            return None;
        }

        let node_type = match element.element_type {
            ElementType::Root => VNodeType::Root,
            ElementType::Box => VNodeType::Box,
            ElementType::Text => VNodeType::Text(compatibility_text(element)),
            ElementType::VirtualText => return None,
        };

        let mut props = Props::with_style(element.style.clone());
        props.key = element.key.clone();
        props.scroll_offset_x = element.scroll_offset_x;
        props.scroll_offset_y = element.scroll_offset_y;

        let mut vnode = VNode::new(node_type, props).with_index(index);

        if element.element_type == ElementType::Root {
            vnode.key = NodeKey::root();
        } else {
            let type_id = vnode.node_type.type_id();
            let synthetic_key = if let Some(user_key) = &element.key {
                format!("{parent_path}#key:{user_key}")
            } else {
                format!("{parent_path}@idx:{index}:type:{:?}", element.element_type)
            };
            vnode.key = NodeKey::with_key(&synthetic_key, type_id, index);
        }

        element_key_map.insert(element.id, vnode.key);
        if let Some(input) = input_from_element(element) {
            text_inputs.insert(vnode.key, input);
        }

        // A keyed node contributes its key, not its position, so that moving
        // it among its siblings does not change the synthetic identity of
        // everything beneath it.
        let node_path = match &element.key {
            Some(user_key) => format!("{parent_path}/key:{user_key}"),
            None => format!("{parent_path}/{index}"),
        };
        vnode.children = element
            .children
            .iter()
            .enumerate()
            .filter_map(|(child_idx, child)| {
                self.element_to_vnode(child, &node_path, child_idx, element_key_map, text_inputs)
            })
            .collect();

        Some(vnode)
    }

    fn sync_element_node_map(&mut self, element_key_map: &HashMap<ElementId, NodeKey>) {
        self.node_map.clear();
        self.element_keys.clear();
        for (element_id, key) in element_key_map {
            self.element_keys.insert(*element_id, *key);
            if let Some(node_id) = self.vnode_map.get(&key.identity()).copied() {
                self.node_map.insert(*element_id, node_id);
            }
        }
    }

    /// Compute layout for VNode tree
    pub fn compute_vnode(&mut self, root: &VNode, width: u16, height: u16) {
        self.try_compute_vnode(root, width, height)
            .unwrap_or_else(|error| panic!("VNode text flow layout failed: {error}"));
    }

    pub fn try_compute_vnode(
        &mut self,
        root: &VNode,
        width: u16,
        height: u16,
    ) -> Result<(), TextFlowError> {
        let mut candidate = self.staged_clone();
        if candidate.build_vnode_tree(root).is_some() {
            candidate.last_width = width;
            candidate.last_height = height;
            candidate.run_layout_and_publish(&mut || false)?;
        }
        *self = candidate;
        Ok(())
    }

    /// Get computed layout for an element
    pub fn get_layout(&self, element_id: ElementId) -> Option<Layout> {
        let node_id = self.node_map.get(&element_id)?;
        let layout = self.taffy.layout(*node_id).ok()?;

        Some(Layout {
            x: layout.location.x,
            y: layout.location.y,
            width: layout.size.width,
            height: layout.size.height,
        })
    }

    /// Get computed layout for a VNode by key
    pub fn get_vnode_layout(&self, key: NodeKey) -> Option<Layout> {
        let node_id = self.vnode_map.get(&key.identity())?;
        let layout = self.taffy.layout(*node_id).ok()?;

        Some(Layout {
            x: layout.location.x,
            y: layout.location.y,
            width: layout.size.width,
            height: layout.size.height,
        })
    }

    /// Get all layouts
    pub fn get_all_layouts(&self) -> HashMap<ElementId, Layout> {
        self.node_map
            .iter()
            .filter_map(|(element_id, node_id)| {
                let layout = self.taffy.layout(*node_id).ok()?;
                Some((
                    *element_id,
                    Layout {
                        x: layout.location.x,
                        y: layout.location.y,
                        width: layout.size.width,
                        height: layout.size.height,
                    },
                ))
            })
            .collect()
    }

    /// Get all VNode layouts, addressed by cross-frame identity
    pub fn get_all_vnode_layouts(&self) -> HashMap<SiblingIdentity, Layout> {
        self.vnode_map
            .iter()
            .filter_map(|(key, node_id)| {
                let layout = self.taffy.layout(*node_id).ok()?;
                Some((
                    *key,
                    Layout {
                        x: layout.location.x,
                        y: layout.location.y,
                        width: layout.size.width,
                        height: layout.size.height,
                    },
                ))
            })
            .collect()
    }

    /// Get the stable node key associated with an element in the current frame.
    pub(crate) fn node_key_for_element(&self, element_id: ElementId) -> Option<NodeKey> {
        self.element_keys.get(&element_id).copied()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn current_text_flow(&self, element_id: ElementId) -> Option<Arc<TextFlow>> {
        self.current_text_flows.get(&element_id).cloned()
    }

    #[allow(dead_code)] // Consumed by the renderer integration lane.
    pub(crate) fn current_vnode_text_flow(&self, key: NodeKey) -> Option<Arc<TextFlow>> {
        self.current_vnode_flows.get(&key.identity()).cloned()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn set_text_flow_policy(
        &mut self,
        tab_stop: usize,
        ellipsis: impl Into<String>,
        revision: u16,
    ) {
        self.text_flow_policy.set(tab_stop, ellipsis, revision);
    }

    /// Check if the engine has a valid tree
    pub fn has_tree(&self) -> bool {
        self.root_node.is_some()
    }

    /// Get the number of nodes in the tree
    pub fn node_count(&self) -> usize {
        self.node_map.len() + self.vnode_map.len()
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn normalized_taffy_style(style: &Style, is_text: bool) -> ::taffy::Style {
    let mut taffy_style = style.to_taffy();
    allow_text_to_shrink(&mut taffy_style, is_text, style.min_width);
    taffy_style
}

/// Let a text node shrink below its own content width.
///
/// A flex item's automatic minimum size is its min-content width, so a text
/// node would otherwise refuse to be narrowed and would overflow its parent
/// instead of wrapping. `min-width: 0` is the standard CSS remedy.
///
/// The alternative — reporting a smaller min-content from the measure callback
/// — makes Taffy's sizing search explode: a 40-deep tree went from 140 measure
/// calls to 396k.
///
/// An explicit `min_width` from the caller always wins.
fn allow_text_to_shrink(style: &mut ::taffy::Style, is_text: bool, explicit_min_width: Dimension) {
    if is_text && matches!(explicit_min_width, Dimension::Auto) {
        style.min_size.width = ::taffy::Dimension::Length(0.0);
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod frame_flow_tests {
    use super::*;
    use crate::core::FlexDirection;
    use crate::reconciler::Patch;

    fn many_distinct_text_nodes() -> (Element, Vec<ElementId>) {
        let mut root = Element::box_element();
        root.style.width = Dimension::Points(16.0);
        root.style.flex_direction = FlexDirection::Column;
        let mut ids = Vec::new();
        for index in 0..=FlowCache::MAX_ENTRIES {
            let child = Element::text(format!("node-{index}")).with_key(format!("node-{index}"));
            ids.push(child.id);
            root.add_child(child);
        }
        (root, ids)
    }

    #[test]
    fn active_frame_flows_remain_identical_beyond_history_limit() {
        let (root, ids) = many_distinct_text_nodes();
        let mut engine = LayoutEngine::new();
        let (current_vnode, _) = engine
            .try_compute_element_incremental(&root, None, 80, 200)
            .unwrap();

        let mut published = Vec::new();
        for element_id in &ids {
            let key = engine.node_key_for_element(*element_id).unwrap();
            let node_id = *engine.node_map.get(element_id).unwrap();
            let context = engine.taffy.get_node_context(node_id).unwrap();
            let measured_flow = context.last_measured_flow().unwrap();
            let active_flow = context.active_flow().unwrap();
            let element_flow = engine.current_text_flow(*element_id).unwrap();
            let vnode_flow = engine.current_vnode_text_flow(key).unwrap();
            assert!(Arc::ptr_eq(measured_flow, active_flow));
            assert!(Arc::ptr_eq(active_flow, &element_flow));
            assert!(Arc::ptr_eq(&element_flow, &vnode_flow));
            published.push((*element_id, key, node_id, element_flow));
        }
        assert_eq!(published.len(), FlowCache::MAX_ENTRIES + 1);
        assert_eq!(engine.flow_cache.len(), FlowCache::MAX_ENTRIES);

        engine.set_text_flow_policy(0, "…", 1);
        let failure = engine.try_compute_element_incremental(&root, Some(&current_vnode), 80, 200);
        assert!(matches!(failure, Err(TextFlowError::InvalidTabStop)));
        for (element_id, key, node_id, before) in published {
            let context = engine.taffy.get_node_context(node_id).unwrap();
            assert!(Arc::ptr_eq(context.active_flow().unwrap(), &before));
            assert!(Arc::ptr_eq(context.last_measured_flow().unwrap(), &before));
            assert!(Arc::ptr_eq(
                &engine.current_text_flow(element_id).unwrap(),
                &before
            ));
            assert!(Arc::ptr_eq(
                &engine.current_vnode_text_flow(key).unwrap(),
                &before
            ));
        }
    }

    #[test]
    fn removing_nested_vnode_purges_descendant_flow() {
        let leaf = VNode::text("gone").with_key("leaf");
        let leaf_key = leaf.key;
        let branch = VNode::box_node().with_key("branch").child(leaf);
        let branch_key = branch.key;
        let sibling = VNode::text("keep").with_key("sibling");
        let root = VNode::box_node().children([branch, sibling]);
        let sibling_key = root.children[1].key;
        let mut engine = LayoutEngine::new();
        engine.compute_vnode(&root, 20, 4);
        let sibling_flow = engine.current_vnode_text_flow(sibling_key).unwrap();

        assert!(engine.apply_patches(&[Patch::remove(branch_key)]));
        assert!(engine.get_vnode_layout(branch_key).is_none());
        assert!(engine.get_vnode_layout(leaf_key).is_none());
        assert!(engine.current_vnode_text_flow(leaf_key).is_none());
        assert!(Arc::ptr_eq(
            &sibling_flow,
            &engine.current_vnode_text_flow(sibling_key).unwrap()
        ));
    }
}

mod context_sync;
mod patch_error;
pub use patch_error::{PatchError, PatchFailure, PatchKind};
mod patching;
mod text_flow_bridge;

use text_flow_bridge::{
    FlowCache, NodeContext, TextFlowPolicy, compatibility_text, input_from_element,
    input_from_vnode,
};
