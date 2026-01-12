//! Sparkline component for inline data visualization
//!
//! Displays a small graph of data points using Unicode block characters.

use crate::components::{Box as TinkBox, Text};
use crate::core::{Color, Element};

/// Block characters for sparkline (from lowest to highest)
const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Sparkline component
#[derive(Debug, Clone)]
pub struct Sparkline {
    /// Data points
    data: Vec<f64>,
    /// Width (number of characters)
    width: Option<u16>,
    /// Minimum value (auto-detect if None)
    min: Option<f64>,
    /// Maximum value (auto-detect if None)
    max: Option<f64>,
    /// Color for the sparkline
    color: Option<Color>,
    /// Show baseline (empty bottom character)
    show_baseline: bool,
    /// Key for reconciliation
    key: Option<String>,
}

impl Sparkline {
    /// Create a new sparkline
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            width: None,
            min: None,
            max: None,
            color: None,
            show_baseline: false,
            key: None,
        }
    }

    /// Create sparkline from data
    pub fn from_data<I>(data: I) -> Self
    where
        I: IntoIterator<Item = f64>,
    {
        Self {
            data: data.into_iter().collect(),
            ..Self::new()
        }
    }

    /// Set data points
    pub fn data<I>(mut self, data: I) -> Self
    where
        I: IntoIterator<Item = f64>,
    {
        self.data = data.into_iter().collect();
        self
    }

    /// Set data from integers
    pub fn data_u64<I>(mut self, data: I) -> Self
    where
        I: IntoIterator<Item = u64>,
    {
        self.data = data.into_iter().map(|v| v as f64).collect();
        self
    }

    /// Set width
    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    /// Set minimum value
    pub fn min(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }

    /// Set maximum value
    pub fn max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Show baseline
    pub fn show_baseline(mut self, show: bool) -> Self {
        self.show_baseline = show;
        self
    }

    /// Set key
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Convert to element
    pub fn into_element(self) -> Element {
        if self.data.is_empty() {
            return TinkBox::new().into_element();
        }

        // Calculate min/max
        let data_min = self.data.iter().cloned().fold(f64::INFINITY, f64::min);
        let data_max = self.data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let min = self.min.unwrap_or(data_min);
        let max = self.max.unwrap_or(data_max);
        let range = max - min;

        // Determine width and which data points to show
        let width = self.width.map(|w| w as usize).unwrap_or(self.data.len());
        let data_to_show = if self.data.len() > width {
            // Sample data to fit width
            self.sample_data(width)
        } else {
            self.data.clone()
        };

        // Convert to block characters
        let mut chars = String::with_capacity(data_to_show.len());
        for value in &data_to_show {
            let normalized = if range == 0.0 {
                0.5
            } else {
                ((value - min) / range).clamp(0.0, 1.0)
            };

            // Map to block index (0-7)
            let block_idx = if self.show_baseline && normalized == 0.0 {
                0 // Use lowest block for zero
            } else {
                ((normalized * 7.0).round() as usize).min(7)
            };

            chars.push(BLOCKS[block_idx]);
        }

        // Pad if needed
        while chars.len() < width {
            chars.push(' ');
        }

        let mut text = Text::new(chars);
        if let Some(color) = self.color {
            text = text.color(color);
        }

        let mut container = TinkBox::new().child(text.into_element());

        if let Some(key) = self.key {
            container = container.key(key);
        }

        container.into_element()
    }

    /// Sample data to fit width
    fn sample_data(&self, width: usize) -> Vec<f64> {
        if width == 0 || self.data.is_empty() {
            return Vec::new();
        }

        let step = self.data.len() as f64 / width as f64;
        let mut result = Vec::with_capacity(width);

        for i in 0..width {
            let start = (i as f64 * step) as usize;
            let end = ((i + 1) as f64 * step) as usize;
            let end = end.min(self.data.len());

            // Average the values in this bucket
            if start < end {
                let sum: f64 = self.data[start..end].iter().sum();
                result.push(sum / (end - start) as f64);
            } else if start < self.data.len() {
                result.push(self.data[start]);
            }
        }

        result
    }
}

impl Default for Sparkline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparkline_creation() {
        let sparkline = Sparkline::from_data(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(sparkline.data.len(), 5);
    }

    #[test]
    fn test_sparkline_sampling() {
        let sparkline =
            Sparkline::from_data(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]);
        let sampled = sparkline.sample_data(5);
        assert_eq!(sampled.len(), 5);
    }

    #[test]
    fn test_sparkline_empty() {
        let sparkline = Sparkline::new();
        // Should not panic
        let _ = sparkline.into_element();
    }

    #[test]
    fn test_sparkline_single_value() {
        let sparkline = Sparkline::from_data(vec![5.0]);
        let _ = sparkline.into_element();
    }
}
