//! Progress/Gauge component for displaying progress
//!
//! Provides progress bars and gauges for showing completion status.

use crate::core::{Color, Element};
use crate::components::{Box as TinkBox, Text, Span, Line};

/// Progress bar style
#[derive(Debug, Clone)]
pub struct ProgressSymbols {
    /// Character for filled portion
    pub filled: char,
    /// Character for empty portion
    pub empty: char,
    /// Character for the current position (head)
    pub head: Option<char>,
    /// Left bracket
    pub bracket_left: Option<char>,
    /// Right bracket
    pub bracket_right: Option<char>,
}

impl Default for ProgressSymbols {
    fn default() -> Self {
        Self {
            filled: '█',
            empty: '░',
            head: None,
            bracket_left: Some('['),
            bracket_right: Some(']'),
        }
    }
}

impl ProgressSymbols {
    /// Block style (default)
    pub fn block() -> Self {
        Self::default()
    }

    /// Line style
    pub fn line() -> Self {
        Self {
            filled: '━',
            empty: '─',
            head: Some('╸'),
            bracket_left: None,
            bracket_right: None,
        }
    }

    /// Dot style
    pub fn dot() -> Self {
        Self {
            filled: '●',
            empty: '○',
            head: None,
            bracket_left: Some('⟨'),
            bracket_right: Some('⟩'),
        }
    }

    /// ASCII style
    pub fn ascii() -> Self {
        Self {
            filled: '#',
            empty: '-',
            head: Some('>'),
            bracket_left: Some('['),
            bracket_right: Some(']'),
        }
    }

    /// Thin line style
    pub fn thin() -> Self {
        Self {
            filled: '─',
            empty: ' ',
            head: Some('●'),
            bracket_left: None,
            bracket_right: None,
        }
    }
}

/// Progress bar component
#[derive(Debug, Clone)]
pub struct Progress {
    /// Progress value (0.0 to 1.0)
    progress: f32,
    /// Width of the progress bar
    width: u16,
    /// Symbols to use
    symbols: ProgressSymbols,
    /// Filled portion color
    filled_color: Option<Color>,
    /// Empty portion color
    empty_color: Option<Color>,
    /// Show percentage label
    show_percent: bool,
    /// Custom label
    label: Option<String>,
    /// Key for reconciliation
    key: Option<String>,
}

impl Progress {
    /// Create a new progress bar
    pub fn new() -> Self {
        Self {
            progress: 0.0,
            width: 20,
            symbols: ProgressSymbols::default(),
            filled_color: None,
            empty_color: None,
            show_percent: false,
            label: None,
            key: None,
        }
    }

    /// Set progress (0.0 to 1.0)
    pub fn progress(mut self, progress: f32) -> Self {
        self.progress = progress.clamp(0.0, 1.0);
        self
    }

    /// Set progress from ratio
    pub fn ratio(mut self, current: usize, total: usize) -> Self {
        if total == 0 {
            self.progress = 0.0;
        } else {
            self.progress = (current as f32 / total as f32).clamp(0.0, 1.0);
        }
        self
    }

    /// Set width
    pub fn width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    /// Set symbols
    pub fn symbols(mut self, symbols: ProgressSymbols) -> Self {
        self.symbols = symbols;
        self
    }

    /// Set filled color
    pub fn filled_color(mut self, color: Color) -> Self {
        self.filled_color = Some(color);
        self
    }

    /// Set empty color
    pub fn empty_color(mut self, color: Color) -> Self {
        self.empty_color = Some(color);
        self
    }

    /// Show percentage
    pub fn show_percent(mut self, show: bool) -> Self {
        self.show_percent = show;
        self
    }

    /// Set custom label
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set key
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Convert to element
    pub fn into_element(self) -> Element {
        let mut spans = Vec::new();

        // Left bracket
        if let Some(bracket) = self.symbols.bracket_left {
            spans.push(Span::new(bracket.to_string()));
        }

        // Calculate bar portions
        let bracket_width = self.symbols.bracket_left.is_some() as u16
            + self.symbols.bracket_right.is_some() as u16;
        let bar_width = self.width.saturating_sub(bracket_width) as usize;

        let filled_width = (self.progress * bar_width as f32).round() as usize;
        let empty_width = bar_width.saturating_sub(filled_width);

        // Handle head character
        let (actual_filled, has_head) = if self.symbols.head.is_some() && filled_width > 0 && filled_width < bar_width {
            (filled_width.saturating_sub(1), true)
        } else {
            (filled_width, false)
        };

        // Filled portion
        if actual_filled > 0 {
            let filled_str: String = std::iter::repeat_n(self.symbols.filled, actual_filled).collect();
            let mut filled_span = Span::new(filled_str);
            if let Some(color) = self.filled_color {
                filled_span = filled_span.color(color);
            }
            spans.push(filled_span);
        }

        // Head character
        if has_head
            && let Some(head) = self.symbols.head {
                let mut head_span = Span::new(head.to_string());
                if let Some(color) = self.filled_color {
                    head_span = head_span.color(color);
                }
                spans.push(head_span);
            }

        // Empty portion
        if empty_width > 0 {
            let empty_str: String = std::iter::repeat_n(self.symbols.empty, empty_width).collect();
            let mut empty_span = Span::new(empty_str);
            if let Some(color) = self.empty_color {
                empty_span = empty_span.color(color);
            } else {
                empty_span = empty_span.dim();
            }
            spans.push(empty_span);
        }

        // Right bracket
        if let Some(bracket) = self.symbols.bracket_right {
            spans.push(Span::new(bracket.to_string()));
        }

        // Percentage or label
        if self.show_percent {
            let percent = format!(" {:3.0}%", self.progress * 100.0);
            spans.push(Span::new(percent));
        }

        if let Some(label) = self.label {
            spans.push(Span::new(format!(" {}", label)));
        }

        let text = Text::line(Line::from_spans(spans));
        let mut container = TinkBox::new()
            .child(text.into_element());

        if let Some(key) = self.key {
            container = container.key(key);
        }

        container.into_element()
    }
}

impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
}

/// Gauge component (circular-style progress, rendered as text)
#[derive(Debug, Clone)]
pub struct Gauge {
    /// Progress value (0.0 to 1.0)
    progress: f32,
    /// Label to display
    label: Option<String>,
    /// Color
    color: Option<Color>,
    /// Key for reconciliation
    key: Option<String>,
}

impl Gauge {
    /// Create a new gauge
    pub fn new() -> Self {
        Self {
            progress: 0.0,
            label: None,
            color: None,
            key: None,
        }
    }

    /// Set progress
    pub fn progress(mut self, progress: f32) -> Self {
        self.progress = progress.clamp(0.0, 1.0);
        self
    }

    /// Set label
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set key
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Convert to element
    pub fn into_element(self) -> Element {
        // Simple gauge representation: [████░░░░] 75%
        let width = 10usize;
        let filled = (self.progress * width as f32).round() as usize;
        let empty = width - filled;

        let mut spans = Vec::new();

        let filled_str: String = std::iter::repeat_n('█', filled).collect();
        let empty_str: String = std::iter::repeat_n('░', empty).collect();

        let mut filled_span = Span::new(filled_str);
        if let Some(color) = self.color {
            filled_span = filled_span.color(color);
        }
        spans.push(filled_span);
        spans.push(Span::new(empty_str).dim());

        // Percentage
        let percent = format!(" {:3.0}%", self.progress * 100.0);
        let mut percent_span = Span::new(percent);
        if let Some(color) = self.color {
            percent_span = percent_span.color(color);
        }
        spans.push(percent_span.bold());

        // Label
        if let Some(label) = self.label {
            spans.push(Span::new(format!(" {}", label)));
        }

        let text = Text::line(Line::from_spans(spans));
        let mut container = TinkBox::new()
            .child(text.into_element());

        if let Some(key) = self.key {
            container = container.key(key);
        }

        container.into_element()
    }
}

impl Default for Gauge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_creation() {
        let progress = Progress::new()
            .progress(0.5)
            .width(20);

        assert_eq!(progress.width, 20);
        assert!((progress.progress - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_progress_ratio() {
        let progress = Progress::new()
            .ratio(50, 100);

        assert!((progress.progress - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_progress_symbols() {
        let block = ProgressSymbols::block();
        assert_eq!(block.filled, '█');

        let ascii = ProgressSymbols::ascii();
        assert_eq!(ascii.filled, '#');
    }

    #[test]
    fn test_gauge_creation() {
        let gauge = Gauge::new()
            .progress(0.75)
            .label("CPU");

        assert!((gauge.progress - 0.75).abs() < 0.01);
        assert_eq!(gauge.label, Some("CPU".to_string()));
    }
}
