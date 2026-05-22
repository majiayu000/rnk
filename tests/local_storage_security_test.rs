use rnk::hooks::use_local_storage_with_dir;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(test_name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("rnk-{test_name}-{suffix}"));
    fs::create_dir_all(&dir).expect("failed to create temp test dir");
    dir
}

fn cleanup(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

#[test]
fn local_storage_parent_directory_key_does_not_read_outside_storage_dir() {
    let root = unique_temp_dir("parent-read");
    let storage_dir = root.join("storage");
    fs::create_dir_all(&storage_dir).expect("failed to create storage dir");
    fs::write(root.join("outside"), "secret").expect("failed to create outside file");

    let storage = use_local_storage_with_dir("../outside", "default", &storage_dir);
    let value = storage.get();

    cleanup(&root);
    assert_eq!(value, "default");
}

#[test]
fn local_storage_parent_directory_key_does_not_write_outside_storage_dir() {
    let root = unique_temp_dir("parent-write");
    let storage_dir = root.join("storage");
    let outside = root.join("outside");

    let storage = use_local_storage_with_dir("../outside", "default", &storage_dir);
    storage.set("secret");
    let escaped = fs::read_to_string(&outside).ok();

    cleanup(&root);
    assert!(escaped.is_none());
}

#[test]
fn local_storage_absolute_key_does_not_write_outside_storage_dir() {
    let root = unique_temp_dir("absolute-write");
    let storage_dir = root.join("storage");
    let outside = root.join("outside");

    let storage = use_local_storage_with_dir(outside.to_string_lossy(), "default", &storage_dir);
    storage.set("secret");
    let escaped = fs::read_to_string(&outside).ok();

    cleanup(&root);
    assert!(escaped.is_none());
}

#[test]
fn local_storage_separator_key_is_stored_as_single_safe_file() {
    let root = unique_temp_dir("separator-write");
    let storage_dir = root.join("storage");

    let storage = use_local_storage_with_dir("nested/key", "default", &storage_dir);
    storage.set("secret");
    let nested_dir_exists = storage_dir.join("nested").exists();
    let file_count = fs::read_dir(&storage_dir)
        .expect("storage dir should exist")
        .count();

    cleanup(&root);
    assert!(!nested_dir_exists);
    assert_eq!(file_count, 1);
}

#[test]
fn local_storage_safe_key_preserves_plain_file_name() {
    let root = unique_temp_dir("safe-key");
    let storage_dir = root.join("storage");

    let storage = use_local_storage_with_dir("theme_1", "default", &storage_dir);
    storage.set("dark");
    let plain_file_value = fs::read_to_string(storage_dir.join("theme_1")).ok();

    cleanup(&root);
    assert_eq!(plain_file_value.as_deref(), Some("dark"));
}

#[test]
fn local_storage_remove_does_not_delete_outside_storage_dir() {
    let root = unique_temp_dir("parent-remove");
    let storage_dir = root.join("storage");
    fs::create_dir_all(&storage_dir).expect("failed to create storage dir");
    let outside = root.join("outside");
    fs::write(&outside, "secret").expect("failed to create outside file");

    let storage = use_local_storage_with_dir("../outside", "default", &storage_dir);
    storage.remove();
    let outside_value = fs::read_to_string(&outside).ok();

    cleanup(&root);
    assert_eq!(outside_value.as_deref(), Some("secret"));
}
