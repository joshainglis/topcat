//! Dependency graph management and analysis.
//!
//! This module provides the core [`TCGraph`] structure for building and analyzing
//! file dependency graphs with topological sorting and layer constraints.
//!
//! # Overview
//!
//! The `file_dag` module is the heart of Topcat, responsible for:
//!
//! - **Graph construction**: Building directed acyclic graphs from file dependencies
//! - **Validation**: Ensuring dependencies exist and layer constraints are met
//! - **Cycle detection**: Finding circular dependencies using Johnson's algorithm
//! - **Topological sorting**: Producing deterministic, layer-aware file orderings
//! - **Schema operations**: Multi-schema project support and filtering
//! - **Traversal**: Finding dependencies, dependents, and graph traversal utilities
//!
//! # Module Organization
//!
//! This module is organized into focused submodules:
//!
//! - `core`: Main [`TCGraph`] struct and fundamental operations
//! - `builder`: Graph construction utilities (file collection, filtering, node creation)
//! - `validation`: Dependency validation and cycle detection algorithms
//! - `schema`: Schema-based filtering and cross-schema dependency analysis
//! - `filters`: Node filtering and graph traversal operations
//!
//! # Architecture
//!
//! ## Graph Structure
//!
//! `TCGraph` maintains multiple graph representations:
//!
//! - **Per-layer graphs**: One `DiGraph` per layer for cycle detection
//! - **Name map**: `HashMap<String, Rc<FileNode>>` for O(1) name lookups
//! - **Path map**: `HashMap<PathBuf, Rc<FileNode>>` for O(1) path lookups
//! - **Layer index maps**: Track node indices within each layer graph
//!
//! ## Why Per-Layer Graphs?
//!
//! Layer separation provides several benefits:
//!
//! 1. **Efficient cycle detection**: Only check within layers (cross-layer cycles impossible)
//! 2. **Clear error messages**: Cycles are reported per-layer
//! 3. **Performance**: Smaller graphs to validate
//! 4. **Correctness**: Layer ordering prevents cross-layer cycles by design
//!
//! ## Graph Building Pipeline
//!
//! ```text
//! 1. Collect files (builder::collect_files)
//!    ↓
//! 2. Filter by globs/extensions (builder::filter_files)
//!    ↓
//! 3. Parse file headers → FileNode (file_node::from_file)
//!    ↓
//! 4. SQL discovery (optional) (builder::perform_sql_discovery)
//!    ↓
//! 5. Populate name_map & path_map (core::load_file_nodes)
//!    ↓
//! 6. Add nodes to layer graphs (builder::add_nodes_to_graphs)
//!    ↓
//! 7. Validate dependencies & layer ordering (validation::validate_dependencies)
//!    ↓
//! 8. Check for cycles (validation::check_cyclic_dependencies)
//!    ↓
//! 9. Graph ready for use!
//! ```
//!
//! # Usage Example
//!
//! ```no_run
//! use topcat::file_dag::TCGraph;
//! use topcat::config::Config;
//! use topcat::sql_config::SqlDiscoveryConfig;
//! use std::path::PathBuf;
//!
//! // Create extension filter
//! let extensions = vec!["sql".to_string()];
//!
//! // Build configuration
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
//!     layers: vec!["ddl".to_string(), "dml".to_string()],
//!     fallback_layer: "dml".to_string(),
//!     auto_mapping: &indexmap::IndexMap::new(),
//!     sql_discovery: SqlDiscoveryConfig::default(),
//!     header_update_mode: Default::default(),
//!     header_output_dir: None,
//! };
//!
//! // Create and build graph
//! let mut graph = TCGraph::new(&config);
//! graph.build_graph().expect("Failed to build graph");
//!
//! // Get sorted files
//! let sorted_files = graph.get_sorted_files().expect("Failed to sort");
//!
//! println!("Files in dependency order:");
//! for path in sorted_files {
//!     println!("  {}", path.display());
//! }
//! ```
//!
//! # Key Operations
//!
//! ## Building a Graph
//!
//! ```no_run
//! # use topcat::file_dag::TCGraph;
//! # use topcat::config::Config;
//! # let config = Config {
//! #   input_dirs: vec![],
//! #   include_globs: None,
//! #   exclude_globs: None,
//! #   include_extensions: None,
//! #   exclude_extensions: None,
//! #   output: Default::default(),
//! #   comment_str: String::new(),
//! #   file_separator_str: String::new(),
//! #   file_end_str: String::new(),
//! #   verbose: false,
//! #   dry_run: false,
//! #   include_node_prefixes: None,
//! #   exclude_node_prefixes: None,
//! #   include_hidden: false,
//! #   subdir_filter: None,
//! #   layers: vec!["normal".to_string()],
//! #   fallback_layer: "normal".to_string(),
//! #   auto_mapping: &indexmap::IndexMap::new(),
//! #   sql_discovery: Default::default(),
//! #   header_update_mode: Default::default(),
//! #   header_output_dir: None,
//! # };
//! let mut graph = TCGraph::new(&config);
//! graph.build_graph()?;
//! # Ok::<(), topcat::exceptions::TopCatError>(())
//! ```
//!
//! ## Getting Sorted Files
//!
//! ```no_run
//! # use topcat::file_dag::TCGraph;
//! # use topcat::config::Config;
//! # let config: Config = unimplemented!();
//! # let mut graph = TCGraph::new(&config);
//! # graph.build_graph().unwrap();
//! let sorted_files = graph.get_sorted_files()?;
//! # Ok::<(), topcat::exceptions::TopCatError>(())
//! ```
//!
//! ## Schema Filtering
//!
//! ```no_run
//! # use topcat::file_dag::TCGraph;
//! # use topcat::config::Config;
//! # let config: Config = unimplemented!();
//! # let mut graph = TCGraph::new(&config);
//! # graph.build_graph().unwrap();
//! let schemas = vec!["public".to_string()];
//! graph.apply_schema_filter(&schemas);
//! ```
//!
//! # Error Handling
//!
//! All operations return `Result<T, TopCatError>`:
//!
//! - [`TopCatError::CyclicDependency`](crate::exceptions::TopCatError::CyclicDependency): Circular dependencies detected
//! - [`TopCatError::MissingDependency`](crate::exceptions::TopCatError::MissingDependency): Required file not found
//! - [`TopCatError::InvalidDependency`](crate::exceptions::TopCatError::InvalidDependency): Layer ordering violation
//! - [`TopCatError::NameClash`](crate::exceptions::TopCatError::NameClash): Duplicate file names
//! - [`TopCatError::GraphMissing`](crate::exceptions::TopCatError::GraphMissing): Operation requires built graph

mod builder;
mod core;
mod filters;
mod schema;
mod validation;

// Re-export the main TCGraph type
pub use core::TCGraph;
