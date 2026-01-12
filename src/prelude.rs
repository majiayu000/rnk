//! Prelude module - commonly used imports
//!
//! This module re-exports the most commonly used types and functions
//! for convenience. Import with:
//!
//! ```ignore
//! use rnk::prelude::*;
//! ```

pub use crate::core::{
    AlignItems, BorderStyle, Color, Display, Element, ElementId, FlexDirection, JustifyContent,
    Overflow, Position, Style, TextWrap,
};

pub use crate::components::{
    Bar, BarChart, Box, Cell, Constraint, Gauge, Line, List, ListItem, ListState, Message,
    MessageRole, Newline, Progress, ProgressSymbols, Row, Scrollbar, ScrollbarSymbols, Spacer,
    Span, Sparkline, Spinner, SpinnerBuilder, Static, Tab, Table, TableState, Tabs, Text,
    ThinkingBlock, ToolCall, Transform, static_output,
};

// Rendering APIs
pub use crate::renderer::{
    AppBuilder,
    AppOptions,
    IntoPrintable,
    ModeSwitch,
    Printable,
    // Types
    RenderHandle,
    enter_alt_screen,
    exit_alt_screen,
    is_alt_screen,
    println,
    // Main entry points
    render,
    render_fullscreen,
    render_handle,
    render_inline,
    // Cross-thread APIs
    request_render,
};

// Hooks
pub use crate::hooks::{
    AppContext, Dimensions, FocusManagerHandle, FocusState, Key, MeasureRef, Mouse, MouseAction,
    MouseButton, ScrollHandle, ScrollState, Signal, StderrHandle, StdinHandle, StdoutHandle,
    UseFocusOptions, measure_element, set_window_title, use_app, use_effect, use_focus,
    use_focus_manager, use_input, use_is_screen_reader_enabled, use_measure, use_mouse, use_scroll,
    use_signal, use_stderr, use_stdin, use_stdout, use_window_title, use_window_title_fn,
};
