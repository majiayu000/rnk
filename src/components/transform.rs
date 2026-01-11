//! Transform component - Apply text transformations

use crate::core::{Element, ElementType, Style};

/// Text transformation function type
pub type TransformFn = Box<dyn Fn(&str) -> String + Send + Sync>;

/// Transform component for applying text transformations
///
/// # Example
///
/// ```ignore
/// Transform::new(|text| text.to_uppercase())
///     .child(Text::new("hello").into_element())
/// ```
pub struct Transform {
    transform: Option<TransformFn>,
    children: Vec<Element>,
}

impl Transform {
    /// Create a new transform with a transformation function
    pub fn new<F>(transform: F) -> Self
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        Self {
            transform: Some(Box::new(transform)),
            children: Vec::new(),
        }
    }

    /// Add a child element
    pub fn child(mut self, element: Element) -> Self {
        self.children.push(element);
        self
    }

    /// Add multiple children
    pub fn children(mut self, elements: impl IntoIterator<Item = Element>) -> Self {
        self.children.extend(elements);
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let mut element = Element::new(ElementType::Box);
        element.style = Style::new();

        // Apply transform to all text children
        for mut child in self.children {
            if let Some(ref transform) = self.transform
                && let Some(text) = child.text_content.take() {
                    child.text_content = Some(transform(&text));
                }
            element.add_child(child);
        }

        element
    }
}

/// Common text transformations
impl Transform {
    /// Create a transform that converts text to uppercase
    pub fn uppercase() -> Self {
        Self::new(|text| text.to_uppercase())
    }

    /// Create a transform that converts text to lowercase
    pub fn lowercase() -> Self {
        Self::new(|text| text.to_lowercase())
    }

    /// Create a transform that capitalizes text
    pub fn capitalize() -> Self {
        Self::new(|text| {
            let mut chars = text.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Text;

    #[test]
    fn test_transform_uppercase() {
        let element = Transform::uppercase()
            .child(Text::new("hello").into_element())
            .into_element();

        let child = element.children.iter().next().unwrap();
        assert_eq!(child.text_content, Some("HELLO".to_string()));
    }

    #[test]
    fn test_transform_lowercase() {
        let element = Transform::lowercase()
            .child(Text::new("HELLO").into_element())
            .into_element();

        let child = element.children.iter().next().unwrap();
        assert_eq!(child.text_content, Some("hello".to_string()));
    }

    #[test]
    fn test_transform_custom() {
        let element = Transform::new(|text| format!(">>> {} <<<", text))
            .child(Text::new("test").into_element())
            .into_element();

        let child = element.children.iter().next().unwrap();
        assert_eq!(child.text_content, Some(">>> test <<<".to_string()));
    }
}
