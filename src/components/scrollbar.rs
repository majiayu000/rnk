//! Scrollbar component for indicating scroll position
//!
//! Provides vertical and horizontal scrollbar widgets.

use crate::components::{Box as TinkBox, Text};
use crate::core::{Color, Element, FlexDirection};

/// Scrollbar orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollbarOrientation {
    /// Vertical scrollbar (default)
    #[default]
    Vertical,
    /// Horizontal scrollbar
    Horizontal,
}

/// Scrollbar symbols
#[derive(Debug, Clone)]
pub struct ScrollbarSymbols {
    /// Track character (background)
    pub track: char,
    /// Thumb character (position indicator)
    pub thumb: char,
    /// Start character (top/left)
    pub begin: Option<char>,
    /// End character (bottom/right)
    pub end: Option<char>,
}

impl Default for ScrollbarSymbols {
    fn default() -> Self {
        Self {
            track: '│',
            thumb: '█',
            begin: Some('▲'),
            end: Some('▼'),
        }
    }
}

impl ScrollbarSymbols {
    /// Vertical scrollbar symbols (default)
    pub fn vertical() -> Self {
        Self::default()
    }

    /// Horizontal scrollbar symbols
    pub fn horizontal() -> Self {
        Self {
            track: '─',
            thumb: '█',
            begin: Some('◄'),
            end: Some('►'),
        }
    }

    /// Simple block style
    pub fn block() -> Self {
        Self {
            track: '░',
            thumb: '█',
            begin: None,
            end: None,
        }
    }

    /// Line style
    pub fn line() -> Self {
        Self {
            track: '│',
            thumb: '┃',
            begin: None,
            end: None,
        }
    }

    /// Double line style
    pub fn double() -> Self {
        Self {
            track: '║',
            thumb: '█',
            begin: Some('╦'),
            end: Some('╩'),
        }
    }
}

/// Scrollbar component builder
#[derive(Debug, Clone)]
pub struct Scrollbar {
    /// Orientation
    orientation: ScrollbarOrientation,
    /// Symbols to use
    symbols: ScrollbarSymbols,
    /// Track color
    track_color: Option<Color>,
    /// Thumb color
    thumb_color: Option<Color>,
    /// Current position (0.0 to 1.0)
    position: f32,
    /// Viewport size ratio (0.0 to 1.0, how much of content is visible)
    viewport_ratio: f32,
    /// Total length in cells
    length: u16,
    /// Key for reconciliation
    key: Option<String>,
}

impl Scrollbar {
    /// Create a new vertical scrollbar
    pub fn new() -> Self {
        Self {
            orientation: ScrollbarOrientation::Vertical,
            symbols: ScrollbarSymbols::default(),
            track_color: None,
            thumb_color: None,
            position: 0.0,
            viewport_ratio: 0.5,
            length: 10,
            key: None,
        }
    }

    /// Create a horizontal scrollbar
    pub fn horizontal() -> Self {
        Self {
            orientation: ScrollbarOrientation::Horizontal,
            symbols: ScrollbarSymbols::horizontal(),
            ..Self::new()
        }
    }

    /// Set orientation
    pub fn orientation(mut self, orientation: ScrollbarOrientation) -> Self {
        self.orientation = orientation;
        if orientation == ScrollbarOrientation::Horizontal {
            self.symbols = ScrollbarSymbols::horizontal();
        }
        self
    }

    /// Set symbols
    pub fn symbols(mut self, symbols: ScrollbarSymbols) -> Self {
        self.symbols = symbols;
        self
    }

    /// Set track color
    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = Some(color);
        self
    }

    /// Set thumb color
    pub fn thumb_color(mut self, color: Color) -> Self {
        self.thumb_color = Some(color);
        self
    }

    /// Set position (0.0 to 1.0)
    pub fn position(mut self, position: f32) -> Self {
        self.position = position.clamp(0.0, 1.0);
        self
    }

    /// Set viewport ratio (what fraction of content is visible)
    pub fn viewport_ratio(mut self, ratio: f32) -> Self {
        self.viewport_ratio = ratio.clamp(0.0, 1.0);
        self
    }

    /// Set from content and viewport sizes
    pub fn from_sizes(mut self, content_size: usize, viewport_size: usize, offset: usize) -> Self {
        if content_size <= viewport_size {
            self.position = 0.0;
            self.viewport_ratio = 1.0;
        } else {
            let max_offset = content_size - viewport_size;
            self.position = offset as f32 / max_offset as f32;
            self.viewport_ratio = viewport_size as f32 / content_size as f32;
        }
        self
    }

    /// Set length in cells
    pub fn length(mut self, length: u16) -> Self {
        self.length = length;
        self
    }

    /// Set key
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Convert to element
    pub fn into_element(self) -> Element {
        let total_length = self.length as usize;
        let has_begin = self.symbols.begin.is_some();
        let has_end = self.symbols.end.is_some();

        // Calculate track length (excluding begin/end arrows)
        let arrow_count = has_begin as usize + has_end as usize;
        let track_length = total_length.saturating_sub(arrow_count);

        if track_length == 0 {
            // Not enough space for a scrollbar
            return TinkBox::new().into_element();
        }

        // Calculate thumb size and position
        let thumb_size = (self.viewport_ratio * track_length as f32).ceil() as usize;
        let thumb_size = thumb_size.max(1).min(track_length);
        let available = track_length - thumb_size;
        let thumb_start = (self.position * available as f32).round() as usize;

        // Build the scrollbar string
        let mut chars = Vec::with_capacity(total_length);

        // Begin arrow
        if let Some(begin) = self.symbols.begin {
            chars.push((begin, false)); // false = not thumb
        }

        // Track with thumb
        for i in 0..track_length {
            let is_thumb = i >= thumb_start && i < thumb_start + thumb_size;
            let ch = if is_thumb {
                self.symbols.thumb
            } else {
                self.symbols.track
            };
            chars.push((ch, is_thumb));
        }

        // End arrow
        if let Some(end) = self.symbols.end {
            chars.push((end, false));
        }

        // For vertical scrollbar, each character is on its own line
        if self.orientation == ScrollbarOrientation::Vertical {
            let mut container = TinkBox::new().flex_direction(FlexDirection::Column);

            if let Some(key) = self.key {
                container = container.key(key);
            }

            for (ch, is_thumb) in chars {
                let mut text = Text::new(ch.to_string());
                if is_thumb {
                    if let Some(color) = self.thumb_color {
                        text = text.color(color);
                    }
                } else if let Some(color) = self.track_color {
                    text = text.color(color);
                }
                container = container.child(text.into_element());
            }

            container.into_element()
        } else {
            // Horizontal scrollbar - all on one line
            let content: String = chars.iter().map(|(ch, _)| *ch).collect();
            let mut text = Text::new(content);

            // For simplicity, use thumb color if set
            if let Some(color) = self.thumb_color {
                text = text.color(color);
            }

            let mut container = TinkBox::new();
            if let Some(key) = self.key {
                container = container.key(key);
            }
            container = container.child(text.into_element());
            container.into_element()
        }
    }
}

impl Default for Scrollbar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrollbar_creation() {
        let scrollbar = Scrollbar::new().position(0.5).length(10);

        assert_eq!(scrollbar.length, 10);
    }

    #[test]
    fn test_scrollbar_from_sizes() {
        let scrollbar = Scrollbar::new().from_sizes(100, 20, 40).length(10);

        // position should be 40/80 = 0.5
        assert!((scrollbar.position - 0.5).abs() < 0.01);
        // viewport ratio should be 20/100 = 0.2
        assert!((scrollbar.viewport_ratio - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_scrollbar_symbols() {
        let vertical = ScrollbarSymbols::vertical();
        assert_eq!(vertical.track, '│');

        let horizontal = ScrollbarSymbols::horizontal();
        assert_eq!(horizontal.track, '─');

        let block = ScrollbarSymbols::block();
        assert_eq!(block.track, '░');
    }

    #[test]
    fn test_scrollbar_orientation() {
        let vertical = Scrollbar::new();
        assert_eq!(vertical.orientation, ScrollbarOrientation::Vertical);

        let horizontal = Scrollbar::horizontal();
        assert_eq!(horizontal.orientation, ScrollbarOrientation::Horizontal);
    }
}
