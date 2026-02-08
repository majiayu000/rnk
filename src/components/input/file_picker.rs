//! FilePicker component for file/directory selection
//!
//! Provides a file browser with navigation, filtering, and selection.

use std::path::{Path, PathBuf};

use crate::components::Text;
use crate::core::{Color, Element};

/// File type for display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// Regular file
    File,
    /// Directory
    Directory,
    /// Symbolic link
    Symlink,
    /// Hidden file/directory
    Hidden,
}

impl FileType {
    /// Get icon for file type
    pub fn icon(&self) -> &'static str {
        match self {
            FileType::File => "📄",
            FileType::Directory => "📁",
            FileType::Symlink => "🔗",
            FileType::Hidden => "👁",
        }
    }

    /// Get simple icon (ASCII)
    pub fn simple_icon(&self) -> &'static str {
        match self {
            FileType::File => "-",
            FileType::Directory => "d",
            FileType::Symlink => "l",
            FileType::Hidden => ".",
        }
    }
}

/// A file entry in the picker
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// File name
    pub name: String,
    /// Full path
    pub path: PathBuf,
    /// File type
    pub file_type: FileType,
    /// File size in bytes (for files)
    pub size: Option<u64>,
    /// Is hidden file
    pub is_hidden: bool,
}

impl FileEntry {
    /// Create a new file entry
    pub fn new(name: impl Into<String>, path: PathBuf, file_type: FileType) -> Self {
        let name = name.into();
        let is_hidden = name.starts_with('.');
        Self {
            name,
            path,
            file_type,
            size: None,
            is_hidden,
        }
    }

    /// Create a directory entry
    pub fn directory(name: impl Into<String>, path: PathBuf) -> Self {
        Self::new(name, path, FileType::Directory)
    }

    /// Create a file entry
    pub fn file(name: impl Into<String>, path: PathBuf) -> Self {
        Self::new(name, path, FileType::File)
    }

    /// Set file size
    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    /// Check if this is a directory
    pub fn is_directory(&self) -> bool {
        self.file_type == FileType::Directory
    }

    /// Check if this is a file
    pub fn is_file(&self) -> bool {
        self.file_type == FileType::File
    }
}

/// File picker filter mode
#[derive(Debug, Clone, Default)]
pub enum FileFilter {
    /// Show all files
    #[default]
    All,
    /// Only directories
    DirectoriesOnly,
    /// Only files
    FilesOnly,
    /// Filter by extensions
    Extensions(Vec<String>),
    /// Custom filter function name (for display)
    Custom(String),
}

impl FileFilter {
    /// Check if entry matches filter
    pub fn matches(&self, entry: &FileEntry) -> bool {
        match self {
            FileFilter::All => true,
            FileFilter::DirectoriesOnly => entry.is_directory(),
            FileFilter::FilesOnly => entry.is_file(),
            FileFilter::Extensions(exts) => {
                if entry.is_directory() {
                    true
                } else {
                    exts.iter().any(|ext| entry.name.ends_with(ext))
                }
            }
            FileFilter::Custom(_) => true, // Custom filters handled externally
        }
    }
}

/// File picker style
#[derive(Debug, Clone)]
pub struct FilePickerStyle {
    /// Show file icons
    pub show_icons: bool,
    /// Use emoji icons (vs ASCII)
    pub emoji_icons: bool,
    /// Show file sizes
    pub show_sizes: bool,
    /// Show hidden files
    pub show_hidden: bool,
    /// Directory color
    pub dir_color: Color,
    /// File color
    pub file_color: Color,
    /// Selected color
    pub selected_color: Color,
    /// Cursor color
    pub cursor_color: Color,
}

impl Default for FilePickerStyle {
    fn default() -> Self {
        Self {
            show_icons: true,
            emoji_icons: false,
            show_sizes: false,
            show_hidden: false,
            dir_color: Color::Blue,
            file_color: Color::White,
            selected_color: Color::Green,
            cursor_color: Color::Cyan,
        }
    }
}

impl FilePickerStyle {
    /// Create a new style
    pub fn new() -> Self {
        Self::default()
    }

    /// Show/hide icons
    pub fn show_icons(mut self, show: bool) -> Self {
        self.show_icons = show;
        self
    }

    /// Use emoji icons
    pub fn emoji_icons(mut self, emoji: bool) -> Self {
        self.emoji_icons = emoji;
        self
    }

    /// Show/hide file sizes
    pub fn show_sizes(mut self, show: bool) -> Self {
        self.show_sizes = show;
        self
    }

    /// Show/hide hidden files
    pub fn show_hidden(mut self, show: bool) -> Self {
        self.show_hidden = show;
        self
    }

    /// Set directory color
    pub fn dir_color(mut self, color: Color) -> Self {
        self.dir_color = color;
        self
    }

    /// Set file color
    pub fn file_color(mut self, color: Color) -> Self {
        self.file_color = color;
        self
    }

    /// Minimal style
    pub fn minimal() -> Self {
        Self::new().show_icons(false).show_sizes(false)
    }

    /// Detailed style
    pub fn detailed() -> Self {
        Self::new()
            .show_icons(true)
            .show_sizes(true)
            .emoji_icons(true)
    }
}

/// File picker state
#[derive(Debug, Clone)]
pub struct FilePickerState {
    /// Current directory
    current_dir: PathBuf,
    /// Entries in current directory
    entries: Vec<FileEntry>,
    /// Cursor position
    cursor: usize,
    /// Selected entries (for multi-select)
    selected: Vec<PathBuf>,
    /// Filter
    filter: FileFilter,
    /// Style
    style: FilePickerStyle,
    /// Search/filter text
    search: String,
    /// Allow multiple selection
    multi_select: bool,
    /// History for back navigation
    history: Vec<PathBuf>,
}

impl Default for FilePickerState {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

impl FilePickerState {
    /// Create a new file picker state
    pub fn new(path: PathBuf) -> Self {
        Self {
            current_dir: path,
            entries: Vec::new(),
            cursor: 0,
            selected: Vec::new(),
            filter: FileFilter::All,
            style: FilePickerStyle::default(),
            search: String::new(),
            multi_select: false,
            history: Vec::new(),
        }
    }

    /// Set filter
    pub fn filter(mut self, filter: FileFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Set style
    pub fn style(mut self, style: FilePickerStyle) -> Self {
        self.style = style;
        self
    }

    /// Enable multi-select
    pub fn multi_select(mut self, enabled: bool) -> Self {
        self.multi_select = enabled;
        self
    }

    /// Get current directory
    pub fn current_dir(&self) -> &Path {
        &self.current_dir
    }

    /// Get entries
    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }

    /// Get visible entries (filtered)
    pub fn visible_entries(&self) -> Vec<&FileEntry> {
        self.entries
            .iter()
            .filter(|e| {
                // Apply filter
                if !self.filter.matches(e) {
                    return false;
                }
                // Apply hidden filter
                if !self.style.show_hidden && e.is_hidden {
                    return false;
                }
                // Apply search filter
                if !self.search.is_empty() {
                    return e.name.to_lowercase().contains(&self.search.to_lowercase());
                }
                true
            })
            .collect()
    }

    /// Get cursor position
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Get focused entry
    pub fn focused(&self) -> Option<&FileEntry> {
        let visible = self.visible_entries();
        visible.get(self.cursor).copied()
    }

    /// Get selected entries
    pub fn selected(&self) -> &[PathBuf] {
        &self.selected
    }

    /// Check if entry is selected
    pub fn is_selected(&self, path: &Path) -> bool {
        self.selected.iter().any(|p| p == path)
    }

    /// Get search text
    pub fn search(&self) -> &str {
        &self.search
    }

    /// Set search text
    pub fn set_search(&mut self, search: impl Into<String>) {
        self.search = search.into();
        self.cursor = 0;
    }

    /// Clear search
    pub fn clear_search(&mut self) {
        self.search.clear();
    }

    /// Set entries (called after reading directory)
    pub fn set_entries(&mut self, entries: Vec<FileEntry>) {
        self.entries = entries;
        self.cursor = 0;
    }

    /// Move cursor up
    pub fn cursor_up(&mut self) {
        let visible_count = self.visible_entries().len();
        if visible_count > 0 && self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor down
    pub fn cursor_down(&mut self) {
        let visible_count = self.visible_entries().len();
        if visible_count > 0 && self.cursor < visible_count - 1 {
            self.cursor += 1;
        }
    }

    /// Move cursor to first entry
    pub fn cursor_first(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to last entry
    pub fn cursor_last(&mut self) {
        let visible_count = self.visible_entries().len();
        if visible_count > 0 {
            self.cursor = visible_count - 1;
        }
    }

    /// Page up
    pub fn page_up(&mut self, page_size: usize) {
        self.cursor = self.cursor.saturating_sub(page_size);
    }

    /// Page down
    pub fn page_down(&mut self, page_size: usize) {
        let visible_count = self.visible_entries().len();
        if visible_count > 0 {
            self.cursor = (self.cursor + page_size).min(visible_count - 1);
        }
    }

    /// Toggle selection of focused entry
    pub fn toggle_selection(&mut self) {
        if let Some(entry) = self.focused() {
            let path = entry.path.clone();
            if self.is_selected(&path) {
                self.selected.retain(|p| p != &path);
            } else {
                if !self.multi_select {
                    self.selected.clear();
                }
                self.selected.push(path);
            }
        }
    }

    /// Select focused entry
    pub fn select(&mut self) {
        if let Some(entry) = self.focused() {
            let path = entry.path.clone();
            if !self.is_selected(&path) {
                if !self.multi_select {
                    self.selected.clear();
                }
                self.selected.push(path);
            }
        }
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    /// Enter directory (if focused on directory)
    pub fn enter_directory(&mut self) -> Option<PathBuf> {
        // Get the path first to avoid borrow issues
        let new_dir = {
            let entry = self.focused()?;
            if !entry.is_directory() {
                return None;
            }
            entry.path.clone()
        };

        self.history.push(self.current_dir.clone());
        self.current_dir = new_dir.clone();
        self.cursor = 0;
        self.search.clear();
        Some(new_dir)
    }

    /// Go to parent directory
    pub fn go_parent(&mut self) -> Option<PathBuf> {
        if let Some(parent) = self.current_dir.parent() {
            self.history.push(self.current_dir.clone());
            let parent_path = parent.to_path_buf();
            self.current_dir = parent_path.clone();
            self.cursor = 0;
            self.search.clear();
            return Some(parent_path);
        }
        None
    }

    /// Go back in history
    pub fn go_back(&mut self) -> Option<PathBuf> {
        if let Some(prev) = self.history.pop() {
            self.current_dir = prev.clone();
            self.cursor = 0;
            self.search.clear();
            return Some(prev);
        }
        None
    }

    /// Navigate to specific directory
    pub fn navigate_to(&mut self, path: PathBuf) {
        self.history.push(self.current_dir.clone());
        self.current_dir = path;
        self.cursor = 0;
        self.search.clear();
    }

    /// Get style
    pub fn get_style(&self) -> &FilePickerStyle {
        &self.style
    }

    /// Toggle hidden files
    pub fn toggle_hidden(&mut self) {
        self.style.show_hidden = !self.style.show_hidden;
        self.cursor = 0;
    }
}

/// File picker component
#[derive(Debug)]
pub struct FilePicker<'a> {
    state: &'a FilePickerState,
    /// Maximum visible entries
    max_visible: usize,
    /// Show path header
    show_path: bool,
    /// Show status bar
    show_status: bool,
}

impl<'a> FilePicker<'a> {
    /// Create a new file picker
    pub fn new(state: &'a FilePickerState) -> Self {
        Self {
            state,
            max_visible: 10,
            show_path: true,
            show_status: true,
        }
    }

    /// Set maximum visible entries
    pub fn max_visible(mut self, max: usize) -> Self {
        self.max_visible = max;
        self
    }

    /// Show/hide path header
    pub fn show_path(mut self, show: bool) -> Self {
        self.show_path = show;
        self
    }

    /// Show/hide status bar
    pub fn show_status(mut self, show: bool) -> Self {
        self.show_status = show;
        self
    }

    /// Render the file picker
    pub fn render(&self) -> String {
        let mut output = String::new();
        let style = self.state.get_style();
        let visible = self.state.visible_entries();

        // Path header
        if self.show_path {
            output.push_str(&format!(
                "\x1b[1m{}\x1b[0m\n",
                self.state.current_dir.display()
            ));
            output.push_str(&"─".repeat(40));
            output.push('\n');
        }

        // Search bar
        if !self.state.search.is_empty() {
            output.push_str(&format!("🔍 {}\n", self.state.search));
        }

        // Calculate visible range
        let total = visible.len();
        let start = if total <= self.max_visible || self.state.cursor < self.max_visible / 2 {
            0
        } else if self.state.cursor > total - self.max_visible / 2 {
            total - self.max_visible
        } else {
            self.state.cursor - self.max_visible / 2
        };
        let end = (start + self.max_visible).min(total);

        // Scroll indicator (above)
        if start > 0 {
            output.push_str(&format!("  \x1b[90m↑ {} more above\x1b[0m\n", start));
        }

        // Entries
        for (i, entry) in visible.iter().enumerate().skip(start).take(end - start) {
            let is_focused = i == self.state.cursor;
            let is_selected = self.state.is_selected(&entry.path);

            // Cursor indicator
            if is_focused {
                output.push_str("\x1b[7m"); // Reverse video
            }

            // Selection indicator
            if is_selected {
                output.push_str("✓ ");
            } else {
                output.push_str("  ");
            }

            // Icon
            if style.show_icons {
                let icon = if style.emoji_icons {
                    entry.file_type.icon()
                } else {
                    entry.file_type.simple_icon()
                };
                output.push_str(icon);
                output.push(' ');
            }

            // Name with color
            let color = if entry.is_directory() {
                style.dir_color
            } else {
                style.file_color
            };
            output.push_str(&color.to_ansi_fg());
            output.push_str(&entry.name);
            output.push_str("\x1b[0m");

            // Size
            if style.show_sizes {
                if let Some(size) = entry.size {
                    output.push_str(&format!("  {}", format_size(size)));
                }
            }

            if is_focused {
                output.push_str("\x1b[0m");
            }

            output.push('\n');
        }

        // Scroll indicator (below)
        if end < total {
            output.push_str(&format!("  \x1b[90m↓ {} more below\x1b[0m\n", total - end));
        }

        // Status bar
        if self.show_status {
            output.push_str(&"─".repeat(40));
            output.push('\n');
            output.push_str(&format!("{}/{} items", self.state.cursor + 1, total));
            if !self.state.selected.is_empty() {
                output.push_str(&format!(" | {} selected", self.state.selected.len()));
            }
        }

        output
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        Text::new(self.render()).into_element()
    }
}

/// Format file size for display
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

#[cfg(test)]
mod tests {
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
        assert!(ext_filter.matches(&dir)); // Directories always match
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

        state.cursor_down(); // Should not go past last
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
        assert_eq!(state.selected().len(), 1); // Single select mode

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

        state.toggle_selection(); // Deselect
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

        assert_eq!(state.visible_entries().len(), 1); // Hidden not shown by default

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
}
