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
    // Theme
    BackgroundColors,
    // Bar chart
    Bar,
    BarChart,
    BarChartOrientation,
    BorderColors,
    // Box
    Box,
    ButtonColors,
    // Confirm
    ButtonStyle,
    // Table
    Cell,
    ComponentColors,
    Confirm,
    ConfirmState,
    ConfirmStyle,
    Constraint,
    // Cursor
    Cursor,
    CursorShape,
    CursorState,
    CursorStyle,
    // Modal
    Dialog,
    DialogState,
    // File picker
    FileEntry,
    FileFilter,
    FilePicker,
    FilePickerState,
    FilePickerStyle,
    FileType,
    // Progress
    Gauge,
    // Gradient
    Gradient,
    // Help
    Help,
    HelpMode,
    HelpStyle,
    // Hyperlink
    Hyperlink,
    HyperlinkBuilder,
    InputColors,
    KeyBinding,
    // Text
    Line,
    // List
    List,
    ListColors,
    ListItem,
    ListState,
    // Message
    Message,
    MessageRole,
    Modal,
    ModalAlign,
    // Multi-select
    MultiSelect,
    MultiSelectItem,
    MultiSelectStyle,
    // Navigation
    NavigationConfig,
    NavigationResult,
    // Newline
    Newline,
    // Notification
    Notification,
    NotificationBorder,
    NotificationItem,
    NotificationLevel,
    NotificationPosition,
    NotificationState,
    NotificationStyle,
    // Paginator
    Paginator,
    PaginatorState,
    PaginatorStyle,
    PaginatorType,
    Progress,
    ProgressColors,
    ProgressSymbols,
    Row,
    // Scrollable
    ScrollableBox,
    // Scrollbar
    Scrollbar,
    ScrollbarOrientation,
    ScrollbarSymbols,
    // Select input
    SelectInput,
    SelectInputStyle,
    SelectItem,
    SelectionState,
    SemanticColor,
    // Spacer
    Spacer,
    Span,
    // Sparkline
    Sparkline,
    // Spinner
    Spinner,
    SpinnerBuilder,
    // Static
    Static,
    // Timer
    StopwatchState,
    // Tabs
    Tab,
    Table,
    TableState,
    Tabs,
    Text,
    TextColors,
    // Text input
    TextInputHandle,
    TextInputOptions,
    TextInputState,
    Theme,
    ThemeBuilder,
    ThinkingBlock,
    TimerState,
    Toast,
    ToolCall,
    // Transform
    Transform,
    // Tree
    Tree,
    TreeNode,
    TreeState,
    TreeStyle,
    calculate_visible_range,
    cursor,
    editor_help,
    file_picker,
    fixed_bottom_layout,
    format_duration_hhmmss,
    format_duration_mmss,
    format_duration_precise,
    get_theme,
    gradient,
    handle_confirm_input,
    handle_list_navigation,
    handle_paginator_input,
    handle_tree_input,
    hyperlink,
    link,
    navigation_help,
    notification,
    rainbow,
    select_input,
    set_hyperlinks_supported,
    set_theme,
    static_output,
    supports_hyperlinks,
    toast,
    use_text_input,
    vim_navigation_help,
    virtual_scroll_view,
    with_theme,
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

// Hooks
pub use crate::hooks::{
    // App context
    AppContext,
    // Paste
    BracketedPasteGuard,
    // Command
    Deps,
    // Measure
    Dimensions,
    // Focus
    FocusManagerHandle,
    FocusState,
    // Input
    Key,
    MeasureContext,
    MeasureRef,
    // Memo
    MemoizedCallback,
    // Mouse
    Mouse,
    MouseAction,
    MouseButton,
    PasteEvent,
    PasteHandler,
    // Scroll
    ScrollHandle,
    ScrollState,
    // Signal
    Signal,
    // Stdio
    StderrHandle,
    StdinHandle,
    StdoutHandle,
    UseFocusOptions,
    // Window title
    WindowTitleGuard,
    clear_mouse_handlers,
    clear_paste_handlers,
    // Accessibility
    clear_screen_reader_cache,
    clear_window_title,
    disable_bracketed_paste,
    dispatch_mouse_event,
    dispatch_paste,
    enable_bracketed_paste,
    get_measure_context,
    is_bracketed_paste_enabled,
    is_mouse_enabled,
    measure_element,
    register_paste_handler,
    set_measure_context,
    set_mouse_enabled,
    set_screen_reader_enabled,
    set_window_title,
    use_app,
    use_callback,
    use_cmd,
    use_cmd_once,
    // Effect
    use_effect,
    use_effect_once,
    use_focus,
    use_focus_manager,
    // Frame rate
    use_frame_rate,
    use_input,
    use_is_screen_reader_enabled,
    use_measure,
    use_memo,
    use_mouse,
    use_paste,
    use_scroll,
    use_signal,
    use_stderr,
    use_stdin,
    use_stdout,
    use_window_title,
    use_window_title_fn,
};
