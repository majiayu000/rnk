use rnk::prelude::*;
use std::path::{Path, PathBuf};

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

fn right_key() -> Key {
    Key {
        right_arrow: true,
        ..Default::default()
    }
}

fn down_key() -> Key {
    Key {
        down_arrow: true,
        ..Default::default()
    }
}

fn space_key() -> Key {
    Key {
        space: true,
        ..Default::default()
    }
}

fn tab_key() -> Key {
    Key {
        tab: true,
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

#[test]
fn multi_select_interaction_contract() {
    let config = NavigationConfig::new().vim_navigation(true);
    let mut state = MultiSelectState::new(0, vec![false, true, false]);

    let outcome = handle_multi_select_input(
        &mut state,
        3,
        "",
        &space_key(),
        &config,
        InteractionMode::Enabled,
    );
    assert_eq!(outcome, InteractionOutcome::Changed(vec![0, 1]));
    assert_eq!(state.selected_indices(), vec![0, 1]);

    let outcome = handle_multi_select_input(
        &mut state,
        3,
        "",
        &enter_key(),
        &config,
        InteractionMode::Enabled,
    );
    assert_eq!(outcome, InteractionOutcome::Submitted(vec![0, 1]));

    let mut read_only_state = MultiSelectState::new(0, vec![false, false]);
    let outcome = handle_multi_select_input(
        &mut read_only_state,
        2,
        "j",
        &Key::default(),
        &config,
        InteractionMode::ReadOnly,
    );
    assert_eq!(outcome, InteractionOutcome::Handled);
    assert_eq!(read_only_state.highlighted(), 1);

    let outcome = handle_multi_select_input(
        &mut read_only_state,
        2,
        "",
        &space_key(),
        &config,
        InteractionMode::ReadOnly,
    );
    assert_eq!(outcome, InteractionOutcome::Ignored);
    assert!(read_only_state.selected_indices().is_empty());

    let outcome = handle_multi_select_input(
        &mut read_only_state,
        2,
        "k",
        &Key::default(),
        &config,
        InteractionMode::Disabled,
    );
    assert_eq!(outcome, InteractionOutcome::Ignored);
    assert_eq!(read_only_state.highlighted(), 1);
}

#[test]
fn confirm_interaction_contract() {
    let mut state = ConfirmState::new("Delete?");

    let outcome =
        handle_confirm_input_with_mode(&mut state, "", &tab_key(), InteractionMode::Enabled);
    assert_eq!(outcome, InteractionOutcome::Handled);
    assert!(state.is_yes_focused());

    let outcome =
        handle_confirm_input_with_mode(&mut state, "", &enter_key(), InteractionMode::Enabled);
    assert_eq!(outcome, InteractionOutcome::Submitted(true));
    assert!(state.is_confirmed());

    let mut read_only_state = ConfirmState::new("Delete?");
    let outcome = handle_confirm_input_with_mode(
        &mut read_only_state,
        "",
        &tab_key(),
        InteractionMode::ReadOnly,
    );
    assert_eq!(outcome, InteractionOutcome::Handled);
    assert!(read_only_state.is_yes_focused());

    let outcome = handle_confirm_input_with_mode(
        &mut read_only_state,
        "y",
        &Key::default(),
        InteractionMode::ReadOnly,
    );
    assert_eq!(outcome, InteractionOutcome::Ignored);
    assert!(!read_only_state.is_answered());

    let outcome = handle_confirm_input_with_mode(
        &mut read_only_state,
        "n",
        &Key::default(),
        InteractionMode::Disabled,
    );
    assert_eq!(outcome, InteractionOutcome::Ignored);
    assert!(!read_only_state.is_answered());
}

#[test]
fn file_picker_interaction_contract() {
    let mut state = FilePickerState::new(PathBuf::from("/home"));
    state.set_entries(vec![
        FileEntry::file("file1.txt", PathBuf::from("/home/file1.txt")),
        FileEntry::file("file2.txt", PathBuf::from("/home/file2.txt")),
    ]);

    let outcome = handle_file_picker_input(&mut state, "", &down_key(), InteractionMode::ReadOnly);
    assert_eq!(outcome, InteractionOutcome::Handled);
    assert_eq!(state.cursor(), 1);

    let outcome = handle_file_picker_input(&mut state, "", &space_key(), InteractionMode::ReadOnly);
    assert_eq!(outcome, InteractionOutcome::Ignored);
    assert!(state.selected().is_empty());

    let outcome = handle_file_picker_input(&mut state, "", &enter_key(), InteractionMode::Enabled);
    assert_eq!(
        outcome,
        InteractionOutcome::Submitted(vec![PathBuf::from("/home/file2.txt")])
    );
    assert_eq!(
        state.submitted(),
        Some(&[PathBuf::from("/home/file2.txt")][..])
    );

    let mut dir_state = FilePickerState::new(PathBuf::from("/home"));
    dir_state.set_entries(vec![FileEntry::directory(
        "src",
        PathBuf::from("/home/src"),
    )]);
    let outcome =
        handle_file_picker_input(&mut dir_state, "", &enter_key(), InteractionMode::ReadOnly);
    assert_eq!(outcome, InteractionOutcome::Handled);
    assert_eq!(dir_state.current_dir(), Path::new("/home/src"));

    let mut disabled_state = FilePickerState::new(PathBuf::from("/home"));
    disabled_state.set_entries(vec![FileEntry::file(
        "file.txt",
        PathBuf::from("/home/file.txt"),
    )]);
    let outcome = handle_file_picker_input(
        &mut disabled_state,
        "",
        &down_key(),
        InteractionMode::Disabled,
    );
    assert_eq!(outcome, InteractionOutcome::Ignored);
    assert_eq!(disabled_state.cursor(), 0);
}

#[test]
fn color_picker_interaction_contract() {
    let palette = ColorPalette::basic();
    let mut state = ColorPickerState::new();

    let outcome = handle_color_picker_input(
        &mut state,
        &palette.colors,
        8,
        &right_key(),
        InteractionMode::Enabled,
    );
    assert_eq!(outcome, InteractionOutcome::Changed(Color::Red));
    assert_eq!(state.selected, 1);

    let outcome = handle_color_picker_input(
        &mut state,
        &palette.colors,
        8,
        &enter_key(),
        InteractionMode::Enabled,
    );
    assert_eq!(outcome, InteractionOutcome::Submitted(Color::Red));

    let outcome = handle_color_picker_input(
        &mut state,
        &palette.colors,
        8,
        &right_key(),
        InteractionMode::ReadOnly,
    );
    assert_eq!(outcome, InteractionOutcome::Ignored);
    assert_eq!(state.selected, 1);

    let outcome = handle_color_picker_input(
        &mut state,
        &palette.colors,
        8,
        &left_key(),
        InteractionMode::Disabled,
    );
    assert_eq!(outcome, InteractionOutcome::Ignored);
    assert_eq!(state.selected, 1);
}
