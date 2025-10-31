// External usage checker - scans external files to determine if SQL objects are referenced

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use glob::glob;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;

/// Checks if SQL object names are used in external code files
pub struct ExternalUsageChecker {
    /// Cache of file contents for fast searching
    file_cache: Vec<(PathBuf, String)>,
    /// Whether to show progress bars
    show_progress: bool,
}

impl ExternalUsageChecker {
    /// Create a new ExternalUsageChecker by scanning directories
    ///
    /// # Arguments
    /// * `check_dirs` - Directories to search recursively
    /// * `file_patterns` - File patterns to match (e.g., ["*.py", "*.rs"])
    /// * `show_progress` - Whether to display progress bars
    pub fn new(
        check_dirs: &[PathBuf],
        file_patterns: &[String],
        show_progress: bool,
    ) -> Result<Self, String> {
        let mut all_files = Vec::new();

        // Collect all matching files from all directories
        for check_dir in check_dirs {
            if !check_dir.exists() {
                return Err(format!(
                    "External check directory does not exist: {}",
                    check_dir.display()
                ));
            }

            if !check_dir.is_dir() {
                return Err(format!(
                    "External check path is not a directory: {}",
                    check_dir.display()
                ));
            }

            for pattern in file_patterns {
                // Create glob pattern: dir/**/*.ext
                let glob_pattern = format!("{}/**/{}", check_dir.display(), pattern);

                match glob(&glob_pattern) {
                    Ok(entries) => {
                        for entry in entries.flatten() {
                            if entry.is_file() {
                                all_files.push(entry);
                            }
                        }
                    }
                    Err(e) => {
                        return Err(format!("Invalid glob pattern '{glob_pattern}': {e}"));
                    }
                }
            }
        }

        if show_progress {
            println!(
                "🔍 Building cache for {} external files...",
                all_files.len()
            );
        }

        // Build file cache with progress bar
        let file_cache = if show_progress {
            let pb = ProgressBar::new(all_files.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(
                        "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
                    )
                    .unwrap()
                    .progress_chars("#>-"),
            );

            let cache: Vec<(PathBuf, String)> = all_files
                .par_iter()
                .filter_map(|path| {
                    pb.inc(1);
                    Self::read_file_cached(path)
                })
                .collect();

            pb.finish_with_message("Cache built");
            cache
        } else {
            all_files
                .par_iter()
                .filter_map(|path| Self::read_file_cached(path))
                .collect()
        };

        if show_progress {
            println!("   ✓ Cached {} files", file_cache.len());
        }

        Ok(ExternalUsageChecker {
            file_cache,
            show_progress,
        })
    }

    /// Read a file and add it to the cache
    fn read_file_cached(path: &Path) -> Option<(PathBuf, String)> {
        match fs::read_to_string(path) {
            Ok(content) => Some((path.to_path_buf(), content)),
            Err(_) => None, // Skip files that can't be read
        }
    }

    /// Check if a node name is used in any of the cached external files
    ///
    /// Returns the first file path where the node was found, or None if not found
    pub fn is_used(&self, node_name: &str) -> Option<&PathBuf> {
        self.file_cache
            .par_iter()
            .find_any(|(_, content)| content.contains(node_name))
            .map(|(path, _)| path)
    }

    /// Filter a set of node names to only those NOT used externally
    ///
    /// Returns a new set containing only nodes that are not referenced in external files
    pub fn filter_unused(&self, node_names: &HashSet<String>) -> HashSet<String> {
        if self.show_progress {
            println!(
                "\n🔍 Checking {} nodes for external usage...",
                node_names.len()
            );
        }

        let pb = if self.show_progress {
            let pb = ProgressBar::new(node_names.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({eta})")
                    .unwrap()
                    .progress_chars("#>-"),
            );
            Some(pb)
        } else {
            None
        };

        let unused: HashSet<String> = node_names
            .par_iter()
            .filter(|name| {
                if let Some(ref pb) = pb {
                    pb.inc(1);
                }
                self.is_used(name).is_none()
            })
            .cloned()
            .collect();

        if let Some(pb) = pb {
            pb.finish_with_message("External usage check complete");
        }

        let excluded_count = node_names.len() - unused.len();
        if self.show_progress && excluded_count > 0 {
            println!("   ✓ Excluded {excluded_count} node(s) found in external code");
        }

        unused
    }

    /// Get the number of files in the cache
    pub fn cache_size(&self) -> usize {
        self.file_cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_external_usage_checker() {
        // Create temp directory with test files
        let dir = tempdir().unwrap();
        let file1_path = dir.path().join("test1.py");
        let file2_path = dir.path().join("test2.py");

        let mut file1 = File::create(&file1_path).unwrap();
        writeln!(file1, "# Using schema.users table").unwrap();
        writeln!(file1, "SELECT * FROM schema.users").unwrap();

        let mut file2 = File::create(&file2_path).unwrap();
        writeln!(file2, "# Using schema.products").unwrap();
        writeln!(file2, "products = db.query('schema.products')").unwrap();

        // Create checker
        let checker =
            ExternalUsageChecker::new(&[dir.path().to_path_buf()], &["*.py".to_string()], false)
                .unwrap();

        assert_eq!(checker.cache_size(), 2);

        // Test is_used
        assert!(checker.is_used("schema.users").is_some());
        assert!(checker.is_used("schema.products").is_some());
        assert!(checker.is_used("schema.orders").is_none());

        // Test filter_unused
        let mut nodes = HashSet::new();
        nodes.insert("schema.users".to_string());
        nodes.insert("schema.products".to_string());
        nodes.insert("schema.orders".to_string());
        nodes.insert("schema.categories".to_string());

        let unused = checker.filter_unused(&nodes);
        assert_eq!(unused.len(), 2);
        assert!(unused.contains("schema.orders"));
        assert!(unused.contains("schema.categories"));
        assert!(!unused.contains("schema.users"));
        assert!(!unused.contains("schema.products"));
    }
}
