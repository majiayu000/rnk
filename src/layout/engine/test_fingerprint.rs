//! Test-only complete committed-engine fingerprint.

use std::{collections::HashMap, sync::Arc};

use taffy::NodeId;

use crate::core::{ElementId, NodeKey, VNode};
use crate::layout::{TextFlow, TextFlowError, TextFlowInput};
use crate::reconciler::ScopedNodeIdentity;

use super::{LayoutEngine, text_flow_bridge::TextFlowPolicy};

#[derive(Clone, Debug, PartialEq)]
struct BackendNodeFingerprint {
    children: Vec<NodeId>,
    style: taffy::Style,
    layout: taffy::Layout,
    dirty: bool,
    input: Option<TextFlowInput>,
    first_error: Option<TextFlowError>,
    active_flow: Option<usize>,
    measured_flow: Option<usize>,
}

/// Full observable committed state used by atomicity tests.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct EngineFingerprint {
    root: Option<NodeId>,
    total_node_count: usize,
    backend: HashMap<NodeId, BackendNodeFingerprint>,
    node_map: HashMap<ElementId, NodeId>,
    element_keys: HashMap<ElementId, NodeKey>,
    element_scopes: HashMap<ElementId, ScopedNodeIdentity>,
    vnode_map: HashMap<ScopedNodeIdentity, NodeId>,
    vnode_legacy_keys: HashMap<ScopedNodeIdentity, NodeKey>,
    last_width: u16,
    last_height: u16,
    flow_cache_len: usize,
    text_flow_policy: TextFlowPolicy,
    current_text_flows: HashMap<ElementId, usize>,
    current_vnode_flows: HashMap<ScopedNodeIdentity, usize>,
    committed_vnode: Option<VNode>,
}

impl EngineFingerprint {
    pub(super) fn capture(engine: &LayoutEngine) -> Self {
        let mut backend = HashMap::with_capacity(engine.vnode_map.len());
        for node_id in engine.vnode_map.values().copied() {
            let context = engine
                .taffy
                .get_node_context(node_id)
                .expect("committed mapped node has context");
            backend.insert(
                node_id,
                BackendNodeFingerprint {
                    children: engine
                        .taffy
                        .children(node_id)
                        .expect("committed mapped node has children"),
                    style: engine
                        .taffy
                        .style(node_id)
                        .expect("committed mapped node has style")
                        .clone(),
                    layout: *engine
                        .taffy
                        .layout(node_id)
                        .expect("committed mapped node has layout"),
                    dirty: engine
                        .taffy
                        .dirty(node_id)
                        .expect("committed mapped node has dirty state"),
                    input: context.input().cloned(),
                    first_error: context.first_error().cloned(),
                    active_flow: context.active_flow().map(flow_address),
                    measured_flow: context.last_measured_flow().map(flow_address),
                },
            );
        }
        Self {
            root: engine.root_node,
            total_node_count: engine.taffy.total_node_count(),
            backend,
            node_map: engine.node_map.clone(),
            element_keys: engine.element_keys.clone(),
            element_scopes: engine.element_scopes.clone(),
            vnode_map: (*engine.vnode_map).clone(),
            vnode_legacy_keys: (*engine.vnode_legacy_keys).clone(),
            last_width: engine.last_width,
            last_height: engine.last_height,
            flow_cache_len: engine.flow_cache.len(),
            text_flow_policy: engine.text_flow_policy.clone(),
            current_text_flows: engine
                .current_text_flows
                .iter()
                .map(|(element_id, flow)| (*element_id, flow_address(flow)))
                .collect(),
            current_vnode_flows: engine
                .current_vnode_flows
                .iter()
                .map(|(identity, flow)| (identity.clone(), flow_address(flow)))
                .collect(),
            committed_vnode: (*engine.committed_vnode).clone(),
        }
    }
}

fn flow_address(flow: &Arc<TextFlow>) -> usize {
    Arc::as_ptr(flow) as usize
}
