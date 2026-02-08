//! Shared capsule-style text helpers for small pill/label components.

use crate::components::Text;
use crate::core::{Color, Element};

#[derive(Debug, Clone, Copy)]
enum CapsuleShape {
    Padded,
    Wrapped {
        left: &'static str,
        right: &'static str,
    },
}

/// Shared builder used by small capsule-like components (badge/tag/chip/highlight).
#[derive(Debug, Clone)]
pub(crate) struct CapsuleLabel {
    content: String,
    fg: Color,
    bg: Color,
    shape: CapsuleShape,
}

#[derive(Debug, Default)]
pub(crate) struct CapsuleContent {
    parts: Vec<String>,
}

impl CapsuleLabel {
    pub(crate) fn padded(content: impl Into<String>, fg: Color, bg: Color) -> Self {
        Self {
            content: content.into(),
            fg,
            bg,
            shape: CapsuleShape::Padded,
        }
    }

    pub(crate) fn wrapped(
        content: impl Into<String>,
        fg: Color,
        bg: Color,
        left: &'static str,
        right: &'static str,
    ) -> Self {
        Self {
            content: content.into(),
            fg,
            bg,
            shape: CapsuleShape::Wrapped { left, right },
        }
    }

    pub(crate) fn into_text(self) -> Text {
        match self.shape {
            CapsuleShape::Padded => Text::new(format!(" {} ", self.content))
                .color(self.fg)
                .background(self.bg),
            CapsuleShape::Wrapped { left, right } => {
                Text::new(format!("{}{}{}", left, self.content, right))
                    .color(self.fg)
                    .background(self.bg)
            }
        }
    }

    pub(crate) fn into_element(self) -> Element {
        self.into_text().into_element()
    }
}

impl CapsuleContent {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push(mut self, part: impl Into<String>) -> Self {
        let part = part.into();
        if !part.is_empty() {
            self.parts.push(part);
        }
        self
    }

    pub(crate) fn build(self) -> String {
        self.parts.join(" ")
    }
}

// No direct unit tests here because `Text` content is not exposed publicly.
