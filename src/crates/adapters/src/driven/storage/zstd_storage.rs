use std::path::{Path, PathBuf};

use anyhow::Result;
use file_pipeline_core::ports::output::StoragePort;
use module_storage::{LocalStoragePort, ZstdLocalStorage};

use super::map_err;

pub struct ZstdStorageAdapter {
    inner: ZstdLocalStorage,
}

impl ZstdStorageAdapter {
    pub fn new(compression_level: i32, temp_dir: PathBuf) -> Self {
        Self {
            inner: ZstdLocalStorage::new(compression_level, temp_dir),
        }
    }

    pub fn compression_level(&self) -> i32 {
        self.inner.compression_level
    }
}

impl StoragePort for ZstdStorageAdapter {
    fn compress_and_store(&self, source: &Path, dest_dir: &Path) -> Result<PathBuf> {
        self.inner.compress_and_store(source, dest_dir).map_err(map_err)
    }

    fn compress_with_level(&self, source: &Path, dest_dir: &Path, level: i32) -> Result<PathBuf> {
        self.inner.compress_with_level(source, dest_dir, level).map_err(map_err)
    }

    fn decompress_temp(&self, compressed: &Path) -> Result<PathBuf> {
        self.inner.decompress_temp(compressed).map_err(map_err)
    }

    fn read_header(&self, compressed: &Path, lines: usize) -> Result<String> {
        self.inner.read_header(compressed, lines).map_err(map_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn temp_dirs() -> (TempDir, TempDir) {
        (TempDir::new().unwrap(), TempDir::new().unwrap())
    }

    #[test]
    fn test_compress_decompress_roundtrip() {
        let (src_dir, dest_dir) = temp_dirs();
        let temp_tmp = TempDir::new().unwrap();
        let adapter = ZstdStorageAdapter::new(3, temp_tmp.path().to_path_buf());

        let src_file = src_dir.path().join("test.txt");
        let mut f = fs::File::create(&src_file).unwrap();
        f.write_all(b"Hello, this is a test content for zstd compression!").unwrap();

        let compressed = adapter.compress_and_store(&src_file, dest_dir.path()).unwrap();
        assert!(compressed.exists());
        assert!(compressed.to_string_lossy().ends_with(".zst"));

        let decompressed = adapter.decompress_temp(&compressed).unwrap();
        let content = fs::read_to_string(&decompressed).unwrap();
        assert_eq!(content, "Hello, this is a test content for zstd compression!");
    }

    #[test]
    fn test_compress_with_level_override() {
        let (src_dir, dest_dir) = temp_dirs();
        let temp_tmp = TempDir::new().unwrap();
        let adapter = ZstdStorageAdapter::new(3, temp_tmp.path().to_path_buf());

        let src_file = src_dir.path().join("large.txt");
        let content = "abcdefghij\n".repeat(100);
        let mut f = fs::File::create(&src_file).unwrap();
        f.write_all(content.as_bytes()).unwrap();

        let compressed = adapter.compress_with_level(&src_file, dest_dir.path(), 19).unwrap();
        assert!(compressed.exists());

        let decompressed = adapter.decompress_temp(&compressed).unwrap();
        let roundtripped = fs::read_to_string(&decompressed).unwrap();
        assert_eq!(roundtripped, content);
    }

    #[test]
    fn test_read_header() {
        let (src_dir, dest_dir) = temp_dirs();
        let temp_tmp = TempDir::new().unwrap();
        let adapter = ZstdStorageAdapter::new(3, temp_tmp.path().to_path_buf());

        let src_file = src_dir.path().join("header.txt");
        let content = "line 1\nline 2\n=== CONTENT ===\nline 4\nline 5\n";
        fs::write(&src_file, content).unwrap();

        let compressed = adapter.compress_and_store(&src_file, dest_dir.path()).unwrap();

        let header = adapter.read_header(&compressed, 10).unwrap();
        assert_eq!(header, "line 1\nline 2\n=== CONTENT ===\n");

        let header2 = adapter.read_header(&compressed, 2).unwrap();
        assert_eq!(header2, "line 1\nline 2\n");
    }
}
