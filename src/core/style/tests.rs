use super::*;

#[test]
fn test_default_style() {
    let style = Style::new();
    assert_eq!(style.flex_direction, FlexDirection::Row);
    assert_eq!(style.flex_shrink, 1.0);
    assert_eq!(style.display, Display::Flex);
}

#[test]
fn test_new_equals_default() {
    assert_eq!(Style::new(), Style::default());
}

#[test]
fn nan_style_clone_is_equal_in_the_style_domain() {
    let mut style = Style::new();
    style.flex_grow = f32::NAN;
    let mut other_nan = style.clone();
    other_nan.flex_grow = f32::from_bits(0x7fc0_0001);

    assert_eq!(style, style.clone());
    assert_eq!(style, other_nan);

    let mut positive_zero = Style::new();
    positive_zero.flex_grow = 0.0;
    let mut negative_zero = positive_zero.clone();
    negative_zero.flex_grow = -0.0;
    assert_eq!(positive_zero, negative_zero);

    let mut positive_infinity = Style::new();
    positive_infinity.flex_grow = f32::INFINITY;
    assert_eq!(positive_infinity, positive_infinity.clone());
    let mut negative_infinity = positive_infinity.clone();
    negative_infinity.flex_grow = f32::NEG_INFINITY;
    assert_ne!(positive_infinity, negative_infinity);
}

#[test]
fn test_edges() {
    let edges = Edges::all(5.0);
    assert_eq!(edges.top, 5.0);
    assert_eq!(edges.right, 5.0);
    assert_eq!(edges.bottom, 5.0);
    assert_eq!(edges.left, 5.0);
}

#[test]
fn test_border_chars() {
    let chars = BorderStyle::Single.chars();
    assert_eq!(chars.0, "┌");
    assert_eq!(chars.4, "─");
}

#[test]
fn test_dimension_conversion() {
    let dim: Dimension = 10u16.into();
    assert_eq!(dim, Dimension::Points(10.0));

    let dim: Dimension = 20i32.into();
    assert_eq!(dim, Dimension::Points(20.0));
}

#[test]
fn test_chainable_colors() {
    let style = Style::new().fg(Color::Red).bg(Color::Blue);
    assert_eq!(style.color, Some(Color::Red));
    assert_eq!(style.background_color, Some(Color::Blue));
}

#[test]
fn test_chainable_text_styles() {
    let style = Style::new().bold().italic().underline().dim();
    assert!(style.bold);
    assert!(style.italic);
    assert!(style.underline);
    assert!(style.dim);
}

#[test]
fn test_chainable_padding() {
    let style = Style::new().p(2.0_f32);
    assert_eq!(style.padding, Edges::all(2.0));

    let style = Style::new().px(3.0_f32).py(1.0_f32);
    assert_eq!(style.padding.left, 3.0);
    assert_eq!(style.padding.right, 3.0);
    assert_eq!(style.padding.top, 1.0);
    assert_eq!(style.padding.bottom, 1.0);

    let style = Style::new().pt(1.0_f32).pr(2.0_f32).pb(3.0_f32).pl(4.0_f32);
    assert_eq!(style.padding.top, 1.0);
    assert_eq!(style.padding.right, 2.0);
    assert_eq!(style.padding.bottom, 3.0);
    assert_eq!(style.padding.left, 4.0);
}

#[test]
fn test_chainable_margin() {
    let style = Style::new().m(2.0_f32);
    assert_eq!(style.margin, Edges::all(2.0));

    let style = Style::new().mx(3.0_f32).my(1.0_f32);
    assert_eq!(style.margin.left, 3.0);
    assert_eq!(style.margin.right, 3.0);
    assert_eq!(style.margin.top, 1.0);
    assert_eq!(style.margin.bottom, 1.0);
}

#[test]
fn test_chainable_border() {
    let style = Style::new()
        .border(BorderStyle::Round)
        .border_fg(Color::Cyan);
    assert_eq!(style.border_style, BorderStyle::Round);
    assert_eq!(style.border_color, Some(Color::Cyan));

    let style = Style::new().rounded();
    assert_eq!(style.border_style, BorderStyle::Round);
}

#[test]
fn test_chainable_size() {
    let style = Style::new().w(80u16).h(24u16);
    assert_eq!(style.width, Dimension::Points(80.0));
    assert_eq!(style.height, Dimension::Points(24.0));
}

#[test]
fn test_chainable_flexbox() {
    let style = Style::new()
        .direction(FlexDirection::Column)
        .grow(1.0)
        .gap_size(2.0)
        .align(AlignItems::Center)
        .justify(JustifyContent::SpaceBetween);

    assert_eq!(style.flex_direction, FlexDirection::Column);
    assert_eq!(style.flex_grow, 1.0);
    assert_eq!(style.gap, 2.0);
    assert_eq!(style.align_items, AlignItems::Center);
    assert_eq!(style.justify_content, JustifyContent::SpaceBetween);
}

#[test]
fn test_style_merge() {
    let base = Style::new().fg(Color::White).p(1.0_f32);
    let overlay = Style::new().fg(Color::Red).bold();

    let merged = base.merge(&overlay);
    assert_eq!(merged.color, Some(Color::Red)); // Overridden
    assert!(merged.bold); // Added
    assert_eq!(merged.padding, Edges::all(1.0)); // Preserved
}

#[test]
fn test_preset_styles() {
    let error = Style::error();
    assert_eq!(error.color, Some(Color::Red));
    assert!(error.bold);

    let success = Style::success();
    assert_eq!(success.color, Some(Color::Green));

    let warning = Style::warning();
    assert_eq!(warning.color, Some(Color::Yellow));

    let info = Style::info();
    assert_eq!(info.color, Some(Color::Cyan));

    let muted = Style::muted();
    assert!(muted.dim);

    let highlight = Style::highlight();
    assert!(highlight.inverse);
}

#[test]
fn test_full_chain() {
    // Test a realistic full chain like the target API
    let style = Style::new()
        .fg(Color::Cyan)
        .bg(Color::Black)
        .bold()
        .p(1.0_f32)
        .px(2.0_f32)
        .border(BorderStyle::Round);

    assert_eq!(style.color, Some(Color::Cyan));
    assert_eq!(style.background_color, Some(Color::Black));
    assert!(style.bold);
    assert_eq!(style.padding.top, 1.0);
    assert_eq!(style.padding.bottom, 1.0);
    assert_eq!(style.padding.left, 2.0);
    assert_eq!(style.padding.right, 2.0);
    assert_eq!(style.border_style, BorderStyle::Round);
}
