use rnk::core::{Dimension, Element, FlexDirection};
use rnk::layout::LayoutEngine;

#[derive(Clone)]
struct Message {
    id: u64,
    text: String,
    padded: bool,
}

fn target(messages: &[Message]) -> Element {
    let mut root = Element::box_element().with_key("root");
    root.style.flex_direction = FlexDirection::Column;
    for message in messages {
        let mut child = Element::text(message.text.clone()).with_key(format!("m-{}", message.id));
        if message.padded {
            child.style.padding.left = 1.0;
        }
        root.add_child(child);
    }
    root
}

fn draw(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
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
        let mut random = seed;
        let mut messages = vec![Message {
            id: 1,
            text: "initial".to_owned(),
            padded: false,
        }];
        let mut next_id = 2_u64;
        let mut incremental = LayoutEngine::new();
        let mut previous = None;
        for step in 0..64 {
            let draws = std::array::from_fn::<_, 8, _>(|_| draw(&mut random));
            let operation = draws[0] % 100;
            let selected = (draws[1] as usize) % messages.len();
            match operation {
                0..=9 => {}
                10..=24 => messages[selected].text.push_str(match draws[4] % 4 {
                    0 => " delta",
                    1 => "世界",
                    2 => "🙂",
                    _ => "e\u{301}",
                }),
                25..=34 => messages[selected].padded = !messages[selected].padded,
                35..=49 => {
                    messages.push(Message {
                        id: next_id,
                        text: format!("append-{}", draws[4] % 1000),
                        padded: false,
                    });
                    next_id += 1;
                }
                50..=59 => {
                    let index = (draws[3] as usize) % (messages.len() + 1);
                    messages.insert(
                        index,
                        Message {
                            id: next_id,
                            text: format!("insert-{}", draws[4] % 1000),
                            padded: false,
                        },
                    );
                    next_id += 1;
                }
                60..=69 if messages.len() > 1 => {
                    messages.remove(selected);
                }
                70..=79 => {
                    messages[selected] = Message {
                        id: next_id,
                        text: format!("replace-{}", draws[4] % 1000),
                        padded: false,
                    };
                    next_id += 1;
                }
                _ if messages.len() > 1 => {
                    let other = (draws[3] as usize) % messages.len();
                    messages.swap(selected, other);
                }
                _ => {}
            }

            let width = 8 + (draws[5] % 25) as u16;
            let height = 4 + (draws[6] % 12) as u16;
            let mut current = target(&messages);
            current.style.width = Dimension::Points(f32::from(width));
            let prepared = incremental
                .prepare_element_incremental(&current, previous.as_ref(), width, height)
                .unwrap_or_else(|error| panic!("seed={seed:#x} step={step}: {error}"));
            let full = LayoutEngine::new()
                .prepare_element_incremental(&current, None, width, height)
                .unwrap();
            assert_eq!(
                prepared.snapshot(),
                full.snapshot(),
                "seed={seed:#x} step={step} operation={operation}"
            );
            let (next, _) = prepared.commit(&mut incremental);
            previous = Some(next);
        }
    }
}
