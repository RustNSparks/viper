//! Bundler module powered by Rolldown
//!
//! Rolldown is a fast Rust-based bundler that's Rollup-compatible and designed for Vite.
//! It provides 10-30x faster bundling than Rollup with full plugin ecosystem support.
//!
//! Features:
//! - Fast Rust-based bundling
//! - Rollup-compatible API
//! - Built-in minification
//! - Tree-shaking
//! - Code splitting
//! - Source maps

#[cfg(feature = "rolldown")]
use rolldown::{Bundler, BundlerOptions, InputOptions, OutputOptions};
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during bundling
#[derive(Error, Debug)]
pub enum BundleError {
    #[error("Bundling failed: {0}")]
    BundleFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Rolldown feature not enabled")]
    FeatureNotEnabled,
}

/// Result type for bundler operations
pub type BundleResult<T> = Result<T, BundleError>;

/// Configuration for the bundler
#[derive(Debug, Clone)]
pub struct BundleConfig {
    /// Entry point(s) for the bundle
    pub entry: Vec<PathBuf>,

    /// Output directory
    pub output_dir: PathBuf,

    /// Output filename
    pub output_filename: Option<String>,

    /// Enable minification
    pub minify: bool,

    /// Enable source maps
    pub source_map: bool,

    /// Target format (esm, cjs, iife)
    pub format: BundleFormat,

    /// Enable tree-shaking
    pub tree_shake: bool,

    /// External modules (won't be bundled)
    pub external: Vec<String>,
}

/// Bundle output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleFormat {
    /// ES Module format
    Esm,
    /// CommonJS format
    Cjs,
    /// Immediately Invoked Function Expression
    Iife,
}

impl Default for BundleConfig {
    fn default() -> Self {
        Self {
            entry: Vec::new(),
            output_dir: PathBuf::from("dist"),
            output_filename: None,
            minify: false,
            source_map: false,
            format: BundleFormat::Esm,
            tree_shake: true,
            external: Vec::new(),
        }
    }
}

/// Bundle TypeScript/JavaScript files using Rolldown
#[cfg(feature = "rolldown")]
pub async fn bundle(config: BundleConfig) -> BundleResult<()> {
    use rolldown::{Bundler, BundlerOptions, InputItem, OutputFormat};

    // Convert entry points to InputItem
    let input_items: Vec<InputItem> = config.entry
        .iter()
        .enumerate()
        .map(|(idx, path)| {
            let name = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&format!("entry{}", idx))
                .to_string();

            InputItem {
                name: Some(name),
                import: path.to_string_lossy().to_string(),
            }
        })
        .collect();

    // Convert format
    let output_format = match config.format {
        BundleFormat::Esm => OutputFormat::Esm,
        BundleFormat::Cjs => OutputFormat::Cjs,
        BundleFormat::Iife => OutputFormat::Iife,
    };

    // Create bundler with options
    let mut bundler = Bundler::new(BundlerOptions {
        input: Some(input_items),
        dir: Some(config.output_dir.to_string_lossy().to_string()),
        format: Some(output_format),
        minify: Some(rolldown::RawMinifyOptions::Bool(config.minify)),
        sourcemap: config.source_map.then(|| rolldown::SourceMapType::File),
        external: if config.external.is_empty() {
            None
        } else {
            Some(config.external)
        },
        ..Default::default()
    });

    // Run bundler
    bundler.write()
        .await
        .map_err(|e| BundleError::BundleFailed(format!("Rolldown bundling failed: {:?}", e)))?;

    Ok(())
}

/// Bundle TypeScript/JavaScript files (when rolldown feature is disabled)
#[cfg(not(feature = "rolldown"))]
pub fn bundle(_config: BundleConfig) -> BundleResult<()> {
    Err(BundleError::FeatureNotEnabled)
}

/// Simple bundler using Viper's existing transpiler
/// This is a basic implementation that concatenates transpiled modules
pub fn simple_bundle(config: BundleConfig) -> BundleResult<String> {
    use crate::transpiler::Transpiler;

    if config.entry.is_empty() {
        return Err(BundleError::BundleFailed("No entry points provided".to_string()));
    }

    let transpiler = Transpiler::new();
    let mut output = String::new();

    // Add banner comment
    output.push_str("// Bundled by Viper\n");
    output.push_str(&format!("// Format: {:?}\n", config.format));
    output.push_str("// Entry points:\n");
    for entry in &config.entry {
        output.push_str(&format!("//   - {}\n", entry.display()));
    }
    output.push_str("\n");

    // Handle different formats
    match config.format {
        BundleFormat::Iife => {
            output.push_str("(function() {\n");
            output.push_str("'use strict';\n\n");
        }
        BundleFormat::Esm => {
            // ES modules don't need wrapper
        }
        BundleFormat::Cjs => {
            output.push_str("'use strict';\n\n");
        }
    }

    // Transpile and concatenate all entry points
    for entry in &config.entry {
        let source = std::fs::read_to_string(entry)?;
        let filename = entry.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("module.ts");

        match transpiler.transpile(&source, filename) {
            Ok(js) => {
                output.push_str(&format!("\n// === {} ===\n", entry.display()));
                output.push_str(&js);
                output.push('\n');
            }
            Err(e) => {
                return Err(BundleError::BundleFailed(format!(
                    "Failed to transpile {}: {}",
                    entry.display(),
                    e
                )));
            }
        }
    }

    // Close IIFE wrapper
    if config.format == BundleFormat::Iife {
        output.push_str("\n})();\n");
    }

    // Minification (basic)
    if config.minify {
        // Simple minification: remove comments and extra whitespace
        output = output
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BundleConfig::default();
        assert_eq!(config.format, BundleFormat::Esm);
        assert!(config.tree_shake);
        assert!(!config.minify);
    }

    #[test]
    fn test_simple_bundle_no_entries() {
        let config = BundleConfig::default();
        let result = simple_bundle(config);
        assert!(result.is_err());
    }
}
