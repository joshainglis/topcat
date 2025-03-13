use crate::file_node::FileNode;
use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;
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
    UnknownError(String),
}

impl fmt::Display for TopCatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::GraphMissing => write!(f, "Graph is None"),
            Self::InvalidFileHeader(x, s) => write!(f, "Invalid file header in {}: {}", x.display(), s),
            Self::NameClash(name, f1, f2) => write!(f, "Name {} found in both {} and {}", name, f1.display(), f2.display()),
            Self::MissingExist(x, s) => write!(f, "MissingExist: {} expects {} to exist but it is not found", x, s),
            Self::MissingDependency(x, s) => write!(f, "MissingDependency: {} depends on {} but it is missing", x, s),
            Self::InvalidDependency(x, s) => write!(f, "InvalidDependency: {} is marked as prepend so it cannot depend on {} which isn't marked as prepend", s, x),
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
                        error_message.push_str(&format!(
                            "      - {} -> {}\n",
                            node.name,
                            next_node.name
                        ));
                    }
                }

                write!(f, "{}", error_message)
            },
            Self::Io(err) => write!(f, "IO error: {}", err),
            Self::UnknownError(s) => write!(f, "UnknownError: {}", s),
        }
    }
}

impl From<io::Error> for TopCatError {
    fn from(err: io::Error) -> TopCatError {
        TopCatError::Io(err)
    }
}

impl Error for TopCatError {}

#[derive(Debug)]
pub enum FileNodeError {
    TooManyNames(PathBuf, Vec<String>),
    NoNameDefined(PathBuf),
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
        }
    }
}

impl Error for FileNodeError {}
