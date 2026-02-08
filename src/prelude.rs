//! # Prelude - Common Imports
//!
//! This module re-exports the most commonly used types and functions
//! for convenience. Import with:
//!
//! ```ignore
//! use rnk::prelude::*;
//! ```
//!
//! ## Functional Groups
//!
//! - **Core**: Element, Style, Color, layout primitives
//! - **Layout Components**: Box, Spacer, Transform, Static
//! - **Display Components**: Text, List, Table, Tree, Tabs
//! - **Input Components**: TextInput, SelectInput, MultiSelect, Confirm
//! - **Feedback Components**: Progress, Spinner, Notification, Toast, Modal
//! - **Navigation Components**: Paginator, Scrollbar, Help
//! - **Hooks - State**: use_signal, use_memo, use_callback
//! - **Hooks - Effects**: use_effect, use_cmd
//! - **Hooks - Input**: use_input, use_mouse, use_paste, use_focus
//! - **Hooks - Utilities**: use_scroll, use_measure, use_app
//! - **Hooks - Animation**: use_animation, use_transition
//! - **Rendering**: render, AppBuilder, render_to_string

// =============================================================================
// Core Types
// =============================================================================

pub use crate::core::{
    AlignItems, BorderStyle, Color, Display, Element, ElementId, FlexDirection, JustifyContent,
    Overflow, Position, Style, TextWrap,
};

// =============================================================================
// Layout Components
// =============================================================================

pub use crate::components::{Box, Spacer, Static, Transform};

// =============================================================================
// Display Components - Text & Content
// =============================================================================

pub use crate::components::{
    Cursor, CursorShape, CursorState, CursorStyle, Gradient, Hyperlink, HyperlinkBuilder, Line,
    Message, MessageRole, Newline, Span, Text, ThinkingBlock, ToolCall, set_hyperlinks_supported,
    supports_hyperlinks,
};

// =============================================================================
// Display Components - Data Visualization
// =============================================================================

pub use crate::components::{
    Bar, BarChart, BarChartOrientation, Cell, Constraint, List, ListColors, ListItem, ListState,
    Row, Sparkline, Tab, Table, TableState, Tabs, Tree, TreeNode, TreeState, TreeStyle,
    calculate_visible_range, handle_list_navigation, handle_tree_input, virtual_scroll_view,
};

// =============================================================================
// Input Components
// =============================================================================

pub use crate::components::{
    Confirm, ConfirmState, ConfirmStyle, FileEntry, FileFilter, FilePicker, FilePickerState,
    FilePickerStyle, FileType, MultiSelect, MultiSelectItem, MultiSelectStyle, SelectInput,
    SelectInputStyle, SelectItem, SelectionState, TextInputHandle, TextInputOptions,
    TextInputState, handle_confirm_input, use_text_input,
};

// =============================================================================
// Feedback Components
// =============================================================================

pub use crate::components::{
    Dialog, DialogState, Gauge, Modal, ModalAlign, Notification, NotificationBorder,
    NotificationItem, NotificationLevel, NotificationPosition, NotificationState,
    NotificationStyle, Progress, ProgressColors, ProgressSymbols, Spinner, SpinnerBuilder,
    StopwatchState, TimerState, Toast,
};

// =============================================================================
// Navigation Components
// =============================================================================

pub use crate::components::{
    Help, HelpMode, HelpStyle, KeyBinding, NavigationConfig, NavigationResult, Paginator,
    PaginatorState, PaginatorStyle, PaginatorType, ScrollableBox, Scrollbar, ScrollbarOrientation,
    ScrollbarSymbols, editor_help, fixed_bottom_layout, handle_paginator_input, navigation_help,
    vim_navigation_help,
};

// =============================================================================
// Theming
// =============================================================================

pub use crate::components::{
    BackgroundColors, BorderColors, ButtonColors, ButtonStyle, ComponentColors, InputColors,
    SemanticColor, TextColors, Theme, ThemeBuilder, get_theme, set_theme, with_theme,
};

// =============================================================================
// Duration Formatting
// =============================================================================

pub use crate::components::{
    format_duration_hhmmss, format_duration_mmss, format_duration_precise,
};

// =============================================================================
// Rendering APIs
// =============================================================================

pub use crate::renderer::{
    // Types
    AppBuilder,
    AppOptions,
    IntoPrintable,
    ModeSwitch,
    Printable,
    RenderHandle,
    // Alt screen control
    enter_alt_screen,
    exit_alt_screen,
    is_alt_screen,
    // Println
    println,
    println_trimmed,
    // Main entry points
    render,
    render_fullscreen,
    render_handle,
    render_inline,
    // Element rendering APIs
    render_to_string,
    render_to_string_auto,
    render_to_string_no_trim,
    // Cross-thread APIs
    request_render,
};

// =============================================================================
// Hooks - State Management
// =============================================================================

pub use crate::hooks::{Deps, MemoizedCallback, Signal, use_callback, use_memo, use_signal};

// =============================================================================
// Hooks - Side Effects
// =============================================================================

pub use crate::hooks::{use_cmd, use_cmd_once, use_effect, use_effect_once};

// =============================================================================
// Hooks - Animation
// =============================================================================

pub use crate::hooks::{
    AnimationHandle, TransitionHandle, use_animation, use_animation_auto, use_transition,
    use_transition_with_easing,
};

// =============================================================================
// Hooks - Input Handling
// =============================================================================

pub use crate::hooks::{
    BracketedPasteGuard, Key, Mouse, MouseAction, MouseButton, PasteEvent, clear_mouse_handlers,
    disable_bracketed_paste, dispatch_mouse_event, dispatch_paste, enable_bracketed_paste,
    is_bracketed_paste_enabled, is_mouse_enabled, set_mouse_enabled, use_input, use_mouse,
    use_paste,
};

// =============================================================================
// Hooks - Focus Management
// =============================================================================

pub use crate::hooks::{
    FocusManagerHandle, FocusState, UseFocusOptions, use_focus, use_focus_manager,
};

// =============================================================================
// Hooks - Scroll & Measure
// =============================================================================

pub use crate::hooks::{
    Dimensions, MeasureContext, MeasureRef, ScrollHandle, ScrollState, measure_element,
    use_measure, use_scroll,
};

// =============================================================================
// Hooks - App Context & Utilities
// =============================================================================

pub use crate::hooks::{
    AppContext, StderrHandle, StdinHandle, StdoutHandle, WindowTitleGuard,
    clear_screen_reader_cache, clear_window_title, set_screen_reader_enabled, set_window_title,
    use_app, use_frame_rate, use_is_screen_reader_enabled, use_stderr, use_stdin, use_stdout,
    use_window_title, use_window_title_fn,
};
