# Root Nodes / Entry Points Feature Design

## Overview

Add the ability to mark certain files as "root nodes" or "entry points" that should never be considered dead, regardless of whether other files in the dependency graph depend on them. This makes the dead branches analysis more practical and safer for production use.

## Motivation

### Current Behavior
Without external usage checking, the dead branches algorithm correctly identifies all nodes that lead to leaf nodes (files with no dependents) as "dead". In a closed system, this means:

```
a -> b -> c (leaf)
```

All three nodes (a, b, c) are marked as dead because `c` has no dependents, making `b` dead, which makes `a` dead.

### The Problem
In real-world scenarios, certain files ARE entry points that should never be deleted:
- API endpoint handlers
- Migration runners
- CLI entry points
- Scheduled job definitions
- Main application files

These files don't have dependents within the SQL/codebase but are referenced externally (by application code, schedulers, etc.).

### The Solution: Root Nodes
Allow users to explicitly mark files as "root nodes" that:
1. Should never be considered dead
2. Protect their entire dependency tree from being marked as dead
3. Can be specified via CLI, config file, or both

## Implementation Design

### 1. Data Structure: `RootNodeMatcher`

```rust
// src/analysis/root_matcher.rs (new file)

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use regex::Regex;

pub struct RootNodeMatcher {
    /// Specific node names to treat as roots (exact matches)
    specific_nodes: HashSet<String>,

    /// Glob patterns for file paths (e.g., "**/api/*.sql")
    glob_patterns: Vec<String>,

    /// Compiled regex patterns for node names (e.g., "^api_.*")
    regex_patterns: Vec<Regex>,

    /// Directory prefixes - all files under these paths are roots
    directory_roots: Vec<PathBuf>,
}

impl RootNodeMatcher {
    pub fn new(
        specific_nodes: Vec<String>,
        glob_patterns: Vec<String>,
        regex_patterns: Vec<String>,
        directory_roots: Vec<PathBuf>,
    ) -> Result<Self, String> {
        // Compile regex patterns
        let compiled_regex: Result<Vec<Regex>, _> = regex_patterns
            .iter()
            .map(|pattern| Regex::new(pattern))
            .collect();

        let compiled_regex = compiled_regex
            .map_err(|e| format!("Invalid regex pattern: {}", e))?;

        Ok(Self {
            specific_nodes: specific_nodes.into_iter().collect(),
            glob_patterns,
            regex_patterns: compiled_regex,
            directory_roots,
        })
    }

    /// Check if a node should be treated as a root
    pub fn is_root(&self, node_name: &str, file_path: &Path) -> bool {
        // Check specific nodes
        if self.specific_nodes.contains(node_name) {
            return true;
        }

        // Check regex patterns on node name
        if self.regex_patterns.iter().any(|re| re.is_match(node_name)) {
            return true;
        }

        // Check glob patterns on file path
        for pattern in &self.glob_patterns {
            if let Ok(glob_matcher) = glob::Pattern::new(pattern) {
                if glob_matcher.matches_path(file_path) {
                    return true;
                }
            }
        }

        // Check directory roots
        for root_dir in &self.directory_roots {
            if file_path.starts_with(root_dir) {
                return true;
            }
        }

        false
    }

    /// Filter a set of nodes to only include non-root nodes
    pub fn filter_non_roots(
        &self,
        nodes: &HashSet<String>,
        node_to_path: &HashMap<String, PathBuf>,
    ) -> HashSet<String> {
        nodes
            .iter()
            .filter(|node_name| {
                if let Some(path) = node_to_path.get(*node_name) {
                    !self.is_root(node_name, path)
                } else {
                    true // Keep if path not found
                }
            })
            .cloned()
            .collect()
    }
}
```

### 2. CLI Arguments

Add to `AnalyzeArgs` in `src/commands/analyze.rs`:

```rust
/// Analyze dependency structure and find cleanup candidates
#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    // ... existing fields ...

    // Root Node Configuration
    #[arg(
        long = "root-nodes",
        help = "Specific node names to always treat as roots (can specify multiple times)",
        value_name = "NODE"
    )]
    root_nodes: Vec<String>,

    #[arg(
        long = "root-pattern",
        help = "Glob patterns for root files (e.g., '**/api/*.sql', can specify multiple times)",
        value_name = "PATTERN"
    )]
    root_patterns: Vec<String>,

    #[arg(
        long = "root-regex",
        help = "Regex patterns for root node names (e.g., '^api_.*', can specify multiple times)",
        value_name = "REGEX"
    )]
    root_regex: Vec<String>,

    #[arg(
        long = "root-dir",
        help = "Directories whose files are all roots (can specify multiple times)",
        value_name = "DIR"
    )]
    root_dirs: Vec<PathBuf>,

    #[command(subcommand)]
    command: AnalyzeCommand,
}
```

### 3. Config File Support

Extend `TopcatConfig` in `src/sql_config.rs`:

```rust
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AnalysisConfig {
    /// Specific nodes to always treat as root/entry points
    #[serde(default)]
    pub root_nodes: Vec<String>,

    /// Glob patterns for root files
    #[serde(default)]
    pub root_patterns: Vec<String>,

    /// Regex patterns for root node names
    #[serde(default)]
    pub root_regex: Vec<String>,

    /// Directories where all files are considered roots
    #[serde(default)]
    pub root_dirs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TopcatConfig {
    #[serde(default)]
    pub sql_discovery: SqlDiscoveryConfig,

    #[serde(default)]
    pub analysis: AnalysisConfig,  // NEW
}
```

Example `topcat.toml`:

```toml
[sql_discovery]
enabled = true
schema_pattern = "myapp_\\w+"

[analysis]
# Specific nodes to always treat as root/entry points
root_nodes = ["api_main", "worker_main", "migration_runner"]

# Glob patterns for root files
root_patterns = ["**/api/*.sql", "**/workers/*.sql", "**/migrations/*.sql"]

# Regex patterns for node names
root_regex = ["^api_.*", "^worker_.*", "^migration_\\d+"]

# Directories where all files are considered roots
root_dirs = ["sql/entry_points/", "sql/migrations/"]
```

### 4. Integration with Analysis

Modify `find_dead_branches()` in `src/analysis/mod.rs`:

```rust
// Update the trait to accept optional root matcher
pub trait GraphAnalyzer {
    fn find_orphans(&self) -> HashSet<String>;
    fn find_unrequired(&self) -> HashSet<String>;
    fn find_leaf_nodes(&self) -> HashSet<String>;
    fn find_root_nodes(&self) -> HashSet<String>;

    // NEW: Accept optional root matcher
    fn find_dead_branches(&self, root_matcher: Option<&RootNodeMatcher>) -> HashSet<String>;
}

impl GraphAnalyzer for TCGraph {
    // ... existing methods ...

    fn find_dead_branches(&self, root_matcher: Option<&RootNodeMatcher>) -> HashSet<String> {
        // Start with leaf nodes (files with dependencies but no dependents)
        let mut dead_nodes = self.find_leaf_nodes();

        // Remove root nodes from consideration
        if let Some(matcher) = root_matcher {
            let node_to_path = self.build_node_to_path_map();
            dead_nodes = matcher.filter_non_roots(&dead_nodes, &node_to_path);
        }

        if dead_nodes.is_empty() {
            return dead_nodes;
        }

        // Build dependents map for efficient lookup
        let dependents_map = self.build_dependents_map();
        let all_nodes = self.get_all_nodes();

        // Iteratively add nodes whose only dependents are already in dead_nodes
        let mut changed = true;
        while changed {
            changed = false;

            for node in &all_nodes {
                // Skip nodes already identified as dead
                if dead_nodes.contains(&node.name) {
                    continue;
                }

                // Skip root nodes
                if let Some(matcher) = root_matcher {
                    if matcher.is_root(&node.name, &node.path) {
                        continue;
                    }
                }

                // Get dependents of this node
                let dependents = dependents_map.get(&node.name);
                if dependents.is_none() {
                    continue;
                }

                let dependents = dependents.unwrap();

                // Check if ALL dependents are in dead_nodes
                if !dependents.is_empty() && dependents.iter().all(|d| dead_nodes.contains(d)) {
                    dead_nodes.insert(node.name.clone());
                    changed = true;
                }
            }
        }

        dead_nodes
    }
}

// Helper method for TCGraph
impl TCGraph {
    fn build_node_to_path_map(&self) -> HashMap<String, PathBuf> {
        self.get_all_nodes()
            .iter()
            .map(|node| (node.name.clone(), node.path.clone()))
            .collect()
    }
}
```

### 5. Update Command Execution

In `src/commands/analyze.rs`:

```rust
impl AnalyzeArgs {
    pub fn execute(&self) -> Result<(), TopCatError> {
        // ... existing setup ...

        // Build root matcher from CLI args and config
        let root_matcher = self.build_root_matcher()?;

        // Execute the requested analysis
        match &self.command {
            AnalyzeCommand::DeadBranches => {
                self.analyze_dead_branches(&graph, external_checker.as_ref(), root_matcher.as_ref())
            }
            // ... other commands ...
        }
    }

    fn build_root_matcher(&self) -> Result<Option<RootNodeMatcher>, TopCatError> {
        // Load from config file if specified
        let mut config_roots = if let Some(ref config_path) = self.sql_config_file {
            let config = TopcatConfig::from_file(config_path)
                .map_err(|e| TopCatError::ConfigError(format!("Failed to load config: {}", e)))?;
            config.analysis
        } else {
            AnalysisConfig::default()
        };

        // Merge CLI args (CLI overrides config)
        if !self.root_nodes.is_empty() {
            config_roots.root_nodes.extend(self.root_nodes.clone());
        }
        if !self.root_patterns.is_empty() {
            config_roots.root_patterns.extend(self.root_patterns.clone());
        }
        if !self.root_regex.is_empty() {
            config_roots.root_regex.extend(self.root_regex.clone());
        }
        if !self.root_dirs.is_empty() {
            config_roots.root_dirs.extend(
                self.root_dirs.iter().map(|p| p.to_string_lossy().to_string())
            );
        }

        // Only create matcher if any patterns were specified
        if config_roots.root_nodes.is_empty()
            && config_roots.root_patterns.is_empty()
            && config_roots.root_regex.is_empty()
            && config_roots.root_dirs.is_empty()
        {
            return Ok(None);
        }

        let dirs: Vec<PathBuf> = config_roots.root_dirs.iter().map(PathBuf::from).collect();

        RootNodeMatcher::new(
            config_roots.root_nodes,
            config_roots.root_patterns,
            config_roots.root_regex,
            dirs,
        )
        .map(Some)
        .map_err(|e| TopCatError::ConfigError(e))
    }

    fn analyze_dead_branches(
        &self,
        graph: &TCGraph,
        external_checker: Option<&ExternalUsageChecker>,
        root_matcher: Option<&RootNodeMatcher>,
    ) -> Result<(), TopCatError> {
        // ... existing code ...

        let mut dead_branches = graph.find_dead_branches(root_matcher);

        // Filter by external usage if checker is provided
        if let Some(checker) = external_checker {
            dead_branches = checker.filter_unused(&dead_branches);
        }

        // ... rest of the method ...
    }
}
```

## Usage Examples

### CLI Examples

```bash
# Mark specific nodes as roots
topcat analyze -i sql/ -e sql --root-nodes api_main --root-nodes worker_main dead-branches

# Use glob patterns to protect entire directories
topcat analyze -i sql/ -e sql --root-pattern "**/api/*.sql" dead-branches

# Use regex for node names (e.g., all migrations)
topcat analyze -i sql/ -e sql --root-regex "^migration_\\d+" dead-branches

# Mark all files in certain directories as roots
topcat analyze -i sql/ -e sql --root-dir sql/entry_points/ dead-branches

# Combine with external usage checking for maximum accuracy
topcat analyze -i sql/ -e sql \
  --root-nodes api_main \
  --root-pattern "**/api/*.sql" \
  --external-check-dir src/api \
  --external-check-pattern "*.py" \
  dead-branches
```

### Config File Usage

Create `topcat.toml` in your project root:

```toml
[analysis]
# Protect specific entry points
root_nodes = [
    "api_main",
    "worker_main",
    "migration_runner",
    "scheduled_jobs"
]

# Protect entire categories via glob
root_patterns = [
    "**/api/*.sql",
    "**/workers/*.sql",
    "**/migrations/*.sql"
]

# Protect by naming convention
root_regex = [
    "^api_.*",      # All API endpoints
    "^worker_.*",   # All worker jobs
    "^migration_\\d+" # All numbered migrations
]

# Protect entire directories
root_dirs = [
    "sql/entry_points/",
    "sql/migrations/"
]
```

Then run:
```bash
topcat analyze -i sql/ -e sql --sql-config topcat.toml dead-branches
```

## Benefits

1. **Production Safety**: Critical entry points can never be accidentally deleted
2. **Accurate Analysis**: Reduces false positives in dead branch detection
3. **Flexible Configuration**: Multiple ways to specify roots (exact names, patterns, directories)
4. **Config File Support**: Project-specific configuration can be version controlled
5. **Better Testing**: Tests can create realistic scenarios with protected entry points
6. **Complementary to External Checking**: Works alongside `--external-check-dir` for maximum accuracy

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_root_matcher_specific_nodes() {
    let matcher = RootNodeMatcher::new(
        vec!["api_main".to_string()],
        vec![],
        vec![],
        vec![],
    ).unwrap();

    assert!(matcher.is_root("api_main", Path::new("sql/api_main.sql")));
    assert!(!matcher.is_root("other", Path::new("sql/other.sql")));
}

#[test]
fn test_root_matcher_glob_pattern() {
    let matcher = RootNodeMatcher::new(
        vec![],
        vec!["**/api/*.sql".to_string()],
        vec![],
        vec![],
    ).unwrap();

    assert!(matcher.is_root("foo", Path::new("sql/api/endpoint.sql")));
    assert!(!matcher.is_root("foo", Path::new("sql/other/file.sql")));
}

#[test]
fn test_root_matcher_regex() {
    let matcher = RootNodeMatcher::new(
        vec![],
        vec![],
        vec!["^api_.*".to_string()],
        vec![],
    ).unwrap();

    assert!(matcher.is_root("api_endpoint", Path::new("sql/file.sql")));
    assert!(!matcher.is_root("worker_job", Path::new("sql/file.sql")));
}
```

### Integration Tests

Update existing tests to use root nodes:

```rust
#[test]
fn test_dead_branches_with_root_nodes() {
    let dir = TempDir::new().unwrap();

    // Create a tree: root -> middle -> leaf
    create_test_file(&dir, "root.sql", "-- name: root\nSELECT 1;");
    create_test_file(&dir, "middle.sql", "-- name: middle\n-- requires: root\nSELECT 1;");
    create_test_file(&dir, "leaf.sql", "-- name: leaf\n-- requires: middle\nSELECT 1;");

    let graph = build_test_graph(&dir);

    // Without root matcher, all are dead
    let dead_no_roots = graph.find_dead_branches(None);
    assert_eq!(dead_no_roots.len(), 3);

    // With root matcher protecting "leaf", nothing is dead
    let root_matcher = RootNodeMatcher::new(
        vec!["leaf".to_string()],
        vec![],
        vec![],
        vec![],
    ).unwrap();

    let dead_with_roots = graph.find_dead_branches(Some(&root_matcher));
    assert_eq!(dead_with_roots.len(), 0); // leaf protects the whole tree!
}
```

## Implementation Checklist

- [ ] Add `regex` crate to `Cargo.toml` dependencies
- [ ] Create `src/analysis/root_matcher.rs` with `RootNodeMatcher` struct
- [ ] Update `src/analysis/mod.rs` to export root matcher
- [ ] Update `GraphAnalyzer` trait signature for `find_dead_branches`
- [ ] Implement updated `find_dead_branches` with root checking
- [ ] Add `build_node_to_path_map()` helper to `TCGraph`
- [ ] Add CLI arguments to `AnalyzeArgs`
- [ ] Add `AnalysisConfig` to `TopcatConfig`
- [ ] Implement `build_root_matcher()` in analyze command
- [ ] Update all analysis method calls to pass root matcher
- [ ] Write unit tests for `RootNodeMatcher`
- [ ] Update integration tests to use root nodes
- [ ] Update documentation and examples
- [ ] Manual testing with real projects

## Future Enhancements

1. **Auto-detection**: Automatically detect common entry point patterns
2. **Dry-run warnings**: Warn if protected nodes would have been deleted
3. **Stats reporting**: Show how many nodes were protected by root rules
4. **Validation**: Warn if root patterns don't match any files
5. **Interactive mode**: Ask user to confirm root nodes during first run
