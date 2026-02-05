//! use_media_query hook for responsive terminal design
//!
//! Provides media query-like functionality for terminal dimensions.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//!
//! fn app() -> Element {
//!     let is_mobile = use_media_query(MediaQuery::max_width(40));
//!     let is_desktop = use_media_query(MediaQuery::min_width(80));
//!
//!     if is_mobile {
//!         Text::new("Mobile view").into_element()
//!     } else if is_desktop {
//!         Text::new("Desktop view").into_element()
//!     } else {
//!         Text::new("Tablet view").into_element()
//!     }
//! }
//! ```

use crate::hooks::use_window_size::use_window_size;

/// Media query condition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaQuery {
    /// Minimum width
    MinWidth(u16),
    /// Maximum width
    MaxWidth(u16),
    /// Minimum height
    MinHeight(u16),
    /// Maximum height
    MaxHeight(u16),
    /// Width range (inclusive)
    WidthRange(u16, u16),
    /// Height range (inclusive)
    HeightRange(u16, u16),
    /// Minimum aspect ratio (width/height * 100)
    MinAspectRatio(u16),
    /// Maximum aspect ratio (width/height * 100)
    MaxAspectRatio(u16),
    /// Portrait orientation (height > width)
    Portrait,
    /// Landscape orientation (width > height)
    Landscape,
    /// Square orientation (width == height)
    Square,
}

impl MediaQuery {
    /// Create a min-width query
    pub fn min_width(width: u16) -> Self {
        Self::MinWidth(width)
    }

    /// Create a max-width query
    pub fn max_width(width: u16) -> Self {
        Self::MaxWidth(width)
    }

    /// Create a min-height query
    pub fn min_height(height: u16) -> Self {
        Self::MinHeight(height)
    }

    /// Create a max-height query
    pub fn max_height(height: u16) -> Self {
        Self::MaxHeight(height)
    }

    /// Create a width range query
    pub fn width_between(min: u16, max: u16) -> Self {
        Self::WidthRange(min, max)
    }

    /// Create a height range query
    pub fn height_between(min: u16, max: u16) -> Self {
        Self::HeightRange(min, max)
    }

    /// Create a portrait query
    pub fn portrait() -> Self {
        Self::Portrait
    }

    /// Create a landscape query
    pub fn landscape() -> Self {
        Self::Landscape
    }

    /// Create a square query
    pub fn square() -> Self {
        Self::Square
    }

    /// Check if the query matches the given dimensions
    pub fn matches(&self, width: u16, height: u16) -> bool {
        match self {
            Self::MinWidth(min) => width >= *min,
            Self::MaxWidth(max) => width <= *max,
            Self::MinHeight(min) => height >= *min,
            Self::MaxHeight(max) => height <= *max,
            Self::WidthRange(min, max) => width >= *min && width <= *max,
            Self::HeightRange(min, max) => height >= *min && height <= *max,
            Self::MinAspectRatio(min_ratio) => {
                let ratio = (width as u32 * 100) / height.max(1) as u32;
                ratio >= *min_ratio as u32
            }
            Self::MaxAspectRatio(max_ratio) => {
                let ratio = (width as u32 * 100) / height.max(1) as u32;
                ratio <= *max_ratio as u32
            }
            Self::Portrait => height > width,
            Self::Landscape => width > height,
            Self::Square => width == height,
        }
    }
}

/// Breakpoint presets for common terminal sizes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breakpoint {
    /// Extra small (< 40 columns)
    Xs,
    /// Small (40-59 columns)
    Sm,
    /// Medium (60-79 columns)
    Md,
    /// Large (80-119 columns)
    Lg,
    /// Extra large (120+ columns)
    Xl,
}

impl Breakpoint {
    /// Get the minimum width for this breakpoint
    pub fn min_width(&self) -> u16 {
        match self {
            Self::Xs => 0,
            Self::Sm => 40,
            Self::Md => 60,
            Self::Lg => 80,
            Self::Xl => 120,
        }
    }

    /// Get the maximum width for this breakpoint
    pub fn max_width(&self) -> u16 {
        match self {
            Self::Xs => 39,
            Self::Sm => 59,
            Self::Md => 79,
            Self::Lg => 119,
            Self::Xl => u16::MAX,
        }
    }

    /// Create a media query for this breakpoint and up
    pub fn up(&self) -> MediaQuery {
        MediaQuery::MinWidth(self.min_width())
    }

    /// Create a media query for this breakpoint and down
    pub fn down(&self) -> MediaQuery {
        MediaQuery::MaxWidth(self.max_width())
    }

    /// Create a media query for exactly this breakpoint
    pub fn only(&self) -> MediaQuery {
        MediaQuery::WidthRange(self.min_width(), self.max_width())
    }

    /// Determine the current breakpoint from width
    pub fn from_width(width: u16) -> Self {
        match width {
            0..=39 => Self::Xs,
            40..=59 => Self::Sm,
            60..=79 => Self::Md,
            80..=119 => Self::Lg,
            _ => Self::Xl,
        }
    }
}

/// Hook to check if a media query matches
pub fn use_media_query(query: MediaQuery) -> bool {
    let (width, height) = use_window_size();
    query.matches(width, height)
}

/// Hook to get the current breakpoint
pub fn use_breakpoint() -> Breakpoint {
    let (width, _) = use_window_size();
    Breakpoint::from_width(width)
}

/// Hook to check if width is at least a breakpoint
pub fn use_breakpoint_up(breakpoint: Breakpoint) -> bool {
    use_media_query(breakpoint.up())
}

/// Hook to check if width is at most a breakpoint
pub fn use_breakpoint_down(breakpoint: Breakpoint) -> bool {
    use_media_query(breakpoint.down())
}

/// Hook to check if width is exactly a breakpoint
pub fn use_breakpoint_only(breakpoint: Breakpoint) -> bool {
    use_media_query(breakpoint.only())
}

/// Hook to check if terminal is in portrait mode
pub fn use_is_portrait() -> bool {
    use_media_query(MediaQuery::Portrait)
}

/// Hook to check if terminal is in landscape mode
pub fn use_is_landscape() -> bool {
    use_media_query(MediaQuery::Landscape)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_query_min_width() {
        let query = MediaQuery::min_width(80);
        assert!(query.matches(80, 24));
        assert!(query.matches(100, 24));
        assert!(!query.matches(79, 24));
    }

    #[test]
    fn test_media_query_max_width() {
        let query = MediaQuery::max_width(80);
        assert!(query.matches(80, 24));
        assert!(query.matches(60, 24));
        assert!(!query.matches(81, 24));
    }

    #[test]
    fn test_media_query_min_height() {
        let query = MediaQuery::min_height(24);
        assert!(query.matches(80, 24));
        assert!(query.matches(80, 30));
        assert!(!query.matches(80, 23));
    }

    #[test]
    fn test_media_query_max_height() {
        let query = MediaQuery::max_height(24);
        assert!(query.matches(80, 24));
        assert!(query.matches(80, 20));
        assert!(!query.matches(80, 25));
    }

    #[test]
    fn test_media_query_width_range() {
        let query = MediaQuery::width_between(60, 100);
        assert!(query.matches(60, 24));
        assert!(query.matches(80, 24));
        assert!(query.matches(100, 24));
        assert!(!query.matches(59, 24));
        assert!(!query.matches(101, 24));
    }

    #[test]
    fn test_media_query_height_range() {
        let query = MediaQuery::height_between(20, 30);
        assert!(query.matches(80, 20));
        assert!(query.matches(80, 25));
        assert!(query.matches(80, 30));
        assert!(!query.matches(80, 19));
        assert!(!query.matches(80, 31));
    }

    #[test]
    fn test_media_query_portrait() {
        let query = MediaQuery::portrait();
        assert!(query.matches(40, 60));
        assert!(!query.matches(60, 40));
        assert!(!query.matches(50, 50));
    }

    #[test]
    fn test_media_query_landscape() {
        let query = MediaQuery::landscape();
        assert!(query.matches(60, 40));
        assert!(!query.matches(40, 60));
        assert!(!query.matches(50, 50));
    }

    #[test]
    fn test_media_query_square() {
        let query = MediaQuery::square();
        assert!(query.matches(50, 50));
        assert!(!query.matches(60, 40));
        assert!(!query.matches(40, 60));
    }

    #[test]
    fn test_breakpoint_from_width() {
        assert_eq!(Breakpoint::from_width(30), Breakpoint::Xs);
        assert_eq!(Breakpoint::from_width(50), Breakpoint::Sm);
        assert_eq!(Breakpoint::from_width(70), Breakpoint::Md);
        assert_eq!(Breakpoint::from_width(100), Breakpoint::Lg);
        assert_eq!(Breakpoint::from_width(150), Breakpoint::Xl);
    }

    #[test]
    fn test_breakpoint_up() {
        let query = Breakpoint::Md.up();
        assert!(query.matches(60, 24));
        assert!(query.matches(100, 24));
        assert!(!query.matches(59, 24));
    }

    #[test]
    fn test_breakpoint_down() {
        let query = Breakpoint::Md.down();
        assert!(query.matches(79, 24));
        assert!(query.matches(60, 24));
        assert!(!query.matches(80, 24));
    }

    #[test]
    fn test_breakpoint_only() {
        let query = Breakpoint::Md.only();
        assert!(query.matches(60, 24));
        assert!(query.matches(79, 24));
        assert!(!query.matches(59, 24));
        assert!(!query.matches(80, 24));
    }

    #[test]
    fn test_use_media_query_compiles() {
        fn _test() {
            let _ = use_media_query(MediaQuery::min_width(80));
        }
    }

    #[test]
    fn test_use_breakpoint_compiles() {
        fn _test() {
            let _ = use_breakpoint();
        }
    }
}
