use rnk::core::{Dimension, Element, FlexDirection, Overflow};
use rnk::layout::{LayoutEngine, LayoutSnapshot, SnapshotBuildStrategy, SnapshotWorkCounters};

#[derive(Clone, Debug)]
struct Message {
    id: u64,
    parent: u64,
    order: usize,
    text: String,
    padded: bool,
    scroll_x: u16,
}

#[derive(Debug)]
struct StepEvidence {
    snapshot: LayoutSnapshot,
    work: SnapshotWorkCounters,
    operation: String,
    raw: [u64; 8],
    state_after: u64,
    viewport: (u16, u16),
    cache_hits: u64,
}

fn first_snapshot_difference(
    full: &LayoutSnapshot,
    incremental: &LayoutSnapshot,
) -> Option<String> {
    let full_nodes: Vec<_> = full.nodes().collect();
    let incremental_nodes: Vec<_> = incremental.nodes().collect();
    for index in 0..full_nodes.len().max(incremental_nodes.len()) {
        let Some(full_node) = full_nodes.get(index) else {
            let incremental_node = incremental_nodes[index];
            return Some(format!(
                "identity={} field=node_presence full=missing incremental=present",
                incremental_node.identity().diagnostic()
            ));
        };
        let Some(incremental_node) = incremental_nodes.get(index) else {
            return Some(format!(
                "identity={} field=node_presence full=present incremental=missing",
                full_node.identity().diagnostic()
            ));
        };
        if full_node.identity() != incremental_node.identity() {
            return Some(format!(
                "identity={} field=identity full={} incremental={}",
                full_node.identity().diagnostic(),
                full_node.identity().diagnostic(),
                incremental_node.identity().diagnostic()
            ));
        }
        macro_rules! compare_field {
            ($name:literal, $full:expr, $incremental:expr) => {
                if $full != $incremental {
                    return Some(format!(
                        "identity={} field={} full={:?} incremental={:?}",
                        full_node.identity().diagnostic(),
                        $name,
                        $full,
                        $incremental
                    ));
                }
            };
        }
        compare_field!("parent", full_node.parent(), incremental_node.parent());
        compare_field!(
            "children",
            full_node.children(),
            incremental_node.children()
        );
        compare_field!(
            "border_bounds",
            full_node.border_bounds(),
            incremental_node.border_bounds()
        );
        compare_field!(
            "content_bounds",
            full_node.content_bounds(),
            incremental_node.content_bounds()
        );
        compare_field!(
            "text_origin",
            full_node.text_origin(),
            incremental_node.text_origin()
        );
        compare_field!(
            "effective_clip",
            full_node.effective_clip(),
            incremental_node.effective_clip()
        );
        compare_field!(
            "scroll_transform",
            full_node.scroll_transform(),
            incremental_node.scroll_transform()
        );
        let flow = |node: &rnk::layout::SnapshotNode| {
            node.text_flow().map(|flow| {
                (
                    flow.max_width(),
                    flow.width_policy_revision(),
                    flow.logical_row_count(),
                )
            })
        };
        compare_field!("text_flow", flow(full_node), flow(incremental_node));
    }
    None
}

fn target(messages: &[Message], width: u16) -> Element {
    fn add_children(parent_id: u64, parent: &mut Element, messages: &[Message]) {
        let mut children: Vec<_> = messages
            .iter()
            .filter(|message| message.parent == parent_id)
            .collect();
        children.sort_by_key(|message| (message.order, message.id));
        for message in children {
            let mut branch = Element::box_element().with_key(format!("m-{}", message.id));
            branch.style.flex_direction = FlexDirection::Column;
            branch.style.padding.left = f32::from(message.padded);
            branch.style.overflow_x = Overflow::Scroll;
            branch.scroll_offset_x = Some(message.scroll_x);
            branch.add_child(
                Element::text(message.text.clone()).with_key(format!("text-{}", message.id)),
            );
            add_children(message.id, &mut branch, messages);
            parent.add_child(branch);
        }
    }

    let mut root = Element::box_element().with_key("root");
    root.style.flex_direction = FlexDirection::Column;
    root.style.width = Dimension::Points(f32::from(width));
    add_children(0, &mut root, messages);
    root
}

fn draw(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn selected_id(messages: &[Message], selector: u64) -> u64 {
    let mut ids: Vec<_> = messages.iter().map(|message| message.id).collect();
    ids.sort_unstable();
    ids[selector as usize % ids.len()]
}

fn selected_parent(messages: &[Message], selector: u64) -> u64 {
    let mut ids = vec![0];
    ids.extend(messages.iter().map(|message| message.id));
    ids.sort_unstable();
    ids[selector as usize % ids.len()]
}

fn remove_subtree(messages: &mut Vec<Message>, root_id: u64) {
    let mut removed = vec![root_id];
    let mut cursor = 0;
    while cursor < removed.len() {
        let parent = removed[cursor];
        removed.extend(
            messages
                .iter()
                .filter(|message| message.parent == parent)
                .map(|message| message.id),
        );
        cursor += 1;
    }
    messages.retain(|message| !removed.contains(&message.id));
}

fn apply_operation(
    messages: &mut Vec<Message>,
    next_id: &mut u64,
    viewport: &mut (u16, u16),
    raw: [u64; 8],
) -> String {
    const PAYLOADS: [&str; 4] = ["ascii", "中", "👩‍💻", "e\u{301}"];
    const VIEWPORTS: [(u16, u16); 4] = [(120, 40), (80, 24), (120, 40), (1, 1)];
    let operation = raw[0] % 100;
    let target_id = selected_id(messages, raw[1]);
    let payload = PAYLOADS[raw[4] as usize % PAYLOADS.len()];
    match operation {
        0..=9 => "unchanged".to_owned(),
        10..=24 => {
            messages
                .iter_mut()
                .find(|item| item.id == target_id)
                .unwrap()
                .text
                .push_str(payload);
            format!("stream target={target_id} payload={payload:?}")
        }
        25..=34 => {
            let target = messages
                .iter_mut()
                .find(|item| item.id == target_id)
                .unwrap();
            target.padded = !target.padded;
            format!("style target={target_id} padded={}", target.padded)
        }
        35..=49 => {
            let parent = selected_parent(messages, raw[2]);
            let order = messages.iter().filter(|item| item.parent == parent).count();
            let id = *next_id;
            *next_id += 1;
            messages.push(Message {
                id,
                parent,
                order,
                text: payload.to_owned(),
                padded: false,
                scroll_x: 0,
            });
            format!("append id={id} parent={parent} order={order} payload={payload:?}")
        }
        50..=59 => {
            let parent = selected_parent(messages, raw[2]);
            let child_count = messages.iter().filter(|item| item.parent == parent).count();
            let order = raw[3] as usize % (child_count + 1);
            for sibling in messages
                .iter_mut()
                .filter(|item| item.parent == parent && item.order >= order)
            {
                sibling.order += 1;
            }
            let id = *next_id;
            *next_id += 1;
            messages.push(Message {
                id,
                parent,
                order,
                text: payload.to_owned(),
                padded: false,
                scroll_x: 0,
            });
            format!("insert id={id} parent={parent} order={order} payload={payload:?}")
        }
        60..=69 => {
            let before = messages.clone();
            let parent = messages
                .iter()
                .find(|item| item.id == target_id)
                .unwrap()
                .parent;
            let order = messages
                .iter()
                .find(|item| item.id == target_id)
                .unwrap()
                .order;
            remove_subtree(messages, target_id);
            if messages.is_empty() {
                *messages = before;
                format!("remove unchanged(last-tree) target={target_id}")
            } else {
                for sibling in messages
                    .iter_mut()
                    .filter(|item| item.parent == parent && item.order > order)
                {
                    sibling.order -= 1;
                }
                format!("remove target={target_id} parent={parent} order={order}")
            }
        }
        70..=79 => {
            let old = messages
                .iter()
                .find(|item| item.id == target_id)
                .unwrap()
                .clone();
            remove_subtree(messages, target_id);
            let id = *next_id;
            *next_id += 1;
            messages.push(Message {
                id,
                parent: old.parent,
                order: old.order,
                text: payload.to_owned(),
                padded: false,
                scroll_x: 0,
            });
            format!(
                "replace old={target_id} new={id} parent={} order={} payload={payload:?}",
                old.parent, old.order
            )
        }
        80..=89 => {
            let mut parents: Vec<_> = std::iter::once(0)
                .chain(messages.iter().map(|message| message.id))
                .filter(|parent| {
                    messages
                        .iter()
                        .filter(|item| item.parent == *parent)
                        .count()
                        >= 2
                })
                .collect();
            parents.sort_unstable();
            if parents.is_empty() {
                "reorder unchanged(no-parent)".to_owned()
            } else {
                let parent = parents[raw[2] as usize % parents.len()];
                let mut children: Vec<_> = messages
                    .iter()
                    .filter(|item| item.parent == parent)
                    .map(|item| item.id)
                    .collect();
                children
                    .sort_by_key(|id| messages.iter().find(|item| item.id == *id).unwrap().order);
                let left = raw[1] as usize % children.len();
                let mut right = raw[3] as usize % children.len();
                if left == right {
                    right = (right + 1) % children.len();
                }
                let left_id = children[left];
                let right_id = children[right];
                let left_order = messages
                    .iter()
                    .find(|item| item.id == left_id)
                    .unwrap()
                    .order;
                let right_order = messages
                    .iter()
                    .find(|item| item.id == right_id)
                    .unwrap()
                    .order;
                messages
                    .iter_mut()
                    .find(|item| item.id == left_id)
                    .unwrap()
                    .order = right_order;
                messages
                    .iter_mut()
                    .find(|item| item.id == right_id)
                    .unwrap()
                    .order = left_order;
                format!("reorder parent={parent} left={left_id} right={right_id}")
            }
        }
        90..=94 => {
            *viewport = VIEWPORTS[raw[5] as usize % VIEWPORTS.len()];
            format!("resize width={} height={}", viewport.0, viewport.1)
        }
        95..=99 => {
            let scroll_x = (raw[7] % 7) as u16;
            messages
                .iter_mut()
                .find(|item| item.id == target_id)
                .unwrap()
                .scroll_x = scroll_x;
            format!("scroll target={target_id} x={scroll_x}")
        }
        _ => unreachable!(),
    }
}

fn run_seed(seed: u64) -> Vec<StepEvidence> {
    let mut random = seed;
    let mut messages = vec![Message {
        id: 1,
        parent: 0,
        order: 0,
        text: "initial".to_owned(),
        padded: false,
        scroll_x: 0,
    }];
    let mut next_id = 2;
    let mut viewport = (120, 40);
    let mut incremental = LayoutEngine::new();
    let mut previous = None;
    let mut evidence = Vec::with_capacity(64);
    for step in 0..64 {
        let raw = std::array::from_fn::<_, 8, _>(|_| draw(&mut random));
        let operation = apply_operation(&mut messages, &mut next_id, &mut viewport, raw);
        let current = target(&messages, viewport.0);
        let prepared = incremental
            .prepare_element_incremental(&current, previous.as_ref(), viewport.0, viewport.1)
            .unwrap_or_else(|error| panic!("seed={seed:#018x} state={random:#018x} step={step} raw={raw:?} normalized={operation}: {error}"));
        let full = LayoutEngine::new()
            .prepare_element_incremental(&current, None, viewport.0, viewport.1)
            .unwrap_or_else(|error| panic!("full seed={seed:#018x} state={random:#018x} step={step} raw={raw:?} normalized={operation}: {error}"));
        if let Some(difference) = first_snapshot_difference(full.snapshot(), prepared.snapshot()) {
            panic!(
                "seed={seed:#018x} state={random:#018x} step={step} raw={raw:?} normalized={operation} {difference}"
            );
        }
        let report = prepared.snapshot_report();
        assert_eq!(
            report.work_counters().snapshot_nodes(),
            prepared.snapshot().nodes().len() as u64
        );
        assert_eq!(
            report.work_counters().visited_nodes(),
            prepared.snapshot().nodes().len() as u64
        );
        assert_eq!(report.work_counters().rebuild_count(), 0);
        assert_eq!(
            full.snapshot_report().strategy(),
            SnapshotBuildStrategy::InitialFull
        );
        assert_eq!(full.snapshot_report().work_counters().rebuild_count(), 0);
        assert_eq!(
            full.snapshot_report().work_counters().mutated_nodes(),
            full.snapshot().nodes().len() as u64
        );
        let item = StepEvidence {
            snapshot: prepared.snapshot().clone(),
            work: report.work_counters(),
            operation,
            raw,
            state_after: random,
            viewport,
            cache_hits: report.cache_hits(),
        };
        let (next, _) = prepared.commit(&mut incremental);
        previous = Some(next);
        evidence.push(item);
    }
    assert!(
        evidence.iter().any(|step| step.cache_hits > 0),
        "seed={seed:#018x} never observed the real FlowCache hit seam"
    );
    evidence
}

#[test]
fn seeded_operations_match_after_every_step() {
    const SEEDS: [u64; 5] = [
        0x0000_0000_0000_0001,
        0x243f_6a88_85a3_08d3,
        0x9e37_79b9_7f4a_7c15,
        0xd1b5_4a32_d192_ed03,
        0xffff_ffff_ffff_ffff,
    ];
    for seed in SEEDS {
        let first = run_seed(seed);
        let replay = run_seed(seed);
        assert_eq!(first.len(), 64);
        for (step, (expected, actual)) in first.iter().zip(&replay).enumerate() {
            if expected.snapshot != actual.snapshot
                || expected.work != actual.work
                || expected.operation != actual.operation
                || expected.raw != actual.raw
                || expected.state_after != actual.state_after
                || expected.viewport != actual.viewport
                || expected.cache_hits != actual.cache_hits
            {
                panic!(
                    "replay first difference seed={seed:#018x} step={step} expected_state={:#018x} actual_state={:#018x} expected_raw={:?} actual_raw={:?} expected_operation={} actual_operation={} expected_work={:?} actual_work={:?} snapshot_difference={}",
                    expected.state_after,
                    actual.state_after,
                    expected.raw,
                    actual.raw,
                    expected.operation,
                    actual.operation,
                    expected.work,
                    actual.work,
                    first_snapshot_difference(&expected.snapshot, &actual.snapshot)
                        .unwrap_or_else(|| "none".to_owned())
                );
            }
        }
    }
}

#[test]
fn snapshot_divergence_diagnostic_names_first_identity_field_and_values() {
    let messages = vec![Message {
        id: 1,
        parent: 0,
        order: 0,
        text: "diagnostic".to_owned(),
        padded: false,
        scroll_x: 0,
    }];
    let full = LayoutEngine::new()
        .prepare_element_incremental(&target(&messages, 12), None, 12, 4)
        .unwrap()
        .snapshot()
        .clone();
    let incremental = LayoutEngine::new()
        .prepare_element_incremental(&target(&messages, 8), None, 8, 4)
        .unwrap()
        .snapshot()
        .clone();
    let diagnostic = first_snapshot_difference(&full, &incremental)
        .expect("different cell snapshots must report the first exact field");
    assert!(diagnostic.contains("identity="));
    assert!(diagnostic.contains("field=border_bounds"));
    assert!(diagnostic.contains("full=CellRect"));
    assert!(diagnostic.contains("incremental=CellRect"));
}
