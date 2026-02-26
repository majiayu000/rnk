//! Theme System for consistent styling across components
//!
//! Provides a centralized theming system with predefined themes and customization.

use crate::core::Color;

/// A complete theme definition
#[derive(Debug, Clone)]
pub struct Theme {
    /// Theme name
    pub name: String,
    /// Primary color (main accent)
    pub primary: Color,
    /// Secondary color (secondary accent)
    pub secondary: Color,
    /// Success color
    pub success: Color,
    /// Warning color
    pub warning: Color,
    /// Error/danger color
    pub error: Color,
    /// Info color
    pub info: Color,
    /// Text colors
    pub text: TextColors,
    /// Background colors
    pub background: BackgroundColors,
    /// Border colors
    pub border: BorderColors,
    /// Component-specific colors
    pub components: ComponentColors,
}

/// Text color variants
#[derive(Debug, Clone)]
pub struct TextColors {
    /// Primary text color
    pub primary: Color,
    /// Secondary/muted text color
    pub secondary: Color,
    /// Disabled text color
    pub disabled: Color,
    /// Inverted text (for dark backgrounds)
    pub inverted: Color,
    /// Link text color
    pub link: Color,
}

/// Background color variants
#[derive(Debug, Clone)]
pub struct BackgroundColors {
    /// Default background
    pub default: Color,
    /// Elevated/card background
    pub elevated: Color,
    /// Selected item background
    pub selected: Color,
    /// Hover background
    pub hover: Color,
    /// Disabled background
    pub disabled: Color,
}

/// Border color variants
#[derive(Debug, Clone)]
pub struct BorderColors {
    /// Default border
    pub default: Color,
    /// Focused border
    pub focused: Color,
    /// Error border
    pub error: Color,
    /// Disabled border
    pub disabled: Color,
}

/// Component-specific colors
#[derive(Debug, Clone)]
pub struct ComponentColors {
    /// Input field colors
    pub input: InputColors,
    /// Button colors
    pub button: ButtonColors,
    /// List/menu colors
    pub list: ListColors,
    /// Progress bar colors
    pub progress: ProgressColors,
}

/// Input field colors
#[derive(Debug, Clone)]
pub struct InputColors {
    /// Input background
    pub background: Color,
    /// Input text
    pub text: Color,
    /// Placeholder text
    pub placeholder: Color,
    /// Cursor color
    pub cursor: Color,
    /// Selection background
    pub selection: Color,
}

/// Button colors
#[derive(Debug, Clone)]
pub struct ButtonColors {
    /// Primary button background
    pub primary_bg: Color,
    /// Primary button text
    pub primary_text: Color,
    /// Secondary button background
    pub secondary_bg: Color,
    /// Secondary button text
    pub secondary_text: Color,
    /// Danger button background
    pub danger_bg: Color,
    /// Danger button text
    pub danger_text: Color,
}

/// List/menu colors
#[derive(Debug, Clone)]
pub struct ListColors {
    /// Item background
    pub item_bg: Color,
    /// Item text
    pub item_text: Color,
    /// Selected item background
    pub selected_bg: Color,
    /// Selected item text
    pub selected_text: Color,
    /// Focused item background
    pub focused_bg: Color,
    /// Focused item text
    pub focused_text: Color,
}

/// Progress bar colors
#[derive(Debug, Clone)]
pub struct ProgressColors {
    /// Track/background color
    pub track: Color,
    /// Fill/progress color
    pub fill: Color,
    /// Completed color
    pub completed: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// Create a theme builder with the given name
    pub fn builder(name: impl Into<String>) -> ThemeBuilder {
        ThemeBuilder::new(name)
    }

    /// Dark theme (default)
    pub fn dark() -> Self {
        Self {
            name: "dark".to_string(),
            primary: Color::Cyan,
            secondary: Color::Magenta,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Blue,
            text: TextColors {
                primary: Color::White,
                secondary: Color::BrightBlack,
                disabled: Color::BrightBlack,
                inverted: Color::Black,
                link: Color::Cyan,
            },
            background: BackgroundColors {
                default: Color::Black,
                elevated: Color::BrightBlack,
                selected: Color::Blue,
                hover: Color::BrightBlack,
                disabled: Color::BrightBlack,
            },
            border: BorderColors {
                default: Color::BrightBlack,
                focused: Color::Cyan,
                error: Color::Red,
                disabled: Color::BrightBlack,
            },
            components: ComponentColors {
                input: InputColors {
                    background: Color::Black,
                    text: Color::White,
                    placeholder: Color::BrightBlack,
                    cursor: Color::Cyan,
                    selection: Color::Blue,
                },
                button: ButtonColors {
                    primary_bg: Color::Cyan,
                    primary_text: Color::Black,
                    secondary_bg: Color::BrightBlack,
                    secondary_text: Color::White,
                    danger_bg: Color::Red,
                    danger_text: Color::White,
                },
                list: ListColors {
                    item_bg: Color::Black,
                    item_text: Color::White,
                    selected_bg: Color::Blue,
                    selected_text: Color::White,
                    focused_bg: Color::Cyan,
                    focused_text: Color::Black,
                },
                progress: ProgressColors {
                    track: Color::BrightBlack,
                    fill: Color::Cyan,
                    completed: Color::Green,
                },
            },
        }
    }

    /// Light theme
    pub fn light() -> Self {
        Self {
            name: "light".to_string(),
            primary: Color::Blue,
            secondary: Color::Magenta,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Cyan,
            text: TextColors {
                primary: Color::Black,
                secondary: Color::BrightBlack,
                disabled: Color::BrightBlack,
                inverted: Color::White,
                link: Color::Blue,
            },
            background: BackgroundColors {
                default: Color::White,
                elevated: Color::BrightWhite,
                selected: Color::Cyan,
                hover: Color::BrightWhite,
                disabled: Color::BrightWhite,
            },
            border: BorderColors {
                default: Color::BrightBlack,
                focused: Color::Blue,
                error: Color::Red,
                disabled: Color::BrightWhite,
            },
            components: ComponentColors {
                input: InputColors {
                    background: Color::White,
                    text: Color::Black,
                    placeholder: Color::BrightBlack,
                    cursor: Color::Blue,
                    selection: Color::Cyan,
                },
                button: ButtonColors {
                    primary_bg: Color::Blue,
                    primary_text: Color::White,
                    secondary_bg: Color::BrightWhite,
                    secondary_text: Color::Black,
                    danger_bg: Color::Red,
                    danger_text: Color::White,
                },
                list: ListColors {
                    item_bg: Color::White,
                    item_text: Color::Black,
                    selected_bg: Color::Cyan,
                    selected_text: Color::Black,
                    focused_bg: Color::Blue,
                    focused_text: Color::White,
                },
                progress: ProgressColors {
                    track: Color::BrightWhite,
                    fill: Color::Blue,
                    completed: Color::Green,
                },
            },
        }
    }

    /// Monokai theme
    pub fn monokai() -> Self {
        Self {
            name: "monokai".to_string(),
            primary: Color::Rgb(166, 226, 46),    // Green
            secondary: Color::Rgb(174, 129, 255), // Purple
            success: Color::Rgb(166, 226, 46),
            warning: Color::Rgb(230, 219, 116), // Yellow
            error: Color::Rgb(249, 38, 114),    // Pink/Red
            info: Color::Rgb(102, 217, 239),    // Cyan
            text: TextColors {
                primary: Color::Rgb(248, 248, 242),
                secondary: Color::Rgb(117, 113, 94),
                disabled: Color::Rgb(117, 113, 94),
                inverted: Color::Rgb(39, 40, 34),
                link: Color::Rgb(102, 217, 239),
            },
            background: BackgroundColors {
                default: Color::Rgb(39, 40, 34),
                elevated: Color::Rgb(49, 50, 44),
                selected: Color::Rgb(73, 72, 62),
                hover: Color::Rgb(59, 60, 54),
                disabled: Color::Rgb(49, 50, 44),
            },
            border: BorderColors {
                default: Color::Rgb(117, 113, 94),
                focused: Color::Rgb(166, 226, 46),
                error: Color::Rgb(249, 38, 114),
                disabled: Color::Rgb(73, 72, 62),
            },
            components: ComponentColors {
                input: InputColors {
                    background: Color::Rgb(39, 40, 34),
                    text: Color::Rgb(248, 248, 242),
                    placeholder: Color::Rgb(117, 113, 94),
                    cursor: Color::Rgb(248, 248, 242),
                    selection: Color::Rgb(73, 72, 62),
                },
                button: ButtonColors {
                    primary_bg: Color::Rgb(166, 226, 46),
                    primary_text: Color::Rgb(39, 40, 34),
                    secondary_bg: Color::Rgb(73, 72, 62),
                    secondary_text: Color::Rgb(248, 248, 242),
                    danger_bg: Color::Rgb(249, 38, 114),
                    danger_text: Color::Rgb(248, 248, 242),
                },
                list: ListColors {
                    item_bg: Color::Rgb(39, 40, 34),
                    item_text: Color::Rgb(248, 248, 242),
                    selected_bg: Color::Rgb(73, 72, 62),
                    selected_text: Color::Rgb(248, 248, 242),
                    focused_bg: Color::Rgb(166, 226, 46),
                    focused_text: Color::Rgb(39, 40, 34),
                },
                progress: ProgressColors {
                    track: Color::Rgb(73, 72, 62),
                    fill: Color::Rgb(166, 226, 46),
                    completed: Color::Rgb(166, 226, 46),
                },
            },
        }
    }

    /// Dracula theme
    pub fn dracula() -> Self {
        Self {
            name: "dracula".to_string(),
            primary: Color::Rgb(189, 147, 249),   // Purple
            secondary: Color::Rgb(255, 121, 198), // Pink
            success: Color::Rgb(80, 250, 123),    // Green
            warning: Color::Rgb(255, 184, 108),   // Orange
            error: Color::Rgb(255, 85, 85),       // Red
            info: Color::Rgb(139, 233, 253),      // Cyan
            text: TextColors {
                primary: Color::Rgb(248, 248, 242),
                secondary: Color::Rgb(98, 114, 164),
                disabled: Color::Rgb(68, 71, 90),
                inverted: Color::Rgb(40, 42, 54),
                link: Color::Rgb(139, 233, 253),
            },
            background: BackgroundColors {
                default: Color::Rgb(40, 42, 54),
                elevated: Color::Rgb(68, 71, 90),
                selected: Color::Rgb(68, 71, 90),
                hover: Color::Rgb(68, 71, 90),
                disabled: Color::Rgb(68, 71, 90),
            },
            border: BorderColors {
                default: Color::Rgb(68, 71, 90),
                focused: Color::Rgb(189, 147, 249),
                error: Color::Rgb(255, 85, 85),
                disabled: Color::Rgb(68, 71, 90),
            },
            components: ComponentColors {
                input: InputColors {
                    background: Color::Rgb(40, 42, 54),
                    text: Color::Rgb(248, 248, 242),
                    placeholder: Color::Rgb(98, 114, 164),
                    cursor: Color::Rgb(248, 248, 242),
                    selection: Color::Rgb(68, 71, 90),
                },
                button: ButtonColors {
                    primary_bg: Color::Rgb(189, 147, 249),
                    primary_text: Color::Rgb(40, 42, 54),
                    secondary_bg: Color::Rgb(68, 71, 90),
                    secondary_text: Color::Rgb(248, 248, 242),
                    danger_bg: Color::Rgb(255, 85, 85),
                    danger_text: Color::Rgb(248, 248, 242),
                },
                list: ListColors {
                    item_bg: Color::Rgb(40, 42, 54),
                    item_text: Color::Rgb(248, 248, 242),
                    selected_bg: Color::Rgb(68, 71, 90),
                    selected_text: Color::Rgb(248, 248, 242),
                    focused_bg: Color::Rgb(189, 147, 249),
                    focused_text: Color::Rgb(40, 42, 54),
                },
                progress: ProgressColors {
                    track: Color::Rgb(68, 71, 90),
                    fill: Color::Rgb(189, 147, 249),
                    completed: Color::Rgb(80, 250, 123),
                },
            },
        }
    }

    /// Nord theme
    pub fn nord() -> Self {
        Self {
            name: "nord".to_string(),
            primary: Color::Rgb(136, 192, 208),   // Frost
            secondary: Color::Rgb(180, 142, 173), // Aurora purple
            success: Color::Rgb(163, 190, 140),   // Aurora green
            warning: Color::Rgb(235, 203, 139),   // Aurora yellow
            error: Color::Rgb(191, 97, 106),      // Aurora red
            info: Color::Rgb(129, 161, 193),      // Frost
            text: TextColors {
                primary: Color::Rgb(236, 239, 244),
                secondary: Color::Rgb(76, 86, 106),
                disabled: Color::Rgb(76, 86, 106),
                inverted: Color::Rgb(46, 52, 64),
                link: Color::Rgb(136, 192, 208),
            },
            background: BackgroundColors {
                default: Color::Rgb(46, 52, 64),
                elevated: Color::Rgb(59, 66, 82),
                selected: Color::Rgb(67, 76, 94),
                hover: Color::Rgb(59, 66, 82),
                disabled: Color::Rgb(59, 66, 82),
            },
            border: BorderColors {
                default: Color::Rgb(76, 86, 106),
                focused: Color::Rgb(136, 192, 208),
                error: Color::Rgb(191, 97, 106),
                disabled: Color::Rgb(59, 66, 82),
            },
            components: ComponentColors {
                input: InputColors {
                    background: Color::Rgb(46, 52, 64),
                    text: Color::Rgb(236, 239, 244),
                    placeholder: Color::Rgb(76, 86, 106),
                    cursor: Color::Rgb(236, 239, 244),
                    selection: Color::Rgb(67, 76, 94),
                },
                button: ButtonColors {
                    primary_bg: Color::Rgb(136, 192, 208),
                    primary_text: Color::Rgb(46, 52, 64),
                    secondary_bg: Color::Rgb(67, 76, 94),
                    secondary_text: Color::Rgb(236, 239, 244),
                    danger_bg: Color::Rgb(191, 97, 106),
                    danger_text: Color::Rgb(236, 239, 244),
                },
                list: ListColors {
                    item_bg: Color::Rgb(46, 52, 64),
                    item_text: Color::Rgb(236, 239, 244),
                    selected_bg: Color::Rgb(67, 76, 94),
                    selected_text: Color::Rgb(236, 239, 244),
                    focused_bg: Color::Rgb(136, 192, 208),
                    focused_text: Color::Rgb(46, 52, 64),
                },
                progress: ProgressColors {
                    track: Color::Rgb(67, 76, 94),
                    fill: Color::Rgb(136, 192, 208),
                    completed: Color::Rgb(163, 190, 140),
                },
            },
        }
    }

    /// Solarized Dark theme
    pub fn solarized_dark() -> Self {
        Self {
            name: "solarized_dark".to_string(),
            primary: Color::Rgb(38, 139, 210),    // Blue
            secondary: Color::Rgb(108, 113, 196), // Violet
            success: Color::Rgb(133, 153, 0),     // Green
            warning: Color::Rgb(181, 137, 0),     // Yellow
            error: Color::Rgb(220, 50, 47),       // Red
            info: Color::Rgb(42, 161, 152),       // Cyan
            text: TextColors {
                primary: Color::Rgb(131, 148, 150),
                secondary: Color::Rgb(88, 110, 117),
                disabled: Color::Rgb(88, 110, 117),
                inverted: Color::Rgb(0, 43, 54),
                link: Color::Rgb(38, 139, 210),
            },
            background: BackgroundColors {
                default: Color::Rgb(0, 43, 54),
                elevated: Color::Rgb(7, 54, 66),
                selected: Color::Rgb(7, 54, 66),
                hover: Color::Rgb(7, 54, 66),
                disabled: Color::Rgb(7, 54, 66),
            },
            border: BorderColors {
                default: Color::Rgb(88, 110, 117),
                focused: Color::Rgb(38, 139, 210),
                error: Color::Rgb(220, 50, 47),
                disabled: Color::Rgb(7, 54, 66),
            },
            components: ComponentColors {
                input: InputColors {
                    background: Color::Rgb(0, 43, 54),
                    text: Color::Rgb(131, 148, 150),
                    placeholder: Color::Rgb(88, 110, 117),
                    cursor: Color::Rgb(131, 148, 150),
                    selection: Color::Rgb(7, 54, 66),
                },
                button: ButtonColors {
                    primary_bg: Color::Rgb(38, 139, 210),
                    primary_text: Color::Rgb(253, 246, 227),
                    secondary_bg: Color::Rgb(7, 54, 66),
                    secondary_text: Color::Rgb(131, 148, 150),
                    danger_bg: Color::Rgb(220, 50, 47),
                    danger_text: Color::Rgb(253, 246, 227),
                },
                list: ListColors {
                    item_bg: Color::Rgb(0, 43, 54),
                    item_text: Color::Rgb(131, 148, 150),
                    selected_bg: Color::Rgb(7, 54, 66),
                    selected_text: Color::Rgb(131, 148, 150),
                    focused_bg: Color::Rgb(38, 139, 210),
                    focused_text: Color::Rgb(253, 246, 227),
                },
                progress: ProgressColors {
                    track: Color::Rgb(7, 54, 66),
                    fill: Color::Rgb(38, 139, 210),
                    completed: Color::Rgb(133, 153, 0),
                },
            },
        }
    }

    /// Get theme by name
    pub fn by_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "dark" => Some(Self::dark()),
            "light" => Some(Self::light()),
            "monokai" => Some(Self::monokai()),
            "dracula" => Some(Self::dracula()),
            "nord" => Some(Self::nord()),
            "solarized" | "solarized_dark" => Some(Self::solarized_dark()),
            _ => None,
        }
    }

    /// List available theme names
    pub fn available_themes() -> Vec<&'static str> {
        vec![
            "dark",
            "light",
            "monokai",
            "dracula",
            "nord",
            "solarized_dark",
        ]
    }

    /// Get color for a semantic purpose
    ///
    /// # Example
    ///
    /// ```
    /// use rnk::components::{Theme, SemanticColor};
    ///
    /// let theme = Theme::dark();
    /// let error_color = theme.semantic_color(SemanticColor::Error);
    /// ```
    pub fn semantic_color(&self, semantic: SemanticColor) -> Color {
        match semantic {
            SemanticColor::Primary => self.primary,
            SemanticColor::Secondary => self.secondary,
            SemanticColor::Success => self.success,
            SemanticColor::Warning => self.warning,
            SemanticColor::Error => self.error,
            SemanticColor::Info => self.info,
            SemanticColor::TextPrimary => self.text.primary,
            SemanticColor::TextSecondary => self.text.secondary,
            SemanticColor::TextDisabled => self.text.disabled,
            SemanticColor::Background => self.background.default,
            SemanticColor::BackgroundElevated => self.background.elevated,
            SemanticColor::Border => self.border.default,
            SemanticColor::BorderFocused => self.border.focused,
        }
    }

    /// Get ANSI foreground escape code for a semantic color
    ///
    /// Convenience method combining semantic_color() and to_ansi_fg()
    pub fn semantic_fg(&self, semantic: SemanticColor) -> String {
        self.semantic_color(semantic).to_ansi_fg()
    }

    /// Get ANSI background escape code for a semantic color
    ///
    /// Convenience method combining semantic_color() and to_ansi_bg()
    pub fn semantic_bg(&self, semantic: SemanticColor) -> String {
        self.semantic_color(semantic).to_ansi_bg()
    }
}

/// Semantic color purposes for theme-aware styling
///
/// Use with `Theme::semantic_color()` to get colors that adapt to the current theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticColor {
    /// Primary accent color
    Primary,
    /// Secondary accent color
    Secondary,
    /// Success/positive color
    Success,
    /// Warning/caution color
    Warning,
    /// Error/danger color
    Error,
    /// Informational color
    Info,
    /// Primary text color
    TextPrimary,
    /// Secondary/muted text color
    TextSecondary,
    /// Disabled text color
    TextDisabled,
    /// Default background color
    Background,
    /// Elevated/card background color
    BackgroundElevated,
    /// Default border color
    Border,
    /// Focused border color
    BorderFocused,
}

/// Builder for creating custom themes
#[derive(Debug, Clone)]
pub struct ThemeBuilder {
    theme: Theme,
}

impl ThemeBuilder {
    /// Create a new theme builder
    pub fn new(name: impl Into<String>) -> Self {
        let mut theme = Theme::dark();
        theme.name = name.into();
        Self { theme }
    }

    /// Set primary color
    pub fn primary(mut self, color: Color) -> Self {
        self.theme.primary = color;
        self
    }

    /// Set secondary color
    pub fn secondary(mut self, color: Color) -> Self {
        self.theme.secondary = color;
        self
    }

    /// Set success color
    pub fn success(mut self, color: Color) -> Self {
        self.theme.success = color;
        self
    }

    /// Set warning color
    pub fn warning(mut self, color: Color) -> Self {
        self.theme.warning = color;
        self
    }

    /// Set error color
    pub fn error(mut self, color: Color) -> Self {
        self.theme.error = color;
        self
    }

    /// Set info color
    pub fn info(mut self, color: Color) -> Self {
        self.theme.info = color;
        self
    }

    /// Set text colors
    pub fn text_colors(mut self, colors: TextColors) -> Self {
        self.theme.text = colors;
        self
    }

    /// Set background colors
    pub fn background_colors(mut self, colors: BackgroundColors) -> Self {
        self.theme.background = colors;
        self
    }

    /// Set border colors
    pub fn border_colors(mut self, colors: BorderColors) -> Self {
        self.theme.border = colors;
        self
    }

    /// Build the theme
    pub fn build(self) -> Theme {
        self.theme
    }
}

// Global theme context (thread-local)
thread_local! {
    static CURRENT_THEME: std::cell::RefCell<Theme> = std::cell::RefCell::new(Theme::dark());
}

/// Set the current theme
pub fn set_theme(theme: Theme) {
    if let Some(ctx) = crate::runtime::current_runtime() {
        ctx.borrow_mut().set_theme(theme);
        return;
    }

    CURRENT_THEME.with(|t| {
        *t.borrow_mut() = theme;
    });
}

/// Get the current theme
pub fn get_theme() -> Theme {
    if let Some(ctx) = crate::runtime::current_runtime() {
        return ctx.borrow().theme();
    }

    CURRENT_THEME.with(|t| t.borrow().clone())
}

/// Execute a closure with a specific theme
pub fn with_theme<F, R>(theme: Theme, f: F) -> R
where
    F: FnOnce(&Theme) -> R,
{
    let old_theme = get_theme();
    set_theme(theme.clone());
    let result = f(&theme);
    set_theme(old_theme);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_dark() {
        let theme = Theme::dark();
        assert_eq!(theme.name, "dark");
        assert_eq!(theme.primary, Color::Cyan);
    }

    #[test]
    fn test_theme_light() {
        let theme = Theme::light();
        assert_eq!(theme.name, "light");
        assert_eq!(theme.primary, Color::Blue);
    }

    #[test]
    fn test_theme_monokai() {
        let theme = Theme::monokai();
        assert_eq!(theme.name, "monokai");
    }

    #[test]
    fn test_theme_dracula() {
        let theme = Theme::dracula();
        assert_eq!(theme.name, "dracula");
    }

    #[test]
    fn test_theme_nord() {
        let theme = Theme::nord();
        assert_eq!(theme.name, "nord");
    }

    #[test]
    fn test_theme_solarized() {
        let theme = Theme::solarized_dark();
        assert_eq!(theme.name, "solarized_dark");
    }

    #[test]
    fn test_theme_by_name() {
        assert!(Theme::by_name("dark").is_some());
        assert!(Theme::by_name("light").is_some());
        assert!(Theme::by_name("monokai").is_some());
        assert!(Theme::by_name("dracula").is_some());
        assert!(Theme::by_name("nord").is_some());
        assert!(Theme::by_name("solarized").is_some());
        assert!(Theme::by_name("nonexistent").is_none());
    }

    #[test]
    fn test_available_themes() {
        let themes = Theme::available_themes();
        assert!(themes.contains(&"dark"));
        assert!(themes.contains(&"light"));
        assert!(themes.contains(&"monokai"));
    }

    #[test]
    fn test_theme_builder() {
        let theme = Theme::builder("custom")
            .primary(Color::Red)
            .secondary(Color::Blue)
            .success(Color::Green)
            .build();

        assert_eq!(theme.name, "custom");
        assert_eq!(theme.primary, Color::Red);
        assert_eq!(theme.secondary, Color::Blue);
        assert_eq!(theme.success, Color::Green);
    }

    #[test]
    fn test_set_get_theme() {
        let original = get_theme();

        set_theme(Theme::light());
        assert_eq!(get_theme().name, "light");

        set_theme(Theme::dark());
        assert_eq!(get_theme().name, "dark");

        set_theme(original);
    }

    #[test]
    fn test_with_theme() {
        let result = with_theme(Theme::monokai(), |theme| {
            assert_eq!(theme.name, "monokai");
            42
        });
        assert_eq!(result, 42);
    }

    #[test]
    fn test_theme_isolated_per_runtime_context() {
        use crate::runtime::{RuntimeContext, set_current_runtime};
        use std::cell::RefCell;
        use std::rc::Rc;

        set_current_runtime(None);
        set_theme(Theme::dark());
        assert_eq!(get_theme().name, "dark");

        let ctx1 = Rc::new(RefCell::new(RuntimeContext::new()));
        let ctx2 = Rc::new(RefCell::new(RuntimeContext::new()));

        set_current_runtime(Some(ctx1));
        set_theme(Theme::light());
        assert_eq!(get_theme().name, "light");

        set_current_runtime(Some(ctx2));
        assert_eq!(get_theme().name, "dark");

        set_current_runtime(None);
        assert_eq!(get_theme().name, "dark");
    }

    #[test]
    fn test_semantic_color() {
        let theme = Theme::dark();

        assert_eq!(theme.semantic_color(SemanticColor::Primary), theme.primary);
        assert_eq!(
            theme.semantic_color(SemanticColor::Secondary),
            theme.secondary
        );
        assert_eq!(theme.semantic_color(SemanticColor::Success), theme.success);
        assert_eq!(theme.semantic_color(SemanticColor::Warning), theme.warning);
        assert_eq!(theme.semantic_color(SemanticColor::Error), theme.error);
        assert_eq!(theme.semantic_color(SemanticColor::Info), theme.info);
        assert_eq!(
            theme.semantic_color(SemanticColor::TextPrimary),
            theme.text.primary
        );
        assert_eq!(
            theme.semantic_color(SemanticColor::Background),
            theme.background.default
        );
        assert_eq!(
            theme.semantic_color(SemanticColor::Border),
            theme.border.default
        );
    }

    #[test]
    fn test_semantic_fg_bg() {
        let theme = Theme::dark();

        // Test that semantic_fg returns valid ANSI codes
        let fg = theme.semantic_fg(SemanticColor::Error);
        assert!(fg.starts_with("\x1b["));

        let bg = theme.semantic_bg(SemanticColor::Error);
        assert!(bg.starts_with("\x1b["));
    }
}
