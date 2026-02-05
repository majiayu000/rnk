//! use_local_storage hook for persistent storage
//!
//! Provides file-based persistent storage for terminal applications.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//!
//! fn app() -> Element {
//!     let theme = use_local_storage("theme", "dark");
//!
//!     use_input(move |input, _| {
//!         if input == "t" {
//!             let current = theme.get();
//!             theme.set(if current == "dark" { "light" } else { "dark" });
//!         }
//!     });
//!
//!     Text::new(format!("Theme: {}", theme.get())).into_element()
//! }
//! ```

use crate::hooks::use_signal::{Signal, use_signal};
use std::fs;
use std::path::PathBuf;

/// Handle for local storage operations
#[derive(Clone)]
pub struct LocalStorageHandle {
    key: String,
    value: Signal<String>,
    storage_dir: PathBuf,
}

impl LocalStorageHandle {
    /// Get the current value
    pub fn get(&self) -> String {
        self.value.get()
    }

    /// Set a new value and persist it
    pub fn set(&self, value: impl Into<String>) {
        let value = value.into();
        self.value.set(value.clone());
        self.persist(&value);
    }

    /// Remove the stored value
    pub fn remove(&self) {
        let path = self.storage_dir.join(&self.key);
        let _ = fs::remove_file(path);
        self.value.set(String::new());
    }

    /// Check if a value exists
    pub fn exists(&self) -> bool {
        !self.value.get().is_empty()
    }

    fn persist(&self, value: &str) {
        let _ = fs::create_dir_all(&self.storage_dir);
        let path = self.storage_dir.join(&self.key);
        let _ = fs::write(path, value);
    }

    fn load(storage_dir: &PathBuf, key: &str) -> Option<String> {
        let path = storage_dir.join(key);
        fs::read_to_string(path).ok()
    }
}

/// Get the default storage directory
fn default_storage_dir() -> PathBuf {
    dirs_next::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rnk")
        .join("storage")
}

/// Create a local storage handle
pub fn use_local_storage(key: impl Into<String>, default: impl Into<String>) -> LocalStorageHandle {
    let key = key.into();
    let default = default.into();
    let storage_dir = default_storage_dir();

    let initial = LocalStorageHandle::load(&storage_dir, &key).unwrap_or(default);
    let value = use_signal(|| initial);

    LocalStorageHandle {
        key,
        value,
        storage_dir,
    }
}

/// Create a local storage handle with custom directory
pub fn use_local_storage_with_dir(
    key: impl Into<String>,
    default: impl Into<String>,
    dir: impl Into<PathBuf>,
) -> LocalStorageHandle {
    let key = key.into();
    let default = default.into();
    let storage_dir = dir.into();

    let initial = LocalStorageHandle::load(&storage_dir, &key).unwrap_or(default);
    let value = use_signal(|| initial);

    LocalStorageHandle {
        key,
        value,
        storage_dir,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_storage_dir() {
        let dir = default_storage_dir();
        assert!(dir.to_string_lossy().contains("rnk"));
    }

    #[test]
    fn test_use_local_storage_compiles() {
        fn _test() {
            let storage = use_local_storage("key", "default");
            let _ = storage.get();
            storage.set("new value");
            let _ = storage.exists();
        }
    }

    #[test]
    fn test_use_local_storage_with_dir_compiles() {
        fn _test() {
            let storage = use_local_storage_with_dir("key", "default", "/tmp/test");
            let _ = storage.get();
        }
    }
}
