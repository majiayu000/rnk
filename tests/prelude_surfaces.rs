#[test]
fn widgets_prelude_exports_beginner_components() {
    use rnk::prelude::widgets::*;

    let text_input_options = TextInputOptions::default();
    let mut text_input = TextInputState::default();
    assert_eq!(
        handle_text_input(&mut text_input, "a", &Key::default(), &text_input_options),
        InteractionOutcome::Changed("a".to_string())
    );

    let select_items = vec![SelectItem::new("One", 1), SelectItem::new("Two", 2)];
    let select = SelectInput::new(select_items).into_element();

    let textarea_state = TextAreaState::new();
    let textarea = TextArea::new(&textarea_state).into_element();

    let commands = vec![Command::new("open", "Open")];
    let palette = CommandPalette::new(commands).into_element();

    let element = Box::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new("widgets").into_element())
        .child(select)
        .child(textarea)
        .child(palette)
        .into_element();

    let output = render_to_string(&element, 60);
    assert!(output.contains("widgets"));
}

#[test]
fn testing_prelude_exports_snapshot_helpers() {
    use rnk::prelude::testing::*;
    use rnk::prelude::widgets::{Text, render_to_string};

    let element = Text::new("testing prelude").into_element();
    assert_renders_containing(&element, "testing prelude");

    let renderer = TestRenderer::new(30, 5);
    let output = renderer.render_to_plain(&element);
    inline_snapshot!(output.trim(), "testing prelude");

    assert_eq!(display_width("abc"), 3);
    assert!(strip_ansi_codes("\u{1b}[31mred\u{1b}[0m").contains("red"));

    let generated = gen_text("generated");
    assert!(render_to_string(&generated, 30).contains("generated"));

    let golden = GoldenTest::new("prelude_surface").with_size(30, 5);
    let _ = golden.compare(&element);

    let snapshot = Snapshot::new("prelude_surface", "testing prelude");
    snapshot.assert_match("testing prelude");
}
