//! Incremental text-context synchronization and frame flow publication.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use taffy::{AvailableSpace, NodeId};

use super::{
    IncrementalInvariantError, LayoutEngine,
    text_flow_bridge::{NodeContext, flow_for_width, measure_text_node},
};
use crate::core::NodeKey;
use crate::layout::{Layout, LayoutLookupError, TextFlow, TextFlowError, TextFlowInput};
use crate::reconciler::{ScopedNodeIdentity, SiblingIdentity};

pub(crate) struct CheckedLayoutSnapshot {
    pub(crate) element: HashMap<crate::core::ElementId, Layout>,
    pub(crate) vnode: HashMap<SiblingIdentity, Layout>,
}
pub(crate) use measurements::CheckedMeasurementSnapshot;
#[derive(Debug)]
pub(crate) enum LegacyLayoutSnapshotError {
    Lookup(LayoutLookupError),
    Invariant(IncrementalInvariantError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LayoutRunError {
    Taffy {
        node_id: Option<NodeId>,
        source: taffy::TaffyError,
    },
    TextFlow {
        node_id: Option<NodeId>,
        source: TextFlowError,
    },
    ReadBackTaffy {
        node_id: Option<NodeId>,
        source: taffy::TaffyError,
    },
    ReadBackTextFlow {
        node_id: Option<NodeId>,
        source: TextFlowError,
    },
    Invariant {
        node_id: Option<NodeId>,
        source: IncrementalInvariantError,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ContextSyncError {
    Taffy {
        node_id: Option<NodeId>,
        key: Option<NodeKey>,
        source: taffy::TaffyError,
    },
    Invariant {
        node_id: Option<NodeId>,
        key: Option<NodeKey>,
        source: IncrementalInvariantError,
    },
}

impl std::fmt::Display for ContextSyncError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Taffy { source, .. } => source.fmt(formatter),
            Self::Invariant { source, .. } => source.fmt(formatter),
        }
    }
}

impl std::error::Error for ContextSyncError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Taffy { source, .. } => Some(source),
            Self::Invariant { source, .. } => Some(source),
        }
    }
}

impl ContextSyncError {
    pub(super) fn node_id(&self) -> Option<NodeId> {
        match self {
            Self::Taffy { node_id, .. } | Self::Invariant { node_id, .. } => *node_id,
        }
    }

    pub(super) fn key(&self) -> Option<NodeKey> {
        match self {
            Self::Taffy { key, .. } | Self::Invariant { key, .. } => *key,
        }
    }
}

impl LayoutRunError {
    fn into_read_back(self, node_id: Option<NodeId>) -> Self {
        match self {
            Self::Taffy {
                node_id: existing,
                source,
            } => Self::ReadBackTaffy {
                node_id: existing.or(node_id),
                source,
            },
            Self::TextFlow {
                node_id: existing,
                source,
            } => Self::ReadBackTextFlow {
                node_id: existing.or(node_id),
                source,
            },
            Self::ReadBackTaffy {
                node_id: existing,
                source,
            } => Self::ReadBackTaffy {
                node_id: existing.or(node_id),
                source,
            },
            Self::ReadBackTextFlow {
                node_id: existing,
                source,
            } => Self::ReadBackTextFlow {
                node_id: existing.or(node_id),
                source,
            },
            Self::Invariant {
                node_id: existing,
                source,
            } => Self::Invariant {
                node_id: existing.or(node_id),
                source,
            },
        }
    }

    pub(super) fn node_id(&self) -> Option<NodeId> {
        match self {
            Self::Taffy { node_id, .. }
            | Self::ReadBackTaffy { node_id, .. }
            | Self::ReadBackTextFlow { node_id, .. }
            | Self::Invariant { node_id, .. }
            | Self::TextFlow { node_id, .. } => *node_id,
        }
    }

    fn invariant(node_id: NodeId, source: IncrementalInvariantError) -> Self {
        Self::Invariant {
            node_id: Some(node_id),
            source,
        }
    }

    fn read_back_taffy(node_id: NodeId, source: taffy::TaffyError) -> Self {
        Self::ReadBackTaffy {
            node_id: Some(node_id),
            source,
        }
    }
}

impl From<taffy::TaffyError> for LayoutRunError {
    fn from(source: taffy::TaffyError) -> Self {
        Self::Taffy {
            node_id: None,
            source,
        }
    }
}

impl From<TextFlowError> for LayoutRunError {
    fn from(source: TextFlowError) -> Self {
        Self::TextFlow {
            node_id: None,
            source,
        }
    }
}

#[cfg(test)]
thread_local! {
    static LAYOUT_COMPUTE_FAULT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static LAYOUT_READ_BACK_FAULT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static CONTEXT_PIN_FAULT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(super) fn set_layout_compute_fault() {
    LAYOUT_COMPUTE_FAULT.with(|fault| fault.set(true));
}

#[cfg(test)]
fn take_layout_compute_fault() -> bool {
    LAYOUT_COMPUTE_FAULT.with(|fault| fault.replace(false))
}

#[cfg(test)]
pub(super) fn set_layout_read_back_fault() {
    LAYOUT_READ_BACK_FAULT.with(|fault| fault.set(true));
}

#[cfg(test)]
fn take_layout_read_back_fault() -> bool {
    LAYOUT_READ_BACK_FAULT.with(|fault| fault.replace(false))
}

#[cfg(test)]
pub(super) fn set_context_pin_fault() {
    CONTEXT_PIN_FAULT.with(|fault| fault.set(true));
}

#[cfg(test)]
fn take_context_pin_fault() -> bool {
    CONTEXT_PIN_FAULT.with(|fault| fault.replace(false))
}

pub(super) trait TextContextKey {
    fn resolve_scope(
        &self,
        engine: &LayoutEngine,
    ) -> Result<Option<ScopedNodeIdentity>, LayoutLookupError>;

    fn legacy_key(&self, engine: &LayoutEngine) -> Option<NodeKey>;
}

impl TextContextKey for ScopedNodeIdentity {
    fn resolve_scope(
        &self,
        _engine: &LayoutEngine,
    ) -> Result<Option<ScopedNodeIdentity>, LayoutLookupError> {
        Ok(Some(self.clone()))
    }

    fn legacy_key(&self, engine: &LayoutEngine) -> Option<NodeKey> {
        engine.vnode_legacy_keys.get(self).copied()
    }
}

impl TextContextKey for NodeKey {
    fn resolve_scope(
        &self,
        engine: &LayoutEngine,
    ) -> Result<Option<ScopedNodeIdentity>, LayoutLookupError> {
        engine.resolve_legacy_scope(*self)
    }

    fn legacy_key(&self, _engine: &LayoutEngine) -> Option<NodeKey> {
        Some(*self)
    }
}

impl LayoutEngine {
    pub(crate) fn try_get_required_layout(
        &self,
        element_id: crate::core::ElementId,
    ) -> Result<Option<Layout>, IncrementalInvariantError> {
        let Some(node_id) = self.node_map.get(&element_id).copied() else {
            return Ok(None);
        };
        if self.taffy.get_node_context(node_id).is_none() {
            return Err(IncrementalInvariantError::InvalidMappedNode);
        }
        let layout = self
            .taffy
            .layout(node_id)
            .copied()
            .map_err(|_| IncrementalInvariantError::MissingComputedLayout)?;
        Ok(Some(public_layout(&layout)))
    }

    pub(super) fn text_contexts_match_scoped(
        &self,
        inputs: &HashMap<ScopedNodeIdentity, TextFlowInput>,
    ) -> bool {
        inputs.iter().all(|(identity, input)| {
            self.vnode_map
                .get(identity)
                .and_then(|node_id| self.taffy.get_node_context(*node_id))
                .is_some_and(|context| context.matches(input, &self.text_flow_policy))
        })
    }

    pub(super) fn resolve_legacy_scope(
        &self,
        key: NodeKey,
    ) -> Result<Option<ScopedNodeIdentity>, LayoutLookupError> {
        let matches: Vec<_> = self
            .vnode_legacy_keys
            .iter()
            .filter(|(_, candidate)| candidate.identity() == key.identity())
            .map(|(identity, _)| identity.clone())
            .collect();
        match matches.as_slice() {
            [] => Ok(None),
            [identity] => Ok(Some(identity.clone())),
            _ => Err(LayoutLookupError::AmbiguousLegacyNodeKey {
                key,
                scoped_match_count: matches.len(),
            }),
        }
    }

    /// Checked layout lookup for a legacy sibling-local key.
    ///
    /// # Errors
    ///
    /// Returns an error when the raw key is ambiguous across parent scopes.
    ///
    /// # Panics
    ///
    /// Panics when a committed compatibility identity references an invalid
    /// backend node or a node without computed layout.
    pub fn try_get_vnode_layout(&self, key: NodeKey) -> Result<Option<Layout>, LayoutLookupError> {
        let Some(identity) = self.resolve_legacy_scope(key)? else {
            return Ok(None);
        };
        let node_id = self
            .vnode_map
            .get(&identity)
            .unwrap_or_else(|| panic!("committed VNode identity has no backend mapping"));
        assert!(
            self.taffy.get_node_context(*node_id).is_some(),
            "committed VNode identity references an invalid backend node"
        );
        let layout = self
            .taffy
            .layout(*node_id)
            .unwrap_or_else(|_| panic!("committed VNode has no computed layout"));
        Ok(Some(Layout {
            x: layout.location.x,
            y: layout.location.y,
            width: layout.size.width,
            height: layout.size.height,
        }))
    }

    /// Compatibility layout lookup.
    ///
    /// # Panics
    ///
    /// Panics when the raw key is ambiguous or a committed compatibility
    /// identity references invalid backend state.
    pub fn get_vnode_layout(&self, key: NodeKey) -> Option<Layout> {
        self.try_get_vnode_layout(key)
            .unwrap_or_else(|error| panic!("VNode layout lookup failed: {error}"))
    }

    /// Checked composite compatibility projection of all scoped layouts.
    pub fn try_get_all_vnode_layouts(
        &self,
    ) -> Result<HashMap<SiblingIdentity, Layout>, LayoutLookupError> {
        match self.try_get_layout_snapshot() {
            Ok(snapshot) => Ok(snapshot.vnode),
            Err(LegacyLayoutSnapshotError::Lookup(source)) => Err(source),
            Err(LegacyLayoutSnapshotError::Invariant(source)) => {
                panic!("target-exact VNode layout snapshot failed: {source}")
            }
        }
    }

    pub(crate) fn try_get_layout_snapshot(
        &self,
    ) -> Result<CheckedLayoutSnapshot, LegacyLayoutSnapshotError> {
        let mut element = HashMap::with_capacity(self.node_map.len());
        for (element_id, node_id) in &self.node_map {
            if self.taffy.get_node_context(*node_id).is_none() {
                return Err(LegacyLayoutSnapshotError::Invariant(
                    IncrementalInvariantError::InvalidMappedNode,
                ));
            }
            let layout = self.taffy.layout(*node_id).map_err(|_| {
                LegacyLayoutSnapshotError::Invariant(
                    IncrementalInvariantError::MissingComputedLayout,
                )
            })?;
            element.insert(*element_id, public_layout(layout));
        }

        let mut layouts = HashMap::with_capacity(self.vnode_map.len());
        let mut projected_scopes = HashMap::with_capacity(self.vnode_map.len());
        for (identity, node_id) in self.vnode_map.iter() {
            if self.taffy.get_node_context(*node_id).is_none() {
                return Err(LegacyLayoutSnapshotError::Invariant(
                    IncrementalInvariantError::InvalidMappedNode,
                ));
            }
            let legacy_key = self.vnode_legacy_keys.get(identity).copied().ok_or(
                LegacyLayoutSnapshotError::Invariant(
                    IncrementalInvariantError::CompatibilityMapMismatch,
                ),
            )?;
            let projected = identity.composite_identity(legacy_key);
            if let Some(existing) = projected_scopes.insert(projected, identity)
                && existing != identity
            {
                return Err(LegacyLayoutSnapshotError::Lookup(
                    LayoutLookupError::CompositeIdentityCollision {
                        identity: projected,
                    },
                ));
            }
            let layout = self.taffy.layout(*node_id).map_err(|_| {
                LegacyLayoutSnapshotError::Invariant(
                    IncrementalInvariantError::MissingComputedLayout,
                )
            })?;
            let layout = public_layout(layout);
            layouts.insert(projected, layout);
        }
        if self.vnode_legacy_keys.len() != self.vnode_map.len() {
            return Err(LegacyLayoutSnapshotError::Invariant(
                IncrementalInvariantError::CompatibilityMapMismatch,
            ));
        }
        Ok(CheckedLayoutSnapshot {
            element,
            vnode: layouts,
        })
    }

    /// Composite compatibility projection of all scoped layouts.
    ///
    /// # Panics
    ///
    /// Panics on a composite projection collision.
    pub fn get_all_vnode_layouts(&self) -> HashMap<SiblingIdentity, Layout> {
        self.try_get_all_vnode_layouts()
            .unwrap_or_else(|error| panic!("VNode layout projection failed: {error}"))
    }

    pub(crate) fn scoped_projection_for_element(
        &self,
        element_id: crate::core::ElementId,
    ) -> Option<(ScopedNodeIdentity, SiblingIdentity)> {
        let identity = self.element_scopes.get(&element_id)?.clone();
        let legacy_key = self.vnode_legacy_keys.get(&identity).copied()?;
        Some((identity.clone(), identity.composite_identity(legacy_key)))
    }

    pub(crate) fn raw_vnode_identity_candidates(
        &self,
    ) -> HashMap<SiblingIdentity, Vec<ScopedNodeIdentity>> {
        let mut candidates = HashMap::new();
        for (identity, key) in self.vnode_legacy_keys.iter() {
            candidates
                .entry(key.identity())
                .or_insert_with(Vec::new)
                .push(identity.clone());
        }
        candidates
    }

    #[cfg(test)]
    pub(super) fn sync_text_contexts<K>(&mut self, inputs: &HashMap<K, TextFlowInput>)
    where
        K: TextContextKey + Eq + std::hash::Hash,
    {
        self.try_sync_text_contexts(inputs)
            .unwrap_or_else(|error| panic!("text context synchronization failed: {error}"));
    }

    pub(super) fn try_sync_text_contexts<K>(
        &mut self,
        inputs: &HashMap<K, TextFlowInput>,
    ) -> Result<(), ContextSyncError>
    where
        K: TextContextKey + Eq + std::hash::Hash,
    {
        for (key, input) in inputs {
            let legacy_key = key.legacy_key(self);
            let identity = key
                .resolve_scope(self)
                .map_err(|_| ContextSyncError::Invariant {
                    node_id: None,
                    key: legacy_key,
                    source: IncrementalInvariantError::CompatibilityMapMismatch,
                })?
                .ok_or(ContextSyncError::Invariant {
                    node_id: None,
                    key: legacy_key,
                    source: IncrementalInvariantError::ScopedMapMismatch,
                })?;
            let node_id =
                self.vnode_map
                    .get(&identity)
                    .copied()
                    .ok_or(ContextSyncError::Invariant {
                        node_id: None,
                        key: legacy_key,
                        source: IncrementalInvariantError::ScopedMapMismatch,
                    })?;
            if self
                .taffy
                .get_node_context(node_id)
                .is_some_and(|context| context.matches(input, &self.text_flow_policy))
            {
                continue;
            }
            self.taffy
                .set_node_context(
                    node_id,
                    Some(NodeContext::new(
                        Some(input.clone()),
                        &self.text_flow_policy,
                    )),
                )
                .map_err(|source| ContextSyncError::Taffy {
                    node_id: Some(node_id),
                    key: legacy_key,
                    source,
                })?;
        }
        Ok(())
    }

    fn try_context_nodes(&self) -> Result<Vec<NodeId>, (NodeId, taffy::TaffyError)> {
        let Some(root) = self.root_node else {
            return Ok(Vec::new());
        };
        let mut reachable = Vec::new();
        let mut visited = HashSet::new();
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            if visited.insert(node) {
                reachable.push(node);
                pending.extend(self.taffy.children(node).map_err(|source| (node, source))?);
            }
        }
        Ok(reachable)
    }

    pub(super) fn run_layout_and_publish(
        &mut self,
        interrupted: &mut impl FnMut() -> bool,
    ) -> Result<(), TextFlowError> {
        match self.run_layout_and_publish_checked(interrupted) {
            Ok(()) => Ok(()),
            Err(
                LayoutRunError::TextFlow { source, .. }
                | LayoutRunError::ReadBackTextFlow { source, .. },
            ) => Err(source),
            Err(
                LayoutRunError::Taffy { source, .. } | LayoutRunError::ReadBackTaffy { source, .. },
            ) => {
                panic!("Taffy layout computation failed: {source}")
            }
            Err(LayoutRunError::Invariant { source, .. }) => {
                panic!("layout invariant failed: {source}")
            }
        }
    }

    pub(super) fn run_layout_and_publish_checked(
        &mut self,
        interrupted: &mut impl FnMut() -> bool,
    ) -> Result<(), LayoutRunError> {
        for node_id in
            self.try_context_nodes()
                .map_err(|(node_id, source)| LayoutRunError::Taffy {
                    node_id: Some(node_id),
                    source,
                })?
        {
            if let Some(context) = self.taffy.get_node_context_mut(node_id) {
                context.begin_frame();
            }
        }
        if let Some(root_node) = self.root_node {
            let cache = &mut self.flow_cache;
            let policy = &self.text_flow_policy;
            #[cfg(test)]
            let compute_result = if take_layout_compute_fault() {
                Err(taffy::TaffyError::InvalidInputNode(root_node))
            } else {
                self.taffy.compute_layout_with_measure(
                    root_node,
                    taffy::Size {
                        width: AvailableSpace::Definite(self.last_width as f32),
                        height: AvailableSpace::Definite(self.last_height as f32),
                    },
                    |known, available, _node_id, context, _style| {
                        measure_text_node(known, available, context, cache, policy, interrupted)
                    },
                )
            };
            #[cfg(not(test))]
            let compute_result = self.taffy.compute_layout_with_measure(
                root_node,
                taffy::Size {
                    width: AvailableSpace::Definite(self.last_width as f32),
                    height: AvailableSpace::Definite(self.last_height as f32),
                },
                |known, available, _node_id, context, _style| {
                    measure_text_node(known, available, context, cache, policy, interrupted)
                },
            );
            compute_result.map_err(|source| LayoutRunError::Taffy {
                node_id: Some(root_node),
                source,
            })?;
        }
        for node_id in self
            .try_context_nodes()
            .map_err(|(node_id, source)| LayoutRunError::read_back_taffy(node_id, source))?
        {
            let context = self
                .taffy
                .get_node_context(node_id)
                .ok_or(LayoutRunError::invariant(
                    node_id,
                    IncrementalInvariantError::CurrentFrameContextMismatch,
                ))?;
            if let Some(error) = context.first_error() {
                return Err(LayoutRunError::TextFlow {
                    node_id: Some(node_id),
                    source: error.clone(),
                });
            }
        }
        self.publish_final_flows(interrupted)
            .map_err(|source| source.into_read_back(None))
    }

    fn publish_final_flows(
        &mut self,
        interrupted: &mut impl FnMut() -> bool,
    ) -> Result<(), LayoutRunError> {
        let mut node_flows = HashMap::new();
        for node_id in self
            .try_context_nodes()
            .map_err(|(node_id, source)| LayoutRunError::read_back_taffy(node_id, source))?
        {
            if node_flows.contains_key(&node_id) {
                continue;
            }
            if let Some(flow) = self
                .flow_at_final_width(node_id, interrupted)
                .map_err(|source| source.into_read_back(Some(node_id)))?
            {
                node_flows.insert(node_id, flow);
            }
        }
        let mut element_flows = HashMap::new();
        for (element_id, node_id) in &self.node_map {
            if let Some(flow) = self.required_text_flow(*node_id, &node_flows)? {
                element_flows.insert(*element_id, flow);
            }
        }
        let mut vnode_flows = HashMap::new();
        for (identity, node_id) in self.vnode_map.iter() {
            if let Some(flow) = self.required_text_flow(*node_id, &node_flows)? {
                vnode_flows.insert(identity.clone(), flow);
            }
        }
        self.current_text_flows = element_flows;
        self.current_vnode_flows = vnode_flows.into();
        Ok(())
    }

    fn required_text_flow(
        &self,
        node_id: NodeId,
        node_flows: &HashMap<NodeId, Arc<TextFlow>>,
    ) -> Result<Option<Arc<TextFlow>>, LayoutRunError> {
        let context = self
            .taffy
            .get_node_context(node_id)
            .ok_or(LayoutRunError::invariant(
                node_id,
                IncrementalInvariantError::CurrentFrameContextMismatch,
            ))?;
        if !context.is_text() {
            return Ok(None);
        }
        node_flows
            .get(&node_id)
            .cloned()
            .map(Some)
            .ok_or(LayoutRunError::invariant(
                node_id,
                IncrementalInvariantError::CurrentFrameContextMismatch,
            ))
    }

    fn flow_at_final_width(
        &mut self,
        node_id: NodeId,
        interrupted: &mut impl FnMut() -> bool,
    ) -> Result<Option<Arc<TextFlow>>, LayoutRunError> {
        let context =
            self.taffy
                .get_node_context(node_id)
                .cloned()
                .ok_or(LayoutRunError::invariant(
                    node_id,
                    IncrementalInvariantError::CurrentFrameContextMismatch,
                ))?;
        if !context.is_text() {
            return Ok(None);
        }
        #[cfg(test)]
        if take_layout_read_back_fault() {
            return Err(LayoutRunError::Taffy {
                node_id: Some(node_id),
                source: taffy::TaffyError::InvalidInputNode(node_id),
            });
        }
        let layout = self.taffy.layout(node_id)?;
        let horizontal_inset =
            layout.padding.left + layout.padding.right + layout.border.left + layout.border.right;
        let width = (layout.size.width - horizontal_inset).max(0.0).floor() as usize;
        let flow = flow_for_width(
            &context,
            width,
            &mut self.flow_cache,
            &self.text_flow_policy,
            interrupted,
        )?;
        #[cfg(test)]
        if flow.is_some() && take_context_pin_fault() {
            self.taffy.set_node_context(node_id, None)?;
        }
        if let Some(flow) = &flow {
            let context =
                self.taffy
                    .get_node_context_mut(node_id)
                    .ok_or(LayoutRunError::invariant(
                        node_id,
                        IncrementalInvariantError::CurrentFrameContextMismatch,
                    ))?;
            context.pin_active_flow(flow);
        }
        Ok(flow)
    }
}

fn public_layout(layout: &taffy::Layout) -> Layout {
    Layout {
        x: layout.location.x,
        y: layout.location.y,
        width: layout.size.width,
        height: layout.size.height,
    }
}

mod measurements;
#[cfg(test)]
mod tests;
