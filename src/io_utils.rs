use std::{fs, io};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use glob::glob;
use log::error;

pub fn walk_dir(dir: &Path) -> io::Result<HashSet<PathBuf>> {
    let mut _files = HashSet::new();
    if dir.is_dir() {
        match fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            if path.is_file() {
                                _files.insert(path);
                            } else if path.is_dir() {
                                for path in walk_dir(&path)? {
                                    _files.insert(path);
                                }
                            }
                        }
                        Err(e) => error!("Unable to read entry in dir: {}", e),
                    }
                }
            }
            Err(e) => error!("Read dir failed: {}", e),
        }
    }
    Ok(_files)
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

        // Create a file and a subdirectory within the temporary directory
        let file_path = temp_path.join("file.txt");
        fs::write(&file_path, "Test file")?;

        let subdir_path = temp_path.join("subdir");
        fs::create_dir(&subdir_path)?;

        let subfile_path = subdir_path.join("subfile.txt");
        fs::write(&subfile_path, "Test subfile")?;

        // Call the walk_dir function with the temporary directory
        let result = walk_dir(temp_path)?;

        // Assert the expected files are returned
        assert!(result.contains(&file_path));
        assert!(result.contains(&subfile_path));

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

        // Create files matching the glob pattern within the temporary directory
        let file1_path = temp_path.join("file1.txt");
        match fs::write(&file1_path, "Test file 1") {
            Ok(x) => x,
            Err(_) => panic!("Failed to write file1.txt"),
        };

        let file2_path = temp_path.join("file2.txt");
        match fs::write(&file2_path, "Test file 2") {
            Ok(x) => x,
            Err(_) => panic!("Failed to write file2.txt"),
        };

        // Create a glob pattern that matches the files
        let glob_pattern = format!("{}/*.txt", temp_path.display());

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
