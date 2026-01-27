//! Hooks system for reactive state management

pub mod context;
mod use_accessibility;
pub(crate) mod use_app;
mod use_cmd;
mod use_effect;
pub mod use_focus;
pub mod use_input;
mod use_measure;
pub mod use_mouse;
mod use_scroll;
mod use_signal;
mod use_stdio;
mod use_window_title;

pub use context::{HookContext, current_context, with_hooks};
pub use use_accessibility::{
    clear_screen_reader_cache, set_screen_reader_enabled, use_is_screen_reader_enabled,
};
pub use use_app::{AppContext, get_app_context, set_app_context, use_app};
pub use use_cmd::{Deps, use_cmd, use_cmd_once};
pub use use_effect::{use_effect, use_effect_once};
pub use use_focus::{
    FocusManagerHandle, FocusState, UseFocusOptions, use_focus, use_focus_manager,
};
pub use use_input::{Key, use_input};
pub use use_measure::{
    Dimensions, MeasureContext, MeasureRef, get_measure_context, measure_element,
    set_measure_context, use_measure,
};
pub use use_mouse::{
    Mouse, MouseAction, MouseButton, clear_mouse_handlers, dispatch_mouse_event, is_mouse_enabled,
    set_mouse_enabled, use_mouse,
};
pub use use_scroll::{ScrollHandle, ScrollState, use_scroll};
pub use use_signal::{Signal, use_signal};
pub use use_stdio::{StderrHandle, StdinHandle, StdoutHandle, use_stderr, use_stdin, use_stdout};
pub use use_window_title::{
    WindowTitleGuard, clear_window_title, set_window_title, use_window_title, use_window_title_fn,
};
