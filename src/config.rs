use std::path::PathBuf;

pub struct Config<'a> {
    pub input_dirs: Vec<PathBuf>,
    pub include_globs: Option<&'a [String]>,
    pub exclude_globs: Option<&'a [String]>,
    pub include_extensions: Option<&'a [String]>,
    pub exclude_extensions: Option<&'a [String]>,
    pub output: PathBuf,
    pub comment_str: String,
    pub file_separator_str: String,
    pub file_end_str: String,
    pub verbose: bool,
    pub dry_run: bool,
    pub include_node_prefixes: Option<&'a [String]>,
    pub exclude_node_prefixes: Option<&'a [String]>,
    pub include_hidden: bool,
}
