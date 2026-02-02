//! Viewport component for scrollable text content
//!
//! A high-performance scrollable view for displaying large text content,
//! similar to Bubbles' viewport component.
//!
//! # Features
//!
//! - Vertical and horizontal scrolling
//! - Keyboard navigation (vim-style and arrow keys)
//! - Mouse wheel support
//! - Line numbers
//! - Scrollbar indicator
//! - Customizable key bindings
//!
//! # Example
//!
//! ```ignore
//! use rnk::components::viewport::{Viewport, ViewportState, ViewportKeyMap};
//! use rnk::hooks::{use_signal, use_input};
//!
//! fn app() -> Element {
//!     let state = use_signal(|| {
//!         let mut s = ViewportState::new(80, 20);
//!         s.set_content(include_str!("long_file.txt"));
//!         s
//!     });
//!
//!     let keymap = ViewportKeyMap::default();
//!
//!     use_input({
//!         let state = state.clone();
//!         let keymap = keymap.clone();
//!         move |input, key| {
//!             let mut s = state.get();
//!             if handle_viewport_input(&mut s, input, key, &keymap) {
//!                 state.set(s);
//!             }
//!         }
//!     });
//!
//!     Viewport::new(&state.get())
//!         .line_numbers(true)
//!         .scrollbar(true)
//!         .into_element()
//! }
//! ```
//!
//! # Key Bindings
//!
//! Default key bindings:
//!
//! | Key | Action |
//! |-----|--------|
//! | `↑` / `k` | Scroll up one line |
//! | `↓` / `j` | Scroll down one line |
//! | `PageUp` / `Ctrl+B` | Page up |
//! | `PageDown` / `Ctrl+F` / `Space` | Page down |
//! | `Ctrl+U` | Half page up |
//! | `Ctrl+D` | Half page down |
//! | `Home` / `g` | Go to top |
//! | `End` / `G` | Go to bottom |
//! | `←` / `h` | Scroll left |
//! | `→` / `l` | Scroll right |
//! | `0` | Go to left edge |
//! | `$` | Go to right edge |

mod component;
mod keymap;
mod state;

pub use component::{Viewport, ViewportStyle, apply_viewport_action, handle_viewport_input};
pub use keymap::{KeyBinding, KeyType, Modifiers, ViewportAction, ViewportKeyMap};
pub use state::ViewportState;
