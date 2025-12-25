//! Viper Package Manager - Fast npm-compatible package management
//!
//! Powered by Orogene's node-maintainer for blazing fast dependency resolution
//! and package installation.
//!
//! # Features
//! - Full npm registry compatibility
//! - Parallel downloads and extraction
//! - Content-addressed global cache
//! - Hardlinks/reflinks for fast installs
//! - Lockfile support (package-lock.json compatible)

mod error;
mod installer;

pub use error::{PmError, PmResult};
pub use installer::{PackageManager, PackageManagerConfig};

/// Default npm registry URL
pub const DEFAULT_REGISTRY: &str = "https://registry.npmjs.org/";

/// Default concurrency for parallel operations
pub const DEFAULT_CONCURRENCY: usize = 50;
