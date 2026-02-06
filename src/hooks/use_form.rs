//! use_form hook for form state management
//!
//! Provides form state management with validation support.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//!
//! fn login_form() -> Element {
//!     let form = use_form(vec![
//!         ("username", ""),
//!         ("password", ""),
//!     ]);
//!
//!     // Get field value
//!     let username = form.get("username");
//!
//!     // Set field value
//!     form.set("username", "john");
//!
//!     // Check if form is valid
//!     if form.is_valid() {
//!         // Submit form
//!     }
//! }
//! ```

use crate::hooks::use_signal::{Signal, use_signal};
use std::collections::HashMap;

/// Form field with value and validation
#[derive(Clone, Debug)]
pub struct FormField {
    pub value: String,
    pub error: Option<String>,
    pub touched: bool,
}

impl FormField {
    fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            error: None,
            touched: false,
        }
    }
}

/// Handle for form operations
#[derive(Clone)]
pub struct FormHandle {
    fields: Signal<HashMap<String, FormField>>,
}

impl FormHandle {
    /// Get a field value
    pub fn get(&self, name: &str) -> String {
        self.fields.with(|f| {
            f.get(name)
                .map(|field| field.value.clone())
                .unwrap_or_default()
        })
    }

    /// Set a field value
    pub fn set(&self, name: &str, value: impl Into<String>) {
        let value = value.into();
        self.fields.update(|f| {
            if let Some(field) = f.get_mut(name) {
                field.value = value;
                field.touched = true;
            }
        });
    }

    /// Get a field's error
    pub fn error(&self, name: &str) -> Option<String> {
        self.fields
            .with(|f| f.get(name).and_then(|field| field.error.clone()))
    }

    /// Set a field's error
    pub fn set_error(&self, name: &str, error: impl Into<String>) {
        self.fields.update(|f| {
            if let Some(field) = f.get_mut(name) {
                field.error = Some(error.into());
            }
        });
    }

    /// Clear a field's error
    pub fn clear_error(&self, name: &str) {
        self.fields.update(|f| {
            if let Some(field) = f.get_mut(name) {
                field.error = None;
            }
        });
    }

    /// Check if a field has been touched
    pub fn is_touched(&self, name: &str) -> bool {
        self.fields
            .with(|f| f.get(name).map(|field| field.touched).unwrap_or(false))
    }

    /// Mark a field as touched
    pub fn touch(&self, name: &str) {
        self.fields.update(|f| {
            if let Some(field) = f.get_mut(name) {
                field.touched = true;
            }
        });
    }

    /// Check if the form is valid (no errors)
    pub fn is_valid(&self) -> bool {
        self.fields
            .with(|f| f.values().all(|field| field.error.is_none()))
    }

    /// Check if any field has been touched
    pub fn is_dirty(&self) -> bool {
        self.fields.with(|f| f.values().any(|field| field.touched))
    }

    /// Get all field values as a HashMap
    pub fn values(&self) -> HashMap<String, String> {
        self.fields.with(|f| {
            f.iter()
                .map(|(k, v)| (k.clone(), v.value.clone()))
                .collect()
        })
    }

    /// Reset all fields to initial values
    pub fn reset(&self, initial: Vec<(&str, &str)>) {
        self.fields.update(|f| {
            f.clear();
            for (name, value) in initial {
                f.insert(name.to_string(), FormField::new(value));
            }
        });
    }

    /// Clear all errors
    pub fn clear_errors(&self) {
        self.fields.update(|f| {
            for field in f.values_mut() {
                field.error = None;
            }
        });
    }

    /// Get all errors
    pub fn errors(&self) -> HashMap<String, String> {
        self.fields.with(|f| {
            f.iter()
                .filter_map(|(k, v)| v.error.as_ref().map(|e| (k.clone(), e.clone())))
                .collect()
        })
    }
}

/// Create a form state
pub fn use_form(initial: Vec<(&str, &str)>) -> FormHandle {
    let fields = use_signal(|| {
        initial
            .into_iter()
            .map(|(name, value)| (name.to_string(), FormField::new(value)))
            .collect()
    });
    FormHandle { fields }
}

/// Create an empty form state
pub fn use_form_empty() -> FormHandle {
    use_form(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_form_compiles() {
        fn _test() {
            let form = use_form(vec![("name", ""), ("email", "")]);
            let _ = form.get("name");
            form.set("name", "John");
            let _ = form.is_valid();
        }
    }

    #[test]
    fn test_form_field() {
        let field = FormField::new("test");
        assert_eq!(field.value, "test");
        assert!(!field.touched);
        assert!(field.error.is_none());
    }

    #[test]
    fn test_form_errors() {
        fn _test() {
            let form = use_form(vec![("email", "")]);
            form.set_error("email", "Invalid email");
            assert!(!form.is_valid());
            form.clear_error("email");
            assert!(form.is_valid());
        }
    }
}
