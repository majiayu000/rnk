//! Border styles for terminal boxes

/// Border style for boxes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderStyle {
    /// No border
    #[default]
    None,
    /// Single line border (─│┌┐└┘)
    Single,
    /// Double line border (═║╔╗╚╝)
    Double,
    /// Rounded corners (─│╭╮╰╯)
    Rounded,
    /// Bold/thick border (━┃┏┓┗┛)
    Bold,
    /// ASCII border (-|++++)
    Ascii,
}

impl BorderStyle {
    /// Get border characters: (top_left, top_right, bottom_left, bottom_right, horizontal, vertical)
    pub fn chars(&self) -> (&'static str, &'static str, &'static str, &'static str, &'static str, &'static str) {
        match self {
            BorderStyle::None => (" ", " ", " ", " ", " ", " "),
            BorderStyle::Single => ("┌", "┐", "└", "┘", "─", "│"),
            BorderStyle::Double => ("╔", "╗", "╚", "╝", "═", "║"),
            BorderStyle::Rounded => ("╭", "╮", "╰", "╯", "─", "│"),
            BorderStyle::Bold => ("┏", "┓", "┗", "┛", "━", "┃"),
            BorderStyle::Ascii => ("+", "+", "+", "+", "-", "|"),
        }
    }

    /// Check if border is visible
    pub fn is_visible(&self) -> bool {
        !matches!(self, BorderStyle::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_border_chars() {
        let (tl, tr, bl, br, h, v) = BorderStyle::Rounded.chars();
        assert_eq!(tl, "╭");
        assert_eq!(tr, "╮");
        assert_eq!(bl, "╰");
        assert_eq!(br, "╯");
        assert_eq!(h, "─");
        assert_eq!(v, "│");
    }

    #[test]
    fn test_is_visible() {
        assert!(!BorderStyle::None.is_visible());
        assert!(BorderStyle::Single.is_visible());
        assert!(BorderStyle::Rounded.is_visible());
    }
}
