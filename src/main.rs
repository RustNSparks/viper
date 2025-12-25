//! Viper - A TypeScript Runtime powered by Boa JS Engine and OXC
//!
//! Viper provides a fast TypeScript execution environment by combining:
//! - OXC: Fast TypeScript-to-JavaScript transpilation
//! - Boa: ECMAScript engine written in Rust
//! - boa_runtime: WebAPI support (console, fetch, URL, etc.)
//! - High-performance event loop for async operations

use clap::{Parser, Subcommand};
use colored::Colorize;
use miette::{IntoDiagnostic, Result};
use std::path::PathBuf;

use viper::bundler::{BundleConfig, BundleFormat, simple_bundle};
use viper::cli::Repl;
#[cfg(feature = "pm")]
use viper::pm::{PackageManager, PackageManagerConfig};
use viper::runtime::{Runtime, RuntimeConfig};
#[cfg(feature = "server")]
use viper::server;
use viper::transpiler::{Transpiler, TranspilerConfig};

#[derive(Parser)]
#[command(name = "viper")]
#[command(author, version, about = "A TypeScript runtime powered by Boa and OXC")]
struct Cli {
    /// TypeScript/JavaScript file to execute
    file: Option<PathBuf>,

    /// Evaluate TypeScript code from command line
    #[arg(short, long)]
    eval: Option<String>,

    /// Print the transpiled JavaScript without executing
    #[arg(long)]
    print: bool,

    /// Minify the output when using --print
    #[arg(long)]
    minify: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a TypeScript file with full event loop support (timers, async/await)
    Run {
        /// TypeScript/JavaScript file to run
        file: PathBuf,
    },
    /// Start an interactive REPL
    Repl,
    /// Transpile TypeScript to JavaScript without executing
    Transpile {
        /// Input TypeScript file
        input: PathBuf,
        /// Output JavaScript file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Minify the output
        #[arg(long)]
        minify: bool,
    },
    /// Run TypeScript code from stdin
    Stdin,
    /// Start HTTP server (requires --features server)
    #[cfg(feature = "server")]
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,
        /// Hostname to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
    /// Bundle TypeScript/JavaScript files
    Bundle {
        /// Entry point file(s)
        #[arg(required = true)]
        entry: Vec<PathBuf>,
        /// Output directory
        #[arg(short, long, default_value = "dist")]
        output: PathBuf,
        /// Output filename
        #[arg(short = 'f', long)]
        filename: Option<String>,
        /// Bundle format (esm, cjs, iife)
        #[arg(long, default_value = "esm")]
        format: String,
        /// Enable minification
        #[arg(long)]
        minify: bool,
        /// Enable source maps
        #[arg(long)]
        source_map: bool,
    },
    /// Install dependencies from package.json (requires --features pm)
    #[cfg(feature = "pm")]
    Install {
        /// Project root directory
        #[arg(short, long)]
        root: Option<PathBuf>,
    },
    /// Add packages to dependencies (requires --features pm)
    #[cfg(feature = "pm")]
    Add {
        /// Package names (e.g., lodash, express@4.18.0)
        #[arg(required = true)]
        packages: Vec<String>,
        /// Add as dev dependency
        #[arg(short = 'D', long)]
        dev: bool,
        /// Project root directory
        #[arg(short, long)]
        root: Option<PathBuf>,
    },
    /// Remove packages from dependencies (requires --features pm)
    #[cfg(feature = "pm")]
    Remove {
        /// Package names to remove
        #[arg(required = true)]
        packages: Vec<String>,
        /// Project root directory
        #[arg(short, long)]
        root: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run { file }) => {
            run_file_with_event_loop(&file)?;
        }
        Some(Commands::Repl) => {
            run_repl()?;
        }
        Some(Commands::Transpile {
            input,
            output,
            minify,
        }) => {
            transpile_file(&input, output.as_deref(), minify)?;
        }
        Some(Commands::Stdin) => {
            run_stdin()?;
        }
        #[cfg(feature = "server")]
        Some(Commands::Serve { port, host }) => {
            serve(port, host)?;
        }
        Some(Commands::Bundle {
            entry,
            output,
            filename,
            format,
            minify,
            source_map,
        }) => {
            bundle_files(entry, output, filename, &format, minify, source_map)?;
        }
        #[cfg(feature = "pm")]
        Some(Commands::Install { root }) => {
            pm_install(root)?;
        }
        #[cfg(feature = "pm")]
        Some(Commands::Add {
            packages,
            dev,
            root,
        }) => {
            pm_add(packages, dev, root)?;
        }
        #[cfg(feature = "pm")]
        Some(Commands::Remove { packages, root }) => {
            pm_remove(packages, root)?;
        }
        None => {
            if let Some(code) = cli.eval {
                // Evaluate code from command line
                if cli.print {
                    print_transpiled(&code, "eval.ts", cli.minify)?;
                } else {
                    eval_code(&code)?;
                }
            } else if let Some(file) = cli.file {
                // Execute file
                if cli.print {
                    let source = std::fs::read_to_string(&file).into_diagnostic()?;
                    let filename = file
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("input.ts");
                    print_transpiled(&source, filename, cli.minify)?;
                } else {
                    // Default behavior: run with event loop support
                    run_file_with_event_loop(&file)?;
                }
            } else {
                // No file or code provided, start REPL
                run_repl()?;
            }
        }
    }

    Ok(())
}

/// Bundle TypeScript/JavaScript files
fn bundle_files(
    entry: Vec<PathBuf>,
    output: PathBuf,
    filename: Option<String>,
    format: &str,
    minify: bool,
    source_map: bool,
) -> Result<()> {
    let bundle_format = match format.to_lowercase().as_str() {
        "esm" => BundleFormat::Esm,
        "cjs" | "commonjs" => BundleFormat::Cjs,
        "iife" => BundleFormat::Iife,
        _ => {
            eprintln!(
                "{}: Unknown format '{}'. Using 'esm'",
                "Warning".yellow(),
                format
            );
            BundleFormat::Esm
        }
    };

    let config = BundleConfig {
        entry,
        output_dir: output.clone(),
        output_filename: filename.clone(),
        minify,
        source_map,
        format: bundle_format,
        tree_shake: true,
        external: Vec::new(),
    };

    println!("{}: Bundling...", "Info".cyan());
    println!("  Entry points: {}", config.entry.len());
    println!("  Format: {:?}", config.format);
    println!("  Minify: {}", config.minify);

    match simple_bundle(config) {
        Ok(bundled) => {
            // Create output directory
            std::fs::create_dir_all(&output).into_diagnostic()?;

            // Determine output filename
            let out_filename = filename.unwrap_or_else(|| "bundle.js".to_string());
            let out_path = output.join(&out_filename);

            // Write bundle
            std::fs::write(&out_path, &bundled).into_diagnostic()?;

            println!(
                "{}: Bundle written to {}",
                "Success".green(),
                out_path.display()
            );
            println!("  Size: {} bytes", bundled.len());

            Ok(())
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            std::process::exit(1);
        }
    }
}

/// Start HTTP server
#[cfg(feature = "server")]
fn serve(port: u16, host: String) -> Result<()> {
    use std::sync::Arc;

    println!("{}: Starting Viper HTTP server...", "Info".cyan());

    let config = server::ServerConfig {
        hostname: host,
        port,
        development: false,
        static_dir: None,
        request_timeout_ms: 30000,
        max_body_size: 10 * 1024 * 1024,
    };

    // Simple example handler using the static handler API
    let handler: server::StaticHandler = Arc::new(|req| {
        println!("{} {} {}", "Request:".cyan(), req.method, req.url);

        // Simple routing
        match req.url.as_str() {
            "/" => server::ResponseBuilder::new().html(
                r#"<!DOCTYPE html>
<html>
<head><title>Viper Server</title></head>
<body>
    <h1>🐍 Viper HTTP Server</h1>
    <p>A fast TypeScript runtime powered by Boa and OXC</p>
    <h2>Endpoints:</h2>
    <ul>
        <li><code>GET /</code> - This page</li>
        <li><code>GET /api/hello</code> - JSON response</li>
        <li><code>GET /health</code> - Health check</li>
    </ul>
    <p>Powered by Axum</p>
</body>
</html>"#,
            ),
            "/api/hello" => server::ResponseBuilder::new()
                .json(r#"{"message":"Hello from Viper!","version":"0.1.0"}"#),
            "/health" => server::ResponseBuilder::new().json(r#"{"status":"ok"}"#),
            _ => server::ResponseBuilder::new()
                .status(404)
                .html("<h1>404 Not Found</h1>"),
        }
    });

    let server = server::Server::with_static_handler(config, handler);

    server.start().into_diagnostic()?;

    Ok(())
}

/// Run the interactive REPL
fn run_repl() -> Result<()> {
    let mut repl = Repl::new().into_diagnostic()?;
    repl.run().into_diagnostic()?;
    Ok(())
}

/// Run code from stdin
fn run_stdin() -> Result<()> {
    use std::io::Read;

    let mut code = String::new();
    std::io::stdin()
        .read_to_string(&mut code)
        .into_diagnostic()?;

    let mut runtime = Runtime::new().into_diagnostic()?;
    match runtime.run(&code, "stdin.ts") {
        Ok(value) => {
            if !value.is_undefined() {
                let result = runtime.value_to_string(&value);
                println!("{}", result);
            }
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Execute a TypeScript/JavaScript file with full event loop support
fn run_file_with_event_loop(path: &PathBuf) -> Result<()> {
    let config = RuntimeConfig {
        base_path: path.parent().map(|p| p.to_path_buf()).unwrap_or_default(),
        use_event_loop: true,
        ..Default::default()
    };

    let mut runtime = Runtime::with_config(config).into_diagnostic()?;

    match runtime.run_file(path) {
        Ok(value) => {
            if !value.is_undefined() {
                let result = runtime.value_to_string(&value);
                println!("{}", result);
            }
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Evaluate TypeScript code from command line
fn eval_code(code: &str) -> Result<()> {
    let mut runtime = Runtime::new().into_diagnostic()?;

    match runtime.run(code, "eval.ts") {
        Ok(value) => {
            if !value.is_undefined() {
                let result = runtime.value_to_string(&value);
                println!("{}", result);
            }
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Transpile and print without executing
fn print_transpiled(code: &str, filename: &str, minify: bool) -> Result<()> {
    let config = TranspilerConfig {
        minify,
        ..Default::default()
    };

    let transpiler = Transpiler::with_config(config);

    match transpiler.transpile(code, filename) {
        Ok(js) => {
            println!("{}", js);
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Transpile a file to JavaScript
fn transpile_file(input: &PathBuf, output: Option<&std::path::Path>, minify: bool) -> Result<()> {
    let config = TranspilerConfig {
        minify,
        ..Default::default()
    };

    let transpiler = Transpiler::with_config(config);

    match transpiler.transpile_file(input) {
        Ok(js) => {
            if let Some(output_path) = output {
                std::fs::write(output_path, &js).into_diagnostic()?;
                println!(
                    "{}: Transpiled {} -> {}",
                    "Success".green(),
                    input.display(),
                    output_path.display()
                );
            } else {
                println!("{}", js);
            }
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Install packages from package.json
#[cfg(feature = "pm")]
fn pm_install(root: Option<PathBuf>) -> Result<()> {
    let root_dir = root.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    println!("{}", "viper install".cyan().bold());

    let config = PackageManagerConfig::new(&root_dir);
    let pm = PackageManager::with_config(config);

    match pm.install() {
        Ok(result) => {
            println!();
            println!("{} {}", "+".green(), result);
            println!();
            println!("{}", result.timing_summary().dimmed());
            Ok(())
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            std::process::exit(1);
        }
    }
}

/// Add packages to dependencies
#[cfg(feature = "pm")]
fn pm_add(packages: Vec<String>, dev: bool, root: Option<PathBuf>) -> Result<()> {
    let root_dir = root.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let dep_type = if dev { "-D" } else { "" };
    println!(
        "{} {}",
        format!("viper add {}", dep_type).cyan().bold(),
        packages.join(" ")
    );

    let config = PackageManagerConfig::new(&root_dir);
    let pm = PackageManager::with_config(config);

    let pkg_refs: Vec<&str> = packages.iter().map(|s| s.as_str()).collect();

    match pm.add(&pkg_refs, dev) {
        Ok(result) => {
            println!();
            println!("{} {}", "+".green(), result);
            println!();
            println!("{}", result.timing_summary().dimmed());
            Ok(())
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            std::process::exit(1);
        }
    }
}

/// Remove packages from dependencies
#[cfg(feature = "pm")]
fn pm_remove(packages: Vec<String>, root: Option<PathBuf>) -> Result<()> {
    let root_dir = root.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    println!(
        "{}: Removing {}...",
        "Viper".cyan().bold(),
        packages.join(", ")
    );

    let config = PackageManagerConfig::new(&root_dir);
    let pm = PackageManager::with_config(config);

    let pkg_refs: Vec<&str> = packages.iter().map(|s| s.as_str()).collect();

    match pm.remove(&pkg_refs) {
        Ok(()) => {
            println!(
                "{}: Removed {} packages",
                "Success".green().bold(),
                packages.len()
            );
            Ok(())
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            std::process::exit(1);
        }
    }
}
