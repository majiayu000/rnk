//! Stepper component for step-by-step wizards
//!
//! Provides a stepper UI for multi-step processes.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//! use rnk::components::Stepper;
//!
//! fn app() -> Element {
//!     let steps = vec![
//!         Step::new("Account", "Create your account"),
//!         Step::new("Profile", "Set up your profile"),
//!         Step::new("Confirm", "Review and confirm"),
//!     ];
//!
//!     Stepper::new(steps)
//!         .current(1)
//!         .into_element()
//! }
//! ```

use crate::components::{Box, Text};
use crate::core::{Color, Element, FlexDirection};

/// A step in the stepper
#[derive(Debug, Clone)]
pub struct Step {
    /// Step title
    pub title: String,
    /// Optional description
    pub description: Option<String>,
    /// Optional icon
    pub icon: Option<String>,
    /// Step status
    pub status: StepStatus,
}

/// Status of a step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StepStatus {
    /// Step is pending (not yet reached)
    #[default]
    Pending,
    /// Step is currently active
    Active,
    /// Step is completed
    Completed,
    /// Step has an error
    Error,
    /// Step is skipped
    Skipped,
}

impl StepStatus {
    /// Get the default icon for this status
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Pending => "○",
            Self::Active => "●",
            Self::Completed => "✓",
            Self::Error => "✗",
            Self::Skipped => "⊘",
        }
    }

    /// Get the default color for this status
    pub fn color(&self) -> Color {
        match self {
            Self::Pending => Color::BrightBlack,
            Self::Active => Color::Blue,
            Self::Completed => Color::Green,
            Self::Error => Color::Red,
            Self::Skipped => Color::Yellow,
        }
    }
}

impl Step {
    /// Create a new step
    pub fn new(title: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: Some(description.into()),
            icon: None,
            status: StepStatus::Pending,
        }
    }

    /// Create a step with just a title
    pub fn titled(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            icon: None,
            status: StepStatus::Pending,
        }
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set custom icon
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set status
    pub fn status(mut self, status: StepStatus) -> Self {
        self.status = status;
        self
    }

    /// Mark as completed
    pub fn completed(mut self) -> Self {
        self.status = StepStatus::Completed;
        self
    }

    /// Mark as active
    pub fn active(mut self) -> Self {
        self.status = StepStatus::Active;
        self
    }

    /// Mark as error
    pub fn error(mut self) -> Self {
        self.status = StepStatus::Error;
        self
    }

    /// Get the display icon
    pub fn display_icon(&self) -> &str {
        self.icon.as_deref().unwrap_or_else(|| self.status.icon())
    }
}

/// Stepper orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StepperOrientation {
    /// Horizontal layout
    #[default]
    Horizontal,
    /// Vertical layout
    Vertical,
}

/// Stepper style
#[derive(Debug, Clone)]
pub struct StepperStyle {
    /// Orientation
    pub orientation: StepperOrientation,
    /// Show step numbers
    pub show_numbers: bool,
    /// Show descriptions
    pub show_descriptions: bool,
    /// Connector character
    pub connector: String,
    /// Active color
    pub active_color: Color,
    /// Completed color
    pub completed_color: Color,
    /// Pending color
    pub pending_color: Color,
    /// Error color
    pub error_color: Color,
    /// Title color
    pub title_color: Color,
    /// Description color
    pub description_color: Color,
}

impl Default for StepperStyle {
    fn default() -> Self {
        Self {
            orientation: StepperOrientation::Horizontal,
            show_numbers: false,
            show_descriptions: true,
            connector: "───".to_string(),
            active_color: Color::Blue,
            completed_color: Color::Green,
            pending_color: Color::BrightBlack,
            error_color: Color::Red,
            title_color: Color::White,
            description_color: Color::BrightBlack,
        }
    }
}

impl StepperStyle {
    /// Create a new style
    pub fn new() -> Self {
        Self::default()
    }

    /// Set orientation
    pub fn orientation(mut self, orientation: StepperOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Set horizontal orientation
    pub fn horizontal(mut self) -> Self {
        self.orientation = StepperOrientation::Horizontal;
        self
    }

    /// Set vertical orientation
    pub fn vertical(mut self) -> Self {
        self.orientation = StepperOrientation::Vertical;
        self
    }

    /// Show step numbers
    pub fn show_numbers(mut self, show: bool) -> Self {
        self.show_numbers = show;
        self
    }

    /// Show descriptions
    pub fn show_descriptions(mut self, show: bool) -> Self {
        self.show_descriptions = show;
        self
    }

    /// Set connector string
    pub fn connector(mut self, connector: impl Into<String>) -> Self {
        self.connector = connector.into();
        self
    }

    /// Minimal style (no descriptions, compact)
    pub fn minimal() -> Self {
        Self::new()
            .show_descriptions(false)
            .connector("─".to_string())
    }

    /// Numbered style
    pub fn numbered() -> Self {
        Self::new().show_numbers(true)
    }
}

/// Stepper component
#[derive(Debug)]
pub struct Stepper {
    /// Steps
    steps: Vec<Step>,
    /// Current step index
    current: usize,
    /// Style
    style: StepperStyle,
}

impl Stepper {
    /// Create a new stepper
    pub fn new(steps: Vec<Step>) -> Self {
        Self {
            steps,
            current: 0,
            style: StepperStyle::default(),
        }
    }

    /// Set the current step
    pub fn current(mut self, index: usize) -> Self {
        self.current = index;
        self
    }

    /// Set the style
    pub fn style(mut self, style: StepperStyle) -> Self {
        self.style = style;
        self
    }

    /// Get the current step
    pub fn current_step(&self) -> Option<&Step> {
        self.steps.get(self.current)
    }

    /// Check if on first step
    pub fn is_first(&self) -> bool {
        self.current == 0
    }

    /// Check if on last step
    pub fn is_last(&self) -> bool {
        self.current >= self.steps.len().saturating_sub(1)
    }

    /// Get progress percentage
    pub fn progress(&self) -> f32 {
        if self.steps.is_empty() {
            return 0.0;
        }
        (self.current as f32 / (self.steps.len() - 1) as f32) * 100.0
    }

    /// Render a single step
    fn render_step(&self, step: &Step, index: usize) -> Element {
        let is_current = index == self.current;
        let is_completed = index < self.current || step.status == StepStatus::Completed;

        let status = if is_current {
            StepStatus::Active
        } else if is_completed {
            StepStatus::Completed
        } else {
            step.status
        };

        let icon_color = status.color();
        let icon = if self.style.show_numbers {
            format!("{}", index + 1)
        } else {
            step.icon
                .clone()
                .unwrap_or_else(|| status.icon().to_string())
        };

        let mut container = Box::new().flex_direction(FlexDirection::Column);

        // Icon/number
        container = container.child(Text::new(&icon).color(icon_color).into_element());

        // Title
        let title_color = if is_current {
            self.style.active_color
        } else if is_completed {
            self.style.completed_color
        } else {
            self.style.title_color
        };

        container = container.child(Text::new(&step.title).color(title_color).into_element());

        // Description
        if self.style.show_descriptions {
            if let Some(desc) = &step.description {
                container = container.child(
                    Text::new(desc)
                        .color(self.style.description_color)
                        .into_element(),
                );
            }
        }

        container.into_element()
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let direction = match self.style.orientation {
            StepperOrientation::Horizontal => FlexDirection::Row,
            StepperOrientation::Vertical => FlexDirection::Column,
        };

        let mut container = Box::new().flex_direction(direction);

        for (i, step) in self.steps.iter().enumerate() {
            // Add step
            container = container.child(self.render_step(step, i));

            // Add connector (except after last step)
            if i < self.steps.len() - 1 {
                let connector_color = if i < self.current {
                    self.style.completed_color
                } else {
                    self.style.pending_color
                };

                container = container.child(
                    Text::new(&self.style.connector)
                        .color(connector_color)
                        .into_element(),
                );
            }
        }

        container.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_creation() {
        let step = Step::new("Title", "Description");
        assert_eq!(step.title, "Title");
        assert_eq!(step.description, Some("Description".to_string()));
    }

    #[test]
    fn test_step_titled() {
        let step = Step::titled("Just Title");
        assert_eq!(step.title, "Just Title");
        assert!(step.description.is_none());
    }

    #[test]
    fn test_step_builder() {
        let step = Step::titled("Test")
            .description("Desc")
            .icon("★")
            .status(StepStatus::Completed);

        assert_eq!(step.description, Some("Desc".to_string()));
        assert_eq!(step.icon, Some("★".to_string()));
        assert_eq!(step.status, StepStatus::Completed);
    }

    #[test]
    fn test_step_status_icon() {
        assert_eq!(StepStatus::Pending.icon(), "○");
        assert_eq!(StepStatus::Active.icon(), "●");
        assert_eq!(StepStatus::Completed.icon(), "✓");
        assert_eq!(StepStatus::Error.icon(), "✗");
    }

    #[test]
    fn test_stepper_creation() {
        let steps = vec![Step::titled("Step 1"), Step::titled("Step 2")];
        let stepper = Stepper::new(steps);
        assert_eq!(stepper.steps.len(), 2);
        assert_eq!(stepper.current, 0);
    }

    #[test]
    fn test_stepper_current() {
        let steps = vec![
            Step::titled("Step 1"),
            Step::titled("Step 2"),
            Step::titled("Step 3"),
        ];
        let stepper = Stepper::new(steps).current(1);
        assert_eq!(stepper.current, 1);
    }

    #[test]
    fn test_stepper_is_first_last() {
        let steps = vec![Step::titled("Step 1"), Step::titled("Step 2")];

        let stepper = Stepper::new(steps.clone()).current(0);
        assert!(stepper.is_first());
        assert!(!stepper.is_last());

        let stepper = Stepper::new(steps).current(1);
        assert!(!stepper.is_first());
        assert!(stepper.is_last());
    }

    #[test]
    fn test_stepper_progress() {
        let steps = vec![
            Step::titled("Step 1"),
            Step::titled("Step 2"),
            Step::titled("Step 3"),
        ];

        let stepper = Stepper::new(steps.clone()).current(0);
        assert_eq!(stepper.progress(), 0.0);

        let stepper = Stepper::new(steps.clone()).current(1);
        assert_eq!(stepper.progress(), 50.0);

        let stepper = Stepper::new(steps).current(2);
        assert_eq!(stepper.progress(), 100.0);
    }

    #[test]
    fn test_stepper_style() {
        let style = StepperStyle::new()
            .vertical()
            .show_numbers(true)
            .connector("│".to_string());

        assert_eq!(style.orientation, StepperOrientation::Vertical);
        assert!(style.show_numbers);
        assert_eq!(style.connector, "│");
    }

    #[test]
    fn test_stepper_into_element() {
        let steps = vec![Step::titled("Step 1"), Step::titled("Step 2")];
        let stepper = Stepper::new(steps).current(0);
        let _ = stepper.into_element();
    }
}
