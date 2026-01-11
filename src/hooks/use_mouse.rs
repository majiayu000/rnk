//! Mouse input handling hook

use crossterm::event::{MouseEvent, MouseEventKind, MouseButton as CrosstermMouseButton};
use std::cell::RefCell;
use std::rc::Rc;

/// Mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl From<CrosstermMouseButton> for MouseButton {
    fn from(btn: CrosstermMouseButton) -> Self {
        match btn {
            CrosstermMouseButton::Left => MouseButton::Left,
            CrosstermMouseButton::Right => MouseButton::Right,
            CrosstermMouseButton::Middle => MouseButton::Middle,
        }
    }
}

/// Mouse action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseAction {
    /// Mouse button pressed
    Press(MouseButton),
    /// Mouse button released
    Release(MouseButton),
    /// Mouse moved (with button held)
    Drag(MouseButton),
    /// Mouse moved (no button)
    Move,
    /// Scroll wheel up
    ScrollUp,
    /// Scroll wheel down
    ScrollDown,
    /// Scroll wheel left
    ScrollLeft,
    /// Scroll wheel right
    ScrollRight,
}

/// Mouse event information
#[derive(Debug, Clone)]
pub struct Mouse {
    /// X coordinate (column)
    pub x: u16,
    /// Y coordinate (row)
    pub y: u16,
    /// The action that occurred
    pub action: MouseAction,
    /// Ctrl key was held
    pub ctrl: bool,
    /// Shift key was held
    pub shift: bool,
    /// Alt key was held
    pub alt: bool,
}

impl Mouse {
    /// Create Mouse info from a crossterm MouseEvent
    pub fn from_event(event: &MouseEvent) -> Self {
        let action = match event.kind {
            MouseEventKind::Down(btn) => MouseAction::Press(btn.into()),
            MouseEventKind::Up(btn) => MouseAction::Release(btn.into()),
            MouseEventKind::Drag(btn) => MouseAction::Drag(btn.into()),
            MouseEventKind::Moved => MouseAction::Move,
            MouseEventKind::ScrollUp => MouseAction::ScrollUp,
            MouseEventKind::ScrollDown => MouseAction::ScrollDown,
            MouseEventKind::ScrollLeft => MouseAction::ScrollLeft,
            MouseEventKind::ScrollRight => MouseAction::ScrollRight,
        };

        Self {
            x: event.column,
            y: event.row,
            action,
            ctrl: event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL),
            shift: event.modifiers.contains(crossterm::event::KeyModifiers::SHIFT),
            alt: event.modifiers.contains(crossterm::event::KeyModifiers::ALT),
        }
    }

    /// Check if this is a click event (press)
    pub fn is_click(&self) -> bool {
        matches!(self.action, MouseAction::Press(_))
    }

    /// Check if this is a left click
    pub fn is_left_click(&self) -> bool {
        matches!(self.action, MouseAction::Press(MouseButton::Left))
    }

    /// Check if this is a right click
    pub fn is_right_click(&self) -> bool {
        matches!(self.action, MouseAction::Press(MouseButton::Right))
    }

    /// Check if this is a scroll event
    pub fn is_scroll(&self) -> bool {
        matches!(
            self.action,
            MouseAction::ScrollUp
                | MouseAction::ScrollDown
                | MouseAction::ScrollLeft
                | MouseAction::ScrollRight
        )
    }

    /// Get scroll delta (-1 for up/left, 1 for down/right, 0 for no scroll)
    pub fn scroll_delta(&self) -> (i8, i8) {
        match self.action {
            MouseAction::ScrollUp => (0, -1),
            MouseAction::ScrollDown => (0, 1),
            MouseAction::ScrollLeft => (-1, 0),
            MouseAction::ScrollRight => (1, 0),
            _ => (0, 0),
        }
    }
}

/// Mouse handler type
pub type MouseHandler = Box<dyn Fn(&Mouse)>;

/// Internal mouse handler type (reference-counted for storage)
type MouseHandlerRc = Rc<dyn Fn(&Mouse)>;

thread_local! {
    static MOUSE_HANDLERS: RefCell<Vec<MouseHandlerRc>> = RefCell::new(Vec::new());
    static MOUSE_ENABLED: RefCell<bool> = const { RefCell::new(false) };
}

/// Register a mouse handler
pub fn register_mouse_handler<F>(handler: F)
where
    F: Fn(&Mouse) + 'static,
{
    MOUSE_HANDLERS.with(|handlers| {
        handlers.borrow_mut().push(Rc::new(handler));
    });
}

/// Clear all mouse handlers
pub fn clear_mouse_handlers() {
    MOUSE_HANDLERS.with(|handlers| {
        handlers.borrow_mut().clear();
    });
}

/// Dispatch mouse event to all handlers
pub fn dispatch_mouse_event(event: &MouseEvent) {
    let mouse = Mouse::from_event(event);
    MOUSE_HANDLERS.with(|handlers| {
        for handler in handlers.borrow().iter() {
            handler(&mouse);
        }
    });
}

/// Check if mouse mode should be enabled
pub fn is_mouse_enabled() -> bool {
    MOUSE_ENABLED.with(|enabled| *enabled.borrow())
}

/// Set mouse enabled state
pub fn set_mouse_enabled(enabled: bool) {
    MOUSE_ENABLED.with(|e| *e.borrow_mut() = enabled);
}

/// Hook to handle mouse events
///
/// # Example
///
/// ```ignore
/// use_mouse(|mouse| {
///     if mouse.is_left_click() {
///         println!("Clicked at ({}, {})", mouse.x, mouse.y);
///     }
///     if mouse.is_scroll() {
///         let (dx, dy) = mouse.scroll_delta();
///         println!("Scrolled: dx={}, dy={}", dx, dy);
///     }
/// });
/// ```
pub fn use_mouse<F>(handler: F)
where
    F: Fn(&Mouse) + 'static,
{
    // Enable mouse mode when use_mouse is called
    set_mouse_enabled(true);
    register_mouse_handler(handler);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_action_from_event() {
        // Just verify the types compile
        let _left = MouseButton::Left;
        let _right = MouseButton::Right;
        let _action = MouseAction::Press(MouseButton::Left);
    }

    #[test]
    fn test_mouse_is_click() {
        let mouse = Mouse {
            x: 10,
            y: 5,
            action: MouseAction::Press(MouseButton::Left),
            ctrl: false,
            shift: false,
            alt: false,
        };
        assert!(mouse.is_click());
        assert!(mouse.is_left_click());
        assert!(!mouse.is_right_click());
    }

    #[test]
    fn test_mouse_scroll_delta() {
        let scroll_up = Mouse {
            x: 0,
            y: 0,
            action: MouseAction::ScrollUp,
            ctrl: false,
            shift: false,
            alt: false,
        };
        assert_eq!(scroll_up.scroll_delta(), (0, -1));

        let scroll_down = Mouse {
            x: 0,
            y: 0,
            action: MouseAction::ScrollDown,
            ctrl: false,
            shift: false,
            alt: false,
        };
        assert_eq!(scroll_down.scroll_delta(), (0, 1));
    }

    #[test]
    fn test_mouse_enabled() {
        set_mouse_enabled(false);
        assert!(!is_mouse_enabled());
        set_mouse_enabled(true);
        assert!(is_mouse_enabled());
        set_mouse_enabled(false); // Reset
    }
}
