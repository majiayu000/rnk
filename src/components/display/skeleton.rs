//! Skeleton component for loading placeholders
//!
//! Displays animated placeholder content while data is loading.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::Skeleton;
//!
//! fn app() -> Element {
//!     let loading = use_signal(|| true);
//!
//!     if loading.get() {
//!         Skeleton::text(20).into_element()
//!     } else {
//!         Text::new("Loaded content").into_element()
//!     }
//! }
//! ```

use crate::components::{Box as RnkBox, Text};
use crate::core::{Color, Element, FlexDirection};

/// Skeleton variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkeletonVariant {
    #[default]
    Text,
    Circle,
    Rectangle,
}

/// A skeleton loading placeholder
#[derive(Debug, Clone)]
pub struct Skeleton {
    variant: SkeletonVariant,
    width: usize,
    height: usize,
    animated: bool,
}

impl Skeleton {
    /// Create a new skeleton
    pub fn new() -> Self {
        Self {
            variant: SkeletonVariant::Text,
            width: 20,
            height: 1,
            animated: true,
        }
    }

    /// Create a text skeleton
    pub fn text(width: usize) -> Self {
        Self {
            variant: SkeletonVariant::Text,
            width,
            height: 1,
            animated: true,
        }
    }

    /// Create a circle skeleton
    pub fn circle(size: usize) -> Self {
        Self {
            variant: SkeletonVariant::Circle,
            width: size,
            height: size,
            animated: true,
        }
    }

    /// Create a rectangle skeleton
    pub fn rectangle(width: usize, height: usize) -> Self {
        Self {
            variant: SkeletonVariant::Rectangle,
            width,
            height,
            animated: true,
        }
    }

    /// Set the width
    pub fn width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    /// Set the height
    pub fn height(mut self, height: usize) -> Self {
        self.height = height;
        self
    }

    /// Disable animation
    pub fn static_display(mut self) -> Self {
        self.animated = false;
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let char = if self.animated { '░' } else { '▒' };
        let bg = Color::Ansi256(238);

        match self.variant {
            SkeletonVariant::Text => {
                let content: String = std::iter::repeat(char).take(self.width).collect();
                Text::new(content)
                    .color(Color::Ansi256(242))
                    .background(bg)
                    .into_element()
            }
            SkeletonVariant::Circle => {
                // Simple circle representation
                let content = format!(
                    "({})",
                    std::iter::repeat(char)
                        .take(self.width.saturating_sub(2))
                        .collect::<String>()
                );
                Text::new(content)
                    .color(Color::Ansi256(242))
                    .background(bg)
                    .into_element()
            }
            SkeletonVariant::Rectangle => {
                let mut children = Vec::new();
                let line: String = std::iter::repeat(char).take(self.width).collect();
                for _ in 0..self.height {
                    children.push(
                        Text::new(&line)
                            .color(Color::Ansi256(242))
                            .background(bg)
                            .into_element(),
                    );
                }
                RnkBox::new()
                    .flex_direction(FlexDirection::Column)
                    .children(children)
                    .into_element()
            }
        }
    }
}

impl Default for Skeleton {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a text skeleton
pub fn skeleton_text(width: usize) -> Element {
    Skeleton::text(width).into_element()
}

/// Create a paragraph skeleton (multiple lines)
pub fn skeleton_paragraph(lines: usize, width: usize) -> Element {
    Skeleton::rectangle(width, lines).into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skeleton_text() {
        let s = Skeleton::text(10);
        assert_eq!(s.width, 10);
        assert_eq!(s.variant, SkeletonVariant::Text);
    }

    #[test]
    fn test_skeleton_circle() {
        let s = Skeleton::circle(5);
        assert_eq!(s.variant, SkeletonVariant::Circle);
    }

    #[test]
    fn test_skeleton_rectangle() {
        let s = Skeleton::rectangle(20, 3);
        assert_eq!(s.width, 20);
        assert_eq!(s.height, 3);
    }

    #[test]
    fn test_skeleton_into_element() {
        let _ = Skeleton::text(10).into_element();
        let _ = Skeleton::circle(5).into_element();
        let _ = Skeleton::rectangle(10, 3).into_element();
    }

    #[test]
    fn test_skeleton_helpers() {
        let _ = skeleton_text(15);
        let _ = skeleton_paragraph(3, 20);
    }
}
