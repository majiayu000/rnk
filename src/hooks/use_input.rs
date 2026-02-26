//! Input handling hook

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MediaKeyCode};

/// Typed key code for pattern matching and robust key handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum KeyCodeKind {
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
    Insert,
    Enter,
    Escape,
    Tab,
    BackTab,
    Backspace,
    Delete,
    Char(char),
    Function(u8),
    Media(MediaKeyKind),
    #[default]
    Unknown,
}

/// Typed media key code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaKeyKind {
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Previous,
    VolumeUp,
    VolumeDown,
    VolumeMute,
}

impl MediaKeyKind {
    fn from_event_code(code: MediaKeyCode) -> Option<Self> {
        match code {
            MediaKeyCode::Play => Some(Self::Play),
            MediaKeyCode::Pause => Some(Self::Pause),
            MediaKeyCode::PlayPause => Some(Self::PlayPause),
            MediaKeyCode::Stop => Some(Self::Stop),
            MediaKeyCode::TrackNext => Some(Self::Next),
            MediaKeyCode::TrackPrevious => Some(Self::Previous),
            MediaKeyCode::RaiseVolume => Some(Self::VolumeUp),
            MediaKeyCode::LowerVolume => Some(Self::VolumeDown),
            MediaKeyCode::MuteVolume => Some(Self::VolumeMute),
            _ => None,
        }
    }
}

impl KeyCodeKind {
    fn from_event_code(code: KeyCode) -> Self {
        match code {
            KeyCode::Up => Self::Up,
            KeyCode::Down => Self::Down,
            KeyCode::Left => Self::Left,
            KeyCode::Right => Self::Right,
            KeyCode::PageUp => Self::PageUp,
            KeyCode::PageDown => Self::PageDown,
            KeyCode::Home => Self::Home,
            KeyCode::End => Self::End,
            KeyCode::Insert => Self::Insert,
            KeyCode::Enter => Self::Enter,
            KeyCode::Esc => Self::Escape,
            KeyCode::Tab => Self::Tab,
            KeyCode::BackTab => Self::BackTab,
            KeyCode::Backspace => Self::Backspace,
            KeyCode::Delete => Self::Delete,
            KeyCode::Char(c) => Self::Char(c),
            KeyCode::F(f) => Self::Function(f),
            KeyCode::Media(media) => MediaKeyKind::from_event_code(media)
                .map(Self::Media)
                .unwrap_or(Self::Unknown),
            _ => Self::Unknown,
        }
    }
}

/// Key information for input handlers
#[derive(Debug, Clone, Copy, Default)]
pub struct Key {
    /// Canonical key code for matching.
    pub code: KeyCodeKind,
    /// Character value for character keys.
    pub character: Option<char>,

    // Arrow keys
    pub up_arrow: bool,
    pub down_arrow: bool,
    pub left_arrow: bool,
    pub right_arrow: bool,

    // Navigation keys
    pub page_up: bool,
    pub page_down: bool,
    pub home: bool,
    pub end: bool,
    pub insert: bool,

    // Action keys
    pub return_key: bool,
    pub escape: bool,
    pub tab: bool,
    pub backspace: bool,
    pub delete: bool,
    pub space: bool,

    // Function keys (F1-F12)
    pub f1: bool,
    pub f2: bool,
    pub f3: bool,
    pub f4: bool,
    pub f5: bool,
    pub f6: bool,
    pub f7: bool,
    pub f8: bool,
    pub f9: bool,
    pub f10: bool,
    pub f11: bool,
    pub f12: bool,

    // Modifiers
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,

    // Media keys
    pub media_play: bool,
    pub media_pause: bool,
    pub media_play_pause: bool,
    pub media_stop: bool,
    pub media_next: bool,
    pub media_previous: bool,
    pub volume_up: bool,
    pub volume_down: bool,
    pub volume_mute: bool,
}

impl Key {
    /// Create Key info from a crossterm KeyEvent
    pub fn from_event(event: &KeyEvent) -> Self {
        let modifiers = event.modifiers;
        let code = KeyCodeKind::from_event_code(event.code);
        let character = match code {
            KeyCodeKind::Char(c) => Some(c),
            _ => None,
        };

        Self {
            code,
            character,
            // Arrow keys
            up_arrow: matches!(code, KeyCodeKind::Up),
            down_arrow: matches!(code, KeyCodeKind::Down),
            left_arrow: matches!(code, KeyCodeKind::Left),
            right_arrow: matches!(code, KeyCodeKind::Right),

            // Navigation keys
            page_up: matches!(code, KeyCodeKind::PageUp),
            page_down: matches!(code, KeyCodeKind::PageDown),
            home: matches!(code, KeyCodeKind::Home),
            end: matches!(code, KeyCodeKind::End),
            insert: matches!(code, KeyCodeKind::Insert),

            // Action keys
            return_key: matches!(code, KeyCodeKind::Enter),
            escape: matches!(code, KeyCodeKind::Escape),
            tab: matches!(code, KeyCodeKind::Tab | KeyCodeKind::BackTab),
            backspace: matches!(code, KeyCodeKind::Backspace),
            delete: matches!(code, KeyCodeKind::Delete),
            space: matches!(code, KeyCodeKind::Char(' ')),

            // Function keys
            f1: matches!(code, KeyCodeKind::Function(1)),
            f2: matches!(code, KeyCodeKind::Function(2)),
            f3: matches!(code, KeyCodeKind::Function(3)),
            f4: matches!(code, KeyCodeKind::Function(4)),
            f5: matches!(code, KeyCodeKind::Function(5)),
            f6: matches!(code, KeyCodeKind::Function(6)),
            f7: matches!(code, KeyCodeKind::Function(7)),
            f8: matches!(code, KeyCodeKind::Function(8)),
            f9: matches!(code, KeyCodeKind::Function(9)),
            f10: matches!(code, KeyCodeKind::Function(10)),
            f11: matches!(code, KeyCodeKind::Function(11)),
            f12: matches!(code, KeyCodeKind::Function(12)),

            // Modifiers
            ctrl: modifiers.contains(KeyModifiers::CONTROL),
            shift: modifiers.contains(KeyModifiers::SHIFT),
            alt: modifiers.contains(KeyModifiers::ALT),
            meta: modifiers.contains(KeyModifiers::SUPER),

            // Media keys
            media_play: matches!(code, KeyCodeKind::Media(MediaKeyKind::Play)),
            media_pause: matches!(code, KeyCodeKind::Media(MediaKeyKind::Pause)),
            media_play_pause: matches!(code, KeyCodeKind::Media(MediaKeyKind::PlayPause)),
            media_stop: matches!(code, KeyCodeKind::Media(MediaKeyKind::Stop)),
            media_next: matches!(code, KeyCodeKind::Media(MediaKeyKind::Next)),
            media_previous: matches!(code, KeyCodeKind::Media(MediaKeyKind::Previous)),
            volume_up: matches!(code, KeyCodeKind::Media(MediaKeyKind::VolumeUp)),
            volume_down: matches!(code, KeyCodeKind::Media(MediaKeyKind::VolumeDown)),
            volume_mute: matches!(code, KeyCodeKind::Media(MediaKeyKind::VolumeMute)),
        }
    }

    /// Return canonical key code.
    pub fn code(&self) -> KeyCodeKind {
        self.code
    }

    /// Match current key against a typed key code.
    pub fn is(&self, code: KeyCodeKind) -> bool {
        self.code == code
    }

    /// Check if current key is a specific character.
    pub fn is_char(&self, c: char) -> bool {
        matches!(self.code, KeyCodeKind::Char(ch) if ch == c)
    }

    /// Get the character input from a key event
    pub fn char_from_event(event: &KeyEvent) -> String {
        match KeyCodeKind::from_event_code(event.code) {
            KeyCodeKind::Char(c) => c.to_string(),
            _ => String::new(),
        }
    }
}

/// Input handler type (boxed, for public use)
pub type InputHandler = Box<dyn Fn(&str, &Key)>;

/// Register an input handler (requires RuntimeContext)
pub fn register_input_handler<F>(handler: F)
where
    F: Fn(&str, &Key) + 'static,
{
    if let Some(ctx) = crate::runtime::current_runtime() {
        ctx.borrow_mut().register_input_handler(handler);
    }
}

/// Clear all input handlers (no-op, clearing is handled by RuntimeContext::prepare_render)
pub fn clear_input_handlers() {}

/// Dispatch input to all handlers
pub fn dispatch_input(input: &str, key: &Key) {
    if let Some(ctx) = crate::runtime::current_runtime() {
        ctx.borrow().dispatch_input(input, key);
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
    // Reserve a hook slot so use_input follows the same ordering rules
    // as other hooks (catches conditional hook calls).
    if let Some(ctx) = crate::hooks::context::current_context() {
        ctx.borrow_mut().use_hook(|| ());
    }
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
        assert_eq!(key.code(), KeyCodeKind::Up);
        assert!(key.is(KeyCodeKind::Up));
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

        let key = Key::from_event(&event);
        assert!(key.is_char('a'));
        assert_eq!(key.character, Some('a'));
        assert_eq!(key.code(), KeyCodeKind::Char('a'));

        let event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let input = Key::char_from_event(&event);
        assert_eq!(input, "");
    }

    #[test]
    fn test_key_code_media_variant() {
        let event = KeyEvent::new(
            KeyCode::Media(crossterm::event::MediaKeyCode::Play),
            KeyModifiers::NONE,
        );
        let key = Key::from_event(&event);
        assert_eq!(key.code(), KeyCodeKind::Media(MediaKeyKind::Play));
        assert!(key.media_play);
    }

    #[test]
    fn test_function_keys() {
        let event = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        let key = Key::from_event(&event);
        assert!(key.f1);
        assert!(!key.f2);

        let event = KeyEvent::new(KeyCode::F(12), KeyModifiers::NONE);
        let key = Key::from_event(&event);
        assert!(key.f12);
        assert!(!key.f11);
    }

    #[test]
    fn test_insert_key() {
        let event = KeyEvent::new(KeyCode::Insert, KeyModifiers::NONE);
        let key = Key::from_event(&event);
        assert!(key.insert);
    }

    #[test]
    fn test_space_key() {
        let event = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        let key = Key::from_event(&event);
        assert!(key.space);
    }

    #[test]
    fn test_meta_modifier() {
        let event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::SUPER);
        let key = Key::from_event(&event);
        assert!(key.meta);
        assert!(!key.ctrl);
    }

    #[test]
    fn test_combined_modifiers() {
        let event = KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        let key = Key::from_event(&event);
        assert!(key.ctrl);
        assert!(key.shift);
        assert!(!key.alt);
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

    #[test]
    fn test_dispatch_input_without_runtime_is_noop() {
        // Without RuntimeContext, dispatch is a no-op and should not panic
        crate::runtime::set_current_runtime(None);
        dispatch_input("test", &Key::default());
    }

    #[test]
    #[should_panic(expected = "Hook order violation")]
    fn test_use_input_participates_in_hook_order() {
        use crate::hooks::context::{HookContext, with_hooks};
        use crate::hooks::use_signal;
        use std::cell::RefCell;
        use std::rc::Rc;

        let ctx = Rc::new(RefCell::new(HookContext::new()));

        // Render 1: use_input occupies slot 0, use_signal occupies slot 1.
        with_hooks(ctx.clone(), || {
            use_input(|_, _| {});
            let _state = use_signal(|| 1usize);
        });

        // Render 2: use_input omitted, use_signal now attempts slot 0 -> panic.
        with_hooks(ctx, || {
            let _state = use_signal(|| 1usize);
        });
    }
}
