//! High-performance file system module for Viper runtime
//!
//! This module provides a blazingly fast file system API with async/await support,
//! optimized for performance using direct syscalls.
//!
//! Features:
//! - Ultra-fast synchronous operations (like Bun)
//! - Full Node.js fs API compatibility
//! - fs.promises support
//! - Zero-copy operations where possible
//! - Direct syscalls without async overhead

pub mod fast;
pub mod simple;

// The BlobFile, FileSink, and register_fs_module code below is temporarily disabled
// due to Boa API compatibility issues. The simple module provides the working API.
// This code will be re-enabled once the Boa APIs are updated.

/*
use boa_engine::{
    js_string, Context, JsArgs, JsData, JsError, JsNativeError, JsObject, JsResult, JsValue,
    NativeFunction,
};
use boa_gc::{Finalize, Trace};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// A file reference that lazily loads file contents
/// Similar to Bun's BunFile, implements the Blob interface
#[derive(Debug, Clone, Trace, Finalize, JsData)]
pub struct BlobFile {
    #[unsafe_ignore_trace]
    path: Arc<PathBuf>,
    #[unsafe_ignore_trace]
    mime_type: String,
}

impl BlobFile {
    /// Create a new file reference
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = Arc::new(path.as_ref().to_path_buf());
        let mime_type = Self::guess_mime_type(&path);

        Self {
            path,
            mime_type,
        }
    }

    /// Create a file reference with explicit MIME type
    pub fn with_type(path: impl AsRef<Path>, mime_type: String) -> Self {
        Self {
            path: Arc::new(path.as_ref().to_path_buf()),
            mime_type,
        }
    }

    /// Guess MIME type from file extension
    fn guess_mime_type(path: &Path) -> String {
        match path.extension().and_then(|e| e.to_str()) {
            Some("txt") => "text/plain;charset=utf-8".to_string(),
            Some("json") => "application/json;charset=utf-8".to_string(),
            Some("html") => "text/html;charset=utf-8".to_string(),
            Some("js") => "text/javascript;charset=utf-8".to_string(),
            Some("ts") => "text/typescript;charset=utf-8".to_string(),
            Some("css") => "text/css;charset=utf-8".to_string(),
            Some("png") => "image/png".to_string(),
            Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
            Some("svg") => "image/svg+xml".to_string(),
            Some("pdf") => "application/pdf".to_string(),
            Some("wasm") => "application/wasm".to_string(),
            _ => "text/plain;charset=utf-8".to_string(),
        }
    }

    /// Get file size (async)
    pub async fn size(&self) -> std::io::Result<u64> {
        let metadata = fs::metadata(&*self.path).await?;
        Ok(metadata.len())
    }

    /// Check if file exists
    pub async fn exists(&self) -> bool {
        fs::metadata(&*self.path).await.is_ok()
    }

    /// Read file contents as string
    pub async fn text(&self) -> std::io::Result<String> {
        fs::read_to_string(&*self.path).await
    }

    /// Read file contents as bytes
    pub async fn bytes(&self) -> std::io::Result<Vec<u8>> {
        fs::read(&*self.path).await
    }

    /// Get the file path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the MIME type
    pub fn mime_type(&self) -> &str {
        &self.mime_type
    }

    /// Delete the file
    pub async fn delete(&self) -> std::io::Result<()> {
        fs::remove_file(&*self.path).await
    }
}

/// Incremental file writer with buffering
/// Similar to Bun's FileSink
#[derive(Debug)]
pub struct FileSink {
    writer: tokio::io::BufWriter<tokio::fs::File>,
    path: PathBuf,
    high_water_mark: usize,
    bytes_written: u64,
}

impl FileSink {
    /// Create a new FileSink
    pub async fn new(path: impl AsRef<Path>, high_water_mark: Option<usize>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = fs::File::create(&path).await?;
        let high_water_mark = high_water_mark.unwrap_or(16 * 1024); // 16KB default
        let writer = tokio::io::BufWriter::with_capacity(high_water_mark, file);

        Ok(Self {
            writer,
            path,
            high_water_mark,
            bytes_written: 0,
        })
    }

    /// Write a chunk to the buffer
    pub async fn write(&mut self, chunk: &[u8]) -> std::io::Result<usize> {
        let written = self.writer.write(chunk).await?;
        self.bytes_written += written as u64;
        Ok(written)
    }

    /// Write a string chunk
    pub async fn write_str(&mut self, s: &str) -> std::io::Result<usize> {
        self.write(s.as_bytes()).await
    }

    /// Flush the buffer to disk
    pub async fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush().await
    }

    /// Flush and close the file
    pub async fn end(&mut self) -> std::io::Result<u64> {
        self.flush().await?;
        Ok(self.bytes_written)
    }

    /// Get the number of bytes written
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    /// Get the high water mark
    pub fn high_water_mark(&self) -> usize {
        self.high_water_mark
    }
}

/// High-performance write function
/// Optimized for different input types using best syscalls available
pub async fn write_file(
    destination: impl AsRef<Path>,
    data: &[u8],
) -> std::io::Result<u64> {
    fs::write(destination.as_ref(), data).await?;
    Ok(data.len() as u64)
}

/// Copy a file using optimized syscalls
pub async fn copy_file(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> std::io::Result<u64> {
    fs::copy(source.as_ref(), destination.as_ref()).await
}

/// Register the file system module in the JavaScript context
pub fn register_fs_module(context: &mut Context) -> JsResult<()> {
    // TODO: This needs to be updated for Boa 0.21 API
    // For now, use simple::register_file_system instead
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_blob_file_creation() {
        let file = BlobFile::new("test.txt");
        assert_eq!(file.mime_type(), "text/plain;charset=utf-8");
    }

    #[tokio::test]
    async fn test_mime_type_detection() {
        let file = BlobFile::new("test.json");
        assert_eq!(file.mime_type(), "application/json;charset=utf-8");

        let file = BlobFile::new("test.js");
        assert_eq!(file.mime_type(), "text/javascript;charset=utf-8");
    }
}
*/
