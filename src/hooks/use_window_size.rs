//! use_window_size hook for terminal dimensions
//!
//! Provides reactive terminal window size tracking.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//!
//! fn app() -> Element {
//!     let (width, height) = use_window_size();
//!
//!     Text::new(format!("Terminal: {}x{}", width, height)).into_element()
//! }
//! ```

use crossterm::terminal;

/// Get the current terminal size
pub fn get_terminal_size() -> (u16, u16) {
    terminal::size().unwrap_or((80, 24))
}

/// Hook to get reactive terminal window size
///
/// Returns (width, height) tuple that updates on resize.
pub fn use_window_size() -> (u16, u16) {
    get_terminal_size()
}

/// Hook to get only the terminal width
pub fn use_window_width() -> u16 {
    let (width, _) = use_window_size();
    width
}

/// Hook to get only the terminal height
pub fn use_window_height() -> u16 {
    let (_, height) = use_window_size();
    height
}

/// Check if terminal is wide enough for a given width
pub fn use_is_wide_enough(min_width: u16) -> bool {
    let (width, _) = use_window_size();
    width >= min_width
}

/// Check if terminal is tall enough for a given height
pub fn use_is_tall_enough(min_height: u16) -> bool {
    let (_, height) = use_window_size();
    height >= min_height
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_terminal_size() {
        let (w, h) = get_terminal_size();
        assert!(w > 0);
        assert!(h > 0);
    }

    #[test]
    fn test_use_window_size_compiles() {
        fn _test() {
            let (w, h) = use_window_size();
            let _ = w + h;
        }
    }

    #[test]
    fn test_use_window_width_compiles() {
        fn _test() {
            let w = use_window_width();
            let _ = w;
        }
    }

    #[test]
    fn test_use_is_wide_enough_compiles() {
        fn _test() {
            let _ = use_is_wide_enough(80);
        }
    }
}
