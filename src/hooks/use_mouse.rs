//! Mouse input handling hook

use crossterm::event::{MouseButton as CrosstermMouseButton, MouseEvent, MouseEventKind};

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
            ctrl: event
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL),
            shift: event
                .modifiers
                .contains(crossterm::event::KeyModifiers::SHIFT),
            alt: event
                .modifiers
                .contains(crossterm::event::KeyModifiers::ALT),
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

/// Register a mouse handler (requires RuntimeContext)
pub fn register_mouse_handler<F>(handler: F)
where
    F: Fn(&Mouse) + 'static,
{
    if let Some(ctx) = crate::runtime::current_runtime() {
        ctx.borrow_mut().register_mouse_handler(handler);
    }
}

/// Clear all mouse handlers (no-op, clearing is handled by RuntimeContext::prepare_render)
pub fn clear_mouse_handlers() {}

/// Dispatch mouse event to all handlers
pub fn dispatch_mouse_event(event: &MouseEvent) {
    let mouse = Mouse::from_event(event);

    if let Some(ctx) = crate::runtime::current_runtime() {
        ctx.borrow().dispatch_mouse(&mouse);
    }
}

/// Check if mouse mode should be enabled
pub fn is_mouse_enabled() -> bool {
    if let Some(ctx) = crate::runtime::current_runtime() {
        ctx.borrow().is_mouse_enabled()
    } else {
        false
    }
}

/// Set mouse enabled state
pub fn set_mouse_enabled(enabled: bool) {
    if let Some(ctx) = crate::runtime::current_runtime() {
        ctx.borrow_mut().set_mouse_enabled(enabled);
    }
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
    // Reserve a hook slot so use_mouse obeys hook ordering constraints.
    if let Some(ctx) = crate::hooks::context::current_context() {
        ctx.borrow_mut().use_hook(|| ());
    }
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
    fn test_mouse_enabled_default() {
        // Without RuntimeContext, mouse is disabled by default
        crate::runtime::set_current_runtime(None);
        assert!(!is_mouse_enabled());
    }

    #[test]
    fn test_mouse_with_runtime() {
        use crate::runtime::{RuntimeContext, with_runtime};
        use std::cell::RefCell;
        use std::rc::Rc;

        let ctx = Rc::new(RefCell::new(RuntimeContext::new()));
        let clicked = Rc::new(RefCell::new(false));
        let clicked_clone = clicked.clone();

        with_runtime(ctx.clone(), || {
            use_mouse(move |mouse| {
                if mouse.is_left_click() {
                    *clicked_clone.borrow_mut() = true;
                }
            });
        });

        // Dispatch within the context
        let mouse = Mouse {
            x: 10,
            y: 5,
            action: MouseAction::Press(MouseButton::Left),
            ctrl: false,
            shift: false,
            alt: false,
        };
        ctx.borrow().dispatch_mouse(&mouse);
        assert!(*clicked.borrow());
    }

    #[test]
    #[should_panic(expected = "Hook order violation")]
    fn test_use_mouse_participates_in_hook_order() {
        use crate::hooks::context::{HookContext, with_hooks};
        use crate::hooks::use_signal;
        use std::cell::RefCell;
        use std::rc::Rc;

        let ctx = Rc::new(RefCell::new(HookContext::new()));

        with_hooks(ctx.clone(), || {
            use_mouse(|_| {});
            let _state = use_signal(|| 1usize);
        });

        with_hooks(ctx, || {
            let _state = use_signal(|| 1usize);
        });
    }
}
