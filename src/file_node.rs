use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt::Display;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io;
use std::io::BufRead;
use std::path::PathBuf;

use crate::exceptions::FileNodeError;

fn get_file_headers(path: &PathBuf, comment_str: &str) -> Vec<String> {
    let file = match File::open(&path) {
        Err(why) => panic!("couldn't open {}: {}", path.display(), why.to_string()),
        Ok(file) => file,
    };

    // fill a vector with the first lines of the file starting with the comment string ignoring empty lines. Stop on the first line without the comment string.
    let reader = io::BufReader::new(file);
    let file_data: Vec<_> = reader
        .lines()
        .take_while(|x| {
            let x = match x.as_ref() {
                Ok(x) => x,
                Err(_) => return false,
            };
            x.starts_with(comment_str) || x.is_empty()
        })
        .collect::<io::Result<_>>()
        .unwrap_or_else(|_| vec![]);

    // remove any empty lines from the vector and return
    file_data
        .iter()
        .filter(|x| !x.is_empty())
        .map(|x| x.to_string())
        .collect()
}

#[derive(Debug, Clone)]
pub struct FileNode {
    pub name: String,
    pub path: PathBuf,
    pub deps: HashSet<String>,
    pub prepend: bool,
    pub append: bool,
    pub ensure_exists: HashSet<String>,
}

// Implementing PartialEq for equality comparisons
impl PartialEq for FileNode {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for FileNode {}

impl PartialOrd for FileNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FileNode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl Hash for FileNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl Display for FileNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl FileNode {
    pub fn new(
        name: String,
        path: PathBuf,
        deps: HashSet<String>,
        prepend: bool,
        append: bool,
        ensure_exists: HashSet<String>,
    ) -> FileNode {
        FileNode {
            name,
            path,
            deps,
            prepend,
            append,
            ensure_exists,
        }
    }

    fn split_dependencies(line: &str) -> Vec<String> {
        line.split(|c: char| c.is_whitespace() || c == ',')
            .filter_map(|x| {
                let x = x.trim().to_string();
                if !x.is_empty() {
                    Some(x)
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn from_file(comment_str: &str, path: &PathBuf) -> Result<FileNode, FileNodeError> {
        let file_data = get_file_headers(&path, comment_str);
        let name_str = format!("{} name:", comment_str);
        let dep_str = format!("{} requires:", comment_str);
        let drop_str = format!("{} dropped_by:", comment_str);
        let prepend_str = format!("{} is_initial", comment_str);
        let append_str = format!("{} is_final", comment_str);
        let ensure_exists_str = format!("{} exists:", comment_str);

        let mut name = String::new();
        let mut deps = HashSet::new();
        let mut is_initial = false;
        let mut is_final = false;
        let mut ensure_exists = HashSet::new();

        for unprocessed_line in &file_data {
            let line = unprocessed_line.trim().to_lowercase();
            if line.starts_with(&name_str) {
                if name.is_empty() {
                    name = line[name_str.len()..].trim().to_string();
                } else {
                    // raise an error that a file has more than one name declared
                    return Err(FileNodeError::TooManyNames(
                        path.clone(),
                        vec![name, line[name_str.len()..].trim().to_string()],
                    ));
                }
            } else if line.starts_with(&dep_str) {
                // -- requires: tomato, potato orange -> ["tomato", "potato", "orange"]
                // Should split on comma or space and then trim. Don't insert empty strings
                for item in Self::split_dependencies(&line[dep_str.len()..]) {
                    deps.insert(item);
                }
            } else if line.starts_with(&drop_str) {
                // -- dropped_by: tomato, potato -> ["tomato", "potato"]
                for item in Self::split_dependencies(&line[drop_str.len()..]) {
                    deps.insert(item);
                }
            } else if line.starts_with(&prepend_str) {
                // -- is_initial -> true
                is_initial = true;
            } else if line.starts_with(&append_str) {
                // -- is_final -> true
                is_final = true;
            } else if line.starts_with(&ensure_exists_str) {
                // --exists: tomato, potato -> ["tomato", "potato"]
                for item in Self::split_dependencies(&line[ensure_exists_str.len()..]) {
                    ensure_exists.insert(item);
                }
            }
        }
        if name.is_empty() {
            return Err(FileNodeError::NoNameDefined(path.clone()));
        }
        Ok(FileNode::new(
            name,
            path.clone(),
            deps,
            is_initial,
            is_final,
            ensure_exists,
        ))
    }
}
