//! Layout engine using Taffy

use crate::core::{Dimension, Element, ElementId, ElementType, NodeKey, Style, VNode};
use crate::layout::{TextFlow, TextFlowError};
use crate::reconciler::{ScopedNodeIdentity, plan_initial_tree};
use std::{collections::HashMap, sync::Arc};
use taffy::{NodeId, TaffyTree};

#[derive(Clone)]
struct CowTaffy(Arc<TaffyTree<NodeContext>>);

impl CowTaffy {
    fn new() -> Self {
        Self(Arc::new(TaffyTree::new()))
    }

    fn clear(&mut self) {
        self.0 = Arc::new(TaffyTree::new());
    }

    fn remove(&mut self, node: NodeId) -> taffy::TaffyResult<NodeId> {
        let tree = Arc::make_mut(&mut self.0);
        tree.set_node_context(node, None)?;
        tree.remove(node)
    }

    #[cfg(test)]
    fn shares_storage(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl std::ops::Deref for CowTaffy {
    type Target = TaffyTree<NodeContext>;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl std::ops::DerefMut for CowTaffy {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Shared<T>(Arc<T>);

impl<T> Shared<T> {
    fn new(value: T) -> Self {
        Self(Arc::new(value))
    }

    #[cfg(test)]
    fn shares_storage(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: Default> Default for Shared<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> std::ops::Deref for Shared<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl<T: Clone> std::ops::DerefMut for Shared<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}

impl<T> From<T> for Shared<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

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
    taffy: CowTaffy,
    node_map: HashMap<ElementId, NodeId>,
    element_keys: HashMap<ElementId, NodeKey>,
    element_scopes: HashMap<ElementId, ScopedNodeIdentity>,
    /// Correctness index. Public sibling-local identities are compatibility
    /// projections only and never address this map directly.
    vnode_map: Shared<HashMap<ScopedNodeIdentity, NodeId>>,
    vnode_legacy_keys: Shared<HashMap<ScopedNodeIdentity, NodeKey>>,
    /// Root node ID for incremental updates
    root_node: Option<NodeId>,
    /// Last computed width
    last_width: u16,
    /// Last computed height
    last_height: u16,
    flow_cache: FlowCache,
    text_flow_policy: TextFlowPolicy,
    current_text_flows: HashMap<ElementId, Arc<TextFlow>>,
    current_vnode_flows: Shared<HashMap<ScopedNodeIdentity, Arc<TextFlow>>>,
    committed_vnode: Shared<Option<VNode>>,
    commit_epoch: Arc<()>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            taffy: CowTaffy::new(),
            node_map: HashMap::new(),
            element_keys: HashMap::new(),
            element_scopes: HashMap::new(),
            vnode_map: Shared::default(),
            vnode_legacy_keys: Shared::default(),
            root_node: None,
            last_width: 0,
            last_height: 0,
            flow_cache: FlowCache::default(),
            text_flow_policy: TextFlowPolicy::default(),
            current_text_flows: HashMap::new(),
            current_vnode_flows: Shared::default(),
            committed_vnode: Shared::default(),
            commit_epoch: Arc::new(()),
        }
    }

    fn rotate_commit_epoch(&mut self) {
        self.commit_epoch = Arc::new(());
    }

    /// Build layout tree from element tree
    pub fn build_tree(&mut self, element: &Element) -> Option<NodeId> {
        let mut candidate = self.staged_clone();
        let root = candidate.build_tree_in_place(element)?;
        candidate.rotate_commit_epoch();
        *self = candidate;
        Some(root)
    }

    fn build_tree_in_place(&mut self, element: &Element) -> Option<NodeId> {
        self.taffy.clear();
        self.node_map.clear();
        self.element_keys.clear();
        self.element_scopes.clear();
        self.vnode_map.clear();
        self.vnode_legacy_keys.clear();
        self.root_node = None;
        self.current_text_flows.clear();
        self.current_vnode_flows.clear();
        self.committed_vnode = Shared::default();
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

    /// Compute layout for the tree.
    ///
    /// # Panics
    ///
    /// Panics when the target has no layout root or layout computation fails.
    pub fn compute(&mut self, root: &Element, width: u16, height: u16) {
        self.try_compute(root, width, height)
            .unwrap_or_else(|error| panic!("text flow layout failed: {error}"));
    }

    /// Tries to compute layout for the tree while preserving text-flow errors.
    ///
    /// # Errors
    ///
    /// Returns an error when text-flow measurement or layout fails.
    ///
    /// # Panics
    ///
    /// Panics when the legacy error type cannot represent an invalid layout root.
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
        let root_node = candidate.build_tree_in_place(root).unwrap_or_else(|| {
            panic!("legacy layout computation cannot represent an invalid layout root")
        });
        candidate.root_node = Some(root_node);
        candidate.last_width = width;
        candidate.last_height = height;
        candidate.run_layout_and_publish(&mut interrupted)?;
        candidate.rotate_commit_epoch();
        *self = candidate;
        Ok(())
    }

    /// Compute layout from an `Element` tree using reconciler diff/patch when possible.
    ///
    /// Returns the current frame VNode snapshot plus incremental execution metadata.
    ///
    /// # Panics
    ///
    /// Panics on any final layout failure, including identity, text-flow,
    /// invalid-target, transaction, or recovery failure. Use
    /// [`try_compute_element_incremental_transactional`](Self::try_compute_element_incremental_transactional)
    /// when every failure must remain recoverable.
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

    /// Legacy adapter that checks text flow but panics on identity failure.
    ///
    /// # Errors
    ///
    /// Returns [`TextFlowError`] when text layout fails.
    ///
    /// # Panics
    ///
    /// Panics when identity planning fails or the generalized transaction
    /// boundary produces a failure that [`TextFlowError`] cannot represent,
    /// including invalid-target and non-text-flow recovery failures. Use
    /// [`try_compute_element_incremental_transactional`](Self::try_compute_element_incremental_transactional)
    /// to handle every failure explicitly.
    pub fn try_compute_element_incremental(
        &mut self,
        root: &Element,
        previous_vnode: Option<&VNode>,
        width: u16,
        height: u16,
    ) -> Result<(VNode, IncrementalLayoutOutcome), TextFlowError> {
        match self.try_compute_element_incremental_checked(root, previous_vnode, width, height) {
            Ok(result) => Ok(result),
            Err(IncrementalLayoutError::TextFlow(source)) => Err(source),
            Err(IncrementalLayoutError::Identity(source)) => {
                panic!("incremental identity planning failed: {source}")
            }
        }
    }

    /// Checked incremental layout boundary.
    ///
    /// Identity/metadata validation and committed-tree preflight happen before
    /// the engine is cloned or any GH60 patch fallback can run.
    ///
    /// # Errors
    ///
    /// Returns [`IncrementalLayoutError`] for invalid identity metadata, a
    /// caller snapshot that differs from the committed tree, or text-flow
    /// failure. The engine remains unchanged.
    ///
    /// # Panics
    ///
    /// Panics if the generalized transaction boundary produces a failure that
    /// [`IncrementalLayoutError`] cannot represent, including invalid-target,
    /// non-text-flow initial-build, transaction, or recovery failures. Use
    /// [`try_compute_element_incremental_transactional`](Self::try_compute_element_incremental_transactional)
    /// to handle every failure explicitly.
    pub fn try_compute_element_incremental_checked(
        &mut self,
        root: &Element,
        previous_vnode: Option<&VNode>,
        width: u16,
        height: u16,
    ) -> Result<(VNode, IncrementalLayoutOutcome), IncrementalLayoutError> {
        match self.try_compute_element_incremental_transactional(
            root,
            previous_vnode,
            width,
            height,
        ) {
            Ok((vnode, report)) => {
                let outcome = match report {
                    CheckedIncrementalLayoutReport::InitialFullBuild => {
                        IncrementalLayoutOutcome::default()
                    }
                    CheckedIncrementalLayoutReport::NoChange => IncrementalLayoutOutcome {
                        used_reconciler: true,
                        ..IncrementalLayoutOutcome::default()
                    },
                    CheckedIncrementalLayoutReport::Incremental { patch_count } => {
                        IncrementalLayoutOutcome {
                            used_reconciler: true,
                            patch_count,
                            ..IncrementalLayoutOutcome::default()
                        }
                    }
                    CheckedIncrementalLayoutReport::RecomputedViewport => {
                        IncrementalLayoutOutcome {
                            used_reconciler: true,
                            ..IncrementalLayoutOutcome::default()
                        }
                    }
                    CheckedIncrementalLayoutReport::RecoveredFullRebuild {
                        patch_count,
                        incremental_failure,
                    } => IncrementalLayoutOutcome {
                        used_reconciler: true,
                        patch_count,
                        fallback_full_rebuild: true,
                        patch_error: Some(incremental_failure.legacy()),
                    },
                };
                Ok((vnode, outcome))
            }
            Err(TransactionalLayoutError::Upstream(source)) => Err(source),
            Err(TransactionalLayoutError::InitialBuild(FullRebuildError {
                source: RebuildFailure::TextFlow(source),
                ..
            })) => Err(source.into()),
            Err(TransactionalLayoutError::RecoveryFailed {
                incremental,
                rebuild,
            }) => match (incremental.source.as_ref(), &rebuild.source) {
                (PatchTransactionCause::TextFlow(source), RebuildFailure::TextFlow(_)) => {
                    Err(source.clone().into())
                }
                _ => panic!(
                    "transactional incremental layout failed: candidate {incremental}; \
                         rebuild {rebuild}"
                ),
            },
            Err(error) => panic!("transactional incremental layout failed: {error}"),
        }
    }

    // ==================== VNode-based Layout ====================

    /// Build layout tree from VNode tree
    pub fn build_vnode_tree(&mut self, vnode: &VNode) -> Option<NodeId> {
        let mut candidate = self.staged_clone();
        let root = candidate.build_vnode_tree_in_place(vnode)?;
        candidate.rotate_commit_epoch();
        *self = candidate;
        Some(root)
    }

    fn build_vnode_tree_in_place(&mut self, vnode: &VNode) -> Option<NodeId> {
        let plan = plan_initial_tree(vnode)
            .unwrap_or_else(|error| panic!("VNode identity validation failed: {error}"));
        self.reset_scoped_vnode_tree();
        self.apply_reconcile_plan(&plan)
            .unwrap_or_else(|error| panic!("VNode tree build failed: {error}"));
        self.committed_vnode = Shared::new(Some(vnode.clone()));
        self.root_node
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
        if candidate.build_vnode_tree_in_place(root).is_some() {
            candidate.last_width = width;
            candidate.last_height = height;
            candidate.run_layout_and_publish(&mut || false)?;
        }
        candidate.rotate_commit_epoch();
        *self = candidate;
        Ok(())
    }

    /// Get computed layout for an element.
    ///
    /// # Panics
    ///
    /// Panics when a committed element alias references an invalid backend
    /// node or a node without computed layout.
    pub fn get_layout(&self, element_id: ElementId) -> Option<Layout> {
        self.try_get_required_layout(element_id)
            .unwrap_or_else(|error| panic!("committed element layout lookup failed: {error}"))
    }

    /// Get all layouts
    pub fn get_all_layouts(&self) -> HashMap<ElementId, Layout> {
        self.try_get_layout_snapshot()
            .unwrap_or_else(|error| {
                panic!("target-exact element layout snapshot failed: {error:?}")
            })
            .element
    }

    /// Get the stable node key associated with an element in the current frame.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn node_key_for_element(&self, element_id: ElementId) -> Option<NodeKey> {
        self.element_keys.get(&element_id).copied()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn current_text_flow(&self, element_id: ElementId) -> Option<Arc<TextFlow>> {
        self.current_text_flows.get(&element_id).cloned()
    }

    #[allow(dead_code)] // Consumed by the renderer integration lane.
    pub(crate) fn current_vnode_text_flow(&self, key: NodeKey) -> Option<Arc<TextFlow>> {
        let identity = self
            .resolve_legacy_scope(key)
            .unwrap_or_else(|error| panic!("VNode text-flow lookup failed: {error}"))?;
        self.current_vnode_flows.get(&identity).cloned()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn set_text_flow_policy(
        &mut self,
        tab_stop: usize,
        ellipsis: impl Into<String>,
        revision: u16,
    ) {
        self.text_flow_policy.set(tab_stop, ellipsis, revision);
        self.rotate_commit_epoch();
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
pub(crate) use context_sync::{CheckedLayoutSnapshot, LayoutSnapshotError};
mod identity_index;
mod incremental;
mod incremental_order;
mod invariant_error;
pub use invariant_error::IncrementalInvariantError;
mod patch_error;
pub use patch_error::{
    CheckedIncrementalLayoutReport, DirectPatchApplyReport, DirectPatchError,
    DirectPatchPreflightCause, DirectPatchPreflightError, FullRebuildError, IncrementalLayoutError,
    IncrementalPatchKind, InvalidLayoutTargetError, LayoutLookupError, PatchError, PatchFailure,
    PatchKind, PatchStage, PatchTransactionCause, PatchTransactionError, RebuildFailure,
    RebuildStage, TransactionalLayoutError,
};
mod patching;
mod postcondition;
#[cfg(test)]
mod test_fingerprint;
mod text_flow_bridge;
mod transaction;
pub use transaction::PreparedLayoutFrame;
pub(crate) use transaction::{BoundPreparedLayoutFrame, PreparedLayoutCommitError};

#[cfg(test)]
impl LayoutEngine {
    pub(crate) fn inject_test_compute_fault() {
        context_sync::set_layout_compute_fault();
    }

    pub(crate) fn inject_test_postcondition_fault() {
        postcondition::set_postcondition_fault(postcondition::PostconditionFault::MissingRoot);
    }

    pub(crate) fn inject_test_required_layout_fault(&mut self, element_id: ElementId) {
        let node_id = self.node_map[&element_id];
        self.taffy
            .set_node_context(node_id, None)
            .expect("fixture node exists in the Taffy backend");
        self.rotate_commit_epoch();
    }
}

use text_flow_bridge::{FlowCache, NodeContext, TextFlowPolicy, input_from_element};
