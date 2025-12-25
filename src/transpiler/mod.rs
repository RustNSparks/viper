//! TypeScript to JavaScript transpiler using OXC
//!
//! This module handles the compilation of TypeScript code to JavaScript
//! using the OXC toolchain (parser, transformer, codegen).
//!
//! Supports:
//! - TypeScript (.ts)
//! - TSX (.tsx)
//! - JavaScript (.js)
//! - JSX (.jsx)

use oxc_allocator::Allocator;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{JsxOptions, JsxRuntime, TransformOptions, Transformer};
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during TypeScript transpilation
#[derive(Error, Debug)]
pub enum TranspileError {
    #[error("Failed to parse TypeScript: {0}")]
    ParseError(String),

    #[error("Failed to transform TypeScript: {0}")]
    TransformError(String),

    #[error("Invalid source type: {0}")]
    InvalidSourceType(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result type for transpilation operations
pub type TranspileResult<T> = Result<T, TranspileError>;

/// Configuration for the TypeScript transpiler
#[derive(Debug, Clone)]
pub struct TranspilerConfig {
    /// Whether to minify the output
    pub minify: bool,
    /// JSX runtime mode (classic or automatic)
    pub jsx_runtime: JsxRuntimeMode,
    /// JSX pragma for classic runtime (e.g., "React.createElement" or "h")
    pub jsx_pragma: Option<String>,
    /// JSX fragment pragma for classic runtime (e.g., "React.Fragment")
    pub jsx_pragma_frag: Option<String>,
    /// Import source for automatic runtime (e.g., "react" or "preact")
    pub jsx_import_source: Option<String>,
}

/// JSX runtime mode
#[derive(Debug, Clone, Default)]
pub enum JsxRuntimeMode {
    /// Classic mode: React.createElement calls
    Classic,
    /// Automatic mode: automatic JSX runtime (React 17+)
    #[default]
    Automatic,
}

impl Default for TranspilerConfig {
    fn default() -> Self {
        Self {
            minify: false,
            jsx_runtime: JsxRuntimeMode::Classic,
            jsx_pragma: Some("__viper_jsx".to_string()),
            jsx_pragma_frag: Some("__viper_fragment".to_string()),
            jsx_import_source: None,
        }
    }
}

/// TypeScript transpiler that converts TypeScript to JavaScript
pub struct Transpiler {
    config: TranspilerConfig,
}

impl Transpiler {
    /// Create a new transpiler with default configuration
    pub fn new() -> Self {
        Self {
            config: TranspilerConfig::default(),
        }
    }

    /// Create a new transpiler with custom configuration
    pub fn with_config(config: TranspilerConfig) -> Self {
        Self { config }
    }

    /// Build JSX options based on config
    fn build_jsx_options(&self) -> JsxOptions {
        let mut jsx_options = JsxOptions::default();

        // Enable JSX transformation
        jsx_options.jsx_plugin = true;

        // Set runtime
        jsx_options.runtime = match self.config.jsx_runtime {
            JsxRuntimeMode::Classic => JsxRuntime::Classic,
            JsxRuntimeMode::Automatic => JsxRuntime::Automatic,
        };

        // Set pragma for classic mode
        if let Some(ref pragma) = self.config.jsx_pragma {
            jsx_options.pragma = Some(pragma.clone().into());
        }

        // Set fragment pragma for classic mode
        if let Some(ref pragma_frag) = self.config.jsx_pragma_frag {
            jsx_options.pragma_frag = Some(pragma_frag.clone().into());
        }

        // Set import source for automatic mode
        if let Some(ref import_source) = self.config.jsx_import_source {
            jsx_options.import_source = Some(import_source.clone().into());
        }

        jsx_options
    }

    /// Transpile TypeScript source code to JavaScript
    ///
    /// # Arguments
    /// * `source` - The TypeScript source code
    /// * `filename` - The filename (used to determine source type, e.g., .ts, .tsx)
    ///
    /// # Returns
    /// The transpiled JavaScript code
    pub fn transpile(&self, source: &str, filename: &str) -> TranspileResult<String> {
        // Create allocator for AST nodes
        let allocator = Allocator::default();

        // Determine source type from filename
        let source_type = SourceType::from_path(filename).map_err(|e| {
            TranspileError::InvalidSourceType(format!("Unknown file extension: {:?}", e))
        })?;

        // Parse the TypeScript source
        let parser_return = Parser::new(&allocator, source, source_type).parse();

        // Check for parse errors
        if !parser_return.errors.is_empty() {
            let error_messages: Vec<String> = parser_return
                .errors
                .iter()
                .map(|e| e.to_string())
                .collect();
            return Err(TranspileError::ParseError(error_messages.join("\n")));
        }

        // Get the parsed program
        let mut program = parser_return.program;

        // Build semantic analysis to get scoping information
        let semantic_ret = SemanticBuilder::new().build(&program);

        // Check for semantic errors (optional - we can continue with warnings)
        if !semantic_ret.errors.is_empty() {
            // Log warnings but don't fail
            for error in &semantic_ret.errors {
                eprintln!("Warning: {}", error);
            }
        }

        // Extract scoping from semantic
        let scoping = semantic_ret.semantic.into_scoping();

        // Set up transform options with JSX support
        let mut transform_options = TransformOptions::default();
        transform_options.jsx = self.build_jsx_options();

        // Create a path for the transformer
        let source_path = Path::new(filename);

        // Transform TypeScript to JavaScript
        let transformer_return = Transformer::new(&allocator, source_path, &transform_options)
            .build_with_scoping(scoping, &mut program);

        // Check for transform errors
        if !transformer_return.errors.is_empty() {
            let error_messages: Vec<String> = transformer_return
                .errors
                .iter()
                .map(|e| e.to_string())
                .collect();
            return Err(TranspileError::TransformError(error_messages.join("\n")));
        }

        // Generate JavaScript code from the transformed AST
        let codegen_options = CodegenOptions {
            minify: self.config.minify,
            ..Default::default()
        };

        let codegen_return = Codegen::new()
            .with_options(codegen_options)
            .build(&program);

        Ok(codegen_return.code)
    }

    /// Transpile TypeScript from a file path
    pub fn transpile_file(&self, path: &std::path::Path) -> TranspileResult<String> {
        let source = std::fs::read_to_string(path)?;

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("input.ts");

        self.transpile(&source, filename)
    }
}

impl Default for Transpiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_typescript_transpilation() {
        let transpiler = Transpiler::new();
        let ts_code = r#"
            const message: string = "Hello, World!";
            console.log(message);
        "#;

        let result = transpiler.transpile(ts_code, "test.ts");
        assert!(result.is_ok());

        let js_code = result.unwrap();
        // TypeScript type annotations should be stripped
        assert!(!js_code.contains(": string"));
        assert!(js_code.contains("Hello, World!"));
    }

    #[test]
    fn test_interface_removal() {
        let transpiler = Transpiler::new();
        let ts_code = r#"
            interface User {
                name: string;
                age: number;
            }

            const user: User = { name: "Alice", age: 30 };
        "#;

        let result = transpiler.transpile(ts_code, "test.ts");
        assert!(result.is_ok());

        let js_code = result.unwrap();
        // Interface should be removed
        assert!(!js_code.contains("interface"));
    }

    #[test]
    fn test_tsx_support() {
        let transpiler = Transpiler::new();
        let tsx_code = r#"
            const element = <div>Hello</div>;
        "#;

        let result = transpiler.transpile(tsx_code, "test.tsx");
        assert!(result.is_ok());

        let js_code = result.unwrap();
        // JSX should be transformed (automatic runtime uses jsx function)
        assert!(js_code.contains("jsx") || js_code.contains("createElement"));
    }

    #[test]
    fn test_tsx_with_props() {
        let transpiler = Transpiler::new();
        let tsx_code = r#"
            interface Props {
                name: string;
            }
            const Greeting = (props: Props) => <div>Hello, {props.name}!</div>;
            const el = <Greeting name="World" />;
        "#;

        let result = transpiler.transpile(tsx_code, "test.tsx");
        assert!(result.is_ok());
    }

    #[test]
    fn test_jsx_classic_mode() {
        let config = TranspilerConfig {
            jsx_runtime: JsxRuntimeMode::Classic,
            jsx_pragma: Some("h".to_string()),
            jsx_pragma_frag: Some("Fragment".to_string()),
            ..Default::default()
        };
        let transpiler = Transpiler::with_config(config);
        let jsx_code = r#"const el = <div>Hello</div>;"#;

        let result = transpiler.transpile(jsx_code, "test.jsx");
        assert!(result.is_ok());

        let js_code = result.unwrap();
        // Classic mode should use the pragma function
        assert!(js_code.contains("h(") || js_code.contains("createElement"));
    }
}
