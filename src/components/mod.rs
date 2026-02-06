//! UI Components

mod accordion;
mod alert;
mod avatar;
mod badge;
mod barchart;
mod box_component;
mod breadcrumb;
mod calendar;
mod capsule;
mod card;
mod chip;
mod code_editor;
mod color_picker;
mod command_palette;
mod confirm;
mod context_menu;
mod cursor;
mod devtools;
mod divider;
mod empty_state;
mod file_picker;
mod gradient;
mod help;
mod highlight;
mod hyperlink;
mod key_hint;
mod line_chart;
mod link;
mod list;
mod markdown;
mod message;
mod modal;
mod multi_select;
pub mod navigation;
mod newline;
mod notification;
mod paginator;
mod popover;
mod progress;
mod quote;
mod rating;
mod scrollable;
mod scrollbar;
mod select_input;
mod selection_list;
mod skeleton;
mod spacer;
mod sparkline;
mod spinner;
mod stat;
mod static_output;
mod status;
mod status_bar;
mod stepper;
mod table;
mod tabs;
mod tag;
pub mod text;
mod text_input;
pub mod textarea;
mod theme;
mod timer;
mod tooltip;
mod transform;
mod tree;
pub mod viewport;

pub use accordion::{Accordion, AccordionItem};
pub use alert::{Alert, AlertLevel, alert_error, alert_info, alert_success, alert_warning};
pub use avatar::{Avatar, AvatarSize, avatar, avatar_initials};
pub use badge::{Badge, BadgeVariant, badge_error, badge_primary, badge_success, badge_warning};
pub use barchart::{Bar, BarChart, BarChartOrientation};
pub use box_component::Box;
pub use breadcrumb::{Breadcrumb, breadcrumb_from_path};
pub use calendar::Calendar;
pub use card::{Card, card, card_full};
pub use chip::{Chip, chip, chip_selected};
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
pub use empty_state::{EmptyState, empty_state, empty_state_with_icon};
pub use file_picker::{
    FileEntry, FileFilter, FilePicker, FilePickerState, FilePickerStyle, FileType, file_picker,
};
pub use gradient::{Gradient, gradient, rainbow};
pub use help::{
    Help, HelpMode, HelpStyle, KeyBinding, editor_help, navigation_help, vim_navigation_help,
};
pub use highlight::{
    Highlight, HighlightVariant, highlight, highlight_error, highlight_primary, highlight_success,
    highlight_warning,
};
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
pub use popover::{
    Popover, PopoverArrow, PopoverBorder, PopoverPosition, PopoverStyle, popover,
    popover_with_content,
};
pub use progress::{Gauge, Progress, ProgressSymbols};
pub use quote::{Quote, QuoteStyle, quote, quote_with_author};
pub use rating::{Rating, RatingStyle, RatingSymbols, rating, rating_of};
pub use scrollable::{ScrollableBox, fixed_bottom_layout, virtual_scroll_view};
pub use scrollbar::{Scrollbar, ScrollbarOrientation, ScrollbarSymbols};
pub use select_input::{SelectInput, SelectInputStyle, SelectItem, select_input};
pub use skeleton::{Skeleton, SkeletonVariant, skeleton_paragraph, skeleton_text};
pub use spacer::Spacer;
pub use sparkline::Sparkline;
pub use spinner::{Spinner, SpinnerBuilder};
pub use stat::{Stat, Trend, stat, stat_down, stat_up};
pub use static_output::{Static, static_output};
pub use status_bar::{StatusBar, status_bar, status_bar_full};
pub use stepper::{Step, StepStatus, Stepper, StepperOrientation, StepperStyle, stepper};
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
pub use tooltip::{Tooltip, TooltipPosition, tooltip, tooltip_left, tooltip_right};
pub use transform::Transform;
pub use tree::{Tree, TreeNode, TreeState, TreeStyle, handle_tree_input};
