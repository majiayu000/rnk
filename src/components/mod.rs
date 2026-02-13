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

pub use display::text;
pub use display::*;
pub use feedback::*;
pub use input::*;
pub use layout::navigation;
pub use layout::*;
pub use theme::{
    BackgroundColors, BorderColors, ButtonColors, ComponentColors, InputColors, ListColors,
    ProgressColors, SemanticColor, TextColors, Theme, ThemeBuilder, get_theme, set_theme,
    with_theme,
};
