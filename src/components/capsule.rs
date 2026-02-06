//! Shared capsule-style text helpers for small pill/label components.

use crate::components::Text;
use crate::core::Color;

/// Create padded capsule text with a single leading/trailing space.
pub(crate) fn capsule_padded(content: impl Into<String>, fg: Color, bg: Color) -> Text {
    let content = content.into();
    Text::new(format!(" {} ", content)).color(fg).background(bg)
}

/// Create wrapped capsule text (no extra padding).
pub(crate) fn capsule_wrapped(
    content: impl Into<String>,
    fg: Color,
    bg: Color,
    left: &'static str,
    right: &'static str,
) -> Text {
    let content = content.into();
    Text::new(format!("{}{}{}", left, content, right))
        .color(fg)
        .background(bg)
}

// No direct unit tests here because `Text` content is not exposed publicly.
