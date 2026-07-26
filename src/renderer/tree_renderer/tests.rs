use super::*;
use crate::components::{Box, Span, Text};
use crate::core::{BorderStyle, Overflow};

fn text_with_border(
    content: &str,
    width: u16,
    height: u16,
    top: bool,
    right: bool,
    bottom: bool,
    left: bool,
) -> Element {
    let mut element = Text::new(content).into_element();
    element.style.width = width.into();
    element.style.height = height.into();
    element.style.border_style = BorderStyle::Single;
    element.style.border_top = top;
    element.style.border_right = right;
    element.style.border_bottom = bottom;
    element.style.border_left = left;
    element
}

fn plain_text_with_border(
    content: &str,
    width: u16,
    height: u16,
    top: bool,
    right: bool,
    bottom: bool,
    left: bool,
) -> Element {
    let mut element = Element::text(content);
    element.style.width = width.into();
    element.style.height = height.into();
    element.style.border_style = BorderStyle::Single;
    element.style.border_top = top;
    element.style.border_right = right;
    element.style.border_bottom = bottom;
    element.style.border_left = left;
    element
}

fn fixed_plain_text(content: &str, width: u16, height: u16) -> Element {
    let mut element = Element::text(content);
    element.style.width = width.into();
    element.style.height = height.into();
    element.style.flex_shrink = 0.0;
    element
}

fn render_tree_for_test(element: &Element, width: u16, height: u16) -> Output {
    let mut engine = LayoutEngine::new();
    engine.compute(element, width, height);

    let mut output = Output::new(width, height);
    render_element_tree(element, &engine, &mut output, 0.0, 0.0);
    output
}

#[test]
fn asymmetric_border_does_not_wrap_text_into_bottom_border() {
    let element = text_with_border("abcde", 6, 3, true, false, true, true);
    let output = render_tree_for_test(&element, 6, 3);

    assert_eq!(output.render(), "┌────┐\r\n│abcde\r\n└────┘");
}

#[test]
fn multiline_plain_text_does_not_overwrite_bottom_border() {
    let element = plain_text_with_border("one\ntwo", 6, 3, true, true, true, true);
    let output = render_tree_for_test(&element, 6, 3);

    assert_eq!(output.render(), "┌────┐\r\n│one │\r\n└────┘");
}

#[test]
fn wrapped_plain_text_does_not_overwrite_bottom_border() {
    let element = plain_text_with_border("abcdefgh", 6, 3, true, true, true, true);
    let output = render_tree_for_test(&element, 6, 3);

    assert_eq!(output.render(), "┌────┐\r\n│abcd│\r\n└────┘");
}

#[test]
fn rich_spans_do_not_overwrite_right_border() {
    let mut element = Text::spans(vec![Span::new("abc"), Span::new("def")]).into_element();
    element.style.width = 6.into();
    element.style.height = 3.into();
    element.style.border_style = BorderStyle::Single;

    let output = render_tree_for_test(&element, 6, 3);

    assert_eq!(output.cell_at(5, 1).map(|cell| cell.ch), Some('│'));
    assert_eq!(output.render(), "┌────┐\r\n│abcd│\r\n└────┘");
}

#[test]
fn padding_cannot_move_text_onto_bottom_border() {
    let mut element = plain_text_with_border("X", 3, 3, true, true, true, true);
    element.style.padding.top = 1.0;

    let output = render_tree_for_test(&element, 3, 3);

    assert_eq!(output.render(), "┌─┐\r\n│ │\r\n└─┘");
}

#[test]
fn one_cell_layout_has_deterministic_border_ownership() {
    let bottom_only = plain_text_with_border("X", 1, 1, false, false, true, false);
    assert_eq!(render_tree_for_test(&bottom_only, 1, 1).render(), "└");

    let right_only = plain_text_with_border("X", 1, 1, false, true, false, false);
    assert_eq!(render_tree_for_test(&right_only, 1, 1).render(), "│");

    // Horizontal rows own shared cells, and the bottom row wins when both
    // horizontal borders occupy the same one-cell layout.
    let all_sides = plain_text_with_border("", 1, 1, true, true, true, true);
    let mut shared_cell = Output::new(1, 1);
    render_border(&all_sides, &mut shared_cell, 0, 0, 1, 1);
    assert_eq!(shared_cell.render(), "└");
}

#[test]
fn hidden_overflow_clips_own_text_before_paint() {
    let mut element = plain_text_with_border("one\ntwo\nthree", 6, 2, true, false, false, false);
    element.style.overflow_y = Overflow::Hidden;

    let output = render_tree_for_test(&element, 6, 4);

    assert_eq!(output.render(), "┌────┐\r\none");
    assert!(!output.render().contains("two"));
    assert!(!output.render().contains("three"));
}

#[test]
fn visible_child_cannot_take_parent_bottom_border_ownership() {
    let mut child = Element::text("one\ntwo");
    child.style.height = 2.into();
    child.style.flex_shrink = 0.0;

    let element = Box::new()
        .width(6)
        .height(3)
        .border_style(BorderStyle::Single)
        .overflow_y(Overflow::Visible)
        .child(child)
        .into_element();
    let output = render_tree_for_test(&element, 6, 3);

    assert_eq!(output.render(), "┌────┐\r\n│one │\r\n└────┘");
}

#[test]
fn hidden_child_is_clipped_to_parent_content_rect() {
    let mut child = Element::text("one\ntwo");
    child.style.height = 2.into();
    child.style.flex_shrink = 0.0;

    let element = Box::new()
        .width(6)
        .height(3)
        .border_style(BorderStyle::Single)
        .overflow_y(Overflow::Hidden)
        .child(child)
        .into_element();
    let output = render_tree_for_test(&element, 6, 3);

    assert_eq!(output.render(), "┌────┐\r\n│one │\r\n└────┘");
}

#[test]
fn horizontal_clip_preserves_visible_vertical_overflow() {
    for overflow_x in [Overflow::Hidden, Overflow::Scroll] {
        let element = Box::new()
            .width(3)
            .height(1)
            .overflow_x(overflow_x)
            .overflow_y(Overflow::Visible)
            .child(fixed_plain_text("abcde\nfghij", 5, 2))
            .into_element();

        assert_eq!(
            render_tree_for_test(&element, 6, 2).render(),
            "abc\r\nfgh",
            "horizontal {overflow_x:?} unexpectedly clipped the visible y axis"
        );
    }
}

#[test]
fn vertical_clip_preserves_visible_horizontal_overflow() {
    for overflow_y in [Overflow::Hidden, Overflow::Scroll] {
        let element = Box::new()
            .width(1)
            .height(1)
            .overflow_x(Overflow::Visible)
            .overflow_y(overflow_y)
            .child(fixed_plain_text("abcde\nfghij", 5, 2))
            .into_element();

        assert_eq!(
            render_tree_for_test(&element, 6, 2).render(),
            "abcde",
            "vertical {overflow_y:?} unexpectedly clipped the visible x axis"
        );
    }
}

#[test]
fn nested_child_clip_intersects_with_ancestor_clip() {
    let vertically_clipped_child = Box::new()
        .width(5)
        .height(1)
        .flex_shrink(0.0)
        .overflow_x(Overflow::Visible)
        .overflow_y(Overflow::Hidden)
        .child(fixed_plain_text("abcde\nfghij", 5, 2))
        .into_element();
    let horizontally_clipped_ancestor = Box::new()
        .width(3)
        .height(2)
        .overflow_x(Overflow::Hidden)
        .overflow_y(Overflow::Visible)
        .child(vertically_clipped_child)
        .into_element();
    assert_eq!(
        render_tree_for_test(&horizontally_clipped_ancestor, 6, 2).render(),
        "abc"
    );

    let horizontally_clipped_child = Box::new()
        .width(3)
        .height(2)
        .flex_shrink(0.0)
        .overflow_x(Overflow::Hidden)
        .overflow_y(Overflow::Visible)
        .child(fixed_plain_text("abcde\nfghij", 5, 2))
        .into_element();
    let vertically_clipped_ancestor = Box::new()
        .width(5)
        .height(1)
        .overflow_x(Overflow::Visible)
        .overflow_y(Overflow::Hidden)
        .child(horizontally_clipped_child)
        .into_element();
    assert_eq!(
        render_tree_for_test(&vertically_clipped_ancestor, 6, 2).render(),
        "abc"
    );
}

#[test]
fn all_border_side_combinations_use_independent_content_cells() {
    const WIDTH: u16 = 6;
    const SOURCE: &str = "abcdef";

    for mask in 0_u8..16 {
        let top = mask & 0b0001 != 0;
        let right = mask & 0b0010 != 0;
        let bottom = mask & 0b0100 != 0;
        let left = mask & 0b1000 != 0;
        let height = 3 + u16::from(top) + u16::from(bottom);

        let style = Style {
            border_style: BorderStyle::Single,
            border_top: top,
            border_right: right,
            border_bottom: bottom,
            border_left: left,
            ..Style::default()
        };
        let expected = ContentRect {
            x: u16::from(left),
            y: u16::from(top),
            width: WIDTH - u16::from(left) - u16::from(right),
            height: 3,
        };
        assert_eq!(
            ContentRect::from_border(&style, WIDTH, height),
            expected,
            "wrong content rect for mask {mask:04b}"
        );

        let content = &SOURCE[..expected.width as usize];
        let element = text_with_border(content, WIDTH, height, top, right, bottom, left);
        assert_eq!(
            ContentRect::from_border(&element.style, WIDTH, height),
            expected,
            "element content rect diverged for mask {mask:04b}"
        );

        let output = render_tree_for_test(&element, WIDTH, height);
        for (offset, expected_char) in content.chars().enumerate() {
            assert_eq!(
                output
                    .cell_at(expected.x as usize + offset, expected.y as usize)
                    .map(|cell| cell.ch),
                Some(expected_char),
                "content cell mismatch for mask {mask:04b}, offset {offset}"
            );
        }

        if left {
            assert!(
                !content.contains(output.cell_at(0, expected.y as usize).unwrap().ch),
                "text overwrote enabled left side for mask {mask:04b}"
            );
        } else {
            assert_eq!(
                output.cell_at(0, expected.y as usize).map(|cell| cell.ch),
                content.chars().next(),
                "disabled left side consumed a cell for mask {mask:04b}"
            );
        }

        if right {
            assert!(
                !content.contains(
                    output
                        .cell_at((WIDTH - 1) as usize, expected.y as usize)
                        .unwrap()
                        .ch
                ),
                "text overwrote enabled right side for mask {mask:04b}"
            );
        } else {
            assert_eq!(
                output
                    .cell_at((WIDTH - 1) as usize, expected.y as usize)
                    .map(|cell| cell.ch),
                content.chars().last(),
                "disabled right side consumed a cell for mask {mask:04b}"
            );
        }

        if top {
            assert_eq!(
                output.cell_at((WIDTH / 2) as usize, 0).map(|cell| cell.ch),
                Some('─'),
                "enabled top side lacks its border glyph for mask {mask:04b}"
            );
        } else {
            assert_eq!(expected.y, 0, "disabled top side consumed a row");
            assert_ne!(
                output.cell_at((WIDTH / 2) as usize, 0).map(|cell| cell.ch),
                Some('─'),
                "disabled top side painted a border glyph for mask {mask:04b}"
            );
        }

        if bottom {
            assert_eq!(
                output
                    .cell_at((WIDTH / 2) as usize, (height - 1) as usize)
                    .map(|cell| cell.ch),
                Some('─'),
                "enabled bottom side lacks its border glyph for mask {mask:04b}"
            );
        } else {
            assert_eq!(
                expected.y + expected.height,
                height,
                "disabled bottom side consumed a row for mask {mask:04b}"
            );
            assert_ne!(
                output
                    .cell_at((WIDTH / 2) as usize, (height - 1) as usize)
                    .map(|cell| cell.ch),
                Some('─'),
                "disabled bottom side painted a border glyph for mask {mask:04b}"
            );
        }

        let vertical_probe_y = expected.y + 1;
        if left {
            assert_eq!(
                output
                    .cell_at(0, vertical_probe_y as usize)
                    .map(|cell| cell.ch),
                Some('│'),
                "enabled left side lacks its border glyph for mask {mask:04b}"
            );
        } else {
            assert_ne!(
                output
                    .cell_at(0, vertical_probe_y as usize)
                    .map(|cell| cell.ch),
                Some('│'),
                "disabled left side painted a border glyph for mask {mask:04b}"
            );
        }
        if right {
            assert_eq!(
                output
                    .cell_at((WIDTH - 1) as usize, vertical_probe_y as usize)
                    .map(|cell| cell.ch),
                Some('│'),
                "enabled right side lacks its border glyph for mask {mask:04b}"
            );
        } else {
            assert_ne!(
                output
                    .cell_at((WIDTH - 1) as usize, vertical_probe_y as usize)
                    .map(|cell| cell.ch),
                Some('│'),
                "disabled right side painted a border glyph for mask {mask:04b}"
            );
        }
    }
}

#[test]
fn asymmetric_border_content_rect_is_used_for_overflow_clip() {
    let element = Box::new()
        .width(6)
        .height(3)
        .border_style(BorderStyle::Single)
        .border(true, false, true, true)
        .overflow_x(Overflow::Hidden)
        .child(Text::new("abcde").into_element())
        .into_element();

    let output = render_tree_for_test(&element, 6, 3);

    assert_eq!(output.render(), "┌────┐\r\n│abcde\r\n└────┘");
}

#[test]
fn zero_and_one_cell_content_rects_are_safe() {
    let all_sides = Style {
        border_style: BorderStyle::Single,
        ..Style::default()
    };
    assert_eq!(
        ContentRect::from_border(&all_sides, 0, 0),
        ContentRect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    );
    assert_eq!(
        ContentRect::from_border(&all_sides, 1, 1),
        ContentRect {
            x: 1,
            y: 1,
            width: 0,
            height: 0,
        }
    );

    let no_visible_border = Style {
        border_style: BorderStyle::None,
        ..Style::default()
    };
    assert_eq!(
        ContentRect::from_border(&no_visible_border, 1, 1),
        ContentRect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        }
    );

    let zero = text_with_border("X", 0, 0, true, true, true, true);
    assert_eq!(render_tree_for_test(&zero, 0, 0).render(), "");

    let one = text_with_border("X", 1, 1, true, true, true, true);
    let one_output = render_tree_for_test(&one, 1, 1).render();
    assert_eq!(one_output, "┌");
    assert!(!one_output.contains('X'));

    let borderless = Text::new("X").into_element();
    assert_eq!(render_tree_for_test(&borderless, 1, 1).render(), "X");
}

#[test]
fn scrolled_out_negative_rows_do_not_paint_at_top() {
    let element = Box::new()
        .flex_direction(crate::core::FlexDirection::Column)
        .width(12)
        .height(1)
        .overflow_y(Overflow::Hidden)
        .scroll_offset_y(1)
        .child(
            Box::new()
                .height(1)
                .flex_shrink(0.0)
                .child(Text::new("hiddenxxxxx").into_element())
                .into_element(),
        )
        .child(
            Box::new()
                .height(1)
                .flex_shrink(0.0)
                .child(Text::new("ok").into_element())
                .into_element(),
        )
        .into_element();

    let output = render_tree_for_test(&element, 12, 1);

    assert_eq!(output.render(), "ok");
}
