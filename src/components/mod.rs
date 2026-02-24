//! UI Components

mod display;
mod feedback;
mod input;
pub mod keymap;
mod layout;

// Existing nested modules
pub mod textarea;
mod theme;
pub mod viewport;

pub(crate) use display::{capsule_variant, status};
pub(crate) use input::selection_list;
pub(crate) use layout::capsule;

// display
pub use display::text;
pub use display::{
    Accordion, AccordionItem, Avatar, AvatarSize, Badge, BadgeVariant, Bar, BarChart,
    BarChartOrientation, Breadcrumb, Calendar, CapsuleVariant, Card, Chip, Divider,
    DividerOrientation, DividerStyle, EmptyState, Gauge, Gradient, Highlight, HighlightVariant,
    Hyperlink, HyperlinkBuilder, KeyHint, Line, LineChart, Link, List, ListItem, ListState,
    Markdown, Message, MessageRole, Newline, Progress, ProgressSymbols, Quote, QuoteStyle, Rating,
    RatingStyle, RatingSymbols, Series, Skeleton, SkeletonVariant, Span, Sparkline, Stat, Static,
    StopwatchState, Tag, Text, ThinkingBlock, TimerState, ToolCall, Trend, breadcrumb_from_path,
    format_duration_hhmmss, format_duration_mmss, format_duration_precise,
    set_hyperlinks_supported, supports_hyperlinks,
};
// feedback
pub use feedback::{
    Alert, AlertLevel, Cursor, CursorShape, CursorState, CursorStyle, DevTools, DevToolsTab,
    Dialog, DialogState, Help, HelpMode, HelpStyle, KeyBinding, Modal, ModalAlign, Notification,
    NotificationBorder, NotificationItem, NotificationLevel, NotificationPosition,
    NotificationState, NotificationStyle, Popover, PopoverArrow, PopoverBorder, PopoverPosition,
    PopoverStyle, Spinner, SpinnerBuilder, StatusBar, Step, StepStatus, Stepper,
    StepperOrientation, StepperStyle, Toast, Tooltip, TooltipPosition, editor_help,
    navigation_help, vim_navigation_help,
};
// input
pub use input::{
    ButtonStyle, CodeEditor, ColorPalette, ColorPicker, ColorPickerState, ColorPickerStyle,
    Command, CommandPalette, CommandPaletteState, CommandPaletteStyle, Confirm, ConfirmState,
    ConfirmStyle, ContextMenu, ContextMenuState, ContextMenuStyle, FileEntry, FileFilter,
    FilePicker, FilePickerState, FilePickerStyle, FileType, Language, MenuItem, MultiSelect,
    MultiSelectItem, MultiSelectStyle, Paginator, PaginatorState, PaginatorStyle, PaginatorType,
    SelectInput, SelectInputStyle, SelectItem, TextInputHandle, TextInputOptions, TextInputState,
    handle_confirm_input, handle_paginator_input, use_text_input,
};
// layout
pub use layout::navigation;
pub use layout::{
    Box, Cell, Constraint, NavigationConfig, NavigationResult, Row, ScrollableBox, Scrollbar,
    ScrollbarOrientation, ScrollbarSymbols, SelectionState, Spacer, Tab, Table, TableState, Tabs,
    Transform, Tree, TreeNode, TreeState, TreeStyle, calculate_visible_range, fixed_bottom_layout,
    handle_list_navigation, handle_tree_input, virtual_scroll_view,
};
pub use theme::{
    BackgroundColors, BorderColors, ButtonColors, ComponentColors, InputColors, ListColors,
    ProgressColors, SemanticColor, TextColors, Theme, ThemeBuilder, get_theme, set_theme,
    with_theme,
};

// Implement From<T> for Element for all components with into_element()
// Excluded: Bar, Gradient, Hyperlink, Line, Spinner (no into_element)
// Excluded: Static, MultiSelect, SelectInput, Tree (generic params)
// Excluded: Cursor, Notification, Toast, Confirm, FilePicker (lifetime params)
crate::impl_into_element!(
    // Display
    Accordion,
    Avatar,
    Badge,
    BarChart,
    Breadcrumb,
    Calendar,
    Card,
    Chip,
    Divider,
    EmptyState,
    Gauge,
    Highlight,
    KeyHint,
    LineChart,
    Link,
    List,
    Markdown,
    Message,
    Newline,
    Progress,
    Quote,
    Rating,
    Skeleton,
    Sparkline,
    Stat,
    Tag,
    Text,
    ThinkingBlock,
    ToolCall,
    // Feedback
    Alert,
    DevTools,
    Dialog,
    Help,
    Modal,
    Popover,
    StatusBar,
    Stepper,
    Tooltip,
    // Input
    CodeEditor,
    ColorPicker,
    CommandPalette,
    ContextMenu,
    Paginator,
    // Layout
    Box,
    ScrollableBox,
    Scrollbar,
    Spacer,
    Table,
    Tabs,
    Transform,
);
