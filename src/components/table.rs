//! Table component for displaying tabular data
//!
//! Provides a table widget with headers, rows, and optional selection.

use crate::core::{Color, Element, Style, FlexDirection};
use crate::components::{Box as TinkBox, Text, Span, Line};

/// Table cell content
#[derive(Debug, Clone)]
pub struct Cell {
    /// Cell content
    pub content: Line,
    /// Cell style
    pub style: Option<Style>,
}

impl Cell {
    /// Create a new cell from text
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: Line::raw(content),
            style: None,
        }
    }

    /// Create a cell from a Line
    pub fn from_line(line: Line) -> Self {
        Self {
            content: line,
            style: None,
        }
    }

    /// Create a cell from spans
    pub fn from_spans(spans: Vec<Span>) -> Self {
        Self {
            content: Line::from_spans(spans),
            style: None,
        }
    }

    /// Set cell style
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Set cell color
    pub fn color(mut self, color: Color) -> Self {
        let style = self.style.get_or_insert(Style::new());
        style.color = Some(color);
        self
    }
}

impl<T: Into<String>> From<T> for Cell {
    fn from(s: T) -> Self {
        Cell::new(s)
    }
}

/// Table row containing cells
#[derive(Debug, Clone)]
pub struct Row {
    /// Row cells
    pub cells: Vec<Cell>,
    /// Row style
    pub style: Option<Style>,
    /// Row height (in lines)
    pub height: u16,
}

impl Row {
    /// Create a new row from cells
    pub fn new<I, T>(cells: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Cell>,
    {
        Self {
            cells: cells.into_iter().map(|c| c.into()).collect(),
            style: None,
            height: 1,
        }
    }

    /// Set row style
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Set row height
    pub fn height(mut self, height: u16) -> Self {
        self.height = height;
        self
    }
}

/// Table state for tracking selection
#[derive(Debug, Clone, Default)]
pub struct TableState {
    /// Selected row index
    pub selected: Option<usize>,
    /// Scroll offset
    pub offset: usize,
}

impl TableState {
    /// Create a new table state
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with initial selection
    pub fn with_selected(selected: Option<usize>) -> Self {
        Self {
            selected,
            offset: 0,
        }
    }

    /// Select the next row
    pub fn select_next(&mut self, len: usize) {
        if len == 0 {
            self.selected = None;
            return;
        }
        self.selected = Some(match self.selected {
            Some(i) => (i + 1).min(len - 1),
            None => 0,
        });
    }

    /// Select the previous row
    pub fn select_previous(&mut self, len: usize) {
        if len == 0 {
            self.selected = None;
            return;
        }
        self.selected = Some(match self.selected {
            Some(i) => i.saturating_sub(1),
            None => 0,
        });
    }

    /// Select a specific row
    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
    }
}

/// Column constraint for width
#[derive(Debug, Clone, Copy)]
pub enum Constraint {
    /// Fixed width
    Length(u16),
    /// Minimum width
    Min(u16),
    /// Maximum width
    Max(u16),
    /// Percentage of available width
    Percentage(u16),
    /// Fill remaining space with ratio
    Ratio(u16, u16),
}

impl Default for Constraint {
    fn default() -> Self {
        Constraint::Min(1)
    }
}

/// Table component builder
#[derive(Debug, Clone)]
pub struct Table {
    /// Header row
    header: Option<Row>,
    /// Data rows
    rows: Vec<Row>,
    /// Column widths/constraints
    widths: Vec<Constraint>,
    /// Highlight style for selected row
    highlight_style: Style,
    /// Highlight symbol
    highlight_symbol: Option<String>,
    /// Column separator
    column_separator: Option<String>,
    /// Key for reconciliation
    key: Option<String>,
}

impl Table {
    /// Create a new empty table
    pub fn new() -> Self {
        Self {
            header: None,
            rows: Vec::new(),
            widths: Vec::new(),
            highlight_style: Style::new(),
            highlight_symbol: None,
            column_separator: Some(" ".to_string()),
            key: None,
        }
    }

    /// Set header row
    pub fn header(mut self, header: Row) -> Self {
        self.header = Some(header);
        self
    }

    /// Set data rows
    pub fn rows<I>(mut self, rows: I) -> Self
    where
        I: IntoIterator<Item = Row>,
    {
        self.rows = rows.into_iter().collect();
        self
    }

    /// Add a row
    pub fn row(mut self, row: Row) -> Self {
        self.rows.push(row);
        self
    }

    /// Set column widths
    pub fn widths<I>(mut self, widths: I) -> Self
    where
        I: IntoIterator<Item = Constraint>,
    {
        self.widths = widths.into_iter().collect();
        self
    }

    /// Set highlight style
    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    /// Set highlight symbol
    pub fn highlight_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.highlight_symbol = Some(symbol.into());
        self
    }

    /// Set column separator
    pub fn column_separator(mut self, sep: impl Into<String>) -> Self {
        self.column_separator = Some(sep.into());
        self
    }

    /// Set key
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Get number of rows
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Render the table with state
    pub fn render(self, state: &TableState) -> Element {
        let selected = state.selected;
        let separator = self.column_separator.as_deref().unwrap_or(" ");
        let symbol_width = self.highlight_symbol.as_ref().map(|s| s.len()).unwrap_or(0);

        let mut container = TinkBox::new()
            .flex_direction(FlexDirection::Column);

        if let Some(ref key) = self.key {
            container = container.key(key.clone());
        }

        // Render header if present
        if let Some(header) = &self.header {
            let header_element = self.render_row(header, separator, false, symbol_width);
            container = container.child(header_element);
        }

        // Render data rows
        for (idx, row) in self.rows.iter().enumerate() {
            let is_selected = selected == Some(idx);
            let row_element = self.render_row(row, separator, is_selected, symbol_width);
            container = container.child(row_element);
        }

        container.into_element()
    }

    /// Render a single row
    fn render_row(&self, row: &Row, separator: &str, is_selected: bool, symbol_width: usize) -> Element {
        let mut spans = Vec::new();

        // Add highlight symbol if configured
        if let Some(ref symbol) = self.highlight_symbol {
            if is_selected {
                spans.push(Span::new(symbol.clone()));
            } else {
                spans.push(Span::new(" ".repeat(symbol_width)));
            }
        }

        // Add cells
        for (i, cell) in row.cells.iter().enumerate() {
            if i > 0 {
                spans.push(Span::new(separator));
            }

            // Clone the cell content spans
            for span in &cell.content.spans {
                spans.push(span.clone());
            }
        }

        let line = Line::from_spans(spans);
        let mut text = Text::line(line);

        if is_selected {
            if let Some(color) = self.highlight_style.color {
                text = text.color(color);
            }
            if let Some(bg) = self.highlight_style.background_color {
                text = text.background(bg);
            }
            if self.highlight_style.bold {
                text = text.bold();
            }
            if self.highlight_style.inverse {
                text = text.inverse();
            }
        }

        text.into_element()
    }

    /// Convert to element (no selection)
    pub fn into_element(self) -> Element {
        self.render(&TableState::new())
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_creation() {
        let cell = Cell::new("Test");
        assert_eq!(cell.content.spans[0].content, "Test");
    }

    #[test]
    fn test_row_creation() {
        let row = Row::new(vec!["A", "B", "C"]);
        assert_eq!(row.cells.len(), 3);
    }

    #[test]
    fn test_table_creation() {
        let table = Table::new()
            .header(Row::new(vec!["Name", "Age", "City"]))
            .rows(vec![
                Row::new(vec!["Alice", "30", "NYC"]),
                Row::new(vec!["Bob", "25", "LA"]),
            ]);

        assert_eq!(table.len(), 2);
        assert!(table.header.is_some());
    }

    #[test]
    fn test_table_state() {
        let mut state = TableState::new();

        state.select_next(5);
        assert_eq!(state.selected, Some(0));

        state.select_next(5);
        assert_eq!(state.selected, Some(1));

        state.select_previous(5);
        assert_eq!(state.selected, Some(0));
    }
}
