//! Configuration validation for Settings.

use super::{HeaderUpdateMode, Settings};

impl Settings {
    /// Validate the configuration.
    ///
    /// Checks:
    /// - At least one layer is defined
    /// - Fallback layer exists in layers list
    /// - Auto-mapping patterns compile and reference valid layers
    /// - `rename_files` is only set when header updates are enabled
    /// - All regex patterns are valid (root_regex, schema_pattern, object_pattern, soft_deps_mapping)
    pub fn validate(&self) -> Result<(), String> {
        // Validate layers
        if self.layers.names.is_empty() {
            return Err("At least one layer must be specified".to_string());
        }

        if !self.layers.names.contains(&self.layers.fallback) {
            return Err(format!(
                "Fallback layer '{}' is not in layers list",
                self.layers.fallback
            ));
        }

        // Validate auto_mapping patterns
        for (pattern, layer) in &self.layers.auto_mapping {
            regex::Regex::new(pattern)
                .map_err(|e| format!("Invalid regex pattern in auto_mapping '{pattern}': {e}"))?;

            if !self.layers.names.contains(layer) {
                return Err(format!(
                    "Auto-mapping pattern '{pattern}' references unknown layer '{layer}'. \
                     Valid layers: {}",
                    self.layers.names.join(", ")
                ));
            }
        }

        // Validate rename_files requires header updates
        if self.rename_files && self.header_update_mode == HeaderUpdateMode::Never {
            return Err(
                "Cannot rename files without header updates. \
                 Use --update-headers or --generate-headers with --rename-files"
                    .to_string(),
            );
        }

        // Validate SQL discovery regex patterns
        if let Some(ref pattern) = self.sql_discovery.schema_pattern {
            regex::Regex::new(pattern)
                .map_err(|e| format!("Invalid schema_pattern regex '{pattern}': {e}"))?;
        }

        if let Some(ref pattern) = self.sql_discovery.object_pattern {
            regex::Regex::new(pattern)
                .map_err(|e| format!("Invalid object_pattern regex '{pattern}': {e}"))?;
        }

        // Validate soft_deps_mappings patterns
        for (node_pattern, dep_pattern) in &self.sql_discovery.soft_deps_mappings {
            regex::Regex::new(node_pattern)
                .map_err(|e| format!("Invalid soft_deps_mapping node pattern '{node_pattern}': {e}"))?;
            regex::Regex::new(dep_pattern)
                .map_err(|e| format!("Invalid soft_deps_mapping dep pattern '{dep_pattern}': {e}"))?;
        }

        // Validate root_regex patterns
        for pattern in &self.analysis.root_regex {
            regex::Regex::new(pattern)
                .map_err(|e| format!("Invalid root_regex pattern '{pattern}': {e}"))?;
        }

        Ok(())
    }
}
