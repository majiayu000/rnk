//! TextArea component for multi-line text editing
//!
//! A multi-line text input component similar to Bubbles' textarea.
//!
//! # Features
//!
//! - Multi-line text editing
//! - Cursor navigation (character, word, line, document)
//! - Text selection
//! - Line numbers
//! - Customizable key bindings
//! - Placeholder text
//! - Character/line limits
//! - Soft tabs (spaces) or hard tabs
//!
//! # Example
//!
//! ```ignore
//! use rnk::components::textarea::{TextArea, TextAreaState, TextAreaKeyMap, handle_textarea_input};
//! use rnk::hooks::{use_signal, use_input};
//!
//! fn app() -> Element {
//!     let state = use_signal(|| {
//!         let mut s = TextAreaState::new();
//!         s.set_placeholder("Enter your text here...");
//!         s
//!     });
//!
//!     let keymap = TextAreaKeyMap::default();
//!
//!     use_input({
//!         let state = state.clone();
//!         let keymap = keymap.clone();
//!         move |input, key| {
//!             let mut s = state.get();
//!             handle_textarea_input(&mut s, input, key, &keymap);
//!             state.set(s);
//!         }
//!     });
//!
//!     TextArea::new(&state.get())
//!         .focused(true)
//!         .line_numbers(true)
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
//! | `←` / `→` | Move cursor left/right |
//! | `↑` / `↓` | Move cursor up/down |
//! | `Ctrl+←` / `Alt+B` | Move to previous word |
//! | `Ctrl+→` / `Alt+F` | Move to next word |
//! | `Home` / `Ctrl+A` | Move to line start |
//! | `End` / `Ctrl+E` | Move to line end |
//! | `Backspace` | Delete before cursor |
//! | `Delete` | Delete after cursor |
//! | `Ctrl+Backspace` / `Ctrl+W` | Delete word before |
//! | `Ctrl+K` | Delete line |
//! | `Enter` | Insert newline |
//! | `Tab` | Insert tab/spaces |

mod component;
mod keymap;
mod state;

pub use component::{TextArea, TextAreaStyle, apply_textarea_action, handle_textarea_input};
pub use keymap::{KeyBinding, KeyType, Modifiers, TextAreaAction, TextAreaKeyMap};
pub use state::{Position, Selection, TextAreaState};
