//! Component instance registry
//!
//! Manages component instances and their associated hook states
//! for efficient reuse during reconciliation.

use std::any::Any;
use std::collections::HashMap;

use crate::core::NodeKey;
use crate::hooks::context::HookContext;

/// Stored component instance with its hook state
pub struct ComponentInstance {
    /// The component instance (type-erased)
    component: Box<dyn Any + Send>,
    /// Hook state for this component
    pub hook_context: HookContext,
    /// Whether this instance was used in the current render
    pub used: bool,
}

impl ComponentInstance {
    /// Create a new component instance
    pub fn new<C: Any + Send + 'static>(component: C) -> Self {
        Self {
            component: Box::new(component),
            hook_context: HookContext::new(),
            used: false,
        }
    }

    /// Get the component as a specific type
    pub fn get<C: 'static>(&self) -> Option<&C> {
        self.component.downcast_ref::<C>()
    }

    /// Get the component as a specific type (mutable)
    pub fn get_mut<C: 'static>(&mut self) -> Option<&mut C> {
        self.component.downcast_mut::<C>()
    }

    /// Mark this instance as used in the current render
    pub fn mark_used(&mut self) {
        self.used = true;
    }

    /// Reset the used flag for a new render cycle
    pub fn reset_used(&mut self) {
        self.used = false;
    }
}

/// Registry for managing component instances
///
/// The registry stores component instances keyed by their NodeKey,
/// allowing efficient reuse during reconciliation.
pub struct ComponentRegistry {
    /// Component instances by key
    instances: HashMap<NodeKey, ComponentInstance>,
    /// Pending cleanup keys (components removed in this cycle)
    pending_cleanup: Vec<NodeKey>,
}

impl ComponentRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            pending_cleanup: Vec::new(),
        }
    }

    /// Get or create a component instance
    ///
    /// If an instance exists for the key, returns it.
    /// Otherwise, creates a new instance using the provided factory.
    pub fn get_or_create<C, F>(&mut self, key: NodeKey, factory: F) -> &mut ComponentInstance
    where
        C: Any + Send + 'static,
        F: FnOnce() -> C,
    {
        self.instances.entry(key).or_insert_with(|| {
            let component = factory();
            ComponentInstance::new(component)
        })
    }

    /// Get an existing component instance
    pub fn get(&self, key: &NodeKey) -> Option<&ComponentInstance> {
        self.instances.get(key)
    }

    /// Get an existing component instance (mutable)
    pub fn get_mut(&mut self, key: &NodeKey) -> Option<&mut ComponentInstance> {
        self.instances.get_mut(key)
    }

    /// Check if a component instance exists
    pub fn contains(&self, key: &NodeKey) -> bool {
        self.instances.contains_key(key)
    }

    /// Begin a new render cycle
    ///
    /// Resets the "used" flag on all instances so we can track
    /// which ones are still in use after rendering.
    pub fn begin_render(&mut self) {
        for instance in self.instances.values_mut() {
            instance.reset_used();
        }
        self.pending_cleanup.clear();
    }

    /// End a render cycle
    ///
    /// Identifies unused instances and schedules them for cleanup.
    pub fn end_render(&mut self) {
        for (key, instance) in &self.instances {
            if !instance.used {
                self.pending_cleanup.push(*key);
            }
        }
    }

    /// Run cleanup for removed components
    ///
    /// This runs effect cleanup functions and removes instances
    /// that were not used in the last render cycle.
    pub fn cleanup(&mut self) {
        for key in self.pending_cleanup.drain(..) {
            if let Some(mut instance) = self.instances.remove(&key) {
                // Run any cleanup effects
                instance.hook_context.run_effects();
            }
        }
    }

    /// Remove a specific component instance
    pub fn remove(&mut self, key: &NodeKey) -> Option<ComponentInstance> {
        self.instances.remove(key)
    }

    /// Get the number of registered instances
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Clear all instances
    pub fn clear(&mut self) {
        // Run cleanup for all instances
        for (_, mut instance) in self.instances.drain() {
            instance.hook_context.run_effects();
        }
        self.pending_cleanup.clear();
    }

    /// Get all keys in the registry
    pub fn keys(&self) -> impl Iterator<Item = &NodeKey> {
        self.instances.keys()
    }

    /// Iterate over all instances
    pub fn iter(&self) -> impl Iterator<Item = (&NodeKey, &ComponentInstance)> {
        self.instances.iter()
    }

    /// Iterate over all instances (mutable)
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&NodeKey, &mut ComponentInstance)> {
        self.instances.iter_mut()
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ComponentRegistry {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;

    struct TestComponent {
        value: i32,
    }

    #[test]
    fn test_component_instance_creation() {
        let instance = ComponentInstance::new(TestComponent { value: 42 });
        let component = instance.get::<TestComponent>().unwrap();
        assert_eq!(component.value, 42);
    }

    #[test]
    fn test_component_instance_mutation() {
        let mut instance = ComponentInstance::new(TestComponent { value: 0 });
        {
            let component = instance.get_mut::<TestComponent>().unwrap();
            component.value = 100;
        }
        let component = instance.get::<TestComponent>().unwrap();
        assert_eq!(component.value, 100);
    }

    #[test]
    fn test_registry_get_or_create() {
        let mut registry = ComponentRegistry::new();
        let key = NodeKey::new(TypeId::of::<TestComponent>(), 0);

        // First call creates
        {
            let instance = registry.get_or_create(key, || TestComponent { value: 1 });
            instance.mark_used();
            let component = instance.get::<TestComponent>().unwrap();
            assert_eq!(component.value, 1);
        }

        // Second call returns existing
        {
            let instance = registry.get_or_create(key, || TestComponent { value: 999 });
            let component = instance.get::<TestComponent>().unwrap();
            assert_eq!(component.value, 1); // Still 1, not 999
        }
    }

    #[test]
    fn test_registry_cleanup_unused() {
        let mut registry = ComponentRegistry::new();
        let key1 = NodeKey::new(TypeId::of::<TestComponent>(), 0);
        let key2 = NodeKey::new(TypeId::of::<TestComponent>(), 1);

        // Create two instances
        registry.get_or_create(key1, || TestComponent { value: 1 });
        registry.get_or_create(key2, || TestComponent { value: 2 });
        assert_eq!(registry.len(), 2);

        // Begin render cycle
        registry.begin_render();

        // Only use key1
        if let Some(instance) = registry.get_mut(&key1) {
            instance.mark_used();
        }

        // End render and cleanup
        registry.end_render();
        registry.cleanup();

        // key2 should be removed
        assert_eq!(registry.len(), 1);
        assert!(registry.contains(&key1));
        assert!(!registry.contains(&key2));
    }

    #[test]
    fn test_registry_clear() {
        let mut registry = ComponentRegistry::new();
        let key = NodeKey::new(TypeId::of::<TestComponent>(), 0);

        registry.get_or_create(key, || TestComponent { value: 1 });
        assert_eq!(registry.len(), 1);

        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_used_flag() {
        let mut instance = ComponentInstance::new(TestComponent { value: 0 });

        assert!(!instance.used);
        instance.mark_used();
        assert!(instance.used);
        instance.reset_used();
        assert!(!instance.used);
    }
}
