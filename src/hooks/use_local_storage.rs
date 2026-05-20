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
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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
        self.persist(&value);
        self.value.set(value);
    }

    /// Remove the stored value
    pub fn remove(&self) {
        if let Err(err) = self.remove_persisted() {
            panic!("failed to remove local storage key {:?}: {err}", self.key);
        }
        self.value.set(String::new());
    }

    /// Check if a value exists
    pub fn exists(&self) -> bool {
        !self.value.get().is_empty()
    }

    fn persist(&self, value: &str) {
        let path = storage_path_for_write(&self.storage_dir, &self.key).unwrap_or_else(|err| {
            panic!("failed to resolve local storage key {:?}: {err}", self.key)
        });
        fs::write(path, value).unwrap_or_else(|err| {
            panic!("failed to persist local storage key {:?}: {err}", self.key)
        });
    }

    fn remove_persisted(&self) -> io::Result<()> {
        let Some(path) = existing_storage_path(&self.storage_dir, &self.key)? else {
            return Ok(());
        };

        fs::remove_file(path)
    }

    fn load(storage_dir: &Path, key: &str) -> io::Result<Option<String>> {
        let Some(path) = existing_storage_path(storage_dir, key)? else {
            return Ok(None);
        };

        fs::read_to_string(path).map(Some)
    }
}

fn storage_file_name(key: &str) -> String {
    if is_safe_storage_key(key) {
        return key.to_owned();
    }

    encoded_storage_key(key)
}

fn is_safe_storage_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn encoded_storage_key(key: &str) -> String {
    let mut encoded = String::with_capacity("key_".len() + key.len() * 2);
    encoded.push_str("key_");

    for byte in key.as_bytes() {
        write!(&mut encoded, "{byte:02x}").expect("writing to String should not fail");
    }

    encoded
}

fn canonical_storage_root(storage_dir: &Path) -> io::Result<PathBuf> {
    let root = storage_dir.canonicalize()?;
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "local storage root is not a directory",
        ));
    }

    Ok(root)
}

fn verify_under_root(root: &Path, path: &Path) -> io::Result<()> {
    if path.starts_with(root) {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        "local storage path escaped storage root",
    ))
}

fn resolve_existing_path(root: &Path, path: &Path) -> io::Result<Option<PathBuf>> {
    match path.canonicalize() {
        Ok(resolved) => {
            verify_under_root(root, &resolved)?;
            Ok(Some(resolved))
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

fn existing_storage_path(storage_dir: &Path, key: &str) -> io::Result<Option<PathBuf>> {
    let root = match canonical_storage_root(storage_dir) {
        Ok(root) => root,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };
    let path = root.join(storage_file_name(key));
    verify_under_root(&root, &path)?;
    resolve_existing_path(&root, &path)
}

fn storage_path_for_write(storage_dir: &Path, key: &str) -> io::Result<PathBuf> {
    fs::create_dir_all(storage_dir)?;
    let root = canonical_storage_root(storage_dir)?;
    let path = root.join(storage_file_name(key));
    verify_under_root(&root, &path)?;

    resolve_existing_path(&root, &path).map(|existing| existing.unwrap_or(path))
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

    let initial = LocalStorageHandle::load(&storage_dir, &key)
        .unwrap_or_else(|err| panic!("failed to load local storage key {key:?}: {err}"))
        .unwrap_or(default);
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

    let initial = LocalStorageHandle::load(&storage_dir, &key)
        .unwrap_or_else(|err| panic!("failed to load local storage key {key:?}: {err}"))
        .unwrap_or(default);
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
