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
    // Accordion
    Accordion,
    AccordionItem,
    // Alert
    Alert,
    AlertLevel,
    // Avatar
    Avatar,
    AvatarSize,
    // Badge
    Badge,
    BadgeVariant,
    // Bar chart
    Bar,
    BarChart,
    BarChartOrientation,
    // Box
    Box,
    // Breadcrumb
    Breadcrumb,
    // Calendar
    Calendar,
    // Card
    Card,
    // Table
    Cell,
    // Chip
    Chip,
    // Code editor
    CodeEditor,
    // Color picker
    ColorPalette,
    ColorPicker,
    ColorPickerState,
    ColorPickerStyle,
    // Command palette
    Command,
    CommandPalette,
    CommandPaletteState,
    CommandPaletteStyle,
    // Confirm
    ButtonStyle,
    Confirm,
    ConfirmState,
    ConfirmStyle,
    Constraint,
    // Context menu
    ContextMenu,
    ContextMenuState,
    ContextMenuStyle,
    // Cursor
    Cursor,
    CursorShape,
    CursorState,
    CursorStyle,
    // DevTools
    DevTools,
    DevToolsTab,
    // Modal
    Dialog,
    DialogState,
    // Divider
    Divider,
    DividerOrientation,
    DividerStyle,
    // Empty state
    EmptyState,
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
    // Highlight
    Highlight,
    HighlightVariant,
    // Hyperlink
    Hyperlink,
    HyperlinkBuilder,
    // Key hint
    KeyHint,
    KeyBinding,
    // Theme
    BackgroundColors,
    BorderColors,
    ButtonColors,
    ComponentColors,
    InputColors,
    ListColors,
    ProgressColors,
    SemanticColor,
    TextColors,
    // Language
    Language,
    // Text
    Line,
    // Line chart
    LineChart,
    // Link
    Link,
    // List
    List,
    ListItem,
    ListState,
    // Markdown
    Markdown,
    // Context menu item
    MenuItem,
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
    // Popover
    Popover,
    PopoverArrow,
    PopoverBorder,
    PopoverPosition,
    PopoverStyle,
    Progress,
    ProgressSymbols,
    // Quote
    Quote,
    QuoteStyle,
    // Rating
    Rating,
    RatingStyle,
    RatingSymbols,
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
    // Skeleton
    Skeleton,
    SkeletonVariant,
    // Spacer
    Spacer,
    Span,
    // Sparkline
    Sparkline,
    // Spinner
    Spinner,
    SpinnerBuilder,
    // Stat
    Stat,
    // Static
    Static,
    // Status bar
    StatusBar,
    // Stepper
    Step,
    StepStatus,
    Stepper,
    StepperOrientation,
    StepperStyle,
    // Timer
    StopwatchState,
    // Tabs
    Tab,
    Table,
    TableState,
    Tabs,
    // Tag
    Tag,
    Text,
    // Text input
    TextInputHandle,
    TextInputOptions,
    TextInputState,
    Theme,
    ThemeBuilder,
    ThinkingBlock,
    TimerState,
    Toast,
    // Tooltip
    Tooltip,
    TooltipPosition,
    ToolCall,
    // Transform
    Transform,
    // Trend
    Trend,
    // Tree
    Tree,
    TreeNode,
    TreeState,
    TreeStyle,
    // Series
    Series,
    // Functions
    alert_error,
    alert_info,
    alert_success,
    alert_warning,
    avatar,
    avatar_initials,
    badge_error,
    badge_primary,
    badge_success,
    badge_warning,
    breadcrumb_from_path,
    calculate_visible_range,
    card,
    card_full,
    chip,
    chip_selected,
    color_picker,
    color_picker_with_palette,
    command_palette,
    context_menu,
    cursor,
    editor_help,
    empty_state,
    empty_state_with_icon,
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
    highlight,
    highlight_error,
    highlight_primary,
    highlight_success,
    highlight_warning,
    hr,
    hr_dashed,
    hr_label,
    hyperlink,
    key_hint,
    key_hints,
    link,
    link_styled,
    link_with_icon,
    navigation_help,
    notification,
    popover,
    popover_with_content,
    quote,
    quote_with_author,
    rainbow,
    rating,
    rating_of,
    select_input,
    set_hyperlinks_supported,
    set_theme,
    skeleton_paragraph,
    skeleton_text,
    stat,
    stat_down,
    stat_up,
    static_output,
    status_bar,
    status_bar_full,
    stepper,
    supports_hyperlinks,
    tag,
    tag_colored,
    toast,
    tooltip,
    tooltip_left,
    tooltip_right,
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
    // Animation
    AnimationHandle,
    // App context
    AppContext,
    // Async
    AsyncHandle,
    AsyncState,
    // Paste
    BracketedPasteGuard,
    // Breakpoint
    Breakpoint,
    // Clipboard
    ClipboardHandle,
    // Counter
    CounterHandle,
    // Debounce
    DebounceHandle,
    // Command
    Deps,
    // Measure
    Dimensions,
    // Reducer
    Dispatch,
    // Focus
    FocusManagerHandle,
    FocusState,
    // Form
    FormField,
    FormHandle,
    // History
    HistoryHandle,
    // Hook context
    HookContext,
    // Idle
    IdleConfig,
    IdleState,
    // Input
    Key,
    // List
    ListHandle,
    // Local storage
    LocalStorageHandle,
    // Map
    MapHandle,
    MeasureContext,
    MeasureRef,
    // Media query
    MediaQuery,
    // Memo
    MemoizedCallback,
    // Keyboard shortcut
    Modifiers,
    // Mouse
    Mouse,
    MouseAction,
    MouseButton,
    // Network
    NetworkStatus,
    PasteEvent,
    PasteHandler,
    // Scroll
    ScrollHandle,
    ScrollState,
    // Set
    SetHandle,
    Shortcut,
    ShortcutKey,
    // Signal
    Signal,
    // Stdio
    StderrHandle,
    StdinHandle,
    StdoutHandle,
    // Toggle
    ToggleHandle,
    // Transition
    TransitionHandle,
    UseFocusOptions,
    // Window title
    WindowTitleGuard,
    // Functions
    check_host_reachable,
    check_online,
    clear_mouse_handlers,
    clear_paste_handlers,
    // Accessibility
    clear_screen_reader_cache,
    clear_window_title,
    current_context,
    disable_bracketed_paste,
    dispatch_mouse_event,
    dispatch_paste,
    enable_bracketed_paste,
    get_app_context,
    get_measure_context,
    get_terminal_size,
    idle_duration,
    is_bracketed_paste_enabled,
    is_clipboard_available,
    is_idle,
    is_mouse_enabled,
    measure_element,
    read_clipboard,
    record_activity,
    register_paste_handler,
    set_app_context,
    set_measure_context,
    set_mouse_enabled,
    set_screen_reader_enabled,
    set_window_title,
    use_animation,
    use_animation_auto,
    use_app,
    use_async_state,
    use_async_state_with,
    use_breakpoint,
    use_breakpoint_down,
    use_breakpoint_only,
    use_breakpoint_up,
    use_callback,
    use_changed,
    use_clipboard,
    use_cmd,
    use_cmd_once,
    use_counter,
    use_counter_zero,
    use_debounce,
    use_debounce_handle,
    // Effect
    use_effect,
    use_effect_once,
    use_focus,
    use_focus_manager,
    use_form,
    use_form_empty,
    // Frame rate
    use_frame_rate,
    use_history,
    use_history_with_size,
    use_host_reachable,
    use_idle,
    use_idle_seconds,
    use_idle_state,
    use_input,
    use_interval,
    use_interval_when,
    use_is_first_render,
    use_is_landscape,
    use_is_portrait,
    use_is_screen_reader_enabled,
    use_is_tall_enough,
    use_is_wide_enough,
    use_keyboard_shortcut,
    use_keyboard_shortcuts,
    use_list,
    use_list_empty,
    use_local_storage,
    use_local_storage_with_dir,
    use_map,
    use_map_empty,
    use_map_from,
    use_measure,
    use_media_query,
    use_memo,
    use_mouse,
    use_network_status,
    use_online,
    use_paste,
    use_previous,
    use_reducer,
    use_reducer_lazy,
    use_scroll,
    use_set,
    use_set_empty,
    use_signal,
    use_stderr,
    use_stdin,
    use_stdout,
    use_throttle,
    use_timeout,
    use_toggle,
    use_toggle_off,
    use_toggle_on,
    use_transition,
    use_transition_with_easing,
    use_window_height,
    use_window_size,
    use_window_title,
    use_window_title_fn,
    use_window_width,
    with_hooks,
    write_clipboard,
};
