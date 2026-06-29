use rnk::prelude::*;

fn enter_key() -> Key {
    Key {
        return_key: true,
        ..Default::default()
    }
}

fn left_key() -> Key {
    Key {
        left_arrow: true,
        ..Default::default()
    }
}

#[test]
fn box_and_text_render_contract() {
    let element = Box::new()
        .padding(1)
        .border_style(BorderStyle::Round)
        .child(Text::new("Hello, core contracts").bold().into_element())
        .into_element();

    let output = render_to_string(&element, 40);
    assert!(output.contains("Hello, core contracts"));
    assert!(element.style.has_border());
}

#[test]
fn text_input_interaction_contract() {
    let mut state = TextInputState::default();
    let options = TextInputOptions::default();

    let outcome = handle_text_input(&mut state, "a", &Key::default(), &options);
    assert_eq!(outcome, InteractionOutcome::Changed("a".to_string()));
    assert_eq!(state.value(), "a");

    let outcome = handle_text_input(&mut state, "", &enter_key(), &options);
    assert_eq!(outcome, InteractionOutcome::Submitted("a".to_string()));

    let read_only = TextInputOptions::default().read_only();
    let outcome = handle_text_input(&mut state, "b", &Key::default(), &read_only);
    assert_eq!(outcome, InteractionOutcome::Ignored);
    assert_eq!(state.value(), "a");

    state.move_to_end();
    let outcome = handle_text_input(&mut state, "", &left_key(), &read_only);
    assert_eq!(outcome, InteractionOutcome::Handled);
    assert_eq!(state.cursor(), 0);

    let disabled = TextInputOptions::default().disabled();
    let outcome = handle_text_input(&mut state, "c", &Key::default(), &disabled);
    assert_eq!(outcome, InteractionOutcome::Ignored);
    assert_eq!(state.value(), "a");
}

#[test]
fn select_input_interaction_contract() {
    let mut state = SelectInputState::new(0);
    let config = NavigationConfig::new().vim_navigation(true);

    let outcome = handle_select_input(
        &mut state,
        3,
        "j",
        &Key::default(),
        &config,
        InteractionMode::Enabled,
    );
    assert_eq!(outcome, InteractionOutcome::Handled);
    assert_eq!(state.highlighted(), 1);

    let outcome = handle_select_input(
        &mut state,
        3,
        "",
        &enter_key(),
        &config,
        InteractionMode::Enabled,
    );
    assert_eq!(outcome, InteractionOutcome::Submitted(1));

    let mut read_only_state = SelectInputState::new(0);
    let outcome = handle_select_input(
        &mut read_only_state,
        3,
        "j",
        &Key::default(),
        &config,
        InteractionMode::ReadOnly,
    );
    assert_eq!(outcome, InteractionOutcome::Handled);
    assert_eq!(read_only_state.highlighted(), 1);

    let outcome = handle_select_input(
        &mut read_only_state,
        3,
        "",
        &enter_key(),
        &config,
        InteractionMode::ReadOnly,
    );
    assert_eq!(outcome, InteractionOutcome::Ignored);
    assert_eq!(read_only_state.submitted(), None);

    let mut disabled_state = SelectInputState::new(0);
    let outcome = handle_select_input(
        &mut disabled_state,
        3,
        "j",
        &Key::default(),
        &config,
        InteractionMode::Disabled,
    );
    assert_eq!(outcome, InteractionOutcome::Ignored);
    assert_eq!(disabled_state.highlighted(), 0);
}

#[test]
fn textarea_interaction_contract() {
    let keymap = TextAreaKeyMap::default();
    let mut state = TextAreaState::new();

    let outcome = handle_textarea_input_with_mode(
        &mut state,
        "a",
        &Key::default(),
        &keymap,
        InteractionMode::Enabled,
    );
    assert_eq!(outcome, InteractionOutcome::Changed("a".to_string()));
    assert_eq!(state.content(), "a");

    let outcome = handle_textarea_input_with_mode(
        &mut state,
        "b",
        &Key::default(),
        &keymap,
        InteractionMode::ReadOnly,
    );
    assert_eq!(outcome, InteractionOutcome::Ignored);
    assert_eq!(state.content(), "a");

    let outcome = handle_textarea_input_with_mode(
        &mut state,
        "",
        &left_key(),
        &keymap,
        InteractionMode::ReadOnly,
    );
    assert_eq!(outcome, InteractionOutcome::Handled);

    let outcome = handle_textarea_input_with_mode(
        &mut state,
        "c",
        &Key::default(),
        &keymap,
        InteractionMode::Disabled,
    );
    assert_eq!(outcome, InteractionOutcome::Ignored);
    assert_eq!(state.content(), "a");
}

#[test]
fn command_palette_interaction_contract() {
    let commands = vec![
        Command::new("file.open", "Open File"),
        Command::new("file.delete", "Delete File").disabled(true),
    ];
    let mut state = CommandPaletteState::new();
    state.open();

    let outcome = handle_command_palette_input(
        &mut state,
        &commands,
        "d",
        &Key::default(),
        InteractionMode::Enabled,
    );
    assert_eq!(outcome, InteractionOutcome::Changed("d".to_string()));
    assert_eq!(state.query, "d");

    let outcome = handle_command_palette_input(
        &mut state,
        &commands,
        "",
        &enter_key(),
        InteractionMode::Enabled,
    );
    assert_eq!(outcome, InteractionOutcome::Ignored);
    assert!(state.open);

    state.set_query("open");
    let outcome = handle_command_palette_input(
        &mut state,
        &commands,
        "",
        &enter_key(),
        InteractionMode::Enabled,
    );
    assert_eq!(
        outcome,
        InteractionOutcome::Submitted("file.open".to_string())
    );
    assert!(!state.open);

    state.open();
    let outcome = handle_command_palette_input(
        &mut state,
        &commands,
        "x",
        &Key::default(),
        InteractionMode::ReadOnly,
    );
    assert_eq!(outcome, InteractionOutcome::Ignored);
    assert!(state.query.is_empty());

    let outcome = handle_command_palette_input(
        &mut state,
        &commands,
        "x",
        &Key::default(),
        InteractionMode::Disabled,
    );
    assert_eq!(outcome, InteractionOutcome::Ignored);
}
