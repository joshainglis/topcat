//! Common utilities for clean commands.
//!
//! This module provides shared functionality used across different clean operations:
//! - `show_deletion_preview`: Display files that will be deleted
//! - `perform_deletion`: Actually delete files with confirmation
//! - `filter_by_root_matcher`: Filter out protected root nodes

use std::collections::HashSet;
use std::io::{self, Write};

use comfy_table::{Cell, Color, Table};

use topcat::analysis::root_matcher::RootNodeMatcher;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;

/// Display a preview of files that will be deleted.
///
/// Shows a formatted table with node names and file paths for all files
/// that will be deleted.
///
/// # Arguments
///
/// * `graph` - The dependency graph
/// * `nodes_to_delete` - Set of node names to delete
/// * `category` - Category label for the output (e.g., "Dead Branches")
///
/// # Returns
///
/// `Ok(())` on success, `Err(TopCatError)` on error
pub fn show_deletion_preview(
    graph: &TCGraph,
    nodes_to_delete: &HashSet<String>,
    category: &str,
) -> Result<(), TopCatError> {
    let node_to_path = graph.build_node_to_path_map();

    let mut table = Table::new();
    table.set_header(vec!["Node Name", "File Path"]);

    let mut paths = Vec::new();
    for node in nodes_to_delete {
        if let Some(path) = node_to_path.get(node) {
            paths.push((node.clone(), path.clone()));
        }
    }

    // Sort for consistent output
    paths.sort_by(|a, b| a.1.cmp(&b.1));

    for (node, path) in &paths {
        table.add_row(vec![
            Cell::new(node).fg(Color::Red),
            Cell::new(path.display()).fg(Color::Red),
        ]);
    }

    println!(
        "📋 {category} to be deleted ({} files):\n",
        nodes_to_delete.len()
    );
    println!("{table}\n");

    Ok(())
}

/// Perform the actual deletion of files.
///
/// Prompts for confirmation (unless `force` is true), then deletes all files
/// in the provided set. Shows a summary of results.
///
/// # Arguments
///
/// * `graph` - The dependency graph
/// * `nodes_to_delete` - Set of node names to delete
/// * `force` - Skip confirmation prompt if true
/// * `verbose` - Show each file as it's deleted
///
/// # Returns
///
/// `Ok(())` if all files deleted successfully, `Err(TopCatError)` if any deletions failed
pub fn perform_deletion(
    graph: &TCGraph,
    nodes_to_delete: &HashSet<String>,
    force: bool,
    verbose: bool,
) -> Result<(), TopCatError> {
    if nodes_to_delete.is_empty() {
        return Ok(());
    }

    // Ask for confirmation unless --force is set
    if !force {
        print!(
            "\n⚠️  Are you sure you want to delete {} files? [y/N]: ",
            nodes_to_delete.len()
        );
        io::stdout().flush().unwrap();

        let mut response = String::new();
        io::stdin().read_line(&mut response).unwrap();
        let response = response.trim().to_lowercase();

        if response != "y" && response != "yes" {
            println!("❌ Deletion cancelled");
            return Ok(());
        }
    }

    println!("\n🗑️  Deleting files...\n");

    let node_to_path = graph.build_node_to_path_map();
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut errors = Vec::new();

    for node in nodes_to_delete {
        if let Some(path) = node_to_path.get(node) {
            match std::fs::remove_file(path) {
                Ok(_) => {
                    success_count += 1;
                    if verbose {
                        println!("✅ Deleted: {}", path.display());
                    }
                }
                Err(e) => {
                    failure_count += 1;
                    let error_msg = format!("Failed to delete {}: {e}", path.display());
                    eprintln!("❌ {error_msg}");
                    errors.push(error_msg);
                }
            }
        }
    }

    // Show summary
    println!("\n📊 Deletion Summary:");
    println!("  ✅ Successfully deleted: {success_count} files");
    if failure_count > 0 {
        println!("  ❌ Failed to delete: {failure_count} files");
    }

    if failure_count > 0 {
        Err(TopCatError::Io(std::io::Error::other(format!(
            "Failed to delete {failure_count} file(s)"
        ))))
    } else {
        println!("\n✅ All files deleted successfully!");
        Ok(())
    }
}

/// Filter out root nodes from a set based on the root matcher.
///
/// Removes any nodes that match the root patterns, protecting them from deletion.
/// Prints a message indicating how many nodes were protected.
///
/// # Arguments
///
/// * `nodes` - Set of node names to filter
/// * `graph` - The dependency graph
/// * `root_matcher` - Optional root matcher to apply
///
/// # Returns
///
/// The filtered set with root nodes removed
pub fn filter_by_root_matcher(
    mut nodes: HashSet<String>,
    graph: &TCGraph,
    root_matcher: Option<&RootNodeMatcher>,
) -> HashSet<String> {
    if let Some(matcher) = root_matcher {
        let node_to_path = graph.build_node_to_path_map();
        let before_count = nodes.len();
        nodes.retain(|node| {
            if let Some(path) = node_to_path.get(node) {
                !matcher.is_root(node, path)
            } else {
                true
            }
        });
        let filtered_count = before_count - nodes.len();
        if filtered_count > 0 {
            println!("🔒 Protected {filtered_count} root node(s) from deletion");
        }
    }
    nodes
}
