use super::super::LayoutEngine;
use super::*;
use crate::{
    components::{Line, Span},
    core::{Color, Dimension, Overflow, TextWrap},
    layout::TextFlowDiagnostic,
    reconciler::Patch,
};

fn vnode_node_id(engine: &LayoutEngine, key: crate::core::NodeKey) -> taffy::NodeId {
    let identity = engine
        .resolve_legacy_scope(key)
        .expect("legacy key must be unambiguous")
        .expect("legacy key must be mapped");
    engine.vnode_map[&identity]
}

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
#[test]
fn engine_cache_is_bounded_with_deterministic_fifo_eviction() {
    let options = TextFlowOptions::new(8, TextWrap::Wrap);
    let mut cache = FlowCache::default();
    let first_input = TextFlowInput::plain("entry-0", TextFlowSourceKind::Exact, Style::new());
    let first = cache
        .get_or_compute(&first_input, &options, &mut || false)
        .unwrap();
    let mut oldest_retained = None;
    for index in 1..=FlowCache::MAX_ENTRIES {
        let input = TextFlowInput::plain(
            format!("entry-{index}"),
            TextFlowSourceKind::Exact,
            Style::new(),
        );
        let flow = cache
            .get_or_compute(&input, &options, &mut || false)
            .unwrap();
        if index == 1 {
            oldest_retained = Some(flow);
        }
    }
    assert_eq!(cache.entries.len(), FlowCache::MAX_ENTRIES);
    let newest_input = TextFlowInput::plain(
        format!("entry-{}", FlowCache::MAX_ENTRIES),
        TextFlowSourceKind::Exact,
        Style::new(),
    );
    let newest = cache
        .get_or_compute(&newest_input, &options, &mut || false)
        .unwrap();
    let newest_again = cache
        .get_or_compute(&newest_input, &options, &mut || false)
        .unwrap();
    assert!(Arc::ptr_eq(&newest, &newest_again));
    let first_again = cache
        .get_or_compute(&first_input, &options, &mut || false)
        .unwrap();
    assert!(!Arc::ptr_eq(&first, &first_again));
    let second_input = TextFlowInput::plain("entry-1", TextFlowSourceKind::Exact, Style::new());
    let second_again = cache
        .get_or_compute(&second_input, &options, &mut || false)
        .unwrap();
    assert!(!Arc::ptr_eq(
        oldest_retained.as_ref().unwrap(),
        &second_again
    ));
    assert_eq!(cache.entries.len(), FlowCache::MAX_ENTRIES);
}

fn identity_element(width: f32, overflow_x: Overflow) -> Element {
    let mut element = Element::text("a\tbcdef").with_key("identity");
    element.style.width = Dimension::Points(width);
    element.style.text_wrap = TextWrap::TruncateEnd;
    element.style.overflow_x = overflow_x;
    element
}

fn publish_incrementally(
    engine: &mut LayoutEngine,
    previous: Option<&VNode>,
    element: &Element,
    expected_patch_count: usize,
) -> (VNode, Arc<TextFlow>) {
    let (current, outcome) = engine
        .try_compute_element_incremental(element, previous, 40, 4)
        .unwrap();
    assert_eq!(outcome.patch_count, expected_patch_count);
    assert_eq!(outcome.used_reconciler, previous.is_some());
    let flow = engine.current_text_flow(element.id).unwrap();
    (current, flow)
}

#[test]
fn incremental_publication_changes_each_engine_cache_input_independently() {
    let mut engine = LayoutEngine::new();
    let initial_element = identity_element(5.0, Overflow::Visible);
    let (initial_vnode, initial) = publish_incrementally(&mut engine, None, &initial_element, 0);
    let width_element = identity_element(4.0, Overflow::Visible);
    let (width_vnode, width) =
        publish_incrementally(&mut engine, Some(&initial_vnode), &width_element, 1);
    let mut expected_input = initial.cache_identity().input.clone();
    expected_input.default_style.width = Dimension::Points(4.0);
    let mut expected = initial.cache_identity().options.clone();
    expected.max_width = 4;
    assert_eq!(width.cache_identity().input, expected_input);
    assert_eq!(width.cache_identity().options, expected);
    assert!(!Arc::ptr_eq(&initial, &width));
    let overflow_element = identity_element(4.0, Overflow::Hidden);
    let (overflow_vnode, overflow) =
        publish_incrementally(&mut engine, Some(&width_vnode), &overflow_element, 1);
    expected_input.default_style.overflow_x = Overflow::Hidden;
    expected.overflow_x = Overflow::Hidden;
    assert_eq!(overflow.cache_identity().input, expected_input);
    assert_eq!(overflow.cache_identity().options, expected);
    assert!(!Arc::ptr_eq(&width, &overflow));
    engine.set_text_flow_policy(2, "…", 1);
    let tab_element = identity_element(4.0, Overflow::Hidden);
    let (tab_vnode, tab) =
        publish_incrementally(&mut engine, Some(&overflow_vnode), &tab_element, 0);
    expected.tab_stop = 2;
    assert_eq!(tab.cache_identity().input, overflow.cache_identity().input);
    assert_eq!(tab.cache_identity().options, expected);
    assert!(!Arc::ptr_eq(&overflow, &tab));
    assert_ne!(tab.rows(), overflow.rows());
    engine.set_text_flow_policy(2, "..", 1);
    let ellipsis_element = identity_element(4.0, Overflow::Hidden);
    let (_, ellipsis) = publish_incrementally(&mut engine, Some(&tab_vnode), &ellipsis_element, 0);
    expected.ellipsis = "..".into();
    assert_eq!(ellipsis.cache_identity().input, tab.cache_identity().input);
    assert_eq!(ellipsis.cache_identity().options, expected);
    assert!(!Arc::ptr_eq(&tab, &ellipsis));
    assert_ne!(ellipsis.rows(), tab.rows());
}

fn split_style_element(source: &str, first: &str, second: &str) -> Element {
    let mut element = Element::text(source);
    element.spans = Some(vec![Line::from_spans(vec![
        Span::new(first).color(Color::Red),
        Span::new(second).color(Color::Blue),
    ])]);
    element
}

#[test]
fn split_combining_span_boundary_preserves_first_source_style() {
    let element = split_style_element("e\u{301}", "e", "\u{301}");
    let id = element.id;
    let mut engine = LayoutEngine::new();
    engine.try_compute(&element, 8, 2).unwrap();
    let flow = engine.current_text_flow(id).unwrap();
    assert_eq!(
        flow.cache_identity().input.source_kind,
        TextFlowSourceKind::Exact
    );
    assert_eq!(flow.tokens().len(), 1);
    assert_eq!(flow.tokens()[0].style.color, Some(Color::Red));
    assert!(flow.diagnostics().iter().any(|diagnostic| matches!(
        diagnostic,
        TextFlowDiagnostic::StyleBoundaryNormalized { boundary: 1, .. }
    )));
}

#[test]
fn split_zwj_span_boundary_preserves_first_source_style() {
    let element = split_style_element("👩‍💻", "👩", "\u{200d}💻");
    let id = element.id;
    let mut engine = LayoutEngine::new();
    engine.try_compute(&element, 8, 2).unwrap();
    let flow = engine.current_text_flow(id).unwrap();
    assert_eq!(
        flow.cache_identity().input.source_kind,
        TextFlowSourceKind::Exact
    );
    assert_eq!(flow.tokens().len(), 1);
    assert_eq!(flow.tokens()[0].style.color, Some(Color::Red));
    assert!(flow.diagnostics().iter().any(|diagnostic| matches!(
        diagnostic,
        TextFlowDiagnostic::StyleBoundaryNormalized { boundary, .. }
            if *boundary == "👩".len()
    )));
}

#[test]
fn span_only_lines_publish_canonical_source_and_merged_styles() {
    let mut element = Element::new(ElementType::Text);
    element.style.bold = true;
    element.spans = Some(vec![
        Line::from_spans(vec![
            Span::new("left").color(Color::Red),
            Span::new(" right").color(Color::Blue),
        ]),
        Line::new(),
        Line::from(Span::new("tail").color(Color::Green)),
    ]);

    let input = input_from_element(&element).unwrap();

    assert_eq!(input.source, "left right\n\ntail");
    assert_eq!(input.source_kind, TextFlowSourceKind::Canonical);
    assert_eq!(
        input
            .styled_ranges
            .iter()
            .map(|range| (range.range.clone(), range.style.color, range.style.bold))
            .collect::<Vec<_>>(),
        vec![
            (0..4, Some(Color::Red), true),
            (4..10, Some(Color::Blue), true),
            (10..11, None, true),
            (11..12, None, true),
            (12..16, Some(Color::Green), true),
        ]
    );
}

#[test]
fn span_only_source_preserves_crlf_combining_and_zwj_bytes() {
    let mut element = Element::new(ElementType::Text);
    element.spans = Some(vec![Line::from_spans(vec![
        Span::new("a\r\n").color(Color::Red),
        Span::new("e").color(Color::Blue),
        Span::new("\u{301}").color(Color::Yellow),
        Span::new("👩").color(Color::Green),
        Span::new("\u{200d}💻").color(Color::Cyan),
    ])]);

    let input = input_from_element(&element).unwrap();

    assert_eq!(input.source, "a\r\ne\u{301}👩\u{200d}💻");
    assert_eq!(input.source_kind, TextFlowSourceKind::Canonical);
    assert_eq!(input.styled_ranges.len(), 5);
    assert_eq!(input.styled_ranges[0].range, 0..3);
    assert_eq!(input.styled_ranges[1].range, 3..4);
    assert_eq!(input.styled_ranges[2].range, 4..6);
    assert_eq!(input.styled_ranges[3].range, 6..10);
    assert_eq!(input.styled_ranges[4].range, 10..17);

    let mut engine = LayoutEngine::new();
    engine.try_compute(&element, 8, 3).unwrap();
    let flow = engine.current_text_flow(element.id).unwrap();
    let combining = flow
        .tokens()
        .iter()
        .find(|token| token.safe_text == "e\u{301}")
        .unwrap();
    let zwj = flow
        .tokens()
        .iter()
        .find(|token| token.safe_text == "👩\u{200d}💻")
        .unwrap();
    assert_eq!(combining.style.color, Some(Color::Blue));
    assert_eq!(zwj.style.color, Some(Color::Green));
    assert!(flow.row_count() >= 2);
}

#[test]
fn present_text_source_keeps_exact_and_reconstructed_policies() {
    let exact = split_style_element("ab", "a", "b");
    let exact_input = input_from_element(&exact).unwrap();
    assert_eq!(exact_input.source, "ab");
    assert_eq!(exact_input.source_kind, TextFlowSourceKind::Exact);
    assert_eq!(exact_input.styled_ranges.len(), 2);

    let inconsistent = split_style_element("source", "different", "");
    let reconstructed_input = input_from_element(&inconsistent).unwrap();
    assert_eq!(reconstructed_input.source, "source");
    assert_eq!(
        reconstructed_input.source_kind,
        TextFlowSourceKind::Reconstructed
    );
    assert!(reconstructed_input.styled_ranges.is_empty());
}

#[test]
fn text_without_source_or_spans_remains_empty_and_exact() {
    let element = Element::new(ElementType::Text);

    let input = input_from_element(&element).unwrap();

    assert!(input.source.is_empty());
    assert_eq!(input.source_kind, TextFlowSourceKind::Exact);
    assert!(input.styled_ranges.is_empty());
}

#[test]
fn replace_and_reorder_preserve_only_live_flows() {
    let old_leaf = VNode::text("old").with_key("old-leaf");
    let old_leaf_key = old_leaf.key;
    let old_branch = VNode::box_node().with_key("branch").child(old_leaf);
    let old_branch_key = old_branch.key;
    let root = VNode::box_node().children([old_branch, VNode::text("keep").with_key("keep")]);
    let sibling_key = root.children[1].key;
    let mut engine = LayoutEngine::new();
    engine.compute_vnode(&root, 20, 4);
    let old_branch_node = vnode_node_id(&engine, old_branch_key);
    let sibling_node = vnode_node_id(&engine, sibling_key);
    let sibling_flow = engine.current_vnode_text_flow(sibling_key).unwrap();
    let new_leaf = VNode::text("new").with_key("new-leaf");
    let new_leaf_key = new_leaf.key;
    let replacement = VNode::box_node().with_key("branch").child(new_leaf);
    assert!(engine.apply_patches(&[Patch::replace(old_branch_key, replacement)]));
    assert_ne!(vnode_node_id(&engine, old_branch_key), old_branch_node);
    assert!(engine.get_vnode_layout(old_leaf_key).is_none());
    assert!(engine.current_vnode_text_flow(old_leaf_key).is_none());
    assert_eq!(
        engine
            .current_vnode_text_flow(new_leaf_key)
            .unwrap()
            .cache_identity()
            .input
            .source,
        "new"
    );
    assert_eq!(vnode_node_id(&engine, sibling_key), sibling_node);
    assert!(Arc::ptr_eq(
        &sibling_flow,
        &engine.current_vnode_text_flow(sibling_key).unwrap()
    ));
    assert!(engine.apply_patches(&[Patch::reorder(root.key, vec![sibling_key, old_branch_key])]));
    assert_eq!(vnode_node_id(&engine, sibling_key), sibling_node);
    assert!(engine.get_vnode_layout(sibling_key).is_some());
    assert!(Arc::ptr_eq(
        &sibling_flow,
        &engine.current_vnode_text_flow(sibling_key).unwrap()
    ));
}
