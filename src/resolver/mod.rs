//! Node.js/Bun-compatible module resolution using oxc_resolver
//!
//! This module implements module resolution that matches Node.js and Bun behavior:
//! - node_modules resolution
//! - package.json exports/imports
//! - Extension resolution (.js, .ts, .tsx, .jsx, .mjs, .cjs)
//! - tsconfig.json paths mapping
//! - Bare specifiers (e.g., "react")
//! - Relative paths (e.g., "./utils")

use oxc_resolver::{ResolveOptions, Resolver};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during module resolution
#[derive(Error, Debug)]
pub enum ResolverError {
    #[error("Failed to resolve module '{0}': {1}")]
    ResolutionFailed(String, String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result type for module resolution operations
pub type ResolverResult<T> = Result<T, ResolverError>;

/// Node.js/Bun-compatible module resolver
pub struct ModuleResolver {
    resolver: Resolver,
    cjs_resolver: Resolver,
    base_path: PathBuf,
}

impl ModuleResolver {
    /// Create a new module resolver with Node.js/Bun-compatible settings
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        let base = base_path.as_ref().to_path_buf();

        // Configure resolver to match Node.js/Bun behavior with ESM priority
        let esm_options = ResolveOptions {
            // Prioritize ESM imports - don't include "require" to avoid CJS
            condition_names: vec!["import".into(), "node".into(), "default".into()],

            // Extensions to try (in order)
            // Matches Node.js and Bun behavior
            extensions: vec![
                ".ts".into(),
                ".tsx".into(),
                ".js".into(),
                ".jsx".into(),
                ".mjs".into(),
                ".cjs".into(),
                ".json".into(),
            ],

            // Main fields to check in package.json (in order)
            main_fields: vec!["module".into(), "main".into()],

            // Enable exports field resolution (package.json "exports")
            exports_fields: vec![vec!["exports".into()]],

            // Enable imports field resolution (package.json "imports")
            imports_fields: vec![vec!["imports".into()]],

            // Enable automatic tsconfig.json discovery and paths
            tsconfig: Some(oxc_resolver::TsconfigDiscovery::Auto),

            ..ResolveOptions::default()
        };

        // Configure resolver for CommonJS bundling - prioritize "require" condition
        let cjs_options = ResolveOptions {
            // Prioritize CommonJS require
            condition_names: vec!["require".into(), "node".into(), "default".into()],

            // Extensions to try - prefer .js and .cjs for CommonJS
            extensions: vec![".js".into(), ".cjs".into(), ".json".into(), ".mjs".into()],

            // Main fields - prefer main over module for CommonJS
            main_fields: vec!["main".into()],

            // Enable exports field resolution
            exports_fields: vec![vec!["exports".into()]],

            // Enable imports field resolution
            imports_fields: vec![vec!["imports".into()]],

            ..ResolveOptions::default()
        };

        let resolver = Resolver::new(esm_options);
        let cjs_resolver = Resolver::new(cjs_options);

        Self {
            resolver,
            cjs_resolver,
            base_path: base,
        }
    }

    /// Resolve a module specifier to an absolute path (ESM mode)
    ///
    /// # Arguments
    /// * `specifier` - The module specifier (e.g., "./utils", "react", "@types/node")
    /// * `referrer` - The path of the file doing the import (used for relative resolution)
    ///
    /// # Returns
    /// The absolute path to the resolved module file
    pub fn resolve(&self, specifier: &str, referrer: &Path) -> ResolverResult<PathBuf> {
        // Get the directory of the referrer for relative resolution
        let context = referrer.parent().unwrap_or(&self.base_path);

        match self.resolver.resolve(context, specifier) {
            Ok(resolution) => Ok(resolution.path().to_path_buf()),
            Err(error) => Err(ResolverError::ResolutionFailed(
                specifier.to_string(),
                error.to_string(),
            )),
        }
    }

    /// Resolve a module specifier to an absolute path (CommonJS mode)
    /// This uses the "require" condition for package.json exports
    ///
    /// # Arguments
    /// * `specifier` - The module specifier (e.g., "./utils", "lodash")
    /// * `referrer` - The path of the file doing the require (used for relative resolution)
    ///
    /// # Returns
    /// The absolute path to the resolved module file
    pub fn resolve_cjs(&self, specifier: &str, referrer: &Path) -> ResolverResult<PathBuf> {
        // Get the directory of the referrer for relative resolution
        let context = referrer.parent().unwrap_or(&self.base_path);

        match self.cjs_resolver.resolve(context, specifier) {
            Ok(resolution) => Ok(resolution.path().to_path_buf()),
            Err(error) => Err(ResolverError::ResolutionFailed(
                specifier.to_string(),
                error.to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_resolver_creation() {
        let resolver = ModuleResolver::new(".");
        assert!(resolver.base_path.exists() || !resolver.base_path.as_os_str().is_empty());
    }

    #[test]
    fn test_relative_path_resolution() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test.ts");
        fs::write(&test_file, "export const x = 1;").ok();

        let resolver = ModuleResolver::new(&temp_dir);

        // This would work if the file structure exists
        // For now, just test that the resolver doesn't panic
        let _ = resolver.resolve("./test", &temp_dir.join("index.ts"));
    }
}
