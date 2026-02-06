//! Component trait for Elm-architecture style components
//!
//! This module provides a trait-based component system that follows the Elm architecture:
//! - **Model**: The component's internal state
//! - **Msg**: Messages that can update the state
//! - **Props**: External properties passed to the component
//! - **init**: Initialize the model from props
//! - **update**: Handle messages and produce side effects
//! - **view**: Render the component to an Element tree
//!
//! # Example
//!
//! ```rust
//! use rnk::core::{Component, Element};
//! use rnk::cmd::Cmd;
//!
//! // Define props
//! #[derive(Clone, Default)]
//! struct CounterProps {
//!     initial: i32,
//! }
//!
//! // Define messages
//! enum CounterMsg {
//!     Increment,
//!     Decrement,
//! }
//!
//! // Define model (state)
//! struct CounterModel {
//!     count: i32,
//! }
//!
//! // Implement the component
//! struct Counter;
//!
//! impl Component for Counter {
//!     type Props = CounterProps;
//!     type Msg = CounterMsg;
//!     type Model = CounterModel;
//!
//!     fn init(props: &Self::Props) -> (Self::Model, Cmd) {
//!         (CounterModel { count: props.initial }, Cmd::none())
//!     }
//!
//!     fn update(model: &mut Self::Model, msg: Self::Msg) -> Cmd {
//!         match msg {
//!             CounterMsg::Increment => model.count += 1,
//!             CounterMsg::Decrement => model.count -= 1,
//!         }
//!         Cmd::none()
//!     }
//!
//!     fn view(model: &Self::Model, _props: &Self::Props) -> Element {
//!         Element::text(format!("Count: {}", model.count))
//!     }
//! }
//! ```

use crate::cmd::Cmd;
use crate::core::Element;

/// A component following the Elm architecture pattern.
///
/// Components encapsulate state (Model), handle updates via messages (Msg),
/// and render to an Element tree. This provides a predictable, testable
/// architecture for building complex UIs.
///
/// # Type Parameters
///
/// - `Props`: External properties passed to the component (should be Clone)
/// - `Msg`: Message type for state updates
/// - `Model`: Internal state type
pub trait Component: Sized {
    /// External properties passed to the component.
    ///
    /// Props are immutable from the component's perspective and are
    /// provided by the parent. They should implement Clone.
    type Props: Clone + Default;

    /// Message type for updating the component's state.
    ///
    /// Messages represent all possible state transitions. Using an enum
    /// ensures exhaustive handling of all cases.
    type Msg;

    /// The component's internal state.
    ///
    /// The model holds all mutable state for the component. It should
    /// only be modified through the `update` function.
    type Model;

    /// Initialize the component's model from props.
    ///
    /// Called once when the component is first mounted. Returns the
    /// initial model and an optional command for side effects (e.g.,
    /// fetching initial data).
    ///
    /// # Arguments
    ///
    /// * `props` - The initial properties
    ///
    /// # Returns
    ///
    /// A tuple of (initial model, initial command)
    fn init(props: &Self::Props) -> (Self::Model, Cmd);

    /// Update the model in response to a message.
    ///
    /// This is the only place where the model should be mutated.
    /// Returns a command for any side effects that should occur.
    ///
    /// # Arguments
    ///
    /// * `model` - Mutable reference to the current model
    /// * `msg` - The message to handle
    ///
    /// # Returns
    ///
    /// A command describing side effects to execute
    fn update(model: &mut Self::Model, msg: Self::Msg) -> Cmd;

    /// Render the component to an Element tree.
    ///
    /// This function should be pure - given the same model and props,
    /// it should always return the same Element tree.
    ///
    /// # Arguments
    ///
    /// * `model` - Reference to the current model
    /// * `props` - Reference to the current props
    ///
    /// # Returns
    ///
    /// The Element tree representing this component's UI
    fn view(model: &Self::Model, props: &Self::Props) -> Element;

    /// Determine if the component should re-render.
    ///
    /// Called before `view` to optimize rendering. If this returns false,
    /// the previous render result is reused.
    ///
    /// Default implementation always returns true (always re-render).
    ///
    /// # Arguments
    ///
    /// * `_model` - Reference to the current model
    /// * `_old_props` - Reference to the previous props
    /// * `_new_props` - Reference to the new props
    ///
    /// # Returns
    ///
    /// true if the component should re-render, false otherwise
    #[allow(unused_variables)]
    fn should_render(
        model: &Self::Model,
        old_props: &Self::Props,
        new_props: &Self::Props,
    ) -> bool {
        true
    }

    /// Called when the component is about to be unmounted.
    ///
    /// Use this for cleanup tasks like canceling subscriptions or
    /// releasing resources.
    ///
    /// Default implementation does nothing.
    ///
    /// # Arguments
    ///
    /// * `_model` - Reference to the current model
    #[allow(unused_variables)]
    fn unmount(model: &Self::Model) {}

    /// Called after the component is first mounted.
    ///
    /// Use this for initialization tasks that require the component to be
    /// fully mounted, such as starting subscriptions or fetching data.
    /// Returns a command for any side effects that should occur.
    ///
    /// Default implementation returns no command.
    ///
    /// # Arguments
    ///
    /// * `_model` - Reference to the current model
    ///
    /// # Returns
    ///
    /// A command describing side effects to execute after mount
    #[allow(unused_variables)]
    fn on_mount(model: &Self::Model) -> Cmd {
        Cmd::none()
    }
}

/// A stateless component that only depends on props.
///
/// Stateless components are simpler and more efficient when no internal
/// state is needed. They're essentially pure functions from Props to Element.
///
/// # Example
///
/// ```rust
/// use rnk::core::{StatelessComponent, Element};
///
/// #[derive(Clone, Default)]
/// struct GreetingProps {
///     name: String,
/// }
///
/// struct Greeting;
///
/// impl StatelessComponent for Greeting {
///     type Props = GreetingProps;
///
///     fn render(props: &Self::Props) -> Element {
///         Element::text(format!("Hello, {}!", props.name))
///     }
/// }
/// ```
pub trait StatelessComponent: Sized {
    /// External properties passed to the component.
    type Props: Clone + Default;

    /// Render the component to an Element tree.
    ///
    /// # Arguments
    ///
    /// * `props` - Reference to the current props
    ///
    /// # Returns
    ///
    /// The Element tree representing this component's UI
    fn render(props: &Self::Props) -> Element;

    /// Determine if the component should re-render.
    ///
    /// Default implementation always returns true.
    #[allow(unused_variables)]
    fn should_render(old_props: &Self::Props, new_props: &Self::Props) -> bool {
        true
    }
}

/// Blanket implementation: StatelessComponent automatically implements Component
impl<T: StatelessComponent> Component for T {
    type Props = <T as StatelessComponent>::Props;
    type Msg = (); // No messages for stateless components
    type Model = (); // No model for stateless components

    fn init(_props: &Self::Props) -> (Self::Model, Cmd) {
        ((), Cmd::none())
    }

    fn update(_model: &mut Self::Model, _msg: Self::Msg) -> Cmd {
        Cmd::none()
    }

    fn view(_model: &Self::Model, props: &Self::Props) -> Element {
        T::render(props)
    }

    fn should_render(
        _model: &Self::Model,
        old_props: &Self::Props,
        new_props: &Self::Props,
    ) -> bool {
        T::should_render(old_props, new_props)
    }
}

/// Runtime state for a mounted component instance.
///
/// This struct holds the runtime state needed to manage a component
/// throughout its lifecycle.
pub struct ComponentInstance<C: Component> {
    /// The component's current model
    pub model: C::Model,
    /// The component's current props
    pub props: C::Props,
    /// Pending commands to execute
    pending_cmds: Vec<Cmd>,
}

impl<C: Component> ComponentInstance<C> {
    /// Create a new component instance with the given props.
    ///
    /// # Arguments
    ///
    /// * `props` - Initial properties for the component
    ///
    /// # Returns
    ///
    /// A new ComponentInstance with initialized model
    pub fn new(props: C::Props) -> Self {
        let (model, cmd) = C::init(&props);
        let mut instance = Self {
            model,
            props,
            pending_cmds: Vec::new(),
        };
        if !cmd.is_none() {
            instance.pending_cmds.push(cmd);
        }
        // Call on_mount lifecycle hook
        let mount_cmd = C::on_mount(&instance.model);
        if !mount_cmd.is_none() {
            instance.pending_cmds.push(mount_cmd);
        }
        instance
    }

    /// Send a message to update the component.
    ///
    /// # Arguments
    ///
    /// * `msg` - The message to send
    pub fn send(&mut self, msg: C::Msg) {
        let cmd = C::update(&mut self.model, msg);
        if !cmd.is_none() {
            self.pending_cmds.push(cmd);
        }
    }

    /// Update the component's props.
    ///
    /// # Arguments
    ///
    /// * `new_props` - The new properties
    ///
    /// # Returns
    ///
    /// true if the component should re-render
    pub fn update_props(&mut self, new_props: C::Props) -> bool {
        let should_render = C::should_render(&self.model, &self.props, &new_props);
        self.props = new_props;
        should_render
    }

    /// Render the component to an Element tree.
    ///
    /// # Returns
    ///
    /// The Element tree representing this component's UI
    pub fn view(&self) -> Element {
        C::view(&self.model, &self.props)
    }

    /// Take all pending commands.
    ///
    /// # Returns
    ///
    /// A vector of pending commands, leaving the internal queue empty
    pub fn take_cmds(&mut self) -> Vec<Cmd> {
        std::mem::take(&mut self.pending_cmds)
    }

    /// Check if there are pending commands.
    ///
    /// # Returns
    ///
    /// true if there are pending commands
    pub fn has_pending_cmds(&self) -> bool {
        !self.pending_cmds.is_empty()
    }

    /// Unmount the component, performing cleanup.
    pub fn unmount(self) {
        C::unmount(&self.model);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test props
    #[derive(Clone, Default)]
    struct TestProps {
        value: i32,
    }

    // Test messages
    enum TestMsg {
        Increment,
        Set(i32),
    }

    // Test model
    struct TestModel {
        count: i32,
    }

    // Test component
    struct TestComponent;

    impl Component for TestComponent {
        type Props = TestProps;
        type Msg = TestMsg;
        type Model = TestModel;

        fn init(props: &Self::Props) -> (Self::Model, Cmd) {
            (TestModel { count: props.value }, Cmd::none())
        }

        fn update(model: &mut Self::Model, msg: Self::Msg) -> Cmd {
            match msg {
                TestMsg::Increment => model.count += 1,
                TestMsg::Set(v) => model.count = v,
            }
            Cmd::none()
        }

        fn view(model: &Self::Model, _props: &Self::Props) -> Element {
            Element::text(format!("Count: {}", model.count))
        }

        fn should_render(
            _model: &Self::Model,
            old_props: &Self::Props,
            new_props: &Self::Props,
        ) -> bool {
            old_props.value != new_props.value
        }
    }

    #[test]
    fn test_component_init() {
        let props = TestProps { value: 42 };
        let (model, cmd) = TestComponent::init(&props);
        assert_eq!(model.count, 42);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_component_update() {
        let props = TestProps { value: 0 };
        let (mut model, _) = TestComponent::init(&props);

        TestComponent::update(&mut model, TestMsg::Increment);
        assert_eq!(model.count, 1);

        TestComponent::update(&mut model, TestMsg::Set(100));
        assert_eq!(model.count, 100);
    }

    #[test]
    fn test_component_instance() {
        let props = TestProps { value: 10 };
        let mut instance = ComponentInstance::<TestComponent>::new(props);

        assert_eq!(instance.model.count, 10);

        instance.send(TestMsg::Increment);
        assert_eq!(instance.model.count, 11);

        instance.send(TestMsg::Set(50));
        assert_eq!(instance.model.count, 50);
    }

    #[test]
    fn test_component_instance_props_update() {
        let props = TestProps { value: 10 };
        let mut instance = ComponentInstance::<TestComponent>::new(props);

        // Same value - should not re-render
        let should_render = instance.update_props(TestProps { value: 10 });
        assert!(!should_render);

        // Different value - should re-render
        let should_render = instance.update_props(TestProps { value: 20 });
        assert!(should_render);
    }

    // Test stateless component
    #[derive(Clone, Default)]
    struct StatelessProps {
        text: String,
    }

    struct StatelessTest;

    impl StatelessComponent for StatelessTest {
        type Props = StatelessProps;

        fn render(props: &Self::Props) -> Element {
            Element::text(props.text.clone())
        }
    }

    #[test]
    fn test_stateless_component() {
        let props = StatelessProps {
            text: "Hello".to_string(),
        };
        let element = StatelessTest::render(&props);
        // Element should be created (we can't easily inspect it, but it shouldn't panic)
        let _ = element;
    }

    #[test]
    fn test_stateless_as_component() {
        // StatelessComponent should work as Component via blanket impl
        let props = StatelessProps {
            text: "Test".to_string(),
        };
        let (model, cmd) = <StatelessTest as Component>::init(&props);
        assert_eq!(model, ());
        assert!(cmd.is_none());

        let mut model = ();
        let cmd = <StatelessTest as Component>::update(&mut model, ());
        assert!(cmd.is_none());
    }

    // Test component with lifecycle hooks
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[derive(Clone, Default)]
    struct LifecycleProps {
        mounted: Arc<AtomicBool>,
        unmounted: Arc<AtomicBool>,
    }

    struct LifecycleModel {
        mounted: Arc<AtomicBool>,
        unmounted: Arc<AtomicBool>,
    }

    struct LifecycleComponent;

    impl Component for LifecycleComponent {
        type Props = LifecycleProps;
        type Msg = ();
        type Model = LifecycleModel;

        fn init(props: &Self::Props) -> (Self::Model, Cmd) {
            (
                LifecycleModel {
                    mounted: props.mounted.clone(),
                    unmounted: props.unmounted.clone(),
                },
                Cmd::none(),
            )
        }

        fn update(_model: &mut Self::Model, _msg: Self::Msg) -> Cmd {
            Cmd::none()
        }

        fn view(_model: &Self::Model, _props: &Self::Props) -> Element {
            Element::text("")
        }

        fn on_mount(model: &Self::Model) -> Cmd {
            model.mounted.store(true, Ordering::SeqCst);
            Cmd::none()
        }

        fn unmount(model: &Self::Model) {
            model.unmounted.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_lifecycle_hooks() {
        let mounted = Arc::new(AtomicBool::new(false));
        let unmounted = Arc::new(AtomicBool::new(false));

        let props = LifecycleProps {
            mounted: mounted.clone(),
            unmounted: unmounted.clone(),
        };

        // Create instance - on_mount should be called
        let instance = ComponentInstance::<LifecycleComponent>::new(props);
        assert!(mounted.load(Ordering::SeqCst), "on_mount should be called");
        assert!(
            !unmounted.load(Ordering::SeqCst),
            "unmount should not be called yet"
        );

        // Unmount - unmount should be called
        instance.unmount();
        assert!(
            unmounted.load(Ordering::SeqCst),
            "unmount should be called after unmount()"
        );
    }
}
