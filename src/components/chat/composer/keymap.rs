//! Key bindings for the composer.
//!
//! Separate from `TextAreaKeyMap` because the two disagree on the key that
//! matters most: in a textarea Enter inserts a newline, in a chat composer it
//! sends. Extending the textarea's public keymap would force that disagreement
//! onto its existing users.
//!
//! Fields are private so a binding set cannot be left in a state the matcher
//! does not expect; use the builder methods.

use crate::components::keymap::{KeyBinding, KeyType, Modifiers};
use crate::hooks::Key;

/// What a keystroke asks the composer to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposerAction {
    /// Send the draft.
    Submit,
    /// Insert a line break into the draft.
    InsertNewline,
    /// Abandon the interaction without discarding the draft.
    Cancel,
    /// Move one grapheme cluster left.
    MoveLeft,
    /// Move one grapheme cluster right.
    MoveRight,
    /// Move to the previous line.
    MoveUp,
    /// Move to the next line.
    MoveDown,
    /// Move to the start of the line.
    MoveLineStart,
    /// Move to the end of the line.
    MoveLineEnd,
    /// Move to the previous word.
    MoveWordLeft,
    /// Move to the next word.
    MoveWordRight,
    /// Delete the cluster before the cursor.
    DeleteBefore,
    /// Delete the cluster after the cursor.
    DeleteAfter,
    /// Delete the word before the cursor.
    DeleteWordBefore,
    /// Empty the draft.
    Clear,
    /// Select the complete draft so the next edit replaces it atomically.
    SelectAll,
}

/// Composer key bindings.
#[derive(Debug, Clone)]
pub struct ChatComposerKeyMap {
    submit: Vec<KeyBinding>,
    newline: Vec<KeyBinding>,
    cancel: Vec<KeyBinding>,
    clear: Vec<KeyBinding>,
    select_all: Vec<KeyBinding>,
}

impl Default for ChatComposerKeyMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatComposerKeyMap {
    /// The default bindings: Enter sends, Shift+Enter and Alt+Enter break.
    ///
    /// Two newline bindings because terminals disagree about Shift+Enter —
    /// several report it as a bare Enter, which would silently send instead of
    /// breaking the line.
    pub fn new() -> Self {
        Self {
            submit: vec![KeyBinding::new(KeyType::Enter, Modifiers::NONE)],
            newline: vec![
                KeyBinding::new(KeyType::Enter, Modifiers::SHIFT),
                KeyBinding::new(KeyType::Enter, Modifiers::ALT),
            ],
            cancel: vec![KeyBinding::new(KeyType::Escape, Modifiers::NONE)],
            clear: vec![KeyBinding::new(KeyType::Char('u'), Modifiers::CTRL)],
            select_all: vec![KeyBinding::new(KeyType::Char('a'), Modifiers::CTRL)],
        }
    }

    /// Replaces the submit bindings.
    pub fn submit(mut self, bindings: Vec<KeyBinding>) -> Self {
        self.submit = bindings;
        self
    }

    /// Replaces the newline bindings.
    pub fn newline(mut self, bindings: Vec<KeyBinding>) -> Self {
        self.newline = bindings;
        self
    }

    /// Replaces the cancel bindings.
    pub fn cancel(mut self, bindings: Vec<KeyBinding>) -> Self {
        self.cancel = bindings;
        self
    }

    /// Replaces the clear bindings.
    pub fn clear(mut self, bindings: Vec<KeyBinding>) -> Self {
        self.clear = bindings;
        self
    }

    /// Replaces the select-all bindings.
    pub fn select_all(mut self, bindings: Vec<KeyBinding>) -> Self {
        self.select_all = bindings;
        self
    }

    /// The action a keystroke maps to, if any.
    ///
    /// Newline is matched before submit: its bindings are the more specific
    /// ones (Enter *with* a modifier), and a plain-Enter submit binding would
    /// otherwise swallow Shift+Enter.
    pub fn action(&self, key: &Key) -> Option<ComposerAction> {
        if matches(&self.newline, key) {
            return Some(ComposerAction::InsertNewline);
        }
        if matches(&self.submit, key) {
            return Some(ComposerAction::Submit);
        }
        if matches(&self.cancel, key) {
            return Some(ComposerAction::Cancel);
        }
        if matches(&self.clear, key) {
            return Some(ComposerAction::Clear);
        }
        if matches(&self.select_all, key) {
            return Some(ComposerAction::SelectAll);
        }

        builtin_action(key)
    }
}

/// Editing and movement keys that are not configurable.
fn builtin_action(key: &Key) -> Option<ComposerAction> {
    let action = if key.left_arrow && (key.ctrl || key.alt) {
        ComposerAction::MoveWordLeft
    } else if key.right_arrow && (key.ctrl || key.alt) {
        ComposerAction::MoveWordRight
    } else if key.left_arrow {
        ComposerAction::MoveLeft
    } else if key.right_arrow {
        ComposerAction::MoveRight
    } else if key.up_arrow {
        ComposerAction::MoveUp
    } else if key.down_arrow {
        ComposerAction::MoveDown
    } else if key.home {
        ComposerAction::MoveLineStart
    } else if key.end {
        ComposerAction::MoveLineEnd
    } else if key.backspace && (key.ctrl || key.alt) {
        ComposerAction::DeleteWordBefore
    } else if key.backspace {
        ComposerAction::DeleteBefore
    } else if key.delete {
        ComposerAction::DeleteAfter
    } else {
        return None;
    };

    Some(action)
}

fn matches(bindings: &[KeyBinding], key: &Key) -> bool {
    bindings.iter().any(|binding| binding_matches(binding, key))
}

fn binding_matches(binding: &KeyBinding, key: &Key) -> bool {
    let modifiers_match = binding.modifiers.ctrl == key.ctrl
        && binding.modifiers.shift == key.shift
        && binding.modifiers.alt == key.alt;

    if !modifiers_match {
        return false;
    }

    match &binding.key {
        KeyType::Enter => key.return_key,
        KeyType::Escape => key.escape,
        KeyType::Tab => key.tab,
        KeyType::Backspace => key.backspace,
        KeyType::Delete => key.delete,
        KeyType::Up => key.up_arrow,
        KeyType::Down => key.down_arrow,
        KeyType::Left => key.left_arrow,
        KeyType::Right => key.right_arrow,
        KeyType::Home => key.home,
        KeyType::End => key.end,
        KeyType::Char(expected) => key.character == Some(*expected),
        _ => false,
    }
}
