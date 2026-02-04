//! UI icons (Nerd Font)

// Folders
/// Folder icon (closed)
pub const fn folder() -> &'static str { "" }

/// Folder icon (open)
pub const fn folder_open() -> &'static str { "" }

/// Empty folder icon
pub const fn folder_empty() -> &'static str { "" }

// Navigation
/// Arrow right
pub const fn arrow_right() -> &'static str { "" }

/// Arrow left
pub const fn arrow_left() -> &'static str { "" }

/// Arrow up
pub const fn arrow_up() -> &'static str { "" }

/// Arrow down
pub const fn arrow_down() -> &'static str { "" }

/// Chevron right
pub const fn chevron_right() -> &'static str { "" }

/// Chevron left
pub const fn chevron_left() -> &'static str { "" }

/// Chevron up
pub const fn chevron_up() -> &'static str { "" }

/// Chevron down
pub const fn chevron_down() -> &'static str { "" }

// Actions
/// Check/checkmark
pub const fn check() -> &'static str { "" }

/// Cross/X
pub const fn cross() -> &'static str { "" }

/// Plus
pub const fn plus() -> &'static str { "" }

/// Minus
pub const fn minus() -> &'static str { "" }

/// Edit/pencil
pub const fn edit() -> &'static str { "" }

/// Delete/trash
pub const fn trash() -> &'static str { "" }

/// Save
pub const fn save() -> &'static str { "" }

/// Copy
pub const fn copy() -> &'static str { "" }

/// Paste
pub const fn paste() -> &'static str { "" }

/// Cut
pub const fn cut() -> &'static str { "" }

/// Undo
pub const fn undo() -> &'static str { "" }

/// Redo
pub const fn redo() -> &'static str { "" }

/// Refresh
pub const fn refresh() -> &'static str { "" }

/// Search
pub const fn search() -> &'static str { "" }

/// Filter
pub const fn filter() -> &'static str { "" }

/// Sort
pub const fn sort() -> &'static str { "" }

// Status
/// Info
pub const fn info() -> &'static str { "" }

/// Warning
pub const fn warning() -> &'static str { "" }

/// Error
pub const fn error() -> &'static str { "" }

/// Success
pub const fn success() -> &'static str { "" }

/// Question
pub const fn question() -> &'static str { "" }

// UI Elements
/// Menu/hamburger
pub const fn menu() -> &'static str { "" }

/// Close
pub const fn close() -> &'static str { "" }

/// Settings/gear
pub const fn settings() -> &'static str { "" }

/// Home
pub const fn home() -> &'static str { "" }

/// User
pub const fn user() -> &'static str { "" }

/// Users/group
pub const fn users() -> &'static str { "" }

/// Lock
pub const fn lock() -> &'static str { "" }

/// Unlock
pub const fn unlock() -> &'static str { "" }

/// Eye (visible)
pub const fn eye() -> &'static str { "" }

/// Eye off (hidden)
pub const fn eye_off() -> &'static str { "" }

/// Star (empty)
pub const fn star() -> &'static str { "" }

/// Star (filled)
pub const fn star_filled() -> &'static str { "" }

/// Heart (empty)
pub const fn heart() -> &'static str { "" }

/// Heart (filled)
pub const fn heart_filled() -> &'static str { "" }

/// Bookmark
pub const fn bookmark() -> &'static str { "" }

/// Tag
pub const fn tag() -> &'static str { "" }

/// Link
pub const fn link() -> &'static str { "" }

/// External link
pub const fn external_link() -> &'static str { "" }

/// Download
pub const fn download() -> &'static str { "" }

/// Upload
pub const fn upload() -> &'static str { "" }

/// Clock/time
pub const fn clock() -> &'static str { "" }

/// Calendar
pub const fn calendar() -> &'static str { "" }

/// Bell/notification
pub const fn bell() -> &'static str { "" }

/// Mail/email
pub const fn mail() -> &'static str { "" }

/// Chat/message
pub const fn chat() -> &'static str { "" }

/// Terminal
pub const fn terminal() -> &'static str { "" }

/// Code
pub const fn code() -> &'static str { "" }

/// Bug
pub const fn bug() -> &'static str { "" }

/// Lightbulb/idea
pub const fn lightbulb() -> &'static str { "" }

/// Fire
pub const fn fire() -> &'static str { "" }

/// Rocket
pub const fn rocket() -> &'static str { "" }

/// Spinner/loading
pub const fn spinner() -> &'static str { "" }

// Tree
/// Tree branch
pub const fn tree_branch() -> &'static str { "├" }

/// Tree last branch
pub const fn tree_last() -> &'static str { "└" }

/// Tree vertical line
pub const fn tree_vertical() -> &'static str { "│" }

// Box drawing
/// Horizontal line
pub const fn line_horizontal() -> &'static str { "─" }

/// Vertical line
pub const fn line_vertical() -> &'static str { "│" }

/// Corner top-left
pub const fn corner_tl() -> &'static str { "┌" }

/// Corner top-right
pub const fn corner_tr() -> &'static str { "┐" }

/// Corner bottom-left
pub const fn corner_bl() -> &'static str { "└" }

/// Corner bottom-right
pub const fn corner_br() -> &'static str { "┘" }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_icons() {
        assert_eq!(folder(), "");
        assert_eq!(check(), "");
        assert_eq!(cross(), "");
    }
}
