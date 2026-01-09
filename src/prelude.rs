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
    Signal,
    Key,
    AppContext,
};
