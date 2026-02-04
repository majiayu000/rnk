//! Miscellaneous icons (Nerd Font)

// Progress/Status
/// Spinner frames for animation
pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Dots spinner frames
pub const DOTS_FRAMES: &[&str] = &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];

/// Progress bar characters
pub const PROGRESS_FULL: &str = "█";
pub const PROGRESS_HALF: &str = "▌";
pub const PROGRESS_EMPTY: &str = "░";

/// Braille patterns for charts
pub const BRAILLE: &[&str] = &["⠀", "⠁", "⠂", "⠃", "⠄", "⠅", "⠆", "⠇", "⡀", "⡁", "⡂", "⡃", "⡄", "⡅", "⡆", "⡇"];

// Symbols
/// Bullet point
pub const fn bullet() -> &'static str { "•" }

/// Circle (empty)
pub const fn circle() -> &'static str { "○" }

/// Circle (filled)
pub const fn circle_filled() -> &'static str { "●" }

/// Square (empty)
pub const fn square() -> &'static str { "□" }

/// Square (filled)
pub const fn square_filled() -> &'static str { "■" }

/// Diamond (empty)
pub const fn diamond() -> &'static str { "◇" }

/// Diamond (filled)
pub const fn diamond_filled() -> &'static str { "◆" }

/// Triangle right
pub const fn triangle_right() -> &'static str { "▶" }

/// Triangle left
pub const fn triangle_left() -> &'static str { "◀" }

/// Triangle up
pub const fn triangle_up() -> &'static str { "▲" }

/// Triangle down
pub const fn triangle_down() -> &'static str { "▼" }

// Separators
/// Powerline arrow right
pub const fn powerline_right() -> &'static str { "" }

/// Powerline arrow left
pub const fn powerline_left() -> &'static str { "" }

/// Powerline arrow right (thin)
pub const fn powerline_right_thin() -> &'static str { "" }

/// Powerline arrow left (thin)
pub const fn powerline_left_thin() -> &'static str { "" }

/// Powerline round right
pub const fn powerline_round_right() -> &'static str { "" }

/// Powerline round left
pub const fn powerline_round_left() -> &'static str { "" }

// Misc
/// Music note
pub const fn music() -> &'static str { "" }

/// Camera
pub const fn camera() -> &'static str { "" }

/// Game controller
pub const fn gamepad() -> &'static str { "" }

/// Coffee
pub const fn coffee() -> &'static str { "" }

/// Beer
pub const fn beer() -> &'static str { "" }

/// Pizza
pub const fn pizza() -> &'static str { "" }

/// Skull
pub const fn skull() -> &'static str { "" }

/// Ghost
pub const fn ghost() -> &'static str { "" }

/// Robot
pub const fn robot() -> &'static str { "" }

/// Alien
pub const fn alien() -> &'static str { "" }

/// Crown
pub const fn crown() -> &'static str { "" }

/// Trophy
pub const fn trophy() -> &'static str { "" }

/// Medal
pub const fn medal() -> &'static str { "" }

/// Flag
pub const fn flag() -> &'static str { "" }

/// Pin/location
pub const fn pin() -> &'static str { "" }

/// Compass
pub const fn compass() -> &'static str { "" }

/// Globe
pub const fn globe() -> &'static str { "" }

/// Plane
pub const fn plane() -> &'static str { "" }

/// Car
pub const fn car() -> &'static str { "" }

/// Bike
pub const fn bike() -> &'static str { "" }

/// Train
pub const fn train() -> &'static str { "" }

/// Ship
pub const fn ship() -> &'static str { "" }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_misc_icons() {
        assert_eq!(bullet(), "•");
        assert_eq!(SPINNER_FRAMES.len(), 10);
        assert_eq!(PROGRESS_FULL, "█");
    }
}
