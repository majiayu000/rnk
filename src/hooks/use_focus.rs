//! Focus management hooks

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

/// Options for scoped focus traversal.
#[derive(Debug, Clone)]
pub struct ScopedFocusOptions {
    /// Base focus options.
    pub focus: UseFocusOptions,
    /// Traversal scope for local Tab movement.
    pub scope: String,
    /// Optional order inside the scope. Lower values receive focus first.
    pub focus_order: Option<i32>,
}

impl ScopedFocusOptions {
    pub fn new(scope: impl Into<String>) -> Self {
        Self {
            focus: UseFocusOptions::new(),
            scope: scope.into(),
            focus_order: None,
        }
    }

    pub fn auto_focus(mut self) -> Self {
        self.focus = self.focus.auto_focus();
        self
    }

    pub fn is_active(mut self, active: bool) -> Self {
        self.focus = self.focus.is_active(active);
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.focus = self.focus.id(id);
        self
    }

    pub fn focus_order(mut self, order: i32) -> Self {
        self.focus_order = Some(order);
        self
    }

    pub fn focus_options(mut self, focus: UseFocusOptions) -> Self {
        self.focus = focus;
        self
    }
}

/// Focus manager state - tracks all focusable elements
#[derive(Debug, Clone)]
struct FocusableElement {
    id: usize,
    custom_id: Option<String>,
    is_active: bool,
    scope: Option<String>,
    focus_order: Option<i32>,
}

/// Global focus manager state
#[derive(Debug, Default)]
pub struct FocusManager {
    elements: Vec<FocusableElement>,
    focused_index: Option<usize>,
    next_id: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusDirection {
    Next,
    Previous,
}

impl FocusManager {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            focused_index: None,
            next_id: 1,
        }
    }

    /// Register a focusable element
    pub fn register(
        &mut self,
        custom_id: Option<String>,
        is_active: bool,
        auto_focus: bool,
    ) -> usize {
        self.register_with_options(custom_id, is_active, auto_focus, None, None)
    }

    /// Register a focusable element with scope and ordering metadata.
    pub fn register_with_options(
        &mut self,
        custom_id: Option<String>,
        is_active: bool,
        auto_focus: bool,
        scope: Option<String>,
        focus_order: Option<i32>,
    ) -> usize {
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .filter(|next| *next != 0)
            .unwrap_or_else(|| panic!("FocusManager ID counter exhausted"));

        self.elements.push(FocusableElement {
            id,
            custom_id,
            is_active,
            scope,
            focus_order,
        });

        // Auto-focus if requested and no element is currently focused
        if auto_focus && self.focused_index.is_none() && is_active {
            self.focused_index = Some(self.elements.len() - 1);
        }

        id
    }

    /// Unregister a focusable element
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

    /// Update a focusable element's metadata
    pub fn update(
        &mut self,
        id: usize,
        custom_id: Option<String>,
        is_active: bool,
        auto_focus: bool,
    ) {
        self.update_with_options(id, custom_id, is_active, auto_focus, None, None);
    }

    /// Update a focusable element's metadata, including scope/order.
    pub fn update_with_options(
        &mut self,
        id: usize,
        custom_id: Option<String>,
        is_active: bool,
        auto_focus: bool,
        scope: Option<String>,
        focus_order: Option<i32>,
    ) {
        if let Some(elem) = self.elements.iter_mut().find(|e| e.id == id) {
            elem.custom_id = custom_id;
            elem.is_active = is_active;
            elem.scope = scope;
            elem.focus_order = focus_order;
        }

        if auto_focus && self.focused_index.is_none() && is_active {
            if let Some(pos) = self.elements.iter().position(|e| e.id == id) {
                self.focused_index = Some(pos);
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

    fn active_indices(&self, scope: Option<&str>) -> Vec<usize> {
        let mut indices: Vec<(usize, i32)> = self
            .elements
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.is_active
                    && scope
                        .map(|scope| e.scope.as_deref() == Some(scope))
                        .unwrap_or(true)
            })
            .map(|(idx, e)| {
                let order = scope.and(e.focus_order).unwrap_or(i32::MAX);
                (idx, order)
            })
            .collect();

        indices.sort_by_key(|(idx, order)| (*order, *idx));
        indices.into_iter().map(|(idx, _)| idx).collect()
    }

    fn move_focus(&mut self, scope: Option<&str>, direction: FocusDirection) {
        let active_elements = self.active_indices(scope);

        if active_elements.is_empty() {
            return;
        }

        let Some(current) = self.focused_index else {
            self.focused_index = Some(fallback_focus_index(&active_elements, direction));
            return;
        };

        let Some(current_pos) = active_elements.iter().position(|&i| i == current) else {
            self.focused_index = Some(fallback_focus_index(&active_elements, direction));
            return;
        };

        let next_pos = match direction {
            FocusDirection::Next => (current_pos + 1) % active_elements.len(),
            FocusDirection::Previous if current_pos == 0 => active_elements.len() - 1,
            FocusDirection::Previous => current_pos - 1,
        };
        self.focused_index = Some(active_elements[next_pos]);
    }

    /// Focus next element
    pub fn focus_next(&mut self) {
        self.move_focus(None, FocusDirection::Next);
    }

    /// Focus previous element
    pub fn focus_previous(&mut self) {
        self.move_focus(None, FocusDirection::Previous);
    }

    /// Focus next element inside a scope.
    pub fn focus_next_in_scope(&mut self, scope: &str) {
        self.move_focus(Some(scope), FocusDirection::Next);
    }

    /// Focus previous element inside a scope.
    pub fn focus_previous_in_scope(&mut self, scope: &str) {
        self.move_focus(Some(scope), FocusDirection::Previous);
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

fn fallback_focus_index(active_elements: &[usize], direction: FocusDirection) -> usize {
    match direction {
        FocusDirection::Next => active_elements[0],
        FocusDirection::Previous => active_elements[active_elements.len() - 1],
    }
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
    use_focus_with_options(options, None, None)
}

/// Hook to make a component focusable within a traversal scope.
pub fn use_scoped_focus(options: ScopedFocusOptions) -> FocusState {
    let scope = Some(options.scope.clone());
    use_focus_with_options(options.focus, scope, options.focus_order)
}

fn use_focus_with_options(
    options: UseFocusOptions,
    scope: Option<String>,
    focus_order: Option<i32>,
) -> FocusState {
    use crate::hooks::use_signal;

    let registration = use_signal(|| {
        if let Some(ctx) = crate::runtime::current_runtime() {
            ctx.borrow_mut().focus_manager_mut().register_with_options(
                options.id.clone(),
                options.is_active,
                options.auto_focus,
                scope.clone(),
                focus_order,
            )
        } else {
            0 // no-op ID when no runtime
        }
    });

    let id = registration.get();

    // Update metadata when options change
    if let Some(ctx) = crate::runtime::current_runtime() {
        ctx.borrow_mut().focus_manager_mut().update_with_options(
            id,
            options.id.clone(),
            options.is_active,
            options.auto_focus,
            scope.clone(),
            focus_order,
        );
    }

    // Unregister on unmount
    crate::hooks::use_effect_once({
        move || {
            Some(Box::new(move || {
                if let Some(ctx) = crate::runtime::current_runtime() {
                    ctx.borrow_mut().focus_manager_mut().unregister(id);
                }
            }))
        }
    });

    let is_focused = crate::runtime::current_runtime()
        .map(|ctx| ctx.borrow().focus_manager().is_focused(id))
        .unwrap_or(false);

    FocusState { is_focused }
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
        }
    }

    /// Focus the previous focusable element
    pub fn focus_previous(&self) {
        if let Some(ctx) = crate::runtime::current_runtime() {
            ctx.borrow_mut().focus_manager_mut().focus_previous();
        }
    }

    /// Focus the next focusable element inside a scope
    pub fn focus_next_in_scope(&self, scope: &str) {
        if let Some(ctx) = crate::runtime::current_runtime() {
            ctx.borrow_mut()
                .focus_manager_mut()
                .focus_next_in_scope(scope);
        }
    }

    /// Focus the previous focusable element inside a scope
    pub fn focus_previous_in_scope(&self, scope: &str) {
        if let Some(ctx) = crate::runtime::current_runtime() {
            ctx.borrow_mut()
                .focus_manager_mut()
                .focus_previous_in_scope(scope);
        }
    }

    /// Focus a specific element by ID
    pub fn focus(&self, id: &str) {
        if let Some(ctx) = crate::runtime::current_runtime() {
            ctx.borrow_mut().focus_manager_mut().focus(id);
        }
    }

    /// Enable/disable focus for the current component
    pub fn enable_focus(&self, id: usize, enabled: bool) {
        if let Some(ctx) = crate::runtime::current_runtime() {
            ctx.borrow_mut()
                .focus_manager_mut()
                .enable_focus(id, enabled);
        }
    }
}

/// Install default Tab and Shift+Tab focus traversal for all focusable elements.
pub fn use_focus_traversal() {
    let fm = use_focus_manager();

    crate::hooks::use_input(move |_, key| {
        if key.back_tab || (key.tab && key.shift) {
            fm.focus_previous();
        } else if key.tab {
            fm.focus_next();
        }
    });
}

/// Install default Tab and Shift+Tab focus traversal inside one scope.
pub fn use_focus_traversal_in_scope(scope: impl Into<String>) {
    let scope = scope.into();
    let fm = use_focus_manager();

    crate::hooks::use_input(move |_, key| {
        if key.back_tab || (key.tab && key.shift) {
            fm.focus_previous_in_scope(&scope);
        } else if key.tab {
            fm.focus_next_in_scope(&scope);
        }
    });
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
    fn test_scoped_focus_navigation_uses_scope_and_order() {
        let mut fm = FocusManager::new();

        let outside = fm.register_with_options(
            Some("outside".to_string()),
            true,
            true,
            Some("toolbar".to_string()),
            Some(0),
        );
        let second = fm.register_with_options(
            Some("second".to_string()),
            true,
            false,
            Some("form".to_string()),
            Some(20),
        );
        let first = fm.register_with_options(
            Some("first".to_string()),
            true,
            false,
            Some("form".to_string()),
            Some(10),
        );

        assert!(fm.is_focused(outside));

        fm.focus_next_in_scope("form");
        assert!(fm.is_focused(first));

        fm.focus_next_in_scope("form");
        assert!(fm.is_focused(second));

        fm.focus_previous_in_scope("form");
        assert!(fm.is_focused(first));
    }

    #[test]
    fn test_previous_focus_fallback_uses_last_active_element() {
        let mut fm = FocusManager::new();

        let _id1 = fm.register(None, true, false);
        let id2 = fm.register(None, true, false);

        fm.focus_previous();
        assert!(fm.is_focused(id2));
    }

    #[test]
    fn test_scoped_previous_fallback_uses_last_in_scope_order() {
        let mut fm = FocusManager::new();

        let outside = fm.register_with_options(
            Some("outside".to_string()),
            true,
            true,
            Some("toolbar".to_string()),
            Some(0),
        );
        let first = fm.register_with_options(
            Some("first".to_string()),
            true,
            false,
            Some("form".to_string()),
            Some(10),
        );
        let second = fm.register_with_options(
            Some("second".to_string()),
            true,
            false,
            Some("form".to_string()),
            Some(20),
        );

        assert!(fm.is_focused(outside));

        fm.focus_previous_in_scope("form");
        assert!(fm.is_focused(second));

        fm.focus_next_in_scope("form");
        assert!(fm.is_focused(first));
    }

    #[test]
    fn test_unscoped_focus_navigation_preserves_registration_order() {
        let mut fm = FocusManager::new();

        let id1 = fm.register_with_options(None, true, true, Some("form".to_string()), Some(30));
        let id2 = fm.register_with_options(None, true, false, Some("form".to_string()), Some(10));

        assert!(fm.is_focused(id1));

        fm.focus_next();
        assert!(fm.is_focused(id2));
    }

    #[test]
    fn test_focus_with_runtime() {
        use crate::runtime::{RuntimeContext, with_runtime};
        use std::cell::RefCell;
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
