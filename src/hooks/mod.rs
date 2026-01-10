//! Hooks system for reactive state management

pub mod context;
mod use_signal;
mod use_effect;
pub mod use_input;
pub(crate) mod use_app;
mod use_focus;
mod use_stdio;
mod use_measure;
mod use_accessibility;

pub use context::{HookContext, with_hooks, current_context};
pub use use_signal::{Signal, use_signal};
pub use use_effect::{use_effect, use_effect_once};
pub use use_input::{use_input, Key};
pub use use_app::{use_app, AppContext, set_app_context, get_app_context};
pub use use_focus::{use_focus, use_focus_manager, FocusState, FocusManagerHandle, UseFocusOptions};
pub use use_stdio::{use_stdin, use_stdout, use_stderr, StdinHandle, StdoutHandle, StderrHandle};
pub use use_measure::{use_measure, measure_element, Dimensions, MeasureRef, MeasureContext, set_measure_context, get_measure_context};
pub use use_accessibility::{use_is_screen_reader_enabled, set_screen_reader_enabled, clear_screen_reader_cache};
