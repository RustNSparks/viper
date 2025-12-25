//! Package manager error types

use thiserror::Error;

/// Package manager error type
#[derive(Error, Debug)]
pub enum PmError {
    #[error("Failed to resolve dependencies: {0}")]
    Resolution(String),

    #[error("Failed to extract packages: {0}")]
    Extraction(String),

    #[error("Package not found: {0}")]
    PackageNotFound(String),

    #[error("Invalid package spec: {0}")]
    InvalidSpec(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse manifest: {0}")]
    ManifestParse(String),

    #[error("Lockfile error: {0}")]
    Lockfile(String),

    #[error("Registry error: {0}")]
    Registry(String),

    #[error("{0}")]
    Other(String),
}

/// Result type for package manager operations
pub type PmResult<T> = Result<T, PmError>;
