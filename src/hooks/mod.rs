//! Hooks system for reactive state management

pub mod context;
mod paste;
mod use_accessibility;
mod use_animation;
pub(crate) mod use_app;
mod use_cmd;
mod use_effect;
pub mod use_focus;
mod use_frame_rate;
pub mod use_input;
mod use_interval;
mod use_measure;
mod use_memo;
pub mod use_mouse;
mod use_scroll;
mod use_signal;
mod use_stdio;
mod use_transition;
mod use_window_title;

pub use context::{HookContext, current_context, with_hooks};
pub use paste::{
    BracketedPasteGuard, PasteEvent, PasteHandler, clear_paste_handlers, disable_bracketed_paste,
    dispatch_paste, enable_bracketed_paste, is_bracketed_paste_enabled, register_paste_handler,
    use_paste,
};
pub use use_accessibility::{
    clear_screen_reader_cache, set_screen_reader_enabled, use_is_screen_reader_enabled,
};
pub use use_animation::{AnimationHandle, use_animation, use_animation_auto};
pub use use_app::{AppContext, get_app_context, set_app_context, use_app};
pub use use_cmd::{Deps, use_cmd, use_cmd_once};
pub use use_effect::{use_effect, use_effect_once};
pub use use_focus::{
    FocusManagerHandle, FocusState, UseFocusOptions, use_focus, use_focus_manager,
};
pub use use_frame_rate::use_frame_rate;
pub use use_input::{Key, use_input};
pub use use_interval::{use_interval, use_interval_when, use_timeout};
pub use use_measure::{
    Dimensions, MeasureContext, MeasureRef, get_measure_context, measure_element,
    set_measure_context, use_measure,
};
pub use use_memo::{MemoizedCallback, use_callback, use_memo};
pub use use_mouse::{
    Mouse, MouseAction, MouseButton, clear_mouse_handlers, dispatch_mouse_event, is_mouse_enabled,
    set_mouse_enabled, use_mouse,
};
pub use use_scroll::{ScrollHandle, ScrollState, use_scroll};
pub use use_signal::{Signal, use_signal};
pub use use_stdio::{StderrHandle, StdinHandle, StdoutHandle, use_stderr, use_stdin, use_stdout};
pub use use_transition::{TransitionHandle, use_transition, use_transition_with_easing};
pub use use_window_title::{
    WindowTitleGuard, clear_window_title, set_window_title, use_window_title, use_window_title_fn,
};
