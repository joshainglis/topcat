//! Soft dependency handling and mapping for FileNode.

use super::FileNode;

impl FileNode {
    /// Check if a dependency should be converted from `requires` to `exists`
    /// based on the configured soft dependency mapper patterns.
    ///
    /// # Arguments
    /// * `dep` - The name of the dependency to check
    ///
    /// # Returns
    /// `true` if the dependency should be converted to `exists`, `false` otherwise
    ///
    /// # Example
    /// ```
    /// use topcat::file_node::FileNode;
    /// use topcat::soft_deps_matcher::SoftDepsMapper;
    /// use indexmap::IndexMap;
    /// use std::path::PathBuf;
    /// use std::collections::HashSet;
    /// use std::rc::Rc;
    ///
    /// let mut node = FileNode::new(
    ///     "codegen_tmf.util_func".to_string(),
    ///     PathBuf::from("test.sql"),
    ///     HashSet::new(),
    ///     "normal".to_string(),
    ///     false,
    ///     HashSet::new(),
    /// );
    ///
    /// let mut mappings = IndexMap::new();
    /// mappings.insert(r"^codegen_tmf\b".to_string(), r"^c_tmf\b".to_string());
    /// let mapper = SoftDepsMapper::new(&mappings).unwrap();
    /// node.soft_deps_mapper = Some(Rc::new(mapper));
    ///
    /// // This dependency should be converted
    /// assert!(node.should_convert_dep_to_exists("c_tmf.ta_headers"));
    /// // This dependency should not be converted
    /// assert!(!node.should_convert_dep_to_exists("md_tmf.table"));
    /// ```
    pub fn should_convert_dep_to_exists(&self, dep: &str) -> bool {
        if let Some(ref mapper) = self.soft_deps_mapper {
            mapper.should_convert(&self.name, dep)
        } else {
            false
        }
    }

    /// Apply soft dependency mappings to convert matching dependencies from `requires` to `exists`.
    /// This moves dependencies from `deps` to `ensure_exists` based on the configured patterns.
    ///
    /// This should be called after dependency discovery and merging, but before graph validation.
    ///
    /// **Note**: By default, this will convert ALL dependencies that match the pattern, including
    /// schema dependencies. The node's own schema dependency is skipped to prevent self-dependency
    /// conversion.
    pub fn apply_soft_deps_mappings(&mut self) {
        if let Some(ref mapper) = self.soft_deps_mapper {
            // Extract this node's schema name if present (to skip self-schema dependency)
            let own_schema = if self.name.contains('.') {
                self.name.split('.').next()
            } else {
                None
            };

            // Find all dependencies that should be converted
            let deps_to_convert: Vec<String> = self
                .deps
                .iter()
                .filter(|dep| {
                    // Skip this node's own schema dependency (e.g., codegen_tmf.func depending on codegen_tmf)
                    if let Some(schema) = own_schema {
                        if dep.as_str() == schema {
                            log::trace!("Skipping own schema dependency: {} -> {}", self.name, dep);
                            return false;
                        }
                    }
                    // Check if pattern matches
                    let should_convert = mapper.should_convert(&self.name, dep);
                    if should_convert {
                        log::debug!("Converting dependency to exists: {} -> {}", self.name, dep);
                    }
                    should_convert
                })
                .cloned()
                .collect();

            // Move them from deps to ensure_exists
            for dep in deps_to_convert {
                self.deps.remove(&dep);
                self.ensure_exists.insert(dep);
            }
        }
    }

    /// Merge discovered dependencies with manual dependencies based on the strategy
    pub fn merge_dependencies(&mut self, merge_strategy: crate::sql_config::MergeStrategy) {
        use crate::sql_config::MergeStrategy;

        let Some(ref discovered) = self.discovered_deps else {
            // No discovered dependencies, nothing to merge
            return;
        };

        match merge_strategy {
            MergeStrategy::HeaderOnly => {
                // Keep only manual dependencies
            }
            MergeStrategy::DiscoveryOnly => {
                // Replace deps with discovered, but keep override_deps
                self.deps = discovered.clone();
                // Add back override deps
                self.deps.extend(self.override_deps.clone());
            }
            MergeStrategy::Union => {
                // Combine both
                self.deps.extend(discovered.clone());
            }
            MergeStrategy::HeaderWithFallback => {
                // If deps is empty (no manual deps), use discovered
                if self.deps.is_empty() {
                    self.deps = discovered.clone();
                }
                // Always add override deps
                self.deps.extend(self.override_deps.clone());
            }
            MergeStrategy::Validate => {
                // Check for discrepancies
                let manual_only: std::collections::HashSet<_> = self.deps.difference(discovered).collect();
                let discovered_only: std::collections::HashSet<_> = discovered.difference(&self.deps).collect();

                if !manual_only.is_empty() || !discovered_only.is_empty() {
                    log::warn!(
                        "Dependency mismatch in {}: manual-only={:?}, discovered-only={:?}",
                        self.path.display(),
                        manual_only,
                        discovered_only
                    );
                }
                // For validate mode, we'll use union to include everything
                self.deps.extend(discovered.clone());
            }
        }
    }
}
