//! FilePicker demo - File/directory selection
//!
//! Run: cargo run --example file_picker_demo

use std::path::PathBuf;

use rnk::components::{
    FileEntry, FileFilter, FilePicker, FilePickerState, FilePickerStyle, FileType,
};

fn main() {
    println!("=== FilePicker Component Demo ===\n");

    // File types
    println!("--- File Types ---");
    let types = [
        FileType::File,
        FileType::Directory,
        FileType::Symlink,
        FileType::Hidden,
    ];

    for ft in types {
        println!("  {:?}: icon={} simple={}", ft, ft.icon(), ft.simple_icon());
    }
    println!();

    // File entries
    println!("--- FileEntry API ---");
    let file = FileEntry::file("main.rs", PathBuf::from("/src/main.rs"));
    println!("  FileEntry::file(\"main.rs\", ...):");
    println!("    name: {}", file.name);
    println!("    is_file: {}", file.is_file());
    println!("    is_directory: {}", file.is_directory());
    println!("    is_hidden: {}", file.is_hidden);
    println!();

    let dir = FileEntry::directory("src", PathBuf::from("/src"));
    println!("  FileEntry::directory(\"src\", ...):");
    println!("    is_directory: {}", dir.is_directory());
    println!();

    let hidden = FileEntry::file(".gitignore", PathBuf::from("/.gitignore"));
    println!("  FileEntry::file(\".gitignore\", ...):");
    println!("    is_hidden: {}", hidden.is_hidden);
    println!();

    // File filters
    println!("--- File Filters ---");
    let file = FileEntry::file("test.rs", PathBuf::from("/test.rs"));
    let dir = FileEntry::directory("src", PathBuf::from("/src"));
    let txt = FileEntry::file("readme.txt", PathBuf::from("/readme.txt"));

    let filters = [
        ("All", FileFilter::All),
        ("FilesOnly", FileFilter::FilesOnly),
        ("DirectoriesOnly", FileFilter::DirectoriesOnly),
        (
            "Extensions([.rs])",
            FileFilter::Extensions(vec![".rs".to_string()]),
        ),
    ];

    println!("  Testing against: test.rs, src/, readme.txt\n");
    for (name, filter) in filters {
        println!("  {}:", name);
        println!("    test.rs: {}", filter.matches(&file));
        println!("    src/: {}", filter.matches(&dir));
        println!("    readme.txt: {}", filter.matches(&txt));
        println!();
    }

    // FilePickerState
    println!("--- FilePickerState API ---");
    let mut state = FilePickerState::new(PathBuf::from("/home/user"));
    state.set_entries(create_sample_entries());

    println!("  FilePickerState::new(\"/home/user\"):");
    println!("    current_dir: {}", state.current_dir().display());
    println!("    entries: {}", state.entries().len());
    println!("    cursor: {}", state.cursor());
    println!();

    // Navigation
    println!("--- Navigation ---");
    println!("  cursor_down():");
    state.cursor_down();
    println!(
        "    cursor: {}, focused: {:?}",
        state.cursor(),
        state.focused().map(|e| &e.name)
    );

    state.cursor_down();
    state.cursor_down();
    println!("  cursor_down() x2:");
    println!(
        "    cursor: {}, focused: {:?}",
        state.cursor(),
        state.focused().map(|e| &e.name)
    );

    println!("  cursor_up():");
    state.cursor_up();
    println!("    cursor: {}", state.cursor());

    println!("  cursor_first():");
    state.cursor_first();
    println!("    cursor: {}", state.cursor());

    println!("  cursor_last():");
    state.cursor_last();
    println!("    cursor: {}", state.cursor());
    println!();

    // Selection
    println!("--- Selection ---");
    let mut state = FilePickerState::new(PathBuf::from("/home")).multi_select(true);
    state.set_entries(create_sample_entries());

    println!("  Initial selected: {:?}", state.selected());

    state.toggle_selection();
    println!(
        "  toggle_selection(): {:?}",
        state
            .selected()
            .iter()
            .map(|p| p.file_name())
            .collect::<Vec<_>>()
    );

    state.cursor_down();
    state.toggle_selection();
    println!(
        "  cursor_down() + toggle_selection(): {} selected",
        state.selected().len()
    );

    state.clear_selection();
    println!("  clear_selection(): {} selected", state.selected().len());
    println!();

    // Search
    println!("--- Search/Filter ---");
    let mut state = FilePickerState::new(PathBuf::from("/home"));
    state.set_entries(create_sample_entries());

    println!("  All entries: {}", state.visible_entries().len());

    state.set_search("rs");
    println!(
        "  set_search(\"rs\"): {} visible",
        state.visible_entries().len()
    );
    for entry in state.visible_entries() {
        println!("    - {}", entry.name);
    }

    state.clear_search();
    println!(
        "  clear_search(): {} visible",
        state.visible_entries().len()
    );
    println!();

    // Hidden files
    println!("--- Hidden Files ---");
    let mut state = FilePickerState::new(PathBuf::from("/home"));
    state.set_entries(vec![
        FileEntry::file(".gitignore", PathBuf::from("/home/.gitignore")),
        FileEntry::file(".env", PathBuf::from("/home/.env")),
        FileEntry::file("README.md", PathBuf::from("/home/README.md")),
    ]);

    println!(
        "  show_hidden=false: {} visible",
        state.visible_entries().len()
    );
    state.toggle_hidden();
    println!(
        "  toggle_hidden(): {} visible",
        state.visible_entries().len()
    );
    println!();

    // Style presets
    println!("--- Style Presets ---");
    println!("  FilePickerStyle::default()");
    println!("  FilePickerStyle::minimal() - no icons, no sizes");
    println!("  FilePickerStyle::detailed() - emoji icons, sizes");
    println!();

    // Render
    println!("--- FilePicker Render ---\n");
    let mut state = FilePickerState::new(PathBuf::from("/home/user/project"));
    state.set_entries(create_sample_entries());
    state.cursor_down();
    state.cursor_down();
    state.toggle_selection();

    let picker = FilePicker::new(&state).max_visible(8);
    println!("{}", picker.render());

    // With emoji icons
    println!("--- With Emoji Icons ---\n");
    let mut state = FilePickerState::new(PathBuf::from("/home/user/project"))
        .style(FilePickerStyle::detailed());
    state.set_entries(create_sample_entries());

    let picker = FilePicker::new(&state).max_visible(6);
    println!("{}", picker.render());

    // Usage example
    println!("\n--- Usage in TUI App ---");
    println!("```rust");
    println!("use rnk::components::{{FilePicker, FilePickerState, FileEntry}};");
    println!("use rnk::hooks::{{use_signal, use_input}};");
    println!("use std::fs;");
    println!();
    println!("fn file_browser() -> Element {{");
    println!("    let state = use_signal(|| {{");
    println!("        let mut s = FilePickerState::new(std::env::current_dir().unwrap());");
    println!("        s.set_entries(read_directory(&s.current_dir()));");
    println!("        s");
    println!("    }});");
    println!();
    println!("    use_input(move |input, key| {{");
    println!("        let mut s = state.get();");
    println!("        match key {{");
    println!("            Key::Up => s.cursor_up(),");
    println!("            Key::Down => s.cursor_down(),");
    println!("            Key::Enter => {{");
    println!("                if let Some(dir) = s.enter_directory() {{");
    println!("                    s.set_entries(read_directory(&dir));");
    println!("                }}");
    println!("            }}");
    println!("            Key::Backspace => {{");
    println!("                if let Some(dir) = s.go_parent() {{");
    println!("                    s.set_entries(read_directory(&dir));");
    println!("                }}");
    println!("            }}");
    println!("            Key::Char(' ') => s.toggle_selection(),");
    println!("            Key::Char('h') => s.toggle_hidden(),");
    println!("            _ => {{}}");
    println!("        }}");
    println!("        state.set(s);");
    println!("    }});");
    println!();
    println!("    FilePicker::new(&state.get())");
    println!("        .max_visible(15)");
    println!("        .into_element()");
    println!("}}");
    println!("```");
}

fn create_sample_entries() -> Vec<FileEntry> {
    vec![
        FileEntry::directory("src", PathBuf::from("/home/user/project/src")),
        FileEntry::directory("tests", PathBuf::from("/home/user/project/tests")),
        FileEntry::file("Cargo.toml", PathBuf::from("/home/user/project/Cargo.toml"))
            .with_size(1024),
        FileEntry::file("Cargo.lock", PathBuf::from("/home/user/project/Cargo.lock"))
            .with_size(45678),
        FileEntry::file("README.md", PathBuf::from("/home/user/project/README.md")).with_size(2048),
        FileEntry::file("main.rs", PathBuf::from("/home/user/project/main.rs")).with_size(512),
        FileEntry::file("lib.rs", PathBuf::from("/home/user/project/lib.rs")).with_size(8192),
        FileEntry::file(".gitignore", PathBuf::from("/home/user/project/.gitignore"))
            .with_size(128),
    ]
}
