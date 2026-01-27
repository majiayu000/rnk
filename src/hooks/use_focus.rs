//! Focus management hooks

use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Unique focus ID generator
static FOCUS_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn generate_focus_id() -> usize {
    FOCUS_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Focus state for a component
#[derive(Debug, Clone)]
pub struct FocusState {
    pub is_focused: bool,
}

/// Options for use_focus hook
#[derive(Debug, Clone, Default)]
pub struct UseFocusOptions {
    /// Whether this element should auto-focus on mount
    pub auto_focus: bool,
    /// Whether this element is focusable (default: true)
    pub is_active: bool,
    /// ID for this focusable element (optional, auto-generated if not provided)
    pub id: Option<String>,
}

impl UseFocusOptions {
    pub fn new() -> Self {
        Self {
            auto_focus: false,
            is_active: true,
            id: None,
        }
    }

    pub fn auto_focus(mut self) -> Self {
        self.auto_focus = true;
        self
    }

    pub fn is_active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

/// Focus manager state - tracks all focusable elements
#[derive(Debug, Clone)]
struct FocusableElement {
    id: usize,
    custom_id: Option<String>,
    is_active: bool,
}

/// Global focus manager state
#[derive(Debug, Default)]
pub struct FocusManager {
    elements: Vec<FocusableElement>,
    focused_index: Option<usize>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            focused_index: None,
        }
    }

    /// Register a focusable element
    pub fn register(
        &mut self,
        custom_id: Option<String>,
        is_active: bool,
        auto_focus: bool,
    ) -> usize {
        let id = generate_focus_id();
        self.elements.push(FocusableElement {
            id,
            custom_id,
            is_active,
        });

        // Auto-focus if requested and no element is currently focused
        if auto_focus && self.focused_index.is_none() && is_active {
            self.focused_index = Some(self.elements.len() - 1);
        }

        id
    }

    /// Unregister a focusable element
    #[allow(dead_code)]
    pub fn unregister(&mut self, id: usize) {
        if let Some(pos) = self.elements.iter().position(|e| e.id == id) {
            self.elements.remove(pos);

            // Adjust focused index if needed
            if let Some(focused) = self.focused_index {
                if pos == focused {
                    self.focused_index = None;
                } else if pos < focused {
                    self.focused_index = Some(focused - 1);
                }
            }
        }
    }

    /// Check if an element is focused
    pub fn is_focused(&self, id: usize) -> bool {
        self.focused_index
            .and_then(|idx| self.elements.get(idx))
            .map(|e| e.id == id)
            .unwrap_or(false)
    }

    /// Focus next element
    pub fn focus_next(&mut self) {
        let active_elements: Vec<usize> = self
            .elements
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_active)
            .map(|(i, _)| i)
            .collect();

        if active_elements.is_empty() {
            return;
        }

        let current = self.focused_index.unwrap_or(0);
        let current_pos = active_elements
            .iter()
            .position(|&i| i == current)
            .unwrap_or(0);
        let next_pos = (current_pos + 1) % active_elements.len();
        self.focused_index = Some(active_elements[next_pos]);
    }

    /// Focus previous element
    pub fn focus_previous(&mut self) {
        let active_elements: Vec<usize> = self
            .elements
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_active)
            .map(|(i, _)| i)
            .collect();

        if active_elements.is_empty() {
            return;
        }

        let current = self.focused_index.unwrap_or(0);
        let current_pos = active_elements
            .iter()
            .position(|&i| i == current)
            .unwrap_or(0);
        let prev_pos = if current_pos == 0 {
            active_elements.len() - 1
        } else {
            current_pos - 1
        };
        self.focused_index = Some(active_elements[prev_pos]);
    }

    /// Focus a specific element by custom ID
    pub fn focus(&mut self, custom_id: &str) {
        if let Some(pos) = self
            .elements
            .iter()
            .position(|e| e.custom_id.as_deref() == Some(custom_id) && e.is_active)
        {
            self.focused_index = Some(pos);
        }
    }

    /// Enable/disable focus for an element
    pub fn enable_focus(&mut self, id: usize, enabled: bool) {
        if let Some(elem) = self.elements.iter_mut().find(|e| e.id == id) {
            elem.is_active = enabled;
        }
    }

    /// Clear focus state for next render
    pub fn clear(&mut self) {
        self.elements.clear();
        // Keep focused_index for persistence across renders
    }
}

// Thread-local storage for focus manager (legacy fallback)
thread_local! {
    static FOCUS_MANAGER: RefCell<FocusManager> = RefCell::new(FocusManager::new());
}

/// Set the focus manager (called by App during render)
#[allow(dead_code)]
pub fn with_focus_manager<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    FOCUS_MANAGER.with(|fm| {
        fm.borrow_mut().clear();
    });
    f()
}

/// Hook to make a component focusable
///
/// # Example
///
/// ```ignore
/// let focus = use_focus(UseFocusOptions::new().auto_focus());
///
/// Box::new()
///     .border_style(if focus.is_focused {
///         BorderStyle::Bold
///     } else {
///         BorderStyle::Single
///     })
/// ```
pub fn use_focus(options: UseFocusOptions) -> FocusState {
    use crate::hooks::use_signal;

    // Try to use RuntimeContext first, fall back to thread-local
    if let Some(ctx) = crate::runtime::current_runtime() {
        let focus_id = use_signal(|| {
            ctx.borrow_mut().focus_manager_mut().register(
                options.id.clone(),
                options.is_active,
                options.auto_focus,
            )
        });

        let is_focused = ctx.borrow().focus_manager().is_focused(focus_id.get());
        FocusState { is_focused }
    } else {
        // Legacy thread-local fallback
        let focus_id = use_signal(|| {
            FOCUS_MANAGER.with(|fm| {
                fm.borrow_mut()
                    .register(options.id.clone(), options.is_active, options.auto_focus)
            })
        });

        let is_focused = FOCUS_MANAGER.with(|fm| fm.borrow().is_focused(focus_id.get()));
        FocusState { is_focused }
    }
}

/// Hook to access the focus manager
///
/// # Example
///
/// ```ignore
/// let fm = use_focus_manager();
///
/// use_input(move |_, key| {
///     if key.tab {
///         fm.focus_next();
///     }
/// });
/// ```
pub fn use_focus_manager() -> FocusManagerHandle {
    FocusManagerHandle
}

/// Handle to the focus manager
#[derive(Clone, Copy)]
pub struct FocusManagerHandle;

impl FocusManagerHandle {
    /// Focus the next focusable element
    pub fn focus_next(&self) {
        if let Some(ctx) = crate::runtime::current_runtime() {
            ctx.borrow_mut().focus_manager_mut().focus_next();
        } else {
            FOCUS_MANAGER.with(|fm| fm.borrow_mut().focus_next());
        }
    }

    /// Focus the previous focusable element
    pub fn focus_previous(&self) {
        if let Some(ctx) = crate::runtime::current_runtime() {
            ctx.borrow_mut().focus_manager_mut().focus_previous();
        } else {
            FOCUS_MANAGER.with(|fm| fm.borrow_mut().focus_previous());
        }
    }

    /// Focus a specific element by ID
    pub fn focus(&self, id: &str) {
        if let Some(ctx) = crate::runtime::current_runtime() {
            ctx.borrow_mut().focus_manager_mut().focus(id);
        } else {
            FOCUS_MANAGER.with(|fm| fm.borrow_mut().focus(id));
        }
    }

    /// Enable/disable focus for the current component
    pub fn enable_focus(&self, id: usize, enabled: bool) {
        if let Some(ctx) = crate::runtime::current_runtime() {
            ctx.borrow_mut()
                .focus_manager_mut()
                .enable_focus(id, enabled);
        } else {
            FOCUS_MANAGER.with(|fm| fm.borrow_mut().enable_focus(id, enabled));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_manager_registration() {
        let mut fm = FocusManager::new();

        let id1 = fm.register(None, true, false);
        let id2 = fm.register(None, true, false);

        assert!(id1 != id2);
        assert_eq!(fm.elements.len(), 2);
    }

    #[test]
    fn test_focus_manager_auto_focus() {
        let mut fm = FocusManager::new();

        let id1 = fm.register(None, true, true); // auto_focus
        let _id2 = fm.register(None, true, false);

        assert!(fm.is_focused(id1));
    }

    #[test]
    fn test_focus_navigation() {
        let mut fm = FocusManager::new();

        let id1 = fm.register(None, true, true);
        let id2 = fm.register(None, true, false);
        let id3 = fm.register(None, true, false);

        assert!(fm.is_focused(id1));

        fm.focus_next();
        assert!(fm.is_focused(id2));

        fm.focus_next();
        assert!(fm.is_focused(id3));

        fm.focus_next();
        assert!(fm.is_focused(id1)); // Wraps around

        fm.focus_previous();
        assert!(fm.is_focused(id3));
    }

    #[test]
    fn test_focus_by_id() {
        let mut fm = FocusManager::new();

        let _id1 = fm.register(Some("first".to_string()), true, true);
        let id2 = fm.register(Some("second".to_string()), true, false);

        fm.focus("second");
        assert!(fm.is_focused(id2));
    }

    #[test]
    fn test_inactive_elements_skipped() {
        let mut fm = FocusManager::new();

        let id1 = fm.register(None, true, true);
        let _id2 = fm.register(None, false, false); // inactive
        let id3 = fm.register(None, true, false);

        assert!(fm.is_focused(id1));

        fm.focus_next();
        assert!(fm.is_focused(id3)); // Skips inactive element
    }

    #[test]
    fn test_focus_with_runtime() {
        use crate::runtime::{RuntimeContext, with_runtime};
        use std::rc::Rc;

        let ctx = Rc::new(RefCell::new(RuntimeContext::new()));

        // Register elements within runtime context
        with_runtime(ctx.clone(), || {
            let fm_handle = use_focus_manager();

            // Register some elements directly on the context
            let id1 = ctx
                .borrow_mut()
                .focus_manager_mut()
                .register(None, true, true);
            let id2 = ctx
                .borrow_mut()
                .focus_manager_mut()
                .register(None, true, false);

            assert!(ctx.borrow().focus_manager().is_focused(id1));

            fm_handle.focus_next();
            assert!(ctx.borrow().focus_manager().is_focused(id2));
        });
    }
}
