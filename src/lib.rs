//! Topcat - Topological file concatenation and dependency analysis
//!
//! Topcat is a Rust library and CLI tool for managing files with complex dependencies.
//! It reads dependency metadata from files, builds a directed acyclic graph (DAG),
//! performs topological sorting with layer constraints, and provides comprehensive
//! analysis and cleanup capabilities.
//!
//! # Primary Use Cases
//!
//! - **SQL Migration Management**: Order SQL files respecting dependencies (tables, views, functions)
//! - **Build Systems**: Concatenate source files in correct dependency order
//! - **Dependency Analysis**: Detect dead code, cycles, and missing dependencies
//! - **Project Cleanup**: Safely remove unused files while preserving dependencies
//!
//! # Core Features
//!
//! ## 1. Dependency Graph Construction
//!
//! - Parse file headers for dependency metadata (`requires`, `exists`, `dropped_by`)
//! - Build directed acyclic graph (DAG) from dependencies
//! - Validate layer ordering constraints
//! - Detect and report circular dependencies
//!
//! ## 2. Topological Sorting
//!
//! - **Deterministic ordering**: Same input always produces same output
//! - **Layer-based**: Enforce hierarchical ordering (e.g., DDL before DML)
//! - **Stable algorithm**: Respects file paths for tie-breaking
//!
//! ## 3. Dependency Analysis
//!
//! - **Dead branches**: Find complete subtrees with no external dependents
//! - **Orphans**: Identify isolated files with no connections
//! - **Cycles**: Detect circular dependencies
//! - **Missing deps**: Find broken dependency references
//! - **Root/Leaf nodes**: Categorize graph structure
//!
//! ## 4. SQL Discovery (Optional)
//!
//! - Automatically extract dependencies from SQL code
//! - Parse `CREATE` statements, type references, extension usage
//! - Eliminate manual header maintenance
//! - Support schema-qualified references
//!
//! ## 5. Schema Management
//!
//! - Multi-schema project support
//! - Cross-schema dependency analysis
//! - Schema-based filtering
//! - Namespace isolation
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```no_run
//! use topcat::file_dag::TCGraph;
//! use topcat::config::Config;
//! use topcat::sql_config::SqlDiscoveryConfig;
//! use std::path::PathBuf;
//!
//! // Create extension filter with proper lifetime
//! let extensions = vec!["sql".to_string()];
//!
//! // Create configuration
//! let config = Config {
//!     input_dirs: vec![PathBuf::from("sql/")],
//!     include_globs: None,
//!     exclude_globs: None,
//!     include_extensions: Some(&extensions),
//!     exclude_extensions: None,
//!     output: PathBuf::from("output.sql"),
//!     comment_str: "--".to_string(),
//!     file_separator_str: "\n".to_string(),
//!     file_end_str: "\n".to_string(),
//!     verbose: false,
//!     dry_run: false,
//!     include_node_prefixes: None,
//!     exclude_node_prefixes: None,
//!     include_hidden: false,
//!     subdir_filter: None,
//!     layers: vec!["prepend".to_string(), "normal".to_string(), "append".to_string()],
//!     fallback_layer: "normal".to_string(),
//!     sql_discovery: SqlDiscoveryConfig::default(),
//!     header_update_mode: Default::default(),
//!     header_output_dir: None,
//! };
//!
//! // Build dependency graph
//! let mut graph = TCGraph::new(&config);
//! graph.build_graph().expect("Failed to build graph");
//!
//! // Get topologically sorted file paths
//! let sorted_files = graph.get_sorted_files().expect("Failed to sort files");
//!
//! // Use sorted files for concatenation or analysis
//! for file_path in sorted_files {
//!     println!("{}", file_path.display());
//! }
//! ```
//!
//! ## File Metadata Format
//!
//! Files include dependency metadata in header comments:
//!
//! ```sql
//! -- name: create_users_table
//! -- requires: create_schema, create_extensions
//! -- exists: uuid_generate_v4
//! -- layer: normal
//!
//! CREATE TABLE users (
//!     id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
//!     username TEXT NOT NULL
//! );
//! ```
//!
//! ### Metadata Fields
//!
//! - `name`: Unique identifier for this file (required)
//! - `requires` / `dropped_by`: Hard dependencies (create ordering edges)
//! - `exists`: Soft dependencies (ensure inclusion without ordering)
//! - `layer`: Hierarchical category (default: fallback layer)
//!
//! # Architecture
//!
//! ## Module Organization
//!
//! - [`file_node`]: File representation with dependency metadata
//! - [`file_dag`]: DAG construction, validation, and traversal
//! - [`config`]: Configuration management
//! - [`analysis`]: Dependency analysis algorithms
//! - [`sql_config`]: SQL discovery configuration
//! - [`schema_utils`]: Schema filtering and operations
//! - [`graph_utils`]: Graph analysis utilities
//! - [`display_utils`]: Output formatting and tables
//! - [`exceptions`]: Error types and handling
//!
//! ## Key Algorithms
//!
//! ### Topological Sort
//!
//! Modified Kahn's algorithm with weight-based tie-breaking:
//! - Time: O(V log V + E)
//! - Guarantees deterministic output
//! - Respects layer constraints
//!
//! ### Cycle Detection
//!
//! Two-phase approach:
//! 1. Fast DFS check: O(V + E)
//! 2. Johnson's algorithm for enumeration: O((V + E)(C + 1))
//!
//! ### Dead Branch Detection
//!
//! Reverse graph traversal:
//! - Identifies complete subtrees with no external dependents
//! - Respects root protection patterns
//! - Checks external usage in other codebases
//!
//! # CLI Tool
//!
//! The `topcat` binary provides commands for all operations:
//!
//! ```bash
//! # Concatenation
//! topcat concat -i sql/ -o output.sql
//!
//! # Analysis
//! topcat analyze -i sql/ -e sql dead-branches
//! topcat analyze -i sql/ -e sql cycles
//!
//! # Cleanup
//! topcat clean -i sql/ -e sql orphans --no-dry-run
//!
//! # Schema operations
//! topcat schema -i sql/ -e sql list
//! topcat schema -i sql/ -e sql analyze my_schema
//!
//! # Export
//! topcat export -i sql/ -e sql -o graph.json json
//! ```
//!
//! # Performance
//!
//! - Optimized with `Rc<FileNode>` to eliminate deep clones
//! - 68% faster orphan detection vs naive implementation
//! - Efficient graph algorithms (O(V + E) for most operations)
//! - Minimal memory overhead with reference counting
//!
//! # Safety
//!
//! - No panics in production code (all use `expect` with clear messages)
//! - Comprehensive error handling via [`TopCatError`](exceptions::TopCatError)
//! - Symlink cycle detection prevents infinite loops
//! - Input validation for regex patterns and paths

pub mod analysis;
pub mod cli;
pub mod config;
pub mod display_utils;
pub mod exceptions;
pub mod file_dag;
pub mod file_node;
pub mod fs;
pub mod graph_utils;
pub mod header_generator;
pub mod logging;
pub mod output;
pub mod platform;
pub mod schema_utils;
pub mod settings;
pub mod sql_config;

mod io_utils;
mod sql_parser;
mod stable_topo;
