use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::{fs, io};

use glob::glob;
use log::error;

fn is_hidden_dir_or_file(path: &Path) -> Result<bool, io::Error> {
    let file_or_dir_name = match path.file_name() {
        Some(name) => name,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid file name",
            ))
        }
    };
    Ok(file_or_dir_name.to_string_lossy().starts_with('.'))
}

pub fn walk_dir(dir: &Path, include_hidden: bool) -> io::Result<HashSet<PathBuf>> {
    let mut files = HashSet::new();

    if !dir.is_dir() {
        return Ok(files);
    }

    if !include_hidden && is_hidden_dir_or_file(dir).unwrap_or(false) {
        return Ok(files);
    }

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            error!("Read dir failed: {}", e);
            return Ok(files);
        }
    };

    for entry in entries {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file() {
                    if !include_hidden && is_hidden_dir_or_file(&path).unwrap_or(false) {
                        continue;
                    }
                    files.insert(path);
                } else if path.is_dir() {
                    let subdir_files = walk_dir(&path, include_hidden)?;
                    files.extend(subdir_files);
                }
            }
            Err(e) => error!("Read dir failed: {}", e),
        }
    }

    Ok(files)
}

pub fn glob_files(glob_patterns: &[String]) -> Result<HashSet<PathBuf>, glob::PatternError> {
    let mut paths = HashSet::new();

    for pattern in glob_patterns {
        let entries = glob(pattern)?;
        for entry in entries {
            if let Ok(path) = entry {
                paths.insert(path);
            } else if let Err(e) = entry {
                error!("Failed to read entry: {:?}", e);
            }
        }
    }

    Ok(paths)
}
#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_walk_dir() -> io::Result<()> {
        // Create a temporary directory for testing
        let temp_dir = tempdir()?;
        let temp_path = temp_dir.path();
        let working_dir_path = temp_path.join("working_dir");
        match fs::create_dir(&working_dir_path) {
            Ok(x) => x,
            Err(_) => panic!("Failed to create working directory"),
        };

        // Create a file and a subdirectory within the temporary directory
        let file_path = working_dir_path.join("file.txt");
        fs::write(&file_path, "Test file")?;

        let subdir_path = working_dir_path.join("subdir");
        fs::create_dir(&subdir_path)?;

        let subfile_path = subdir_path.join("subfile.txt");
        fs::write(&subfile_path, "Test subfile")?;

        let hidden_file_path = working_dir_path.join(".hidden_file.txt");
        fs::write(&hidden_file_path, "Test hidden file")?;

        let hidden_subdir_path = working_dir_path.join(".hidden_subdir");
        fs::create_dir(&hidden_subdir_path)?;

        let normal_file_in_hidden_subfile_path = hidden_subdir_path.join("hidden_subfile.txt");
        fs::write(&normal_file_in_hidden_subfile_path, "Test hidden subfile")?;

        // Call the walk_dir function with the temporary directory
        let result = match walk_dir(&working_dir_path, false) {
            Ok(x) => x,
            Err(_) => panic!("Failed to walk directory"),
        };

        assert_eq!(result.len(), 2);

        // Assert the expected files are returned
        assert!(result.contains(&file_path));
        assert!(result.contains(&subfile_path));

        // Assert the expected hidden files are not returned
        assert!(!result.contains(&hidden_file_path));

        // Assert the expected normal files in hidden subdirectories are not returned
        assert!(!result.contains(&normal_file_in_hidden_subfile_path));

        let result_2 = match walk_dir(&working_dir_path, true) {
            Ok(x) => x,
            Err(_) => panic!("Failed to walk directory"),
        };

        // Assert all files are returned both hidden and unhidden
        assert!(result_2.contains(&hidden_file_path));
        assert!(result_2.contains(&normal_file_in_hidden_subfile_path));

        Ok(())
    }

    #[test]
    fn test_glob_files() {
        // Create a temporary directory for testing
        let temp_dir = match tempdir() {
            Ok(x) => x,
            Err(_) => panic!("Failed to create temporary directory"),
        };
        let temp_path = temp_dir.path();
        let working_dir_path = temp_path.join("working_dir");
        match fs::create_dir(&working_dir_path) {
            Ok(x) => x,
            Err(_) => panic!("Failed to create working directory"),
        };

        // Create files matching the glob pattern within the temporary directory
        let file1_path = working_dir_path.join("file1.txt");
        match fs::write(&file1_path, "Test file 1") {
            Ok(x) => x,
            Err(_) => panic!("Failed to write file1.txt"),
        };

        let file2_path = working_dir_path.join("file2.txt");
        match fs::write(&file2_path, "Test file 2") {
            Ok(x) => x,
            Err(_) => panic!("Failed to write file2.txt"),
        };

        // Create a glob pattern that matches the files
        let glob_pattern = format!("{}/*.txt", working_dir_path.display());

        // Call the glob_files function with the glob pattern
        let result = glob_files(&vec![glob_pattern]);

        // Assert the expected files are returned
        match result {
            Ok(files) => {
                assert!(files.contains(&file1_path));
                assert!(files.contains(&file2_path));
            }
            Err(e) => panic!("Error occurred: {:?}", e),
        }
    }
}
