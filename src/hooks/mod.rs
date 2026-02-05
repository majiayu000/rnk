//! Hooks system for reactive state management

pub mod context;
mod paste;
mod use_accessibility;
mod use_animation;
pub(crate) mod use_app;
mod use_async;
mod use_clipboard;
mod use_cmd;
mod use_counter;
mod use_debounce;
mod use_effect;
pub mod use_focus;
mod use_form;
mod use_frame_rate;
mod use_history;
pub mod use_input;
mod use_interval;
mod use_list;
mod use_local_storage;
mod use_map;
mod use_measure;
mod use_media_query;
mod use_memo;
pub mod use_mouse;
mod use_previous;
mod use_reducer;
mod use_scroll;
mod use_set;
mod use_signal;
mod use_stdio;
mod use_toggle;
mod use_transition;
mod use_window_size;
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
pub use use_async::{AsyncHandle, AsyncState, use_async_state, use_async_state_with};
pub use use_clipboard::{ClipboardHandle, is_clipboard_available, read_clipboard, use_clipboard, write_clipboard};
pub use use_cmd::{Deps, use_cmd, use_cmd_once};
pub use use_counter::{CounterHandle, use_counter, use_counter_zero};
pub use use_debounce::{DebounceHandle, use_debounce, use_debounce_handle, use_throttle};
pub use use_effect::{use_effect, use_effect_once};
pub use use_focus::{
    FocusManagerHandle, FocusState, UseFocusOptions, use_focus, use_focus_manager,
};
pub use use_form::{FormField, FormHandle, use_form, use_form_empty};
pub use use_frame_rate::use_frame_rate;
pub use use_history::{HistoryHandle, use_history, use_history_with_size};
pub use use_input::{Key, use_input};
pub use use_interval::{use_interval, use_interval_when, use_timeout};
pub use use_list::{ListHandle, use_list, use_list_empty};
pub use use_local_storage::{LocalStorageHandle, use_local_storage, use_local_storage_with_dir};
pub use use_map::{MapHandle, use_map, use_map_empty, use_map_from};
pub use use_measure::{
    Dimensions, MeasureContext, MeasureRef, get_measure_context, measure_element,
    set_measure_context, use_measure,
};
pub use use_media_query::{
    Breakpoint, MediaQuery, use_breakpoint, use_breakpoint_down, use_breakpoint_only,
    use_breakpoint_up, use_is_landscape, use_is_portrait, use_media_query,
};
pub use use_memo::{MemoizedCallback, use_callback, use_memo};
pub use use_mouse::{
    Mouse, MouseAction, MouseButton, clear_mouse_handlers, dispatch_mouse_event, is_mouse_enabled,
    set_mouse_enabled, use_mouse,
};
pub use use_previous::{use_changed, use_is_first_render, use_previous};
pub use use_reducer::{Dispatch, use_reducer, use_reducer_lazy};
pub use use_scroll::{ScrollHandle, ScrollState, use_scroll};
pub use use_set::{SetHandle, use_set, use_set_empty};
pub use use_signal::{Signal, use_signal};
pub use use_stdio::{StderrHandle, StdinHandle, StdoutHandle, use_stderr, use_stdin, use_stdout};
pub use use_toggle::{ToggleHandle, use_toggle, use_toggle_off, use_toggle_on};
pub use use_transition::{TransitionHandle, use_transition, use_transition_with_easing};
pub use use_window_size::{
    get_terminal_size, use_is_tall_enough, use_is_wide_enough, use_window_height, use_window_size,
    use_window_width,
};
pub use use_window_title::{
    WindowTitleGuard, clear_window_title, set_window_title, use_window_title, use_window_title_fn,
};
