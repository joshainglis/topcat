use std::path::Path;

pub trait FileSystem {
    fn read_to_string(&mut self, path: &Path) -> Result<String, std::io::Error>;
}

pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn read_to_string(&mut self, path: &Path) -> Result<String, std::io::Error> {
        std::fs::read_to_string(path)
    }
}

pub struct MockFileSystem {
    pub files: Vec<String>,
    file_counter: usize,
}

impl MockFileSystem {
    pub fn new(files: Vec<String>) -> Self {
        Self {
            files,
            file_counter: 0,
        }
    }
}

impl FileSystem for MockFileSystem {
    fn read_to_string(&mut self, _path: &Path) -> Result<String, std::io::Error> {
        let file = &self.files[self.file_counter];
        self.file_counter += 1;
        Ok(file.to_string())
    }
}
