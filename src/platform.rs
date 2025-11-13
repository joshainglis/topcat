/// Platform-specific utilities for cross-platform compatibility.
///
/// This module provides abstractions over platform-specific differences
/// such as null device paths, temporary directories, and file operations.
use std::io;
use std::path::PathBuf;

/// Returns the platform-specific null device path.
///
/// This is used when building graphs for analysis or export where no actual
/// output file is needed, but the Config struct requires an output path.
///
/// # Returns
///
/// - `/dev/null` on Unix systems
/// - `NUL` on Windows
/// - `/dev/null` as fallback for other platforms
///
/// # Examples
///
/// ```
/// use topcat::platform::null_device;
///
/// let null = null_device();
/// // On Unix: "/dev/null"
/// // On Windows: "NUL"
/// ```
#[cfg(unix)]
pub fn null_device() -> &'static str {
    "/dev/null"
}

#[cfg(windows)]
pub fn null_device() -> &'static str {
    "NUL"
}

#[cfg(not(any(unix, windows)))]
pub fn null_device() -> &'static str {
    "/dev/null" // Fallback for other platforms
}

/// Returns a suitable temporary directory for the platform.
///
/// This uses the standard library's `std::env::temp_dir()` which respects
/// platform-specific environment variables like `TMPDIR`, `TEMP`, or `TMP`.
///
/// # Examples
///
/// ```
/// use topcat::platform::temp_dir;
///
/// let dir = temp_dir();
/// println!("Temporary directory: {}", dir.display());
/// ```
pub fn temp_dir() -> PathBuf {
    std::env::temp_dir()
}

/// Creates a temporary file with the given prefix.
///
/// The file is created in the system's temporary directory with a unique
/// name based on the provided prefix and a timestamp.
///
/// # Arguments
///
/// * `prefix` - A prefix for the temporary file name
///
/// # Returns
///
/// A `PathBuf` pointing to the created temporary file.
///
/// # Errors
///
/// Returns an `io::Error` if:
/// - The temporary directory is not accessible
/// - File creation fails
///
/// # Examples
///
/// ```no_run
/// use topcat::platform::temp_file;
///
/// let temp = temp_file("topcat_test").expect("Failed to create temp file");
/// println!("Created temporary file: {}", temp.display());
/// ```
pub fn temp_file(prefix: &str) -> io::Result<PathBuf> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let temp_dir = temp_dir();
    let file_name = format!("{prefix}_{timestamp}.tmp");
    let temp_path = temp_dir.join(file_name);

    // Create an empty file to reserve the name
    std::fs::File::create(&temp_path)?;

    Ok(temp_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_device() {
        let device = null_device();
        #[cfg(unix)]
        assert_eq!(device, "/dev/null");
        #[cfg(windows)]
        assert_eq!(device, "NUL");
    }

    #[test]
    fn test_temp_dir_exists() {
        let dir = temp_dir();
        assert!(dir.exists(), "Temporary directory should exist");
        assert!(dir.is_dir(), "Temporary path should be a directory");
    }

    #[test]
    fn test_temp_file_creation() {
        let temp = temp_file("topcat_test").expect("Failed to create temp file");
        assert!(temp.exists(), "Temporary file should exist");
        assert!(temp.is_file(), "Temporary path should be a file");

        // Clean up
        std::fs::remove_file(temp).ok();
    }

    #[test]
    fn test_temp_file_has_prefix() {
        let temp = temp_file("my_prefix").expect("Failed to create temp file");
        let file_name = temp.file_name().unwrap().to_string_lossy();
        assert!(
            file_name.starts_with("my_prefix"),
            "File name should start with prefix"
        );

        // Clean up
        std::fs::remove_file(temp).ok();
    }

    #[test]
    fn test_temp_file_unique() {
        let temp1 = temp_file("test").expect("Failed to create first temp file");
        // Small sleep to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(10));
        let temp2 = temp_file("test").expect("Failed to create second temp file");

        assert_ne!(temp1, temp2, "Multiple temp files should have unique names");

        // Clean up
        std::fs::remove_file(temp1).ok();
        std::fs::remove_file(temp2).ok();
    }
}
