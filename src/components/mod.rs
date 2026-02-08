//! UI Components

mod display;
mod feedback;
mod input;
mod layout;

// Existing nested modules
pub mod textarea;
mod theme;
pub mod viewport;

pub(crate) use layout::capsule;
pub(crate) use display::{capsule_variant, status};
pub(crate) use input::selection_list;

pub use display::*;
pub use display::text;
pub use feedback::*;
pub use input::*;
pub use layout::*;
pub use layout::navigation;
pub use theme::{
    BackgroundColors, BorderColors, ButtonColors, ComponentColors, InputColors, ListColors,
    ProgressColors, SemanticColor, TextColors, Theme, ThemeBuilder, get_theme, set_theme,
    with_theme,
};
