use super::*;

#[test]
fn test_file_type_icons() {
    assert_eq!(FileType::File.icon(), "📄");
    assert_eq!(FileType::Directory.icon(), "📁");
    assert_eq!(FileType::File.simple_icon(), "-");
    assert_eq!(FileType::Directory.simple_icon(), "d");
}

#[test]
fn test_file_entry_creation() {
    let entry = FileEntry::file("test.txt", PathBuf::from("/test.txt"));
    assert!(entry.is_file());
    assert!(!entry.is_directory());
    assert!(!entry.is_hidden);

    let dir = FileEntry::directory("src", PathBuf::from("/src"));
    assert!(dir.is_directory());
    assert!(!dir.is_file());
}

#[test]
fn test_file_entry_hidden() {
    let hidden = FileEntry::file(".gitignore", PathBuf::from("/.gitignore"));
    assert!(hidden.is_hidden);

    let visible = FileEntry::file("README.md", PathBuf::from("/README.md"));
    assert!(!visible.is_hidden);
}

#[test]
fn test_file_filter() {
    let file = FileEntry::file("test.rs", PathBuf::from("/test.rs"));
    let dir = FileEntry::directory("src", PathBuf::from("/src"));

    assert!(FileFilter::All.matches(&file));
    assert!(FileFilter::All.matches(&dir));

    assert!(FileFilter::FilesOnly.matches(&file));
    assert!(!FileFilter::FilesOnly.matches(&dir));

    assert!(!FileFilter::DirectoriesOnly.matches(&file));
    assert!(FileFilter::DirectoriesOnly.matches(&dir));

    let ext_filter = FileFilter::Extensions(vec![".rs".to_string()]);
    assert!(ext_filter.matches(&file));
    assert!(ext_filter.matches(&dir));
}

#[test]
fn test_file_picker_state_navigation() {
    let mut state = FilePickerState::new(PathBuf::from("/home"));
    state.set_entries(vec![
        FileEntry::directory("dir1", PathBuf::from("/home/dir1")),
        FileEntry::file("file1.txt", PathBuf::from("/home/file1.txt")),
        FileEntry::file("file2.txt", PathBuf::from("/home/file2.txt")),
    ]);

    assert_eq!(state.cursor(), 0);

    state.cursor_down();
    assert_eq!(state.cursor(), 1);

    state.cursor_down();
    assert_eq!(state.cursor(), 2);

    state.cursor_down();
    assert_eq!(state.cursor(), 2);

    state.cursor_up();
    assert_eq!(state.cursor(), 1);

    state.cursor_first();
    assert_eq!(state.cursor(), 0);

    state.cursor_last();
    assert_eq!(state.cursor(), 2);
}

#[test]
fn test_file_picker_state_selection() {
    let mut state = FilePickerState::new(PathBuf::from("/home"));
    state.set_entries(vec![
        FileEntry::file("file1.txt", PathBuf::from("/home/file1.txt")),
        FileEntry::file("file2.txt", PathBuf::from("/home/file2.txt")),
    ]);

    assert!(state.selected().is_empty());

    state.select();
    assert_eq!(state.selected().len(), 1);

    state.cursor_down();
    state.select();
    assert_eq!(state.selected().len(), 1);

    state.clear_selection();
    assert!(state.selected().is_empty());
}

#[test]
fn test_file_picker_state_multi_select() {
    let mut state = FilePickerState::new(PathBuf::from("/home")).multi_select(true);
    state.set_entries(vec![
        FileEntry::file("file1.txt", PathBuf::from("/home/file1.txt")),
        FileEntry::file("file2.txt", PathBuf::from("/home/file2.txt")),
    ]);

    state.toggle_selection();
    assert_eq!(state.selected().len(), 1);

    state.cursor_down();
    state.toggle_selection();
    assert_eq!(state.selected().len(), 2);

    state.toggle_selection();
    assert_eq!(state.selected().len(), 1);
}

#[test]
fn test_file_picker_state_search() {
    let mut state = FilePickerState::new(PathBuf::from("/home"));
    state.set_entries(vec![
        FileEntry::file("apple.txt", PathBuf::from("/home/apple.txt")),
        FileEntry::file("banana.txt", PathBuf::from("/home/banana.txt")),
        FileEntry::file("cherry.txt", PathBuf::from("/home/cherry.txt")),
    ]);

    assert_eq!(state.visible_entries().len(), 3);

    state.set_search("an");
    assert_eq!(state.visible_entries().len(), 1);
    assert_eq!(state.visible_entries()[0].name, "banana.txt");

    state.clear_search();
    assert_eq!(state.visible_entries().len(), 3);
}

#[test]
fn test_file_picker_state_hidden() {
    let mut state = FilePickerState::new(PathBuf::from("/home"));
    state.set_entries(vec![
        FileEntry::file(".hidden", PathBuf::from("/home/.hidden")),
        FileEntry::file("visible.txt", PathBuf::from("/home/visible.txt")),
    ]);

    assert_eq!(state.visible_entries().len(), 1);

    state.toggle_hidden();
    assert_eq!(state.visible_entries().len(), 2);
}

#[test]
fn test_format_size() {
    assert_eq!(format_size(500), "500B");
    assert_eq!(format_size(1024), "1.0K");
    assert_eq!(format_size(1536), "1.5K");
    assert_eq!(format_size(1048576), "1.0M");
    assert_eq!(format_size(1073741824), "1.0G");
}

#[test]
fn test_file_picker_render() {
    let mut state = FilePickerState::new(PathBuf::from("/home"));
    state.set_entries(vec![
        FileEntry::directory("src", PathBuf::from("/home/src")),
        FileEntry::file("main.rs", PathBuf::from("/home/main.rs")),
    ]);

    let picker = FilePicker::new(&state);
    let rendered = picker.render();

    assert!(rendered.contains("/home"));
    assert!(rendered.contains("src"));
    assert!(rendered.contains("main.rs"));
}

#[test]
fn test_handle_file_picker_input_submit_cancel_and_modes() {
    let mut state = FilePickerState::new(PathBuf::from("/home"));
    state.set_entries(vec![
        FileEntry::file("file1.txt", PathBuf::from("/home/file1.txt")),
        FileEntry::file("file2.txt", PathBuf::from("/home/file2.txt")),
    ]);

    let outcome = handle_file_picker_input(
        &mut state,
        "",
        &crate::hooks::Key {
            down_arrow: true,
            ..Default::default()
        },
        InteractionMode::ReadOnly,
    );
    assert_eq!(outcome, InteractionOutcome::Handled);
    assert_eq!(state.cursor(), 1);

    let outcome = handle_file_picker_input(
        &mut state,
        "",
        &crate::hooks::Key {
            space: true,
            ..Default::default()
        },
        InteractionMode::ReadOnly,
    );
    assert_eq!(outcome, InteractionOutcome::Ignored);
    assert!(state.selected().is_empty());

    let outcome = handle_file_picker_input(
        &mut state,
        "",
        &crate::hooks::Key {
            return_key: true,
            ..Default::default()
        },
        InteractionMode::Enabled,
    );
    assert_eq!(
        outcome,
        InteractionOutcome::Submitted(vec![PathBuf::from("/home/file2.txt")])
    );
    assert_eq!(
        state.submitted(),
        Some(&[PathBuf::from("/home/file2.txt")][..])
    );

    let outcome = handle_file_picker_input(
        &mut state,
        "",
        &crate::hooks::Key {
            escape: true,
            ..Default::default()
        },
        InteractionMode::Enabled,
    );
    assert_eq!(outcome, InteractionOutcome::Cancelled);
    assert!(state.is_cancelled());
}

#[test]
fn test_handle_file_picker_read_only_enters_directory() {
    let mut state = FilePickerState::new(PathBuf::from("/home"));
    state.set_entries(vec![FileEntry::directory(
        "src",
        PathBuf::from("/home/src"),
    )]);

    let outcome = handle_file_picker_input(
        &mut state,
        "",
        &crate::hooks::Key {
            return_key: true,
            ..Default::default()
        },
        InteractionMode::ReadOnly,
    );

    assert_eq!(outcome, InteractionOutcome::Handled);
    assert_eq!(state.current_dir(), Path::new("/home/src"));
    assert!(state.selected().is_empty());
}
