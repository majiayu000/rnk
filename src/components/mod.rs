//! UI Components

// Display components
#[path = "display/accordion.rs"]
mod accordion;
#[path = "display/avatar.rs"]
mod avatar;
#[path = "display/badge.rs"]
mod badge;
#[path = "display/barchart.rs"]
mod barchart;
#[path = "display/breadcrumb.rs"]
mod breadcrumb;
#[path = "display/calendar.rs"]
mod calendar;
#[path = "display/card.rs"]
mod card;
#[path = "display/chip.rs"]
mod chip;
#[path = "display/divider.rs"]
mod divider;
#[path = "display/empty_state.rs"]
mod empty_state;
#[path = "display/gradient.rs"]
mod gradient;
#[path = "display/highlight.rs"]
mod highlight;
#[path = "display/hyperlink.rs"]
mod hyperlink;
#[path = "display/key_hint.rs"]
mod key_hint;
#[path = "display/line_chart.rs"]
mod line_chart;
#[path = "display/link.rs"]
mod link;
#[path = "display/list.rs"]
mod list;
#[path = "display/markdown.rs"]
mod markdown;
#[path = "display/message.rs"]
mod message;
#[path = "display/newline.rs"]
mod newline;
#[path = "display/progress.rs"]
mod progress;
#[path = "display/quote.rs"]
mod quote;
#[path = "display/rating.rs"]
mod rating;
#[path = "display/skeleton.rs"]
mod skeleton;
#[path = "display/sparkline.rs"]
mod sparkline;
#[path = "display/stat.rs"]
mod stat;
#[path = "display/static_output.rs"]
mod static_output;
#[path = "display/status.rs"]
mod status;
#[path = "display/tag.rs"]
mod tag;
#[path = "display/text.rs"]
pub mod text;
#[path = "display/timer.rs"]
mod timer;

// Input components
#[path = "input/code_editor.rs"]
mod code_editor;
#[path = "input/color_picker.rs"]
mod color_picker;
#[path = "input/command_palette.rs"]
mod command_palette;
#[path = "input/confirm.rs"]
mod confirm;
#[path = "input/context_menu.rs"]
mod context_menu;
#[path = "input/file_picker.rs"]
mod file_picker;
#[path = "input/multi_select.rs"]
mod multi_select;
#[path = "input/paginator.rs"]
mod paginator;
#[path = "input/select_input.rs"]
mod select_input;
#[path = "input/selection_list.rs"]
mod selection_list;
#[path = "input/text_input.rs"]
mod text_input;

// Layout components
#[path = "layout/box_component.rs"]
mod box_component;
#[path = "layout/capsule.rs"]
mod capsule;
#[path = "layout/navigation.rs"]
pub mod navigation;
#[path = "layout/scrollable.rs"]
mod scrollable;
#[path = "layout/scrollbar.rs"]
mod scrollbar;
#[path = "layout/spacer.rs"]
mod spacer;
#[path = "layout/table.rs"]
mod table;
#[path = "layout/tabs.rs"]
mod tabs;
#[path = "layout/transform.rs"]
mod transform;
#[path = "layout/tree.rs"]
mod tree;

// Feedback components
#[path = "feedback/alert.rs"]
mod alert;
#[path = "feedback/cursor.rs"]
mod cursor;
#[path = "feedback/devtools.rs"]
mod devtools;
#[path = "feedback/help.rs"]
mod help;
#[path = "feedback/modal.rs"]
mod modal;
#[path = "feedback/notification.rs"]
mod notification;
#[path = "feedback/popover.rs"]
mod popover;
#[path = "feedback/spinner.rs"]
mod spinner;
#[path = "feedback/status_bar.rs"]
mod status_bar;
#[path = "feedback/stepper.rs"]
mod stepper;
#[path = "feedback/tooltip.rs"]
mod tooltip;

// Existing nested modules
pub mod textarea;
mod theme;
pub mod viewport;

pub use accordion::{Accordion, AccordionItem};
pub use alert::{Alert, AlertLevel};
pub use avatar::{Avatar, AvatarSize};
pub use badge::{Badge, BadgeVariant};
pub use barchart::{Bar, BarChart, BarChartOrientation};
pub use box_component::Box;
pub use breadcrumb::{Breadcrumb, breadcrumb_from_path};
pub use calendar::Calendar;
pub use card::Card;
pub use chip::Chip;
pub use code_editor::{CodeEditor, Language};
pub use color_picker::{
    ColorPalette, ColorPicker, ColorPickerState, ColorPickerStyle, color_picker,
    color_picker_with_palette,
};
pub use command_palette::{
    Command, CommandPalette, CommandPaletteState, CommandPaletteStyle, command_palette,
};
pub use confirm::{ButtonStyle, Confirm, ConfirmState, ConfirmStyle, handle_confirm_input};
pub use context_menu::{ContextMenu, ContextMenuState, ContextMenuStyle, MenuItem, context_menu};
pub use cursor::{Cursor, CursorShape, CursorState, CursorStyle, cursor};
pub use devtools::{DevTools, DevToolsTab};
pub use divider::{Divider, DividerOrientation, DividerStyle, hr, hr_dashed, hr_label};
pub use empty_state::EmptyState;
pub use file_picker::{
    FileEntry, FileFilter, FilePicker, FilePickerState, FilePickerStyle, FileType, file_picker,
};
pub use gradient::{Gradient, gradient, rainbow};
pub use help::{
    Help, HelpMode, HelpStyle, KeyBinding, editor_help, navigation_help, vim_navigation_help,
};
pub use highlight::{Highlight, HighlightVariant};
pub use hyperlink::{
    Hyperlink, HyperlinkBuilder, hyperlink, link, set_hyperlinks_supported, supports_hyperlinks,
};
pub use key_hint::{KeyHint, key_hint, key_hints};
pub use line_chart::{LineChart, Series};
pub use link::{Link, link_styled, link_with_icon};
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
pub use popover::{Popover, PopoverArrow, PopoverBorder, PopoverPosition, PopoverStyle};
pub use progress::{Gauge, Progress, ProgressSymbols};
pub use quote::{Quote, QuoteStyle};
pub use rating::{Rating, RatingStyle, RatingSymbols};
pub use scrollable::{ScrollableBox, fixed_bottom_layout, virtual_scroll_view};
pub use scrollbar::{Scrollbar, ScrollbarOrientation, ScrollbarSymbols};
pub use select_input::{SelectInput, SelectInputStyle, SelectItem, select_input};
pub use skeleton::{Skeleton, SkeletonVariant};
pub use spacer::Spacer;
pub use sparkline::Sparkline;
pub use spinner::{Spinner, SpinnerBuilder};
pub use stat::{Stat, Trend};
pub use static_output::{Static, static_output};
pub use status_bar::StatusBar;
pub use stepper::{Step, StepStatus, Stepper, StepperOrientation, StepperStyle};
pub use table::{Cell, Constraint, Row, Table, TableState};
pub use tabs::{Tab, Tabs};
pub use tag::Tag;
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
pub use tooltip::{Tooltip, TooltipPosition};
pub use transform::Transform;
pub use tree::{Tree, TreeNode, TreeState, TreeStyle, handle_tree_input};
