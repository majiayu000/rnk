//! Prelude module - commonly used imports

pub use crate::core::{
    Color,
    Element,
    ElementId,
    Style,
    FlexDirection,
    AlignItems,
    JustifyContent,
    Display,
    Position,
    Overflow,
    TextWrap,
    BorderStyle,
};

pub use crate::components::{
    Box,
    Text,
    Newline,
    Spacer,
    Transform,
    Static,
    static_output,
};

pub use crate::renderer::render;

pub use crate::hooks::{
    use_signal,
    use_effect,
    use_input,
    use_app,
    use_focus,
    use_focus_manager,
    use_stdin,
    use_stdout,
    use_stderr,
    use_measure,
    measure_element,
    Signal,
    Key,
    AppContext,
    FocusState,
    FocusManagerHandle,
    UseFocusOptions,
    StdinHandle,
    StdoutHandle,
    StderrHandle,
    Dimensions,
    MeasureRef,
    use_is_screen_reader_enabled,
};
