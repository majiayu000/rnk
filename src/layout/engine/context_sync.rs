//! Incremental text-context synchronization and frame flow publication.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use taffy::{AvailableSpace, NodeId};

use super::{
    LayoutEngine,
    text_flow_bridge::{NodeContext, flow_for_width, measure_text_node},
};
use crate::layout::{TextFlow, TextFlowError, TextFlowInput};

impl LayoutEngine {
    pub(super) fn staged_clone(&self) -> Self {
        Self {
            taffy: self.taffy.clone(),
            node_map: self.node_map.clone(),
            element_keys: self.element_keys.clone(),
            vnode_map: self.vnode_map.clone(),
            root_node: self.root_node,
            last_width: self.last_width,
            last_height: self.last_height,
            flow_cache: self.flow_cache.clone(),
            text_flow_policy: self.text_flow_policy.clone(),
            current_text_flows: self.current_text_flows.clone(),
            current_vnode_flows: self.current_vnode_flows.clone(),
        }
    }

    pub(super) fn sync_text_contexts(
        &mut self,
        inputs: &HashMap<crate::core::NodeKey, TextFlowInput>,
    ) {
        for (key, input) in inputs {
            let Some(node_id) = self.vnode_map.get(&key.identity()).copied() else {
                continue;
            };
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
                .expect("mapped text node must remain in the Taffy tree");
        }
    }

    fn context_nodes(&self) -> Vec<NodeId> {
        let Some(root) = self.root_node else {
            return Vec::new();
        };
        let mut reachable = Vec::new();
        let mut visited = HashSet::new();
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            if visited.insert(node) {
                reachable.push(node);
                pending.extend(
                    self.taffy
                        .children(node)
                        .expect("reachable node must remain in the Taffy tree"),
                );
            }
        }
        reachable
    }

    pub(super) fn run_layout_and_publish(
        &mut self,
        interrupted: &mut impl FnMut() -> bool,
    ) -> Result<(), TextFlowError> {
        for node_id in self.context_nodes() {
            if let Some(context) = self.taffy.get_node_context_mut(node_id) {
                context.begin_frame();
            }
        }
        if let Some(root_node) = self.root_node {
            let cache = &mut self.flow_cache;
            let policy = &self.text_flow_policy;
            let _ = self.taffy.compute_layout_with_measure(
                root_node,
                taffy::Size {
                    width: AvailableSpace::Definite(self.last_width as f32),
                    height: AvailableSpace::Definite(self.last_height as f32),
                },
                |known, available, _node_id, context, _style| {
                    measure_text_node(known, available, context, cache, policy, interrupted)
                },
            );
        }
        for node_id in self.context_nodes() {
            if let Some(error) = self
                .taffy
                .get_node_context(node_id)
                .and_then(NodeContext::first_error)
            {
                return Err(error.clone());
            }
        }
        self.publish_final_flows(interrupted)
    }

    fn publish_final_flows(
        &mut self,
        interrupted: &mut impl FnMut() -> bool,
    ) -> Result<(), TextFlowError> {
        let mut node_flows = HashMap::new();
        for node_id in self.context_nodes() {
            if node_flows.contains_key(&node_id) {
                continue;
            }
            if let Some(flow) = self.flow_at_final_width(node_id, interrupted)? {
                node_flows.insert(node_id, flow);
            }
        }
        let element_flows = self
            .node_map
            .iter()
            .filter_map(|(element_id, node_id)| {
                Some((*element_id, Arc::clone(node_flows.get(node_id)?)))
            })
            .collect();
        let vnode_flows = self
            .vnode_map
            .iter()
            .filter_map(|(key, node_id)| Some((*key, Arc::clone(node_flows.get(node_id)?))))
            .collect();
        self.current_text_flows = element_flows;
        self.current_vnode_flows = vnode_flows;
        Ok(())
    }

    fn flow_at_final_width(
        &mut self,
        node_id: NodeId,
        interrupted: &mut impl FnMut() -> bool,
    ) -> Result<Option<Arc<TextFlow>>, TextFlowError> {
        let Some(context) = self.taffy.get_node_context(node_id).cloned() else {
            return Ok(None);
        };
        let Some(layout) = self.taffy.layout(node_id).ok() else {
            return Ok(None);
        };
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
        if let Some(flow) = &flow
            && let Some(context) = self.taffy.get_node_context_mut(node_id)
        {
            context.pin_active_flow(flow);
        }
        Ok(flow)
    }
}

#[cfg(test)]
mod tests;
