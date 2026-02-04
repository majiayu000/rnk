//! Avatar component for user representation
//!
//! Displays user avatars with initials or icons.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::Avatar;
//!
//! fn app() -> Element {
//!     Box::new()
//!         .flex_direction(FlexDirection::Row)
//!         .gap(1.0)
//!         .children(vec![
//!             Avatar::new("John Doe").into_element(),
//!             Avatar::new("AB").size(AvatarSize::Large).into_element(),
//!             Avatar::initials("CD").color(Color::Cyan).into_element(),
//!         ])
//!         .into_element()
//! }
//! ```

use crate::components::Text;
use crate::core::{Color, Element};

/// Avatar size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AvatarSize {
    Small,
    #[default]
    Medium,
    Large,
}

/// An avatar component for user representation
#[derive(Debug, Clone)]
pub struct Avatar {
    initials: String,
    color: Color,
    background: Color,
    size: AvatarSize,
}

impl Avatar {
    /// Create an avatar from a name (extracts initials)
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let initials = Self::extract_initials(&name);
        Self {
            initials,
            color: Color::White,
            background: Color::Blue,
            size: AvatarSize::Medium,
        }
    }

    /// Create an avatar with explicit initials
    pub fn initials(initials: impl Into<String>) -> Self {
        Self {
            initials: initials.into(),
            color: Color::White,
            background: Color::Blue,
            size: AvatarSize::Medium,
        }
    }

    /// Set the text color
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the background color
    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    /// Set the avatar size
    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    /// Extract initials from a name
    fn extract_initials(name: &str) -> String {
        name.split_whitespace()
            .filter_map(|word| word.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase()
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let (left, right) = match self.size {
            AvatarSize::Small => ("(", ")"),
            AvatarSize::Medium => ("[", "]"),
            AvatarSize::Large => ("【", "】"),
        };

        let content = format!("{}{}{}", left, self.initials, right);

        Text::new(content)
            .color(self.color)
            .background(self.background)
            .bold()
            .into_element()
    }
}

impl Default for Avatar {
    fn default() -> Self {
        Self::initials("?")
    }
}

/// Create an avatar from a name
pub fn avatar(name: impl Into<String>) -> Element {
    Avatar::new(name).into_element()
}

/// Create an avatar with custom initials
pub fn avatar_initials(initials: impl Into<String>) -> Element {
    Avatar::initials(initials).into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avatar_from_name() {
        let av = Avatar::new("John Doe");
        assert_eq!(av.initials, "JD");
    }

    #[test]
    fn test_avatar_single_name() {
        let av = Avatar::new("Alice");
        assert_eq!(av.initials, "A");
    }

    #[test]
    fn test_avatar_initials() {
        let av = Avatar::initials("XY");
        assert_eq!(av.initials, "XY");
    }

    #[test]
    fn test_avatar_into_element() {
        let _ = Avatar::new("Test User").into_element();
        let _ = Avatar::initials("TU").size(AvatarSize::Large).into_element();
    }

    #[test]
    fn test_avatar_helpers() {
        let _ = avatar("John");
        let _ = avatar_initials("AB");
    }
}
