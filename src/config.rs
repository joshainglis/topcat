use crate::exceptions::TopCatError;
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

impl<'a> Config<'a> {
    /// Create a new ConfigBuilder for constructing a Config with validation.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use topcat::config::Config;
    /// use std::path::PathBuf;
    ///
    /// let config = Config::builder()
    ///     .input_dir(PathBuf::from("sql/"))
    ///     .output(PathBuf::from("output.sql"))
    ///     .build()
    ///     .expect("valid config");
    /// ```
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }
}

/// Builder for constructing Config instances with validation and sensible defaults.
///
/// This builder owns all configuration data and provides a fluent API for
/// setting values. The `build()` method validates the configuration and
/// returns a `Config` with borrowed references to the builder's owned data.
///
/// # Examples
///
/// ## Basic usage
///
/// ```no_run
/// use topcat::config::Config;
/// use std::path::PathBuf;
///
/// let config = Config::builder()
///     .input_dir(PathBuf::from("sql/"))
///     .include_extension("sql")
///     .output(PathBuf::from("output.sql"))
///     .verbose(true)
///     .build()
///     .expect("valid configuration");
/// ```
///
/// ## With SQL discovery
///
/// ```no_run
/// use topcat::config::Config;
/// use topcat::sql_config::SqlDiscoveryConfig;
/// use std::path::PathBuf;
///
/// let sql_discovery = SqlDiscoveryConfig {
///     enabled: true,
///     schema_pattern: Some("myapp_\\w+".to_string()),
///     ..Default::default()
/// };
///
/// let config = Config::builder()
///     .input_dir(PathBuf::from("sql/"))
///     .output(PathBuf::from("output.sql"))
///     .sql_discovery(sql_discovery)
///     .build()
///     .expect("valid configuration");
/// ```
///
/// ## With custom layers
///
/// ```no_run
/// use topcat::config::Config;
/// use std::path::PathBuf;
///
/// let config = Config::builder()
///     .input_dir(PathBuf::from("sql/"))
///     .output(PathBuf::from("output.sql"))
///     .layers(vec!["schema".to_string(), "tables".to_string(), "views".to_string()])
///     .fallback_layer("tables")
///     .build()
///     .expect("valid configuration");
/// ```
#[derive(Debug, Clone)]
pub struct ConfigBuilder {
    input_dirs: Vec<PathBuf>,
    include_globs: Option<Vec<String>>,
    exclude_globs: Option<Vec<String>>,
    include_extensions: Option<Vec<String>>,
    exclude_extensions: Option<Vec<String>>,
    output: Option<PathBuf>,
    comment_str: String,
    file_separator_str: String,
    file_end_str: String,
    verbose: bool,
    dry_run: bool,
    include_node_prefixes: Option<Vec<String>>,
    exclude_node_prefixes: Option<Vec<String>>,
    include_hidden: bool,
    subdir_filter: Option<PathBuf>,
    layers: Vec<String>,
    fallback_layer: String,
    sql_discovery: SqlDiscoveryConfig,
    header_update_mode: HeaderUpdateMode,
    header_output_dir: Option<PathBuf>,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            input_dirs: Vec::new(),
            include_globs: None,
            exclude_globs: None,
            include_extensions: None,
            exclude_extensions: None,
            output: None,
            comment_str: "--".to_string(),
            file_separator_str: String::new(),
            file_end_str: String::new(),
            verbose: false,
            dry_run: false,
            include_node_prefixes: None,
            exclude_node_prefixes: None,
            include_hidden: false,
            subdir_filter: None,
            layers: vec![
                "prepend".to_string(),
                "normal".to_string(),
                "append".to_string(),
            ],
            fallback_layer: "normal".to_string(),
            sql_discovery: SqlDiscoveryConfig::default(),
            header_update_mode: HeaderUpdateMode::Never,
            header_output_dir: None,
        }
    }
}

impl ConfigBuilder {
    /// Add an input directory to search for files.
    pub fn input_dir(mut self, dir: PathBuf) -> Self {
        self.input_dirs.push(dir);
        self
    }

    /// Set the input directories (replaces any previously set).
    pub fn input_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.input_dirs = dirs;
        self
    }

    /// Add a glob pattern to include files.
    pub fn include_glob(mut self, pattern: String) -> Self {
        self.include_globs
            .get_or_insert_with(Vec::new)
            .push(pattern);
        self
    }

    /// Set include glob patterns (replaces any previously set).
    pub fn include_globs(mut self, patterns: Vec<String>) -> Self {
        self.include_globs = Some(patterns);
        self
    }

    /// Add a glob pattern to exclude files.
    pub fn exclude_glob(mut self, pattern: String) -> Self {
        self.exclude_globs
            .get_or_insert_with(Vec::new)
            .push(pattern);
        self
    }

    /// Set exclude glob patterns (replaces any previously set).
    pub fn exclude_globs(mut self, patterns: Vec<String>) -> Self {
        self.exclude_globs = Some(patterns);
        self
    }

    /// Add a file extension to include (without leading dot).
    pub fn include_extension(mut self, ext: &str) -> Self {
        self.include_extensions
            .get_or_insert_with(Vec::new)
            .push(ext.to_string());
        self
    }

    /// Set file extensions to include (replaces any previously set).
    pub fn include_extensions(mut self, exts: Vec<String>) -> Self {
        self.include_extensions = Some(exts);
        self
    }

    /// Add a file extension to exclude (without leading dot).
    pub fn exclude_extension(mut self, ext: &str) -> Self {
        self.exclude_extensions
            .get_or_insert_with(Vec::new)
            .push(ext.to_string());
        self
    }

    /// Set file extensions to exclude (replaces any previously set).
    pub fn exclude_extensions(mut self, exts: Vec<String>) -> Self {
        self.exclude_extensions = Some(exts);
        self
    }

    /// Set the output file path.
    pub fn output(mut self, path: PathBuf) -> Self {
        self.output = Some(path);
        self
    }

    /// Set the comment string for the target language.
    pub fn comment_str(mut self, comment: String) -> Self {
        self.comment_str = comment;
        self
    }

    /// Set the file separator string for concatenated output.
    pub fn file_separator_str(mut self, separator: String) -> Self {
        self.file_separator_str = separator;
        self
    }

    /// Set the file end string for concatenated output.
    pub fn file_end_str(mut self, end: String) -> Self {
        self.file_end_str = end;
        self
    }

    /// Enable or disable verbose output.
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Enable or disable dry-run mode.
    pub fn dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Add a node name prefix to include.
    pub fn include_node_prefix(mut self, prefix: String) -> Self {
        self.include_node_prefixes
            .get_or_insert_with(Vec::new)
            .push(prefix);
        self
    }

    /// Set node name prefixes to include (replaces any previously set).
    pub fn include_node_prefixes(mut self, prefixes: Vec<String>) -> Self {
        self.include_node_prefixes = Some(prefixes);
        self
    }

    /// Add a node name prefix to exclude.
    pub fn exclude_node_prefix(mut self, prefix: String) -> Self {
        self.exclude_node_prefixes
            .get_or_insert_with(Vec::new)
            .push(prefix);
        self
    }

    /// Set node name prefixes to exclude (replaces any previously set).
    pub fn exclude_node_prefixes(mut self, prefixes: Vec<String>) -> Self {
        self.exclude_node_prefixes = Some(prefixes);
        self
    }

    /// Enable or disable inclusion of hidden files.
    pub fn include_hidden(mut self, include: bool) -> Self {
        self.include_hidden = include;
        self
    }

    /// Set the subdirectory filter.
    pub fn subdir_filter(mut self, subdir: PathBuf) -> Self {
        self.subdir_filter = Some(subdir);
        self
    }

    /// Set the layer names for hierarchical ordering.
    pub fn layers(mut self, layers: Vec<String>) -> Self {
        self.layers = layers;
        self
    }

    /// Set the fallback layer name.
    pub fn fallback_layer(mut self, layer: &str) -> Self {
        self.fallback_layer = layer.to_string();
        self
    }

    /// Set the SQL discovery configuration.
    pub fn sql_discovery(mut self, config: SqlDiscoveryConfig) -> Self {
        self.sql_discovery = config;
        self
    }

    /// Set the header update mode.
    pub fn header_update_mode(mut self, mode: HeaderUpdateMode) -> Self {
        self.header_update_mode = mode;
        self
    }

    /// Set the header output directory.
    pub fn header_output_dir(mut self, dir: PathBuf) -> Self {
        self.header_output_dir = Some(dir);
        self
    }

    /// Build and validate the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No input directories are specified
    /// - No output path is specified
    /// - The fallback layer is not in the layers list
    /// - Layers list is empty
    pub fn build(&self) -> Result<Config, TopCatError> {
        // Validation
        if self.input_dirs.is_empty() {
            return Err(TopCatError::ConfigError(
                "At least one input directory must be specified".to_string(),
            ));
        }

        if self.output.is_none() {
            return Err(TopCatError::ConfigError(
                "Output path must be specified".to_string(),
            ));
        }

        if self.layers.is_empty() {
            return Err(TopCatError::ConfigError(
                "At least one layer must be specified".to_string(),
            ));
        }

        if !self.layers.contains(&self.fallback_layer) {
            return Err(TopCatError::ConfigError(format!(
                "Fallback layer '{}' is not in layers list",
                self.fallback_layer
            )));
        }

        // Build Config with borrowed references to our owned data
        Ok(Config {
            input_dirs: self.input_dirs.clone(),
            include_globs: self.include_globs.as_deref(),
            exclude_globs: self.exclude_globs.as_deref(),
            include_extensions: self.include_extensions.as_deref(),
            exclude_extensions: self.exclude_extensions.as_deref(),
            output: self.output.clone().expect("validated above"),
            comment_str: self.comment_str.clone(),
            file_separator_str: self.file_separator_str.clone(),
            file_end_str: self.file_end_str.clone(),
            verbose: self.verbose,
            dry_run: self.dry_run,
            include_node_prefixes: self.include_node_prefixes.as_deref(),
            exclude_node_prefixes: self.exclude_node_prefixes.as_deref(),
            include_hidden: self.include_hidden,
            subdir_filter: self.subdir_filter.clone(),
            layers: self.layers.clone(),
            fallback_layer: self.fallback_layer.clone(),
            sql_discovery: self.sql_discovery.clone(),
            header_update_mode: self.header_update_mode,
            header_output_dir: self.header_output_dir.clone(),
        })
    }
}
