//! UI Components

mod barchart;
mod box_component;
mod gradient;
mod help;
mod hyperlink;
mod list;
mod message;
mod modal;
mod multi_select;
mod newline;
mod progress;
mod scrollable;
mod scrollbar;
mod select_input;
mod spacer;
mod sparkline;
mod spinner;
mod static_output;
mod table;
mod tabs;
pub mod text;
mod text_input;
mod timer;
mod transform;

pub use barchart::{Bar, BarChart, BarChartOrientation};
pub use box_component::Box;
pub use gradient::{Gradient, gradient, rainbow};
pub use help::{
    Help, HelpMode, HelpStyle, KeyBinding, editor_help, navigation_help, vim_navigation_help,
};
pub use hyperlink::{
    Hyperlink, HyperlinkBuilder, hyperlink, link, set_hyperlinks_supported, supports_hyperlinks,
};
pub use list::{List, ListItem, ListState};
pub use message::{Message, MessageRole, ThinkingBlock, ToolCall};
pub use modal::{Dialog, DialogState, Modal, ModalAlign};
pub use multi_select::{MultiSelect, MultiSelectItem, MultiSelectStyle};
pub use newline::Newline;
pub use progress::{Gauge, Progress, ProgressSymbols};
pub use scrollable::{ScrollableBox, fixed_bottom_layout, virtual_scroll_view};
pub use scrollbar::{Scrollbar, ScrollbarOrientation, ScrollbarSymbols};
pub use select_input::{SelectInput, SelectInputStyle, SelectItem, select_input};
pub use spacer::Spacer;
pub use sparkline::Sparkline;
pub use spinner::{Spinner, SpinnerBuilder};
pub use static_output::{Static, static_output};
pub use table::{Cell, Constraint, Row, Table, TableState};
pub use tabs::{Tab, Tabs};
pub use text::{Line, Span, Text};
pub use text_input::{TextInputHandle, TextInputOptions, TextInputState, use_text_input};
pub use timer::{
    StopwatchState, TimerState, format_duration_hhmmss, format_duration_mmss,
    format_duration_precise,
};
pub use transform::Transform;
