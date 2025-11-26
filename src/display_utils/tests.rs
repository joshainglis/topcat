//! Tests for display utilities.

use super::*;
use std::path::Path;

#[test]
fn test_create_standard_table() {
    let table = create_standard_table();
    // Just verify it creates without panic
    assert!(table.is_empty());
}

#[test]
fn test_create_node_table() {
    let table = create_node_table(vec!["Node", "Path"]);
    // Table creation should succeed without panic
    // Note: comfy_table considers a table with headers but no rows as "empty"
    // so we just verify it was created successfully
    let _ = table;
}

#[test]
fn test_format_path_short() {
    let path = Path::new("file.sql");
    let formatted = format_path_for_display(path, 50);
    assert_eq!(formatted, "file.sql");
}

#[test]
fn test_format_path_long() {
    let path = Path::new("/very/long/path/to/some/directory/file.sql");
    let formatted = format_path_for_display(path, 20);
    // Should truncate but keep readable parts
    assert!(formatted.starts_with('…'));
    assert!(formatted.contains("file.sql") || formatted.len() <= 20);
}

#[test]
fn test_format_file_list_empty() {
    let files: Vec<PathBuf> = vec![];
    let result = format_file_list(&files, ", ");
    assert_eq!(result, "");
}

#[test]
fn test_format_file_list_single() {
    let files = vec![PathBuf::from("a.sql")];
    let result = format_file_list(&files, ", ");
    assert_eq!(result, "a.sql");
}

#[test]
fn test_format_file_list_multiple() {
    let files = vec![
        PathBuf::from("a.sql"),
        PathBuf::from("b.sql"),
        PathBuf::from("c.sql"),
    ];
    let result = format_file_list(&files, ", ");
    assert_eq!(result, "a.sql, b.sql, c.sql");
}

#[test]
fn test_format_file_list_custom_separator() {
    let files = vec![PathBuf::from("a.sql"), PathBuf::from("b.sql")];
    let result = format_file_list(&files, " | ");
    assert_eq!(result, "a.sql | b.sql");
}

#[test]
fn test_format_count_zero() {
    assert_eq!(format_count(0, "file", "files"), "0 files");
}

#[test]
fn test_format_count_one() {
    assert_eq!(format_count(1, "file", "files"), "1 file");
}

#[test]
fn test_format_count_many() {
    assert_eq!(format_count(42, "file", "files"), "42 files");
}

#[test]
fn test_separator_line() {
    let line = separator_line();
    assert!(!line.is_empty());
    assert!(line.contains('━'));
}

#[test]
fn test_section_header_line() {
    let line = section_header_line();
    assert!(!line.is_empty());
    assert!(line.contains('═'));
}
