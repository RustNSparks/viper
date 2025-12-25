//! Viper - A TypeScript Runtime powered by Boa JS Engine and OXC
//!
//! Viper provides a fast TypeScript execution environment by combining:
//! - OXC: Fast TypeScript-to-JavaScript transpilation
//! - Boa: ECMAScript engine written in Rust
//! - boa_runtime: WebAPI support (console, fetch, URL, etc.)
//! - High-performance event loop for async operations

pub mod bundler;
pub mod cli;
pub mod fs;
#[cfg(feature = "pm")]
pub mod pm;
pub mod resolver;
pub mod runtime;
#[cfg(feature = "server")]
pub mod server;
pub mod transpiler;

// Re-export commonly used types
pub use bundler::{BundleConfig, BundleError, BundleFormat, BundleResult};
#[cfg(feature = "pm")]
pub use pm::{PackageManager, PackageManagerConfig, PmError, PmResult};
pub use resolver::ModuleResolver;
pub use runtime::{Runtime, RuntimeConfig, RuntimeError, RuntimeResult};
#[cfg(feature = "server")]
pub use server::{Server, ServerConfig, ServerError, ServerResult};
pub use transpiler::{TranspileError, Transpiler, TranspilerConfig};
