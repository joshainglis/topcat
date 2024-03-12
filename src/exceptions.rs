use std::{fmt, io};
use std::error::Error;
use std::path::PathBuf;

#[derive(Debug)]
pub enum TopCatError {
    Io(io::Error),
    InvalidFileHeader(PathBuf, String),
    GraphMissing,
    NameClash(String, PathBuf, PathBuf),
    NoNameDefined(PathBuf),
    MissingExist(String, String),
    MissingDependency(String, String),
    InvalidDependency(String, String),
    CyclicDependency(String),
    UnknownError(String),
}

impl fmt::Display for TopCatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::GraphMissing => write!(f, "Graph is None"),
            Self::InvalidFileHeader(x, s) => write!(f, "Invalid file header in {}: {}", x.display(), s),
            Self::NameClash(name, f1, f2) => write!(f, "Name {} found in both {} and {}", name, f1.display(), f2.display()),
            Self::NoNameDefined(x) => write!(f, "No name defined in {}", x.display()),
            Self::MissingExist(x, s) => write!(f, "MissingExist: {} expects {} to exist but it is not found", s, x),
            Self::MissingDependency(x, s) => write!(f, "MissingDependency: {} depends on {} bit it is missing", s, x),
            Self::InvalidDependency(x, s) => write!(f, "InvalidDependency: {} is marked as prepend so it cannot depend on {} which isn't marked as prepend", s, x),
            Self::CyclicDependency(x) => write!(f, "CyclicDependency: {} has a cyclic dependency", x),
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
    InvalidPath(PathBuf),
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
            Self::InvalidPath(x) => write!(f, "Invalid path: {}", x.display()),
            Self::NoNameDefined(x) => write!(f, "No name defined in {}", x.display()),
        }
    }
}

impl Error for FileNodeError {}
