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
