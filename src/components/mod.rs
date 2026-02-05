//! UI Components

mod accordion;
mod avatar;
mod badge;
mod barchart;
mod box_component;
mod breadcrumb;
mod calendar;
mod chip;
mod code_editor;
mod confirm;
mod cursor;
mod devtools;
mod divider;
mod empty_state;
mod file_picker;
mod gradient;
mod help;
mod hyperlink;
mod key_hint;
mod line_chart;
mod list;
mod markdown;
mod message;
mod modal;
mod multi_select;
pub mod navigation;
mod newline;
mod notification;
mod paginator;
mod progress;
mod quote;
mod scrollable;
mod scrollbar;
mod select_input;
mod spacer;
mod sparkline;
mod spinner;
mod static_output;
mod status_bar;
mod table;
mod tabs;
mod tag;
pub mod text;
mod text_input;
pub mod textarea;
mod theme;
mod timer;
mod transform;
mod tree;
pub mod viewport;

pub use accordion::{Accordion, AccordionItem};
pub use avatar::{Avatar, AvatarSize, avatar, avatar_initials};
pub use badge::{Badge, BadgeVariant, badge_error, badge_primary, badge_success, badge_warning};
pub use barchart::{Bar, BarChart, BarChartOrientation};
pub use box_component::Box;
pub use breadcrumb::{Breadcrumb, breadcrumb_from_path};
pub use calendar::Calendar;
pub use chip::{Chip, chip, chip_selected};
pub use code_editor::{CodeEditor, Language};
pub use confirm::{ButtonStyle, Confirm, ConfirmState, ConfirmStyle, handle_confirm_input};
pub use cursor::{Cursor, CursorShape, CursorState, CursorStyle, cursor};
pub use devtools::{DevTools, DevToolsTab};
pub use divider::{Divider, DividerOrientation, DividerStyle, hr, hr_dashed, hr_label};
pub use empty_state::{EmptyState, empty_state, empty_state_with_icon};
pub use file_picker::{
    FileEntry, FileFilter, FilePicker, FilePickerState, FilePickerStyle, FileType, file_picker,
};
pub use gradient::{Gradient, gradient, rainbow};
pub use help::{
    Help, HelpMode, HelpStyle, KeyBinding, editor_help, navigation_help, vim_navigation_help,
};
pub use hyperlink::{
    Hyperlink, HyperlinkBuilder, hyperlink, link, set_hyperlinks_supported, supports_hyperlinks,
};
pub use key_hint::{KeyHint, key_hint, key_hints};
pub use line_chart::{LineChart, Series};
pub use list::{List, ListItem, ListState};
pub use markdown::Markdown;
pub use message::{Message, MessageRole, ThinkingBlock, ToolCall};
pub use modal::{Dialog, DialogState, Modal, ModalAlign};
pub use multi_select::{MultiSelect, MultiSelectItem, MultiSelectStyle};
pub use navigation::{
    NavigationConfig, NavigationResult, SelectionState, calculate_visible_range,
    handle_list_navigation,
};
pub use newline::Newline;
pub use notification::{
    Notification, NotificationBorder, NotificationItem, NotificationLevel, NotificationPosition,
    NotificationState, NotificationStyle, Toast, notification, toast,
};
pub use paginator::{
    Paginator, PaginatorState, PaginatorStyle, PaginatorType, handle_paginator_input,
};
pub use progress::{Gauge, Progress, ProgressSymbols};
pub use quote::{Quote, QuoteStyle, quote, quote_with_author};
pub use scrollable::{ScrollableBox, fixed_bottom_layout, virtual_scroll_view};
pub use scrollbar::{Scrollbar, ScrollbarOrientation, ScrollbarSymbols};
pub use select_input::{SelectInput, SelectInputStyle, SelectItem, select_input};
pub use spacer::Spacer;
pub use sparkline::Sparkline;
pub use spinner::{Spinner, SpinnerBuilder};
pub use static_output::{Static, static_output};
pub use status_bar::{StatusBar, status_bar, status_bar_full};
pub use table::{Cell, Constraint, Row, Table, TableState};
pub use tabs::{Tab, Tabs};
pub use tag::{Tag, tag, tag_colored};
pub use text::{Line, Span, Text};
pub use text_input::{TextInputHandle, TextInputOptions, TextInputState, use_text_input};
pub use theme::{
    BackgroundColors, BorderColors, ButtonColors, ComponentColors, InputColors, ListColors,
    ProgressColors, SemanticColor, TextColors, Theme, ThemeBuilder, get_theme, set_theme,
    with_theme,
};
pub use timer::{
    StopwatchState, TimerState, format_duration_hhmmss, format_duration_mmss,
    format_duration_precise,
};
pub use transform::Transform;
pub use tree::{Tree, TreeNode, TreeState, TreeStyle, handle_tree_input};
