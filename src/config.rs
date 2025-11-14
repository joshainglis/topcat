use crate::sql_config::{HeaderUpdateMode, SqlDiscoveryConfig};
use std::path::PathBuf;

/// Configuration for building and analyzing a dependency graph.
///
/// This struct controls all aspects of file collection, dependency parsing,
/// graph construction, and output generation. It is used by all Topcat commands
/// (concat, analyze, clean, export, schema).
///
/// # Core Concepts
///
/// - **File Collection**: Control which files are included via directories, globs, and extensions
/// - **Node Filtering**: Filter dependency graph nodes by name prefixes or subdirectory
/// - **Layer Ordering**: Define hierarchical ordering of files via layers
/// - **SQL Discovery**: Automatically extract dependencies from SQL code (optional)
/// - **Output Control**: Configure output formatting and verbosity
///
/// # Examples
///
/// ## Basic concatenation configuration
///
/// ```no_run
/// use topcat::config::Config;
/// use topcat::sql_config::{SqlDiscoveryConfig, HeaderUpdateMode};
/// use std::path::PathBuf;
///
/// let config = Config {
///     input_dirs: vec![PathBuf::from("sql/")],
///     include_globs: None,
///     exclude_globs: None,
///     include_extensions: Some(&["sql".to_string()]),
///     exclude_extensions: None,
///     output: PathBuf::from("output.sql"),
///     comment_str: "--".to_string(),
///     file_separator_str: "\n-- FILE: {}\n".to_string(),
///     file_end_str: "\n".to_string(),
///     verbose: false,
///     dry_run: false,
///     include_node_prefixes: None,
///     exclude_node_prefixes: None,
///     include_hidden: false,
///     subdir_filter: None,
///     layers: vec!["prepend".to_string(), "normal".to_string(), "append".to_string()],
///     fallback_layer: "normal".to_string(),
///     sql_discovery: SqlDiscoveryConfig::default(),
///     header_update_mode: HeaderUpdateMode::Never,
///     header_output_dir: None,
/// };
/// ```
///
/// ## Configuration with SQL discovery
///
/// ```no_run
/// # use topcat::config::Config;
/// # use topcat::sql_config::{SqlDiscoveryConfig, HeaderUpdateMode};
/// # use std::path::PathBuf;
/// # use std::collections::HashMap;
/// let sql_config = SqlDiscoveryConfig {
///     enabled: true,
///     schema_pattern: Some("myapp_\\w+".to_string()),
///     object_pattern: None,
///     type_mappings: HashMap::new(),
///     extension_mappings: HashMap::new(),
///     strip_suffixes: Vec::new(),
///     model_gen_patterns: Vec::new(),
///     merge_strategy: Default::default(),
/// };
///
/// let config = Config {
///     input_dirs: vec![PathBuf::from("sql/")],
///     include_extensions: Some(&["sql".to_string()]),
///     sql_discovery: sql_config,
///     // ... other fields
/// #   include_globs: None,
/// #   exclude_globs: None,
/// #   exclude_extensions: None,
/// #   output: PathBuf::from("output.sql"),
/// #   comment_str: "--".to_string(),
/// #   file_separator_str: "\n".to_string(),
/// #   file_end_str: "\n".to_string(),
/// #   verbose: false,
/// #   dry_run: false,
/// #   include_node_prefixes: None,
/// #   exclude_node_prefixes: None,
/// #   include_hidden: false,
/// #   subdir_filter: None,
/// #   layers: vec!["prepend".to_string(), "normal".to_string(), "append".to_string()],
/// #   fallback_layer: "normal".to_string(),
/// #   header_update_mode: HeaderUpdateMode::Never,
/// #   header_output_dir: None,
/// };
/// ```
///
/// ## Configuration with layer filtering
///
/// ```no_run
/// # use topcat::config::Config;
/// # use topcat::sql_config::{SqlDiscoveryConfig, HeaderUpdateMode};
/// # use std::path::PathBuf;
/// let config = Config {
///     input_dirs: vec![PathBuf::from("sql/")],
///     layers: vec!["ddl".to_string(), "dml".to_string(), "views".to_string()],
///     fallback_layer: "dml".to_string(),
///     // ... other fields
/// #   include_globs: None,
/// #   exclude_globs: None,
/// #   include_extensions: Some(&["sql".to_string()]),
/// #   exclude_extensions: None,
/// #   output: PathBuf::from("output.sql"),
/// #   comment_str: "--".to_string(),
/// #   file_separator_str: "\n".to_string(),
/// #   file_end_str: "\n".to_string(),
/// #   verbose: false,
/// #   dry_run: false,
/// #   include_node_prefixes: None,
/// #   exclude_node_prefixes: None,
/// #   include_hidden: false,
/// #   subdir_filter: None,
/// #   sql_discovery: SqlDiscoveryConfig::default(),
/// #   header_update_mode: HeaderUpdateMode::Never,
/// #   header_output_dir: None,
/// };
/// ```
pub struct Config<'a> {
    /// Directories to search for input files.
    ///
    /// All files in these directories (and subdirectories) will be considered,
    /// subject to glob and extension filters.
    pub input_dirs: Vec<PathBuf>,

    /// Glob patterns for files to include (e.g., `["**/*.sql"]`).
    ///
    /// If specified, only files matching these patterns are included.
    /// Patterns use standard glob syntax (`*`, `**`, `?`, `[...]`).
    pub include_globs: Option<&'a [String]>,

    /// Glob patterns for files to exclude (e.g., `["**/test_*.sql"]`).
    ///
    /// Files matching these patterns are excluded even if they match include patterns.
    pub exclude_globs: Option<&'a [String]>,

    /// File extensions to include (e.g., `["sql", "ddl"]`).
    ///
    /// If specified, only files with these extensions are included.
    /// Extensions should be specified without the leading dot.
    pub include_extensions: Option<&'a [String]>,

    /// File extensions to exclude (e.g., `["tmp", "bak"]`).
    ///
    /// Files with these extensions are excluded even if they match include filters.
    pub exclude_extensions: Option<&'a [String]>,

    /// Output file path for concatenation or export operations.
    pub output: PathBuf,

    /// Comment string for the target language (e.g., `"--"` for SQL, `"#"` for Python).
    ///
    /// Used for file separator comments in concatenated output.
    pub comment_str: String,

    /// Separator string inserted between files in concatenated output.
    ///
    /// Can include `{}` placeholder which will be replaced with the file path.
    /// Example: `"\n-- FILE: {}\n"` for SQL files.
    pub file_separator_str: String,

    /// String appended at the end of each file in concatenated output.
    pub file_end_str: String,

    /// Enable verbose output with detailed logging.
    pub verbose: bool,

    /// Enable dry-run mode (preview operations without making changes).
    ///
    /// Used primarily by clean commands to show what would be deleted.
    pub dry_run: bool,

    /// Node name prefixes to include (e.g., `["auth::", "billing::"]`).
    ///
    /// If specified, only nodes whose names start with these prefixes are included
    /// in the filtered graph. Dependencies are preserved even if they don't match.
    pub include_node_prefixes: Option<&'a [String]>,

    /// Node name prefixes to exclude (e.g., `["test::", "deprecated::"]`).
    ///
    /// Nodes whose names start with these prefixes are excluded from the filtered graph.
    pub exclude_node_prefixes: Option<&'a [String]>,

    /// Include hidden files and directories (those starting with `.`).
    pub include_hidden: bool,

    /// Filter to only include files within a specific subdirectory.
    ///
    /// When set, only files under this path (and their dependencies) are included.
    /// Dependencies outside the subdirectory are automatically pulled in.
    pub subdir_filter: Option<PathBuf>,

    /// Layer names defining hierarchical ordering.
    ///
    /// Files in earlier layers always come before files in later layers in the
    /// topological sort. Cross-layer dependencies are validated to prevent
    /// violations (later layers cannot depend on earlier layers).
    ///
    /// Default: `["prepend", "normal", "append"]`
    pub layers: Vec<String>,

    /// Default layer for files that don't specify a layer.
    ///
    /// Must be one of the layers defined in the `layers` field.
    pub fallback_layer: String,

    /// SQL dependency discovery configuration.
    ///
    /// When enabled, automatically extracts dependencies from SQL code
    /// (CREATE statements, type references, etc.) instead of requiring
    /// manual header annotations.
    pub sql_discovery: SqlDiscoveryConfig,

    /// Mode for updating file headers with discovered dependencies.
    ///
    /// - `Never`: Don't update headers (default)
    /// - `Always`: Always update headers in place
    /// - `IfMissing`: Only add headers to files that lack them
    pub header_update_mode: HeaderUpdateMode,

    /// Output directory for updated headers (if different from source).
    ///
    /// When set, updated files are written to this directory instead of
    /// modifying files in place.
    pub header_output_dir: Option<PathBuf>,
}
