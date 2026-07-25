//! Layout engine using Taffy

use crate::core::{Dimension, Element, ElementId, ElementType, NodeKey, Props, VNode, VNodeType};
use crate::reconciler::{Patch, diff};
use std::collections::HashMap;
use taffy::{AvailableSpace, NodeId, TaffyTree};

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
}

/// Layout engine that computes element positions
pub struct LayoutEngine {
    taffy: TaffyTree<NodeContext>,
    node_map: HashMap<ElementId, NodeId>,
    element_keys: HashMap<ElementId, NodeKey>,
    /// Map from NodeKey to Taffy NodeId (for VNode-based layout)
    vnode_map: HashMap<NodeKey, NodeId>,
    /// Root node ID for incremental updates
    root_node: Option<NodeId>,
    /// Last computed width
    last_width: u16,
    /// Last computed height
    last_height: u16,
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
        }
    }

    /// Build layout tree from element tree
    pub fn build_tree(&mut self, element: &Element) -> Option<NodeId> {
        self.taffy.clear();
        self.node_map.clear();
        self.element_keys.clear();
        self.vnode_map.clear();
        self.root_node = None;
        self.build_node(element)
    }

    fn build_node(&mut self, element: &Element) -> Option<NodeId> {
        // Skip virtual text nodes (they don't have layout)
        if element.element_type == ElementType::VirtualText {
            return None;
        }

        let mut taffy_style = element.style.to_taffy();
        allow_text_to_shrink(&mut taffy_style, element.is_text(), element.style.min_width);

        // Build children first
        let child_nodes: Vec<NodeId> = element
            .children
            .iter()
            .filter_map(|child| self.build_node(child))
            .collect();

        let context = NodeContext::new(element.text_content.clone(), element.style.text_wrap);

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
        if let Some(root_node) = self.build_tree(root) {
            self.root_node = Some(root_node);
            self.last_width = width;
            self.last_height = height;
            let _ = self.taffy.compute_layout_with_measure(
                root_node,
                taffy::Size {
                    width: AvailableSpace::Definite(width as f32),
                    height: AvailableSpace::Definite(height as f32),
                },
                |known_dimensions, available_space, _node_id, node_context, _style| {
                    measure_text_node(known_dimensions, available_space, node_context)
                },
            );
        }
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
        let mut element_key_map = HashMap::new();
        let current_vnode = self
            .element_to_vnode(root, "root", 0, &mut element_key_map)
            .unwrap_or_else(VNode::root);

        self.last_width = width;
        self.last_height = height;

        let mut outcome = IncrementalLayoutOutcome::default();

        let can_use_incremental = previous_vnode.is_some() && self.has_tree();
        if can_use_incremental {
            let prev = previous_vnode.expect("checked is_some");
            let patches = diff(prev, &current_vnode);
            outcome.patch_count = patches.len();
            outcome.used_reconciler = true;

            if patches.is_empty() {
                self.recompute_layout();
                self.sync_element_node_map(&element_key_map);
                return (current_vnode, outcome);
            }

            if self.apply_patches(&patches) {
                self.sync_element_node_map(&element_key_map);
                return (current_vnode, outcome);
            }
        }

        // Fallback path: no previous tree or incremental update failed.
        self.compute_vnode(&current_vnode, width, height);
        self.sync_element_node_map(&element_key_map);
        outcome.fallback_full_rebuild = can_use_incremental;
        (current_vnode, outcome)
    }

    // ==================== VNode-based Layout ====================

    /// Build layout tree from VNode tree
    pub fn build_vnode_tree(&mut self, vnode: &VNode) -> Option<NodeId> {
        self.taffy.clear();
        self.node_map.clear();
        self.element_keys.clear();
        self.vnode_map.clear();
        self.root_node = self.build_vnode(vnode);
        self.root_node
    }

    fn build_vnode(&mut self, vnode: &VNode) -> Option<NodeId> {
        let mut taffy_style = vnode.props.to_taffy();
        allow_text_to_shrink(
            &mut taffy_style,
            vnode.is_text(),
            vnode.props.style.min_width,
        );

        // Build children first
        let child_nodes: Vec<NodeId> = vnode
            .children
            .iter()
            .filter_map(|child| self.build_vnode(child))
            .collect();

        let text_content = match &vnode.node_type {
            VNodeType::Text(s) => Some(s.clone()),
            _ => None,
        };

        let context = NodeContext::new(text_content, vnode.props.style.text_wrap);

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

        self.vnode_map.insert(vnode.key, node_id);
        Some(node_id)
    }

    fn element_to_vnode(
        &self,
        element: &Element,
        parent_path: &str,
        index: usize,
        element_key_map: &mut HashMap<ElementId, NodeKey>,
    ) -> Option<VNode> {
        if element.element_type == ElementType::VirtualText {
            return None;
        }

        let node_type = match element.element_type {
            ElementType::Root => VNodeType::Root,
            ElementType::Box => VNodeType::Box,
            ElementType::Text => VNodeType::Text(element.text_content.clone().unwrap_or_default()),
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

        let node_path = format!("{parent_path}/{index}");
        vnode.children = element
            .children
            .iter()
            .enumerate()
            .filter_map(|(child_idx, child)| {
                self.element_to_vnode(child, &node_path, child_idx, element_key_map)
            })
            .collect();

        Some(vnode)
    }

    fn sync_element_node_map(&mut self, element_key_map: &HashMap<ElementId, NodeKey>) {
        self.node_map.clear();
        self.element_keys.clear();
        for (element_id, key) in element_key_map {
            self.element_keys.insert(*element_id, *key);
            if let Some(node_id) = self.vnode_map.get(key).copied() {
                self.node_map.insert(*element_id, node_id);
            }
        }
    }

    /// Compute layout for VNode tree
    pub fn compute_vnode(&mut self, root: &VNode, width: u16, height: u16) {
        if let Some(root_node) = self.build_vnode_tree(root) {
            self.last_width = width;
            self.last_height = height;
            let _ = self.taffy.compute_layout_with_measure(
                root_node,
                taffy::Size {
                    width: AvailableSpace::Definite(width as f32),
                    height: AvailableSpace::Definite(height as f32),
                },
                |known_dimensions, available_space, _node_id, node_context, _style| {
                    measure_text_node(known_dimensions, available_space, node_context)
                },
            );
        }
    }

    /// Apply patches incrementally instead of rebuilding the entire tree
    ///
    /// This is the key optimization for the reconciliation system.
    /// Instead of rebuilding the entire Taffy tree on every render,
    /// we apply only the changes detected by the diff algorithm.
    pub fn apply_patches(&mut self, patches: &[Patch]) -> bool {
        if patches.is_empty() {
            return false;
        }

        let mut needs_recompute = false;

        for patch in patches {
            match patch {
                Patch::Create { node, parent, .. } => {
                    if self.create_vnode(node, *parent).is_some() {
                        needs_recompute = true;
                    }
                }
                Patch::Update { key, new_props, .. } => {
                    if self.update_node_props(*key, new_props) {
                        needs_recompute = true;
                    }
                }
                Patch::Remove { key } => {
                    if self.remove_node(*key) {
                        needs_recompute = true;
                    }
                }
                Patch::Replace { key, node, .. } => {
                    if self.replace_node(*key, node) {
                        needs_recompute = true;
                    }
                }
                Patch::Reorder { parent, moves } => {
                    if self.reorder_children(*parent, moves) {
                        needs_recompute = true;
                    }
                }
            }
        }

        // Recompute layout if any changes were made
        if needs_recompute {
            self.recompute_layout();
        }

        needs_recompute
    }

    /// Create a new node and add it to a parent
    fn create_vnode(&mut self, vnode: &VNode, parent_key: NodeKey) -> Option<NodeId> {
        // Get parent node ID first (copy it to avoid borrow issues)
        let parent_node = *self.vnode_map.get(&parent_key)?;

        // Build the new subtree
        let new_node_id = self.build_vnode(vnode)?;

        // Add to parent
        let _ = self.taffy.add_child(parent_node, new_node_id);

        Some(new_node_id)
    }

    /// Update a node's props/style
    fn update_node_props(&mut self, key: NodeKey, props: &Props) -> bool {
        if let Some(&node_id) = self.vnode_map.get(&key) {
            let new_style = props.to_taffy();
            if self.taffy.set_style(node_id, new_style).is_ok() {
                return true;
            }
        }
        false
    }

    /// Remove a node from the tree
    fn remove_node(&mut self, key: NodeKey) -> bool {
        if let Some(node_id) = self.vnode_map.remove(&key) {
            // Remove from Taffy tree
            if self.taffy.remove(node_id).is_ok() {
                return true;
            }
        }
        false
    }

    /// Replace a node with a new one
    fn replace_node(&mut self, old_key: NodeKey, new_node: &VNode) -> bool {
        if let Some(&old_node_id) = self.vnode_map.get(&old_key) {
            // Get parent before removing
            if let Some(parent_id) = self.taffy.parent(old_node_id) {
                // Find the index of the old node in parent's children
                let children: Vec<_> = self.taffy.children(parent_id).unwrap_or_default();
                let index = children.iter().position(|&id| id == old_node_id);

                // Remove old node
                let _ = self.taffy.remove(old_node_id);
                self.vnode_map.remove(&old_key);

                // Build new subtree
                if let Some(new_node_id) = self.build_vnode(new_node) {
                    // Insert at same position if possible
                    if let Some(idx) = index {
                        let _ = self
                            .taffy
                            .insert_child_at_index(parent_id, idx, new_node_id);
                    } else {
                        let _ = self.taffy.add_child(parent_id, new_node_id);
                    }
                    return true;
                }
            }
        }
        false
    }

    /// Reorder children of a node
    fn reorder_children(&mut self, parent_key: NodeKey, moves: &[(usize, usize)]) -> bool {
        if moves.is_empty() {
            return false;
        }

        if let Some(&parent_id) = self.vnode_map.get(&parent_key) {
            let old_children: Vec<_> = self.taffy.children(parent_id).unwrap_or_default();

            // Build the new order by placing each old child at its target position.
            // `moves` contains (from, to) pairs where `from` is the index in the
            // old array and `to` is the desired index in the new array.
            let mut new_children = old_children.clone();
            for &(from, to) in moves {
                if from < old_children.len() && to < new_children.len() {
                    new_children[to] = old_children[from];
                }
            }

            // Set new children order
            if self.taffy.set_children(parent_id, &new_children).is_ok() {
                return true;
            }
        }
        false
    }

    /// Recompute layout after patches
    fn recompute_layout(&mut self) {
        if let Some(root_node) = self.root_node {
            let _ = self.taffy.compute_layout_with_measure(
                root_node,
                taffy::Size {
                    width: AvailableSpace::Definite(self.last_width as f32),
                    height: AvailableSpace::Definite(self.last_height as f32),
                },
                |known_dimensions, available_space, _node_id, node_context, _style| {
                    measure_text_node(known_dimensions, available_space, node_context)
                },
            );
        }
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
        let node_id = self.vnode_map.get(&key)?;
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

    /// Get all VNode layouts
    pub fn get_all_vnode_layouts(&self) -> HashMap<NodeKey, Layout> {
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

mod text_measure;

use text_measure::{NodeContext, measure_text_node};
