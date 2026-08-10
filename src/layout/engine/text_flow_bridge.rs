//! TextFlow input, caching, and Taffy measurement bridge.

use std::{collections::VecDeque, sync::Arc};

use taffy::AvailableSpace;

use crate::components::Line;
use crate::core::{Element, ElementType, Style, VNode, VNodeType};
use crate::layout::{
    StyledTextRange, TextFlow, TextFlowError, TextFlowInput, TextFlowOptions, TextFlowSourceKind,
    UnicodeWidthPolicy,
};

#[derive(Clone)]
pub(super) struct NodeContext {
    input: Option<TextFlowInput>,
    policy: TextFlowPolicy,
    first_error: Option<TextFlowError>,
    active_flow: Option<Arc<TextFlow>>,
    #[cfg(test)]
    last_measured_flow: Option<Arc<TextFlow>>,
}

impl NodeContext {
    pub(super) fn new(input: Option<TextFlowInput>, policy: &TextFlowPolicy) -> Self {
        Self {
            input,
            policy: policy.clone(),
            first_error: None,
            active_flow: None,
            #[cfg(test)]
            last_measured_flow: None,
        }
    }

    pub(super) fn input(&self) -> Option<&TextFlowInput> {
        self.input.as_ref()
    }

    pub(super) fn is_text(&self) -> bool {
        self.input.is_some()
    }

    pub(super) fn matches(&self, input: &TextFlowInput, policy: &TextFlowPolicy) -> bool {
        self.input
            .as_ref()
            .is_some_and(|current| same_text_flow_input(current, input))
            && self.policy == *policy
    }

    pub(super) fn begin_frame(&mut self) {
        self.first_error = None;
        self.active_flow = None;
        #[cfg(test)]
        {
            self.last_measured_flow = None;
        }
    }

    pub(super) fn first_error(&self) -> Option<&TextFlowError> {
        self.first_error.as_ref()
    }

    pub(super) fn active_flow(&self) -> Option<&Arc<TextFlow>> {
        self.active_flow.as_ref()
    }

    pub(super) fn pin_active_flow(&mut self, flow: &Arc<TextFlow>) {
        self.active_flow = Some(Arc::clone(flow));
    }

    fn pin_measured_flow(&mut self, flow: &Arc<TextFlow>) {
        self.pin_active_flow(flow);
        #[cfg(test)]
        {
            self.last_measured_flow = Some(Arc::clone(flow));
        }
    }

    #[cfg(test)]
    pub(super) fn last_measured_flow(&self) -> Option<&Arc<TextFlow>> {
        self.last_measured_flow.as_ref()
    }

    fn record_error(&mut self, error: TextFlowError) {
        if self.first_error.is_none() {
            self.first_error = Some(error);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub(super) fn options(&self, input: &TextFlowInput, max_width: usize) -> TextFlowOptions {
        let mut options = TextFlowOptions::new(max_width, input.default_style.text_wrap);
        options.overflow_x = input.default_style.overflow_x;
        options.overflow_y = input.default_style.overflow_y;
        options.tab_stop = self.tab_stop;
        options.ellipsis.clone_from(&self.ellipsis);
        options.width_policy = self.width_policy;
        options
    }
}

/// Engine-local logical cache with deterministic FIFO eviction.
///
/// Hits compare complete identity; a fixed limit bounds retained history.
#[derive(Clone, Default)]
pub(super) struct FlowCache {
    entries: VecDeque<Arc<TextFlow>>,
    successful_recomputes: u64,
    successful_hits: u64,
}

impl FlowCache {
    pub(super) const MAX_ENTRIES: usize = 64;

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(super) const fn successful_recomputes(&self) -> u64 {
        self.successful_recomputes
    }

    pub(super) const fn successful_hits(&self) -> u64 {
        self.successful_hits
    }

    pub(super) fn get_or_compute(
        &mut self,
        input: &TextFlowInput,
        options: &TextFlowOptions,
        interrupted: &mut impl FnMut() -> bool,
    ) -> Result<Arc<TextFlow>, TextFlowError> {
        if interrupted() {
            return Err(TextFlowError::Interrupted);
        }
        if let Some(flow) = self
            .entries
            .iter()
            .find(|flow| {
                same_text_flow_input(&flow.cache_identity().input, input)
                    && flow.cache_identity().options == *options
            })
            .cloned()
        {
            self.successful_hits = self
                .successful_hits
                .checked_add(1)
                .ok_or(TextFlowError::ArithmeticOverflow)?;
            return Ok(flow);
        }
        let flow = Arc::new(TextFlow::try_build_interruptible(
            input,
            options,
            interrupted,
        )?);
        self.successful_recomputes = self
            .successful_recomputes
            .checked_add(1)
            .ok_or(TextFlowError::ArithmeticOverflow)?;
        if self.entries.len() >= Self::MAX_ENTRIES {
            self.entries.pop_front();
        }
        self.entries.push_back(Arc::clone(&flow));
        Ok(flow)
    }
}

pub(super) fn input_from_element(element: &Element) -> Option<TextFlowInput> {
    if element.element_type != ElementType::Text {
        return None;
    }
    Some(match (&element.text_content, &element.spans) {
        (Some(source), Some(lines)) => aligned_input(source, lines, &element.style),
        (Some(source), None) => {
            TextFlowInput::plain(source, TextFlowSourceKind::Exact, element.style.clone())
        }
        (None, Some(lines)) => canonical_span_input(lines, &element.style),
        (None, None) => TextFlowInput::plain("", TextFlowSourceKind::Exact, element.style.clone()),
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

pub(super) fn same_text_flow_input(actual: &TextFlowInput, expected: &TextFlowInput) -> bool {
    actual == expected || format!("{actual:?}") == format!("{expected:?}")
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
    let mut cursor = 0;
    let mut ranges = Vec::new();

    for (line_index, line) in lines.iter().enumerate() {
        for span in &line.spans {
            let start = cursor;
            if !consume_aligned_fragment(source, &mut cursor, &span.content) {
                return reconstructed_input(source, style);
            }
            ranges.push(StyledTextRange {
                range: start..cursor,
                style: style.clone().merge(&span.style),
            });
        }
        if line_index + 1 < lines.len() {
            let Some(break_len) = hard_break_prefix_len(&source[cursor..]) else {
                return reconstructed_input(source, style);
            };
            ranges.push(StyledTextRange {
                range: cursor..cursor + break_len,
                style: style.clone(),
            });
            cursor += break_len;
        }
    }

    while cursor < source.len() {
        let Some(break_len) = hard_break_prefix_len(&source[cursor..]) else {
            return reconstructed_input(source, style);
        };
        cursor += break_len;
    }
    TextFlowInput::plain(source, TextFlowSourceKind::Exact, style.clone())
        .with_styled_ranges(ranges)
}

fn canonical_span_input(lines: &[Line], style: &Style) -> TextFlowInput {
    let mut source = String::new();
    let mut ranges = Vec::new();

    for (line_index, line) in lines.iter().enumerate() {
        for span in &line.spans {
            let start = source.len();
            source.push_str(&span.content);
            ranges.push(StyledTextRange {
                range: start..source.len(),
                style: style.clone().merge(&span.style),
            });
        }
        if line_index + 1 < lines.len() {
            let start = source.len();
            source.push('\n');
            ranges.push(StyledTextRange {
                range: start..source.len(),
                style: style.clone(),
            });
        }
    }

    TextFlowInput::plain(source, TextFlowSourceKind::Canonical, style.clone())
        .with_styled_ranges(ranges)
}

fn consume_aligned_fragment(source: &str, source_cursor: &mut usize, visible: &str) -> bool {
    let mut visible_cursor = 0;
    while visible_cursor < visible.len() {
        let source_rest = &source[*source_cursor..];
        let visible_rest = &visible[visible_cursor..];
        if let (Some(source_break), Some(visible_break)) = (
            hard_break_prefix_len(source_rest),
            hard_break_prefix_len(visible_rest),
        ) {
            *source_cursor += source_break;
            visible_cursor += visible_break;
            continue;
        }
        let Some(visible_scalar) = visible_rest.chars().next() else {
            return false;
        };
        if !source_rest.starts_with(visible_scalar) {
            return false;
        }
        *source_cursor += visible_scalar.len_utf8();
        visible_cursor += visible_scalar.len_utf8();
    }
    true
}

fn reconstructed_input(source: &str, style: &Style) -> TextFlowInput {
    TextFlowInput::plain(source, TextFlowSourceKind::Reconstructed, style.clone())
}

fn hard_break_prefix_len(text: &str) -> Option<usize> {
    if text.starts_with("\r\n") {
        Some(2)
    } else if text.starts_with(['\r', '\n']) {
        Some(1)
    } else {
        None
    }
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
    context.pin_measured_flow(&flow);
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
    if let Some(flow) = context.active_flow().filter(|flow| {
        same_text_flow_input(&flow.cache_identity().input, input)
            && flow.cache_identity().options == options
    }) {
        return Ok(Some(Arc::clone(flow)));
    }
    cache.get_or_compute(input, &options, interrupted).map(Some)
}

#[cfg(test)]
mod tests;
