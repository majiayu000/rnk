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
//! - **Text Editing**: TextArea, Viewport
//! - **Feedback Components**: Progress, Spinner, Notification, Toast, Modal
//! - **Navigation Components**: Paginator, Scrollbar, Help
//! - **Hooks - State**: use_signal, use_memo, use_callback
//! - **Hooks - Effects**: use_effect, use_cmd
//! - **Hooks - Input**: use_input, use_mouse, use_paste, use_focus
//! - **Hooks - Utilities**: use_scroll, use_measure, use_app
//! - **Hooks - Animation**: use_animation, use_transition
//! - **Rendering**: render, AppBuilder, render_to_string
//!
//! This is the recommended stable import surface for application code. It is a
//! broad convenience prelude for complete apps.
//!
//! Use `rnk::prelude::lite::*` when a small example wants fewer names in scope,
//! `rnk::prelude::widgets::*` for examples centered on the recommended beginner
//! widgets, and `rnk::prelude::testing::*` for snapshot or interaction tests.
//! Lower-level modules such as `rnk::renderer`, `rnk::runtime`,
//! `rnk::components`, and `rnk::hooks` remain public for extension work, but
//! they are not the preferred starting point for new applications.

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

pub use crate::components::{Box, Box as LayoutBox, Spacer, Static, Transform};

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
    ColorPalette, ColorPicker, ColorPickerState, ColorPickerStyle, Command, CommandPalette,
    CommandPaletteState, CommandPaletteStyle, Confirm, ConfirmState, ConfirmStyle, FileEntry,
    FileFilter, FilePicker, FilePickerState, FilePickerStyle, FileType, InteractionMode,
    InteractionOutcome, MultiSelect, MultiSelectItem, MultiSelectState, MultiSelectStyle,
    SelectInput, SelectInputState, SelectInputStyle, SelectItem, SelectionState, TextInputHandle,
    TextInputOptions, TextInputState, handle_color_picker_input, handle_command_palette_input,
    handle_confirm_input, handle_confirm_input_with_mode, handle_file_picker_input,
    handle_multi_select_input, handle_select_input, handle_text_input, use_text_input,
};

pub use crate::components::{
    TextArea, TextAreaAction, TextAreaKeyMap, TextAreaPosition, TextAreaSelection, TextAreaState,
    TextAreaStyle, Viewport, ViewportAction, ViewportKeyMap, ViewportState, ViewportStyle,
    apply_textarea_action, apply_viewport_action, handle_textarea_input,
    handle_textarea_input_with_mode, handle_viewport_input, handle_viewport_input_with_mode,
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
    RenderOptions,
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
    render_to_string_raw,
    render_to_string_with_options,
    // Cross-thread APIs
    request_render,
};

// =============================================================================
// Hooks - State Management
// =============================================================================

pub use crate::hooks::{
    Context, Deps, DepsHash, MemoizedCallback, RefHandle, Signal, StateSetter, create_context,
    use_callback, use_context, use_memo, use_ref, use_signal, use_state, with_context,
};

// =============================================================================
// Hooks - Side Effects
// =============================================================================

pub use crate::hooks::{
    use_cmd, use_cmd_once, use_effect, use_effect_once, use_layout_effect, use_layout_effect_once,
};

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
    BracketedPasteGuard, Key, KeyCodeKind, MediaKeyKind, Mouse, MouseAction, MouseButton,
    PasteEvent, disable_bracketed_paste, dispatch_paste, enable_bracketed_paste,
    is_bracketed_paste_enabled, is_mouse_enabled, use_input, use_mouse, use_paste,
};

// =============================================================================
// Hooks - Focus Management
// =============================================================================

pub use crate::hooks::{
    FocusManagerHandle, FocusState, ScopedFocusOptions, UseFocusOptions, use_focus,
    use_focus_manager, use_focus_traversal, use_focus_traversal_in_scope, use_scoped_focus,
};
pub use crate::{AccessibilityProps, AccessibilityRole};

// =============================================================================
// Hooks - Scroll & Measure
// =============================================================================

pub use crate::hooks::{
    Dimensions, MeasureContext, MeasureRef, ScrollHandle, ScrollState, measure_element,
    measure_element_by_key, use_measure, use_scroll,
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

// =============================================================================
// Lite Prelude (Low-Conflict Imports)
// =============================================================================

/// A minimal import set for users who want less namespace pollution.
///
/// ```ignore
/// use rnk::prelude::lite::*;
/// ```
pub mod lite {
    pub use crate::components::{Box as LayoutBox, Spacer, Text};
    pub use crate::core::{
        AccessibilityProps, AccessibilityRole, Color, Element, FlexDirection, Style,
    };
    pub use crate::hooks::{Key, use_app, use_effect, use_input, use_signal};
    pub use crate::renderer::{render, render_to_string};
}

// =============================================================================
// Widgets Prelude (Focused Component Imports)
// =============================================================================

/// A focused import set for examples built around the recommended core widgets.
///
/// ```ignore
/// use rnk::prelude::widgets::*;
/// ```
pub mod widgets {
    pub use crate::components::{
        Box, Box as LayoutBox, Command, CommandPalette, CommandPaletteState, CommandPaletteStyle,
        InteractionMode, InteractionOutcome, NavigationConfig, SelectInput, SelectInputState,
        SelectInputStyle, SelectItem, Text, TextArea, TextAreaKeyMap, TextAreaState, TextAreaStyle,
        TextInputHandle, TextInputOptions, TextInputState, handle_command_palette_input,
        handle_select_input, handle_text_input, handle_textarea_input_with_mode, use_text_input,
    };
    pub use crate::core::{
        AccessibilityProps, AccessibilityRole, BorderStyle, Color, Element, FlexDirection, Style,
    };
    pub use crate::hooks::{Key, use_app, use_input, use_signal};
    pub use crate::renderer::{render, render_to_string};
}

// =============================================================================
// Testing Prelude
// =============================================================================

/// A convenience import set for test and snapshot helpers.
///
/// This mirrors `rnk::testing`, which remains an experimental test-support
/// surface before `1.0`.
///
/// ```ignore
/// use rnk::prelude::testing::*;
/// ```
pub mod testing {
    pub use crate::testing::*;
    pub use crate::{assert_snapshot, golden_test, inline_snapshot};
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_layout_box_alias_compiles() {
        use super::*;
        let _element = LayoutBox::new()
            .child(Text::new("ok").into_element())
            .into_element();
    }

    #[test]
    fn test_lite_prelude_compiles() {
        use super::lite::*;
        let _element = LayoutBox::new()
            .child(Text::new("lite").into_element())
            .into_element();
        let _style = Style::new().fg(Color::Green);
    }

    #[test]
    fn test_widgets_prelude_compiles() {
        use super::widgets::*;
        let mut state = TextInputState::default();
        let outcome = handle_text_input(
            &mut state,
            "x",
            &Key::default(),
            &TextInputOptions::default(),
        );
        assert_eq!(outcome, InteractionOutcome::Changed("x".to_string()));

        let _element = LayoutBox::new()
            .child(Text::new("widgets").into_element())
            .into_element();
    }

    #[test]
    fn test_testing_prelude_compiles() {
        use super::testing::*;
        let renderer = TestRenderer::new(20, 5);
        assert_eq!(renderer.width(), 20);
    }
}
