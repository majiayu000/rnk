//! rnk-icons - Terminal icons library
//!
//! A collection of Nerd Font icons for terminal UI applications.
//!
//! # Example
//!
//! ```rust
//! use rnk_icons::{Icon, icons};
//!
//! // Get a file icon
//! let rust_icon = icons::file::rust();
//! println!("{} main.rs", rust_icon);
//!
//! // Get a UI icon
//! let folder = icons::ui::folder();
//! println!("{} src/", folder);
//!
//! // With color (requires rnk or rnk-style)
//! let icon = Icon::new(icons::file::rust()).colored("#DEA584");
//! ```

mod icon;
pub mod icons;

pub use icon::Icon;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::icon::Icon;
    pub use crate::icons;
}
