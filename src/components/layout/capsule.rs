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

#[derive(Debug)]
enum CapsuleElementShape {
    Padded,
    Wrapped {
        left: &'static str,
        right: &'static str,
    },
}

/// Shared element-level builder used by capsule-like components.
#[derive(Debug)]
pub(crate) struct CapsuleElementBuilder {
    text: String,
    fg: Color,
    bg: Color,
    shape: CapsuleElementShape,
    prefix: Option<String>,
    icon: Option<String>,
    suffix: Option<String>,
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

impl CapsuleElementBuilder {
    pub(crate) fn new(text: impl Into<String>, fg: Color, bg: Color) -> Self {
        Self {
            text: text.into(),
            fg,
            bg,
            shape: CapsuleElementShape::Padded,
            prefix: None,
            icon: None,
            suffix: None,
        }
    }

    pub(crate) fn wrapped(mut self, left: &'static str, right: &'static str) -> Self {
        self.shape = CapsuleElementShape::Wrapped { left, right };
        self
    }

    pub(crate) fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    pub(crate) fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub(crate) fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    pub(crate) fn into_element(self) -> Element {
        let mut content = CapsuleContent::new();

        if let Some(prefix) = self.prefix {
            content = content.push(prefix);
        }
        if let Some(icon) = self.icon {
            content = content.push(icon);
        }
        content = content.push(self.text);
        if let Some(suffix) = self.suffix {
            content = content.push(suffix);
        }

        match self.shape {
            CapsuleElementShape::Padded => {
                CapsuleLabel::padded(content.build(), self.fg, self.bg).into_element()
            }
            CapsuleElementShape::Wrapped { left, right } => {
                CapsuleLabel::wrapped(content.build(), self.fg, self.bg, left, right).into_element()
            }
        }
    }
}

// No direct unit tests here because `Text` content is not exposed publicly.
