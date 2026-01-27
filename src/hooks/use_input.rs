//! Input handling hook

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Key information for input handlers
#[derive(Debug, Clone, Default)]
pub struct Key {
    pub up_arrow: bool,
    pub down_arrow: bool,
    pub left_arrow: bool,
    pub right_arrow: bool,
    pub page_up: bool,
    pub page_down: bool,
    pub home: bool,
    pub end: bool,
    pub return_key: bool,
    pub escape: bool,
    pub tab: bool,
    pub backspace: bool,
    pub delete: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Key {
    /// Create Key info from a crossterm KeyEvent
    pub fn from_event(event: &KeyEvent) -> Self {
        let modifiers = event.modifiers;

        Self {
            up_arrow: event.code == KeyCode::Up,
            down_arrow: event.code == KeyCode::Down,
            left_arrow: event.code == KeyCode::Left,
            right_arrow: event.code == KeyCode::Right,
            page_up: event.code == KeyCode::PageUp,
            page_down: event.code == KeyCode::PageDown,
            home: event.code == KeyCode::Home,
            end: event.code == KeyCode::End,
            return_key: event.code == KeyCode::Enter,
            escape: event.code == KeyCode::Esc,
            tab: event.code == KeyCode::Tab,
            backspace: event.code == KeyCode::Backspace,
            delete: event.code == KeyCode::Delete,
            ctrl: modifiers.contains(KeyModifiers::CONTROL),
            shift: modifiers.contains(KeyModifiers::SHIFT),
            alt: modifiers.contains(KeyModifiers::ALT),
        }
    }

    /// Get the character input from a key event
    pub fn char_from_event(event: &KeyEvent) -> String {
        match event.code {
            KeyCode::Char(c) => {
                if event.modifiers.contains(KeyModifiers::CONTROL) {
                    // Return the character name for ctrl combinations
                    c.to_string()
                } else {
                    c.to_string()
                }
            }
            KeyCode::Enter => String::new(),
            KeyCode::Tab => String::new(),
            KeyCode::Backspace => String::new(),
            KeyCode::Delete => String::new(),
            KeyCode::Esc => String::new(),
            _ => String::new(),
        }
    }
}

/// Input handler type (boxed, for public use)
pub type InputHandler = Box<dyn Fn(&str, &Key)>;

/// Input handlers storage (global for the app)
use std::cell::RefCell;
use std::rc::Rc;

/// Internal input handler type (reference-counted for storage)
type InputHandlerRc = Rc<dyn Fn(&str, &Key)>;

thread_local! {
    static INPUT_HANDLERS: RefCell<Vec<InputHandlerRc>> = RefCell::new(Vec::new());
}

/// Register an input handler (legacy thread-local storage)
pub fn register_input_handler<F>(handler: F)
where
    F: Fn(&str, &Key) + 'static,
{
    // Try to use RuntimeContext first, fall back to thread-local
    if let Some(ctx) = crate::runtime::current_runtime() {
        ctx.borrow_mut().register_input_handler(handler);
    } else {
        INPUT_HANDLERS.with(|handlers| {
            handlers.borrow_mut().push(Rc::new(handler));
        });
    }
}

/// Clear all input handlers
pub fn clear_input_handlers() {
    INPUT_HANDLERS.with(|handlers| {
        handlers.borrow_mut().clear();
    });
}

/// Dispatch input to all handlers
pub fn dispatch_input(input: &str, key: &Key) {
    // Try RuntimeContext first, fall back to thread-local
    if let Some(ctx) = crate::runtime::current_runtime() {
        ctx.borrow().dispatch_input(input, key);
    } else {
        INPUT_HANDLERS.with(|handlers| {
            for handler in handlers.borrow().iter() {
                handler(input, key);
            }
        });
    }
}

/// Dispatch a key event
pub fn dispatch_key_event(event: &KeyEvent) {
    let key = Key::from_event(event);
    let input = Key::char_from_event(event);
    dispatch_input(&input, &key);
}

/// Hook to handle keyboard input
///
/// # Example
///
/// ```ignore
/// use_input(|input, key| {
///     if key.up_arrow {
///         // Handle up arrow
///     }
///     if input == "q" {
///         // Handle 'q' key
///     }
/// });
/// ```
pub fn use_input<F>(handler: F)
where
    F: Fn(&str, &Key) + 'static,
{
    register_input_handler(handler);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_from_event() {
        let event = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        let key = Key::from_event(&event);

        assert!(key.up_arrow);
        assert!(!key.down_arrow);
        assert!(!key.ctrl);
    }

    #[test]
    fn test_key_with_modifiers() {
        let event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let key = Key::from_event(&event);

        assert!(key.ctrl);
        assert!(!key.shift);
    }

    #[test]
    fn test_char_from_event() {
        let event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let input = Key::char_from_event(&event);
        assert_eq!(input, "a");

        let event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let input = Key::char_from_event(&event);
        assert_eq!(input, "");
    }

    #[test]
    fn test_dispatch_input_legacy() {
        use std::cell::RefCell;
        use std::rc::Rc;

        clear_input_handlers();

        let received = Rc::new(RefCell::new(String::new()));
        let received_clone = received.clone();

        // Use thread-local directly for this test
        INPUT_HANDLERS.with(|handlers| {
            handlers
                .borrow_mut()
                .push(Rc::new(move |input: &str, _key: &Key| {
                    *received_clone.borrow_mut() = input.to_string();
                }));
        });

        // Dispatch using thread-local
        INPUT_HANDLERS.with(|handlers| {
            for handler in handlers.borrow().iter() {
                handler("test", &Key::default());
            }
        });

        assert_eq!(*received.borrow(), "test");

        clear_input_handlers();
    }

    #[test]
    fn test_dispatch_input_with_runtime() {
        use crate::runtime::{RuntimeContext, with_runtime};
        use std::cell::RefCell;
        use std::rc::Rc;

        let ctx = Rc::new(RefCell::new(RuntimeContext::new()));
        let received = Rc::new(RefCell::new(String::new()));
        let received_clone = received.clone();

        with_runtime(ctx.clone(), || {
            use_input(move |input, _key| {
                *received_clone.borrow_mut() = input.to_string();
            });
        });

        // Dispatch within the context
        ctx.borrow().dispatch_input("hello", &Key::default());
        assert_eq!(*received.borrow(), "hello");
    }
}
