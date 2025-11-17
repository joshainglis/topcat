use crate::file_node::FileNode;
use std::collections::HashSet;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::{fmt, io};

#[derive(Debug)]
pub enum TopCatError {
    Io(io::Error),
    InvalidFileHeader(PathBuf, String),
    GraphMissing,
    NameClash(String, PathBuf, PathBuf),
    MissingExist(String, String),
    MissingDependency(String, String),
    InvalidDependency(String, String),
    CyclicDependency(Vec<Vec<FileNode>>),
    ConfigError(String),
    SerializationError(String),
    UnknownError(String),
    GraphBuildError(String),
    ValidationError(String),
}

impl fmt::Display for TopCatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::GraphMissing => write!(f, "Graph is None"),
            Self::InvalidFileHeader(x, s) => {
                write!(f, "Invalid file header in {}: {}", x.display(), s)
            }
            Self::NameClash(name, f1, f2) => write!(
                f,
                "Name {} found in both {} and {}",
                name,
                f1.display(),
                f2.display()
            ),
            Self::MissingExist(x, s) => write!(
                f,
                "MissingExist: {x} expects {s} to exist but it is not found"
            ),
            Self::MissingDependency(x, s) => {
                write!(f, "MissingDependency: {x} depends on {s} but it is missing")
            }
            Self::InvalidDependency(x, s) => write!(f, "InvalidDependency: {x}: {s}"),
            Self::CyclicDependency(x) => {
                let mut error_message = "Cyclic dependency detected:\n".to_string();
                for (i, cycle) in x.iter().enumerate() {
                    error_message.push_str(&format!("  Cycle {}:\n", i + 1));
                    let cycle_participants: HashSet<FileNode> = cycle.iter().cloned().collect();
                    error_message.push_str("    Participants:\n");
                    for participant in cycle_participants {
                        error_message.push_str(&format!(
                            "      - {} ({})\n",
                            participant.name,
                            participant.path.display()
                        ));
                    }
                    error_message.push_str("    Edges:\n");
                    for (i, node) in cycle.iter().enumerate() {
                        let next_node = &cycle[(i + 1) % cycle.len()];
                        error_message
                            .push_str(&format!("      - {} -> {}\n", node.name, next_node.name));
                    }
                }

                write!(f, "{error_message}")
            }
            Self::Io(err) => write!(f, "IO error: {err}"),
            Self::ConfigError(s) => write!(f, "{s}"),
            Self::SerializationError(s) => write!(f, "Serialization error: {s}"),
            Self::UnknownError(s) => write!(f, "UnknownError: {s}"),
            Self::GraphBuildError(s) => write!(f, "Graph build error: {s}"),
            Self::ValidationError(s) => write!(f, "Validation error: {s}"),
        }
    }
}

impl From<io::Error> for TopCatError {
    fn from(err: io::Error) -> TopCatError {
        TopCatError::Io(err)
    }
}

impl Error for TopCatError {}

/// Trait for adding contextual information to errors
pub trait ErrorContext {
    /// Add a contextual message to the error
    fn with_context<C: fmt::Display>(self, context: C) -> Self;

    /// Add a file path to the error context
    fn with_file_path(self, path: &Path) -> Self;

    /// Add a node name to the error context
    fn with_node_name(self, name: &str) -> Self;
}

impl ErrorContext for TopCatError {
    fn with_context<C: fmt::Display>(self, context: C) -> Self {
        match self {
            TopCatError::UnknownError(msg) => {
                TopCatError::UnknownError(format!("{context}: {msg}"))
            }
            TopCatError::ConfigError(msg) => TopCatError::ConfigError(format!("{context}: {msg}")),
            TopCatError::GraphBuildError(msg) => {
                TopCatError::GraphBuildError(format!("{context}: {msg}"))
            }
            TopCatError::ValidationError(msg) => {
                TopCatError::ValidationError(format!("{context}: {msg}"))
            }
            TopCatError::SerializationError(msg) => {
                TopCatError::SerializationError(format!("{context}: {msg}"))
            }
            // For other error types, wrap in UnknownError with context
            other => TopCatError::UnknownError(format!("{context}: {other}")),
        }
    }

    fn with_file_path(self, path: &Path) -> Self {
        self.with_context(format!("in file {}", path.display()))
    }

    fn with_node_name(self, name: &str) -> Self {
        self.with_context(format!("for node '{name}'"))
    }
}

impl TopCatError {
    /// Create a configuration error
    pub fn config_error(message: impl Into<String>) -> Self {
        TopCatError::ConfigError(message.into())
    }

    /// Create a graph build error
    pub fn graph_build_error(message: impl Into<String>) -> Self {
        TopCatError::GraphBuildError(message.into())
    }

    /// Create a validation error
    pub fn validation_error(message: impl Into<String>) -> Self {
        TopCatError::ValidationError(message.into())
    }
}

#[derive(Debug)]
pub enum FileNodeError {
    TooManyNames(PathBuf, Vec<String>),
    NoNameDefined(PathBuf),
    InvalidLayer(PathBuf, String),
    FileOpen(PathBuf, io::Error),
}

impl fmt::Display for FileNodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::TooManyNames(x, s) => write!(
                f,
                "Too many names declared in {}: {}",
                x.display(),
                s.join(", ")
            ),
            Self::NoNameDefined(x) => write!(f, "No name defined in {}", x.display()),
            Self::InvalidLayer(x, layer) => {
                write!(f, "Invalid layer '{}' declared in {}", layer, x.display())
            }
            Self::FileOpen(path, err) => {
                write!(f, "Failed to open file {}: {err}", path.display())
            }
        }
    }
}

impl Error for FileNodeError {}
