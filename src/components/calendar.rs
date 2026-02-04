//! Calendar component for date selection
//!
//! Displays a monthly calendar with navigation and date selection.

use crate::components::{Box as TinkBox, Text};
use crate::core::{AlignItems, Color, Element, FlexDirection, JustifyContent};

/// Calendar component
#[derive(Debug, Clone)]
pub struct Calendar {
    /// Currently displayed year
    year: i32,
    /// Currently displayed month (1-12)
    month: u32,
    /// Selected date (day of month)
    selected_day: Option<u32>,
    /// Highlighted dates
    highlighted: Vec<u32>,
    /// First day of week (0 = Sunday, 1 = Monday)
    first_day_of_week: u8,
    /// Show week numbers
    show_week_numbers: bool,
    /// Header color
    header_color: Color,
    /// Selected color
    selected_color: Color,
    /// Today color
    today_color: Color,
    /// Highlighted color
    highlighted_color: Color,
    /// Today's date (year, month, day)
    today: Option<(i32, u32, u32)>,
    /// Key for reconciliation
    key: Option<String>,
}

impl Calendar {
    /// Create a new calendar for the current month
    pub fn new(year: i32, month: u32) -> Self {
        Self {
            year,
            month: month.clamp(1, 12),
            selected_day: None,
            highlighted: Vec::new(),
            first_day_of_week: 0, // Sunday
            show_week_numbers: false,
            header_color: Color::Cyan,
            selected_color: Color::Green,
            today_color: Color::Yellow,
            highlighted_color: Color::Magenta,
            today: None,
            key: None,
        }
    }

    /// Set selected day
    pub fn selected(mut self, day: u32) -> Self {
        self.selected_day = Some(day);
        self
    }

    /// Set highlighted dates
    pub fn highlighted(mut self, days: Vec<u32>) -> Self {
        self.highlighted = days;
        self
    }

    /// Set first day of week (0 = Sunday, 1 = Monday)
    pub fn first_day_of_week(mut self, day: u8) -> Self {
        self.first_day_of_week = day % 7;
        self
    }

    /// Start week on Monday
    pub fn monday_first(mut self) -> Self {
        self.first_day_of_week = 1;
        self
    }

    /// Show week numbers
    pub fn show_week_numbers(mut self, show: bool) -> Self {
        self.show_week_numbers = show;
        self
    }

    /// Set header color
    pub fn header_color(mut self, color: Color) -> Self {
        self.header_color = color;
        self
    }

    /// Set selected color
    pub fn selected_color(mut self, color: Color) -> Self {
        self.selected_color = color;
        self
    }

    /// Set today color
    pub fn today_color(mut self, color: Color) -> Self {
        self.today_color = color;
        self
    }

    /// Set highlighted color
    pub fn highlighted_color(mut self, color: Color) -> Self {
        self.highlighted_color = color;
        self
    }

    /// Set today's date for highlighting
    pub fn today(mut self, year: i32, month: u32, day: u32) -> Self {
        self.today = Some((year, month, day));
        self
    }

    /// Set key
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Convert to element
    pub fn into_element(self) -> Element {
        let mut rows = Vec::new();

        // Month/Year header
        let month_name = month_name(self.month);
        let header = Text::new(format!("{} {}", month_name, self.year))
            .color(self.header_color)
            .bold();
        rows.push(
            TinkBox::new()
                .justify_content(JustifyContent::Center)
                .child(header.into_element())
                .into_element(),
        );

        // Day headers
        let day_headers = self.build_day_headers();
        rows.push(day_headers);

        // Calendar grid
        let weeks = self.build_weeks();
        for week in weeks {
            rows.push(week);
        }

        let mut container = TinkBox::new()
            .flex_direction(FlexDirection::Column)
            .gap(0.0)
            .children(rows);

        if let Some(key) = self.key {
            container = container.key(key);
        }

        container.into_element()
    }

    fn build_day_headers(&self) -> Element {
        let days = if self.first_day_of_week == 1 {
            ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]
        } else {
            ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"]
        };

        let mut children = Vec::new();

        if self.show_week_numbers {
            children.push(
                TinkBox::new()
                    .width(3)
                    .child(Text::new("Wk").dim().into_element())
                    .into_element(),
            );
        }

        for day in days {
            children.push(
                TinkBox::new()
                    .width(3)
                    .justify_content(JustifyContent::Center)
                    .child(Text::new(day).dim().into_element())
                    .into_element(),
            );
        }

        TinkBox::new()
            .flex_direction(FlexDirection::Row)
            .children(children)
            .into_element()
    }

    fn build_weeks(&self) -> Vec<Element> {
        let days_in_month = days_in_month(self.year, self.month);
        let first_day = day_of_week(self.year, self.month, 1);

        // Adjust for first day of week setting
        let offset = (first_day + 7 - self.first_day_of_week as u32) % 7;

        let mut weeks = Vec::new();
        let mut current_day = 1u32;
        let mut week_num = week_number(self.year, self.month, 1);

        // First week (may have empty cells at start)
        let mut first_week = Vec::new();

        if self.show_week_numbers {
            first_week.push(
                TinkBox::new()
                    .width(3)
                    .child(Text::new(format!("{:2}", week_num)).dim().into_element())
                    .into_element(),
            );
        }

        for i in 0..7 {
            if i < offset {
                first_week.push(
                    TinkBox::new()
                        .width(3)
                        .child(Text::new("  ").into_element())
                        .into_element(),
                );
            } else {
                first_week.push(self.build_day_cell(current_day));
                current_day += 1;
            }
        }

        weeks.push(
            TinkBox::new()
                .flex_direction(FlexDirection::Row)
                .children(first_week)
                .into_element(),
        );

        // Remaining weeks
        while current_day <= days_in_month {
            week_num += 1;
            let mut week = Vec::new();

            if self.show_week_numbers {
                week.push(
                    TinkBox::new()
                        .width(3)
                        .child(Text::new(format!("{:2}", week_num % 53)).dim().into_element())
                        .into_element(),
                );
            }

            for _ in 0..7 {
                if current_day <= days_in_month {
                    week.push(self.build_day_cell(current_day));
                    current_day += 1;
                } else {
                    week.push(
                        TinkBox::new()
                            .width(3)
                            .child(Text::new("  ").into_element())
                            .into_element(),
                    );
                }
            }

            weeks.push(
                TinkBox::new()
                    .flex_direction(FlexDirection::Row)
                    .children(week)
                    .into_element(),
            );
        }

        weeks
    }

    fn build_day_cell(&self, day: u32) -> Element {
        let is_selected = self.selected_day == Some(day);
        let is_today = self
            .today
            .map(|(y, m, d)| y == self.year && m == self.month && d == day)
            .unwrap_or(false);
        let is_highlighted = self.highlighted.contains(&day);

        let text = format!("{:2}", day);
        let mut text_elem = Text::new(text);

        if is_selected {
            text_elem = text_elem.color(self.selected_color).bold();
        } else if is_today {
            text_elem = text_elem.color(self.today_color);
        } else if is_highlighted {
            text_elem = text_elem.color(self.highlighted_color);
        }

        TinkBox::new()
            .width(3)
            .justify_content(JustifyContent::Center)
            .align_items(AlignItems::Center)
            .child(text_elem.into_element())
            .into_element()
    }
}

/// Get month name
fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}

/// Get number of days in a month
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Check if year is a leap year
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Get day of week (0 = Sunday, 1 = Monday, ..., 6 = Saturday)
/// Using Zeller's congruence
fn day_of_week(year: i32, month: u32, day: u32) -> u32 {
    let (y, m) = if month < 3 {
        (year - 1, month + 12)
    } else {
        (year, month)
    };

    let q = day as i32;
    let m = m as i32;
    let k = y % 100;
    let j = y / 100;

    let h = (q + (13 * (m + 1)) / 5 + k + k / 4 + j / 4 - 2 * j) % 7;
    ((h + 6) % 7) as u32 // Convert to 0 = Sunday
}

/// Get ISO week number
fn week_number(year: i32, month: u32, day: u32) -> u32 {
    // Simplified week number calculation
    let day_of_year = day_of_year(year, month, day);
    let first_day = day_of_week(year, 1, 1);
    let offset = (7 - first_day) % 7;

    if day_of_year <= offset {
        // Last week of previous year
        week_number(year - 1, 12, 31)
    } else {
        ((day_of_year - offset - 1) / 7 + 1).min(52)
    }
}

/// Get day of year (1-366)
fn day_of_year(year: i32, month: u32, day: u32) -> u32 {
    let mut total = day;
    for m in 1..month {
        total += days_in_month(year, m);
    }
    total
}

impl Default for Calendar {
    fn default() -> Self {
        Self::new(2024, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calendar_creation() {
        let cal = Calendar::new(2024, 6);
        assert_eq!(cal.year, 2024);
        assert_eq!(cal.month, 6);
    }

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(2024, 1), 31);
        assert_eq!(days_in_month(2024, 2), 29); // Leap year
        assert_eq!(days_in_month(2023, 2), 28);
        assert_eq!(days_in_month(2024, 4), 30);
    }

    #[test]
    fn test_leap_year() {
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
    }

    #[test]
    fn test_day_of_week() {
        // January 1, 2024 was a Monday
        assert_eq!(day_of_week(2024, 1, 1), 1);
        // July 4, 2024 was a Thursday
        assert_eq!(day_of_week(2024, 7, 4), 4);
    }

    #[test]
    fn test_calendar_with_selection() {
        let cal = Calendar::new(2024, 6)
            .selected(15)
            .highlighted(vec![1, 10, 20])
            .today(2024, 6, 3);

        assert_eq!(cal.selected_day, Some(15));
        assert_eq!(cal.highlighted, vec![1, 10, 20]);
    }

    #[test]
    fn test_calendar_monday_first() {
        let cal = Calendar::new(2024, 6).monday_first();
        assert_eq!(cal.first_day_of_week, 1);
    }

    #[test]
    fn test_calendar_into_element() {
        let cal = Calendar::new(2024, 6).selected(15);
        let _ = cal.into_element();
    }
}
