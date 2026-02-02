//! Declarative UI macros for rnk
//!
//! This module provides macros for building UI in a declarative, JSX-like style.
//!
//! # The `row!` and `col!` Macros
//!
//! Convenience macros for horizontal and vertical layouts:
//!
//! ```rust,ignore
//! use rnk::{row, col, components::Text};
//!
//! let layout = col! {
//!     Text::new("Header").into_element(),
//!     row! {
//!         Text::new("Left").into_element(),
//!         Text::new("Right").into_element(),
//!     },
//!     Text::new("Footer").into_element(),
//! };
//! ```

/// Create a horizontal row layout (flex-direction: row)
///
/// # Example
///
/// ```rust
/// use rnk::{row, components::Text};
///
/// let element = row! {
///     Text::new("Left").into_element(),
///     Text::new("Right").into_element(),
/// };
/// ```
#[macro_export]
macro_rules! row {
    ($($child:expr),* $(,)?) => {{
        use $crate::core::{Element, ElementType, FlexDirection};

        let mut element = Element::new(ElementType::Box);
        element.style.flex_direction = FlexDirection::Row;
        $(
            element.add_child($child);
        )*
        element
    }};
}

/// Create a vertical column layout (flex-direction: column)
///
/// # Example
///
/// ```rust
/// use rnk::{col, components::Text};
///
/// let element = col! {
///     Text::new("Top").into_element(),
///     Text::new("Bottom").into_element(),
/// };
/// ```
#[macro_export]
macro_rules! col {
    ($($child:expr),* $(,)?) => {{
        use $crate::core::{Element, ElementType, FlexDirection};

        let mut element = Element::new(ElementType::Box);
        element.style.flex_direction = FlexDirection::Column;
        $(
            element.add_child($child);
        )*
        element
    }};
}

/// Create a Box element with children
///
/// # Example
///
/// ```rust
/// use rnk::{box_element, components::Text};
///
/// // Simple box with children
/// let element = box_element! {
///     Text::new("Child 1").into_element(),
///     Text::new("Child 2").into_element(),
/// };
/// ```
#[macro_export]
macro_rules! box_element {
    // Box with children only
    ($($child:expr),* $(,)?) => {{
        use $crate::core::{Element, ElementType};

        let mut element = Element::new(ElementType::Box);
        $(
            element.add_child($child);
        )*
        element
    }};
}

/// Create a Text element
///
/// # Example
///
/// ```rust
/// use rnk::text;
///
/// let element = text!("Hello, World!");
/// let formatted = text!("Count: {}", 42);
/// ```
#[macro_export]
macro_rules! text {
    ($content:expr) => {{
        $crate::components::Text::new($content).into_element()
    }};
    ($fmt:expr, $($arg:tt)*) => {{
        $crate::components::Text::new(format!($fmt, $($arg)*)).into_element()
    }};
}

/// Create a styled Text element
///
/// # Example
///
/// ```rust
/// use rnk::{styled_text, core::Color};
///
/// let element = styled_text!("Error!", color: Color::Red);
/// ```
#[macro_export]
macro_rules! styled_text {
    ($content:expr $(, $prop:ident : $val:expr)* $(,)?) => {{
        let mut text = $crate::components::Text::new($content);
        $(
            text = text.$prop($val);
        )*
        text.into_element()
    }};
}

/// Create a Spacer element
///
/// # Example
///
/// ```rust
/// use rnk::{row, spacer, text};
///
/// let element = row! {
///     text!("Left"),
///     spacer!(),
///     text!("Right"),
/// };
/// ```
#[macro_export]
macro_rules! spacer {
    () => {{ $crate::components::Spacer::new().into_element() }};
}

/// Conditional rendering helper
///
/// # Example
///
/// ```rust
/// use rnk::{when, text};
///
/// let show_error = true;
/// let element = when!(show_error => text!("Error occurred!"));
/// ```
#[macro_export]
macro_rules! when {
    ($cond:expr => $then:expr) => {{ if $cond { Some($then) } else { None } }};
    ($cond:expr => $then:expr; else $else:expr) => {{ if $cond { $then } else { $else } }};
}

/// Create a list of elements from an iterator
///
/// # Example
///
/// ```rust
/// use rnk::{col, list, text};
///
/// let items = vec!["Apple", "Banana", "Cherry"];
/// let element = col! {
///     list!(items.iter(), |item| text!("{}", item)),
/// };
/// ```
#[macro_export]
macro_rules! list {
    ($iter:expr, $render:expr) => {{
        use $crate::core::{Element, ElementType};

        let mut container = Element::new(ElementType::Box);
        for item in $iter {
            container.add_child($render(item));
        }
        container
    }};
    ($iter:expr, |$item:ident| $body:expr) => {{
        use $crate::core::{Element, ElementType};

        let mut container = Element::new(ElementType::Box);
        for $item in $iter {
            container.add_child($body);
        }
        container
    }};
}

/// Create a list of elements with index from an iterator
///
/// # Example
///
/// ```rust
/// use rnk::{col, list_indexed, text};
///
/// let items = vec!["Apple", "Banana", "Cherry"];
/// let element = col! {
///     list_indexed!(items.iter(), |item, idx| text!("{}: {}", idx, item)),
/// };
/// ```
#[macro_export]
macro_rules! list_indexed {
    ($iter:expr, |$item:ident, $idx:ident| $body:expr) => {{
        use $crate::core::{Element, ElementType};

        let mut container = Element::new(ElementType::Box);
        for ($idx, $item) in $iter.enumerate() {
            container.add_child($body);
        }
        container
    }};
}

#[cfg(test)]
mod tests {
    use crate::components::Text;
    use crate::core::{ElementType, FlexDirection};

    #[test]
    fn test_row_macro() {
        let element = row! {
            Text::new("A").into_element(),
            Text::new("B").into_element(),
        };

        assert!(matches!(element.element_type, ElementType::Box));
        assert_eq!(element.style.flex_direction, FlexDirection::Row);
        assert_eq!(element.children.len(), 2);
    }

    #[test]
    fn test_col_macro() {
        let element = col! {
            Text::new("A").into_element(),
            Text::new("B").into_element(),
        };

        assert!(matches!(element.element_type, ElementType::Box));
        assert_eq!(element.style.flex_direction, FlexDirection::Column);
        assert_eq!(element.children.len(), 2);
    }

    #[test]
    fn test_box_element_macro() {
        let element = box_element! {
            Text::new("Child").into_element(),
        };

        assert!(matches!(element.element_type, ElementType::Box));
        assert_eq!(element.children.len(), 1);
    }

    #[test]
    fn test_text_macro() {
        let element = text!("Hello");
        assert!(matches!(element.element_type, ElementType::Text));
    }

    #[test]
    fn test_text_macro_formatted() {
        let count = 42;
        let element = text!("Count: {}", count);
        assert!(matches!(element.element_type, ElementType::Text));
    }

    #[test]
    fn test_spacer_macro() {
        let element = spacer!();
        assert!(matches!(element.element_type, ElementType::Box));
        assert_eq!(element.style.flex_grow, 1.0);
    }

    #[test]
    fn test_when_macro() {
        let show = true;
        let result = when!(show => text!("Visible"));
        assert!(result.is_some());

        let hide = false;
        let result = when!(hide => text!("Hidden"));
        assert!(result.is_none());
    }

    #[test]
    fn test_when_macro_else() {
        let condition = true;
        let element = when!(condition => text!("Yes"); else text!("No"));
        assert!(matches!(element.element_type, ElementType::Text));

        let condition = false;
        let element = when!(condition => text!("Yes"); else text!("No"));
        assert!(matches!(element.element_type, ElementType::Text));
    }

    #[test]
    fn test_list_macro() {
        let items = vec!["A", "B", "C"];
        let element = list!(items.iter(), |item| text!("{}", item));

        assert!(matches!(element.element_type, ElementType::Box));
        assert_eq!(element.children.len(), 3);
    }

    #[test]
    fn test_list_indexed_macro() {
        let items = vec!["A", "B", "C"];
        let element = list_indexed!(items.iter(), |item, idx| text!("{}: {}", idx, item));

        assert!(matches!(element.element_type, ElementType::Box));
        assert_eq!(element.children.len(), 3);
    }

    #[test]
    fn test_nested_macros() {
        let element = col! {
            text!("Header"),
            row! {
                text!("Left"),
                spacer!(),
                text!("Right"),
            },
            text!("Footer"),
        };

        assert_eq!(element.children.len(), 3);
        // Check the middle child (row) has 3 children
        let row_child = element.children.iter().nth(1).unwrap();
        assert_eq!(row_child.children.len(), 3);
    }
}
