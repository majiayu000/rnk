//! Line chart component for data visualization
//!
//! Displays a line chart using Unicode braille characters for high resolution.

use crate::components::{Box as RnkBox, Text};
use crate::core::{Color, Element, FlexDirection};

/// Braille patterns for line chart rendering
/// Each braille character is a 2x4 grid of dots
const BRAILLE_BASE: u32 = 0x2800;

/// Line chart component
#[derive(Debug, Clone)]
pub struct LineChart {
    /// Data series (multiple lines supported)
    series: Vec<Series>,
    /// Chart width in characters
    width: u16,
    /// Chart height in characters
    height: u16,
    /// Minimum Y value (auto-detect if None)
    min_y: Option<f64>,
    /// Maximum Y value (auto-detect if None)
    max_y: Option<f64>,
    /// Show X axis
    show_x_axis: bool,
    /// Show Y axis
    show_y_axis: bool,
    /// Show labels
    show_labels: bool,
    /// Title
    title: Option<String>,
    /// Key for reconciliation
    key: Option<String>,
}

/// A data series for the line chart
#[derive(Debug, Clone)]
pub struct Series {
    /// Data points (x, y)
    pub data: Vec<(f64, f64)>,
    /// Series color
    pub color: Color,
    /// Series label
    pub label: Option<String>,
}

impl Series {
    /// Create a new series with data
    pub fn new(data: Vec<(f64, f64)>) -> Self {
        Self {
            data,
            color: Color::White,
            label: None,
        }
    }

    /// Create series from Y values only (X will be 0, 1, 2, ...)
    pub fn from_y_values(values: Vec<f64>) -> Self {
        let data = values
            .into_iter()
            .enumerate()
            .map(|(i, y)| (i as f64, y))
            .collect();
        Self::new(data)
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set label
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl LineChart {
    /// Create a new line chart
    pub fn new() -> Self {
        Self {
            series: Vec::new(),
            width: 60,
            height: 15,
            min_y: None,
            max_y: None,
            show_x_axis: true,
            show_y_axis: true,
            show_labels: true,
            title: None,
            key: None,
        }
    }

    /// Add a data series
    pub fn series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    /// Add data from Y values only (convenience method for single series)
    pub fn data(mut self, values: Vec<f64>) -> Self {
        self.series.push(Series::from_y_values(values));
        self
    }

    /// Add data with color
    pub fn data_colored(mut self, values: Vec<f64>, color: Color) -> Self {
        self.series.push(Series::from_y_values(values).color(color));
        self
    }

    /// Set chart width
    pub fn width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    /// Set chart height
    pub fn height(mut self, height: u16) -> Self {
        self.height = height;
        self
    }

    /// Set minimum Y value
    pub fn min_y(mut self, min: f64) -> Self {
        self.min_y = Some(min);
        self
    }

    /// Set maximum Y value
    pub fn max_y(mut self, max: f64) -> Self {
        self.max_y = Some(max);
        self
    }

    /// Show/hide X axis
    pub fn show_x_axis(mut self, show: bool) -> Self {
        self.show_x_axis = show;
        self
    }

    /// Show/hide Y axis
    pub fn show_y_axis(mut self, show: bool) -> Self {
        self.show_y_axis = show;
        self
    }

    /// Show/hide labels
    pub fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Set title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set key
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Convert to element
    pub fn into_element(self) -> Element {
        if self.series.is_empty() || self.series.iter().all(|s| s.data.is_empty()) {
            return RnkBox::new().into_element();
        }

        // Calculate bounds
        let (min_x, max_x, min_y, max_y) = self.calculate_bounds();
        let x_range = max_x - min_x;
        let y_range = max_y - min_y;

        // Braille resolution: each char is 2 dots wide, 4 dots tall
        let dot_width = self.width as usize * 2;
        let dot_height = self.height as usize * 4;

        // Create dot grid
        let mut grid = vec![vec![false; dot_width]; dot_height];

        // Plot each series
        for series in &self.series {
            self.plot_series(
                &mut grid, series, min_x, x_range, min_y, y_range, dot_width, dot_height,
            );
        }

        // Convert grid to braille characters
        let mut lines = Vec::new();

        // Title
        if let Some(ref title) = self.title {
            lines.push(Text::new(title.clone()).bold().into_element());
        }

        // Y axis label area width
        let y_label_width = if self.show_y_axis && self.show_labels {
            8
        } else {
            0
        };

        // Render chart rows
        for row in 0..self.height as usize {
            let mut row_text = String::new();

            // Y axis label
            if self.show_y_axis && self.show_labels {
                let y_val = max_y - (row as f64 / self.height as f64) * y_range;
                row_text.push_str(&format!("{:>7.1} ", y_val));
            }

            // Chart content
            for col in 0..self.width as usize {
                let braille = self.get_braille_char(&grid, col, row);
                row_text.push(braille);
            }

            lines.push(Text::new(row_text).into_element());
        }

        // X axis
        if self.show_x_axis && self.show_labels {
            let mut x_axis = String::new();
            for _ in 0..y_label_width {
                x_axis.push(' ');
            }
            x_axis.push_str(&format!("{:<.1}", min_x));
            let mid_pos = self.width as usize / 2 - 4;
            for _ in 0..mid_pos {
                x_axis.push(' ');
            }
            x_axis.push_str(&format!("{:.1}", (min_x + max_x) / 2.0));
            let end_pos = self.width as usize - x_axis.len() + y_label_width - 4;
            for _ in 0..end_pos.max(1) {
                x_axis.push(' ');
            }
            x_axis.push_str(&format!("{:.1}", max_x));
            lines.push(Text::new(x_axis).dim().into_element());
        }

        // Legend
        if self.show_labels && self.series.iter().any(|s| s.label.is_some()) {
            let mut legend_children = Vec::new();
            for series in &self.series {
                if let Some(ref label) = series.label {
                    legend_children.push(
                        Text::new(format!("● {}", label))
                            .color(series.color)
                            .into_element(),
                    );
                }
            }
            if !legend_children.is_empty() {
                lines.push(
                    RnkBox::new()
                        .flex_direction(FlexDirection::Row)
                        .gap(2.0)
                        .children(legend_children)
                        .into_element(),
                );
            }
        }

        let mut container = RnkBox::new()
            .flex_direction(FlexDirection::Column)
            .children(lines);

        if let Some(key) = self.key {
            container = container.key(key);
        }

        container.into_element()
    }

    fn calculate_bounds(&self) -> (f64, f64, f64, f64) {
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for series in &self.series {
            for &(x, y) in &series.data {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }

        let min_y = self.min_y.unwrap_or(min_y);
        let max_y = self.max_y.unwrap_or(max_y);

        // Ensure non-zero ranges
        let min_x = if min_x == max_x { min_x - 1.0 } else { min_x };
        let max_x = if min_x == max_x { max_x + 1.0 } else { max_x };
        let min_y = if min_y == max_y { min_y - 1.0 } else { min_y };
        let max_y = if min_y == max_y { max_y + 1.0 } else { max_y };

        (min_x, max_x, min_y, max_y)
    }

    #[allow(clippy::too_many_arguments)]
    fn plot_series(
        &self,
        grid: &mut [Vec<bool>],
        series: &Series,
        min_x: f64,
        x_range: f64,
        min_y: f64,
        y_range: f64,
        dot_width: usize,
        dot_height: usize,
    ) {
        if series.data.len() < 2 {
            // Plot single point
            if let Some(&(x, y)) = series.data.first() {
                let dx = ((x - min_x) / x_range * (dot_width - 1) as f64) as usize;
                let dy = ((max_f64(y_range, 0.001) - (y - min_y)) / max_f64(y_range, 0.001)
                    * (dot_height - 1) as f64) as usize;
                if dx < dot_width && dy < dot_height {
                    grid[dy][dx] = true;
                }
            }
            return;
        }

        // Plot line segments
        for i in 0..series.data.len() - 1 {
            let (x1, y1) = series.data[i];
            let (x2, y2) = series.data[i + 1];

            let dx1 = ((x1 - min_x) / x_range * (dot_width - 1) as f64) as i32;
            let dy1 = ((y_range - (y1 - min_y)) / y_range * (dot_height - 1) as f64) as i32;
            let dx2 = ((x2 - min_x) / x_range * (dot_width - 1) as f64) as i32;
            let dy2 = ((y_range - (y2 - min_y)) / y_range * (dot_height - 1) as f64) as i32;

            // Bresenham's line algorithm
            self.draw_line(grid, dx1, dy1, dx2, dy2, dot_width, dot_height);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_line(
        &self,
        grid: &mut [Vec<bool>],
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        dot_width: usize,
        dot_height: usize,
    ) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let mut x = x0;
        let mut y = y0;

        loop {
            if x >= 0 && x < dot_width as i32 && y >= 0 && y < dot_height as i32 {
                grid[y as usize][x as usize] = true;
            }

            if x == x1 && y == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                if x == x1 {
                    break;
                }
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                if y == y1 {
                    break;
                }
                err += dx;
                y += sy;
            }
        }
    }

    fn get_braille_char(&self, grid: &[Vec<bool>], col: usize, row: usize) -> char {
        let base_x = col * 2;
        let base_y = row * 4;

        let mut pattern: u32 = 0;

        // Braille dot positions:
        // 0 3
        // 1 4
        // 2 5
        // 6 7
        let dot_positions = [
            (0, 0, 0),
            (0, 1, 1),
            (0, 2, 2),
            (1, 0, 3),
            (1, 1, 4),
            (1, 2, 5),
            (0, 3, 6),
            (1, 3, 7),
        ];

        for (dx, dy, bit) in dot_positions {
            let x = base_x + dx;
            let y = base_y + dy;
            if y < grid.len() && x < grid[0].len() && grid[y][x] {
                pattern |= 1 << bit;
            }
        }

        char::from_u32(BRAILLE_BASE + pattern).unwrap_or(' ')
    }
}

fn max_f64(a: f64, b: f64) -> f64 {
    if a > b { a } else { b }
}

impl Default for LineChart {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_chart_creation() {
        let chart = LineChart::new()
            .data(vec![1.0, 2.0, 3.0, 4.0, 5.0])
            .width(40)
            .height(10);

        assert_eq!(chart.width, 40);
        assert_eq!(chart.height, 10);
    }

    #[test]
    fn test_series_creation() {
        let series = Series::from_y_values(vec![1.0, 2.0, 3.0])
            .color(Color::Red)
            .label("Test");

        assert_eq!(series.data.len(), 3);
        assert_eq!(series.color, Color::Red);
        assert_eq!(series.label, Some("Test".to_string()));
    }

    #[test]
    fn test_empty_chart() {
        let chart = LineChart::new();
        let _ = chart.into_element();
    }

    #[test]
    fn test_single_point() {
        let chart = LineChart::new().data(vec![5.0]);
        let _ = chart.into_element();
    }

    #[test]
    fn test_multiple_series() {
        let chart = LineChart::new()
            .series(
                Series::from_y_values(vec![1.0, 2.0, 3.0])
                    .color(Color::Red)
                    .label("A"),
            )
            .series(
                Series::from_y_values(vec![3.0, 2.0, 1.0])
                    .color(Color::Blue)
                    .label("B"),
            );

        assert_eq!(chart.series.len(), 2);
    }
}
