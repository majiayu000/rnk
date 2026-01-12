//! Bar chart component for data visualization
//!
//! Displays horizontal or vertical bar charts.

use crate::components::{Box as TinkBox, Line, Span, Text};
use crate::core::{Color, Element, FlexDirection};

/// Bar chart orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarChartOrientation {
    /// Horizontal bars (default)
    #[default]
    Horizontal,
    /// Vertical bars
    Vertical,
}

/// A single bar in the chart
#[derive(Debug, Clone)]
pub struct Bar {
    /// Bar label
    pub label: String,
    /// Bar value
    pub value: f64,
    /// Bar color
    pub color: Option<Color>,
}

impl Bar {
    /// Create a new bar
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            color: None,
        }
    }

    /// Set bar color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// Bar chart component
#[derive(Debug, Clone)]
pub struct BarChart {
    /// Bars to display
    bars: Vec<Bar>,
    /// Orientation
    orientation: BarChartOrientation,
    /// Maximum bar width/height
    bar_max_size: u16,
    /// Show values
    show_values: bool,
    /// Show labels
    show_labels: bool,
    /// Default bar color
    default_color: Option<Color>,
    /// Bar character
    bar_char: char,
    /// Gap between bars (for vertical)
    bar_gap: u16,
    /// Key for reconciliation
    key: Option<String>,
}

impl BarChart {
    /// Create a new bar chart
    pub fn new() -> Self {
        Self {
            bars: Vec::new(),
            orientation: BarChartOrientation::Horizontal,
            bar_max_size: 20,
            show_values: true,
            show_labels: true,
            default_color: None,
            bar_char: '█',
            bar_gap: 1,
            key: None,
        }
    }

    /// Create from bars
    pub fn from_bars<I>(bars: I) -> Self
    where
        I: IntoIterator<Item = Bar>,
    {
        Self {
            bars: bars.into_iter().collect(),
            ..Self::new()
        }
    }

    /// Set bars
    pub fn bars<I>(mut self, bars: I) -> Self
    where
        I: IntoIterator<Item = Bar>,
    {
        self.bars = bars.into_iter().collect();
        self
    }

    /// Add a bar
    pub fn bar(mut self, bar: Bar) -> Self {
        self.bars.push(bar);
        self
    }

    /// Set orientation
    pub fn orientation(mut self, orientation: BarChartOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Create horizontal bar chart
    pub fn horizontal(mut self) -> Self {
        self.orientation = BarChartOrientation::Horizontal;
        self
    }

    /// Create vertical bar chart
    pub fn vertical(mut self) -> Self {
        self.orientation = BarChartOrientation::Vertical;
        self
    }

    /// Set maximum bar size
    pub fn bar_max_size(mut self, size: u16) -> Self {
        self.bar_max_size = size;
        self
    }

    /// Show values
    pub fn show_values(mut self, show: bool) -> Self {
        self.show_values = show;
        self
    }

    /// Show labels
    pub fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Set default color
    pub fn default_color(mut self, color: Color) -> Self {
        self.default_color = Some(color);
        self
    }

    /// Set bar character
    pub fn bar_char(mut self, ch: char) -> Self {
        self.bar_char = ch;
        self
    }

    /// Set bar gap (for vertical orientation)
    pub fn bar_gap(mut self, gap: u16) -> Self {
        self.bar_gap = gap;
        self
    }

    /// Set key
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Convert to element
    pub fn into_element(self) -> Element {
        if self.bars.is_empty() {
            return TinkBox::new().into_element();
        }

        match self.orientation {
            BarChartOrientation::Horizontal => self.render_horizontal(),
            BarChartOrientation::Vertical => self.render_vertical(),
        }
    }

    /// Render horizontal bar chart
    fn render_horizontal(self) -> Element {
        // Find max value and max label length
        let max_value = self.bars.iter().map(|b| b.value).fold(0.0f64, f64::max);
        let max_label_len = if self.show_labels {
            self.bars.iter().map(|b| b.label.len()).max().unwrap_or(0)
        } else {
            0
        };

        let mut container = TinkBox::new().flex_direction(FlexDirection::Column);

        if let Some(ref key) = self.key {
            container = container.key(key.clone());
        }

        for bar in &self.bars {
            let mut spans = Vec::new();

            // Label
            if self.show_labels {
                let padded_label = format!("{:>width$} ", bar.label, width = max_label_len);
                spans.push(Span::new(padded_label));
            }

            // Bar
            let bar_len = if max_value > 0.0 {
                ((bar.value / max_value) * self.bar_max_size as f64).round() as usize
            } else {
                0
            };
            let bar_str: String = std::iter::repeat_n(self.bar_char, bar_len).collect();
            let mut bar_span = Span::new(bar_str);
            if let Some(color) = bar.color.or(self.default_color) {
                bar_span = bar_span.color(color);
            }
            spans.push(bar_span);

            // Value
            if self.show_values {
                let value_str = if bar.value == bar.value.trunc() {
                    format!(" {:.0}", bar.value)
                } else {
                    format!(" {:.1}", bar.value)
                };
                spans.push(Span::new(value_str).dim());
            }

            let text = Text::line(Line::from_spans(spans));
            container = container.child(text.into_element());
        }

        container.into_element()
    }

    /// Render vertical bar chart
    fn render_vertical(self) -> Element {
        // Find max value
        let max_value = self.bars.iter().map(|b| b.value).fold(0.0f64, f64::max);
        let height = self.bar_max_size as usize;

        let mut container = TinkBox::new().flex_direction(FlexDirection::Column);

        if let Some(ref key) = self.key {
            container = container.key(key.clone());
        }

        // Render from top to bottom
        for row in (0..height).rev() {
            let threshold = (row as f64 + 0.5) / height as f64 * max_value;
            let mut spans = Vec::new();

            for (i, bar) in self.bars.iter().enumerate() {
                if i > 0 {
                    // Gap between bars
                    spans.push(Span::new(" ".repeat(self.bar_gap as usize)));
                }

                if bar.value >= threshold {
                    let mut bar_span = Span::new(self.bar_char.to_string());
                    if let Some(color) = bar.color.or(self.default_color) {
                        bar_span = bar_span.color(color);
                    }
                    spans.push(bar_span);
                } else {
                    spans.push(Span::new(" "));
                }
            }

            let text = Text::line(Line::from_spans(spans));
            container = container.child(text.into_element());
        }

        // Labels row
        if self.show_labels {
            let mut label_spans = Vec::new();
            for (i, bar) in self.bars.iter().enumerate() {
                if i > 0 {
                    label_spans.push(Span::new(" ".repeat(self.bar_gap as usize)));
                }
                // Truncate label to 1 char for vertical chart
                let label_char = bar.label.chars().next().unwrap_or(' ');
                label_spans.push(Span::new(label_char.to_string()));
            }
            let label_text = Text::line(Line::from_spans(label_spans));
            container = container.child(label_text.into_element());
        }

        container.into_element()
    }
}

impl Default for BarChart {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bar_creation() {
        let bar = Bar::new("CPU", 75.0).color(Color::Green);

        assert_eq!(bar.label, "CPU");
        assert_eq!(bar.value, 75.0);
        assert_eq!(bar.color, Some(Color::Green));
    }

    #[test]
    fn test_barchart_creation() {
        let chart = BarChart::from_bars(vec![
            Bar::new("A", 10.0),
            Bar::new("B", 20.0),
            Bar::new("C", 30.0),
        ]);

        assert_eq!(chart.bars.len(), 3);
    }

    #[test]
    fn test_barchart_orientation() {
        let horizontal = BarChart::new().horizontal();
        assert_eq!(horizontal.orientation, BarChartOrientation::Horizontal);

        let vertical = BarChart::new().vertical();
        assert_eq!(vertical.orientation, BarChartOrientation::Vertical);
    }

    #[test]
    fn test_barchart_empty() {
        let chart = BarChart::new();
        // Should not panic
        let _ = chart.into_element();
    }
}
