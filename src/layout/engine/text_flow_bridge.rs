//! TextFlow input, caching, and Taffy measurement bridge.

use std::{collections::HashMap, sync::Arc};

use taffy::{AvailableSpace, NodeId};
use unicode_segmentation::UnicodeSegmentation;

use crate::components::Line;
use crate::core::{Element, ElementType, Style, VNode, VNodeType};
use crate::layout::{
    StyledTextRange, TextFlow, TextFlowError, TextFlowInput, TextFlowOptions, TextFlowSourceKind,
    UnicodeWidthPolicy,
};

use super::LayoutEngine;

#[derive(Clone)]
pub(super) struct NodeContext {
    input: Option<TextFlowInput>,
    first_error: Option<TextFlowError>,
}

impl NodeContext {
    pub(super) fn new(input: Option<TextFlowInput>) -> Self {
        Self {
            input,
            first_error: None,
        }
    }

    pub(super) fn input(&self) -> Option<&TextFlowInput> {
        self.input.as_ref()
    }

    pub(super) fn is_text(&self) -> bool {
        self.input.is_some()
    }

    pub(super) fn clear_error(&mut self) {
        self.first_error = None;
    }

    pub(super) fn first_error(&self) -> Option<&TextFlowError> {
        self.first_error.as_ref()
    }

    fn record_error(&mut self, error: TextFlowError) {
        if self.first_error.is_none() {
            self.first_error = Some(error);
        }
    }
}

#[derive(Clone)]
pub(super) struct TextFlowPolicy {
    tab_stop: usize,
    ellipsis: String,
    width_policy: UnicodeWidthPolicy,
}

impl Default for TextFlowPolicy {
    fn default() -> Self {
        Self {
            tab_stop: 4,
            ellipsis: "…".to_string(),
            width_policy: UnicodeWidthPolicy { revision: 1 },
        }
    }
}

impl TextFlowPolicy {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn set(&mut self, tab_stop: usize, ellipsis: impl Into<String>, revision: u16) {
        self.tab_stop = tab_stop;
        self.ellipsis = ellipsis.into();
        self.width_policy = UnicodeWidthPolicy { revision };
    }

    fn options(&self, input: &TextFlowInput, max_width: usize) -> TextFlowOptions {
        let mut options = TextFlowOptions::new(max_width, input.default_style.text_wrap);
        options.overflow_x = input.default_style.overflow_x;
        options.overflow_y = input.default_style.overflow_y;
        options.tab_stop = self.tab_stop;
        options.ellipsis.clone_from(&self.ellipsis);
        options.width_policy = self.width_policy;
        options
    }
}

/// Engine-local logical cache. Hits always compare the complete identity.
#[derive(Clone, Default)]
pub(super) struct FlowCache {
    entries: Vec<Arc<TextFlow>>,
}

impl FlowCache {
    pub(super) fn get_or_compute(
        &mut self,
        input: &TextFlowInput,
        options: &TextFlowOptions,
        interrupted: &mut impl FnMut() -> bool,
    ) -> Result<Arc<TextFlow>, TextFlowError> {
        if interrupted() {
            return Err(TextFlowError::Interrupted);
        }
        if let Some(flow) = self.entries.iter().find(|flow| {
            flow.cache_identity().input == *input && flow.cache_identity().options == *options
        }) {
            return Ok(Arc::clone(flow));
        }
        let flow = Arc::new(TextFlow::try_build_interruptible(
            input,
            options,
            interrupted,
        )?);
        self.entries.push(Arc::clone(&flow));
        Ok(flow)
    }
}

pub(super) fn input_from_element(element: &Element) -> Option<TextFlowInput> {
    if element.element_type != ElementType::Text {
        return None;
    }
    let source = element.text_content.as_deref().unwrap_or_default();
    Some(match &element.spans {
        Some(lines) => aligned_input(source, lines, &element.style),
        None => TextFlowInput::plain(source, TextFlowSourceKind::Exact, element.style.clone()),
    })
}

pub(super) fn input_from_vnode(vnode: &VNode) -> Option<TextFlowInput> {
    match &vnode.node_type {
        VNodeType::Text(source) => Some(TextFlowInput::plain(
            source.clone(),
            TextFlowSourceKind::Exact,
            vnode.props.style.clone(),
        )),
        _ => None,
    }
}

pub(super) fn compatibility_text(element: &Element) -> String {
    let Some(lines) = &element.spans else {
        return element.text_content.clone().unwrap_or_default();
    };
    lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_str())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn aligned_input(source: &str, lines: &[Line], style: &Style) -> TextFlowInput {
    let source_parts: Vec<_> = source
        .grapheme_indices(true)
        .map(|(start, text)| (text, start..start + text.len()))
        .collect();
    let mut cursor = 0;
    let mut ranges = Vec::new();

    for (line_index, line) in lines.iter().enumerate() {
        for span in &line.spans {
            let start = source_parts
                .get(cursor)
                .map_or(source.len(), |(_, range)| range.start);
            for grapheme in span.content.graphemes(true) {
                let Some((source_grapheme, _)) = source_parts.get(cursor) else {
                    return reconstructed_input(source, style);
                };
                if !graphemes_align(source_grapheme, grapheme) {
                    return reconstructed_input(source, style);
                }
                cursor += 1;
            }
            let end = source_parts
                .get(cursor)
                .map_or(source.len(), |(_, range)| range.start);
            ranges.push(StyledTextRange {
                range: start..end,
                style: style.clone().merge(&span.style),
            });
        }
        if line_index + 1 < lines.len() {
            let Some((source_grapheme, range)) = source_parts.get(cursor) else {
                return reconstructed_input(source, style);
            };
            if !is_hard_break(source_grapheme) {
                return reconstructed_input(source, style);
            }
            ranges.push(StyledTextRange {
                range: range.clone(),
                style: style.clone(),
            });
            cursor += 1;
        }
    }

    if source_parts[cursor..]
        .iter()
        .any(|(grapheme, _)| !is_hard_break(grapheme))
    {
        return reconstructed_input(source, style);
    }
    TextFlowInput::plain(source, TextFlowSourceKind::Exact, style.clone())
        .with_styled_ranges(ranges)
}

fn reconstructed_input(source: &str, style: &Style) -> TextFlowInput {
    TextFlowInput::plain(source, TextFlowSourceKind::Reconstructed, style.clone())
}

fn graphemes_align(source: &str, visible: &str) -> bool {
    source == visible || is_hard_break(source) && is_hard_break(visible)
}

fn is_hard_break(grapheme: &str) -> bool {
    matches!(grapheme, "\n" | "\r" | "\r\n")
}

#[derive(Clone, Copy)]
enum EffectiveWidth {
    Resolved(f32),
    Available(f32),
    Intrinsic,
}

impl EffectiveWidth {
    fn max_width(self) -> usize {
        match self {
            Self::Resolved(width) | Self::Available(width) => width.max(0.0).floor() as usize,
            Self::Intrinsic => usize::MAX,
        }
    }
}

pub(super) fn measure_text_node(
    known: taffy::Size<Option<f32>>,
    available: taffy::Size<AvailableSpace>,
    context: Option<&mut NodeContext>,
    cache: &mut FlowCache,
    policy: &TextFlowPolicy,
    interrupted: &mut impl FnMut() -> bool,
) -> taffy::Size<f32> {
    let sentinel = taffy::Size {
        width: known.width.unwrap_or(0.0),
        height: known.height.unwrap_or(0.0),
    };
    let Some(context) = context else {
        return sentinel;
    };
    let Some(input) = context.input() else {
        return sentinel;
    };
    let effective = match (known.width, available.width) {
        (Some(width), _) => EffectiveWidth::Resolved(width),
        (None, AvailableSpace::Definite(width)) => EffectiveWidth::Available(width),
        (None, AvailableSpace::MinContent | AvailableSpace::MaxContent) => {
            EffectiveWidth::Intrinsic
        }
    };
    let options = policy.options(input, effective.max_width());
    let flow = match cache.get_or_compute(input, &options, interrupted) {
        Ok(flow) => flow,
        Err(error) => {
            context.record_error(error);
            return taffy::Size::zero();
        }
    };
    let width = match effective {
        EffectiveWidth::Resolved(width) => width,
        EffectiveWidth::Available(width) => (flow.max_row_width() as f32).min(width),
        EffectiveWidth::Intrinsic => flow.max_row_width() as f32,
    };
    taffy::Size {
        width,
        height: known.height.unwrap_or(flow.row_count() as f32),
    }
}

pub(super) fn flow_for_width(
    context: &NodeContext,
    width: usize,
    cache: &mut FlowCache,
    policy: &TextFlowPolicy,
    interrupted: &mut impl FnMut() -> bool,
) -> Result<Option<Arc<TextFlow>>, TextFlowError> {
    let Some(input) = context.input() else {
        return Ok(None);
    };
    let options = policy.options(input, width);
    cache.get_or_compute(input, &options, interrupted).map(Some)
}

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
            let Some(node_id) = self.vnode_map.get(key).copied() else {
                continue;
            };
            self.taffy
                .set_node_context(node_id, Some(NodeContext::new(Some(input.clone()))))
                .expect("mapped text node must remain in the Taffy tree");
        }
    }

    fn context_nodes(&self) -> Vec<NodeId> {
        self.node_map
            .values()
            .chain(self.vnode_map.values())
            .copied()
            .collect()
    }

    pub(super) fn run_layout_and_publish(
        &mut self,
        interrupted: &mut impl FnMut() -> bool,
    ) -> Result<(), TextFlowError> {
        for node_id in self.context_nodes() {
            if let Some(context) = self.taffy.get_node_context_mut(node_id) {
                context.clear_error();
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
        let mut element_flows = HashMap::new();
        let mut vnode_flows = HashMap::new();
        let elements: Vec<_> = self
            .node_map
            .iter()
            .map(|(element_id, node_id)| (*element_id, *node_id))
            .collect();
        let vnodes: Vec<_> = self
            .vnode_map
            .iter()
            .map(|(key, node_id)| (*key, *node_id))
            .collect();
        for (element_id, node_id) in elements {
            if let Some(flow) = self.flow_at_final_width(node_id, interrupted)? {
                element_flows.insert(element_id, flow);
            }
        }
        for (key, node_id) in vnodes {
            if let Some(flow) = self.flow_at_final_width(node_id, interrupted)? {
                vnode_flows.insert(key, flow);
            }
        }
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
        flow_for_width(
            &context,
            width,
            &mut self.flow_cache,
            &self.text_flow_policy,
            interrupted,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Color, Overflow, TextWrap};

    #[test]
    fn engine_cache_compares_every_logical_identity_value() {
        let input = TextFlowInput::plain("ab", TextFlowSourceKind::Exact, Style::new())
            .with_styled_ranges(vec![StyledTextRange {
                range: 0..1,
                style: Style::new(),
            }]);
        let mut options = TextFlowOptions::new(8, TextWrap::Wrap);
        let mut cache = FlowCache::default();
        let baseline = cache
            .get_or_compute(&input, &options, &mut || false)
            .unwrap();
        let reused = cache
            .get_or_compute(&input, &options, &mut || false)
            .unwrap();
        assert!(Arc::ptr_eq(&baseline, &reused));

        let mut identities = Vec::new();
        let mut changed_input = input.clone();
        changed_input.source = "ac".into();
        identities.push((changed_input, options.clone()));
        let mut changed_input = input.clone();
        changed_input.source_kind = TextFlowSourceKind::Canonical;
        identities.push((changed_input, options.clone()));
        let mut changed_input = input.clone();
        changed_input.default_style.bold = true;
        identities.push((changed_input, options.clone()));
        let mut changed_input = input.clone();
        changed_input.styled_ranges[0].style.color = Some(Color::Red);
        identities.push((changed_input, options.clone()));
        let mut changed_input = input.clone();
        changed_input.styled_ranges[0].range = 1..2;
        identities.push((changed_input, options.clone()));

        options.max_width = 7;
        identities.push((input.clone(), options.clone()));
        options.max_width = 8;
        options.text_wrap = TextWrap::TruncateEnd;
        identities.push((input.clone(), options.clone()));
        options.text_wrap = TextWrap::Wrap;
        options.overflow_x = Overflow::Hidden;
        identities.push((input.clone(), options.clone()));
        options.overflow_x = Overflow::Visible;
        options.overflow_y = Overflow::Scroll;
        identities.push((input.clone(), options.clone()));
        options.overflow_y = Overflow::Visible;
        options.tab_stop = 3;
        identities.push((input.clone(), options.clone()));
        options.tab_stop = 4;
        options.ellipsis = "..".into();
        identities.push((input.clone(), options.clone()));
        options.ellipsis = "…".into();
        options.width_policy.revision = 2;
        identities.push((input.clone(), options));

        for (changed_input, changed_options) in identities {
            let changed = cache
                .get_or_compute(&changed_input, &changed_options, &mut || false)
                .unwrap();
            assert!(!Arc::ptr_eq(&baseline, &changed));
        }
    }
}
