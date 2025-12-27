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

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "viper")]
#[command(version, about = format!("Viper is a fast TypeScript runtime, package manager, and bundler. ({})", VERSION))]
#[command(after_help = format!(r#"{}
  run       ./my-script.ts       Execute a file with Viper
  test                           Run unit tests with Viper
  repl                           Start a REPL session with Viper

  install                        Install dependencies for a package.json (viper i)
  add       express              Add a dependency to package.json (viper a)
  remove    is-array             Remove a dependency from package.json (viper rm)
  update    react                Update outdated dependencies
  link      [<package>]          Register or link a local npm package
  pm        <subcommand>         Additional package management utilities

  build     ./a.ts ./b.jsx       Bundle TypeScript & JavaScript into a single file

  init                           Start an empty Viper project from a blank template
  upgrade                        Upgrade to latest version of Viper.

Learn more about Viper:          https://github.com/user/viper
"#, "Commands:".bold()))]
#[command(disable_help_subcommand = true)]
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
    /// Execute a file with Viper
    Run {
        /// TypeScript/JavaScript file to run
        file: PathBuf,
    },

    /// Start a REPL session with Viper
    Repl,

    /// Bundle TypeScript & JavaScript into a single file
    Build {
        /// Entry point file(s)
        #[arg(required = true)]
        entry: Vec<PathBuf>,

        /// Output directory
        #[arg(short, long, default_value = "dist")]
        outdir: PathBuf,

        /// Output filename
        #[arg(short = 'n', long = "outfile")]
        outfile: Option<String>,

        /// Bundle format (esm, cjs, iife)
        #[arg(long, default_value = "esm")]
        format: String,

        /// Enable minification
        #[arg(long)]
        minify: bool,

        /// Enable source maps
        #[arg(long)]
        sourcemap: bool,
    },

    /// Transpile TypeScript to JavaScript without executing
    #[command(hide = true)]
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

    /// Read and execute code from stdin
    #[command(hide = true)]
    Stdin,

    // =========================================================================
    // Package Manager Commands
    // =========================================================================
    /// Install dependencies for a package.json
    #[cfg(feature = "pm")]
    #[command(alias = "i")]
    Install {
        /// Project root directory
        #[arg(short, long)]
        cwd: Option<PathBuf>,
    },

    /// Add a dependency to package.json
    #[cfg(feature = "pm")]
    #[command(alias = "a")]
    Add {
        /// Package names (e.g., lodash, express@4.18.0)
        #[arg(required = true)]
        packages: Vec<String>,

        /// Add as dev dependency
        #[arg(short = 'd', long)]
        dev: bool,

        /// Project root directory
        #[arg(short, long)]
        cwd: Option<PathBuf>,
    },

    /// Remove a dependency from package.json
    #[cfg(feature = "pm")]
    #[command(alias = "rm")]
    Remove {
        /// Package names to remove
        #[arg(required = true)]
        packages: Vec<String>,

        /// Project root directory
        #[arg(short, long)]
        cwd: Option<PathBuf>,
    },

    /// Update outdated dependencies
    #[cfg(feature = "pm")]
    Update {
        /// Specific packages to update (updates all if not specified)
        packages: Vec<String>,

        /// Project root directory
        #[arg(short, long)]
        cwd: Option<PathBuf>,
    },

    /// Link a local npm package
    #[cfg(feature = "pm")]
    Link {
        /// Package to link
        package: Option<String>,
    },

    /// Display latest versions of outdated dependencies
    #[cfg(feature = "pm")]
    Outdated {
        /// Project root directory
        #[arg(short, long)]
        cwd: Option<PathBuf>,
    },

    /// Additional package management utilities
    #[cfg(feature = "pm")]
    Pm {
        #[command(subcommand)]
        command: PmCommands,
    },

    /// Display package metadata from the registry
    #[cfg(feature = "pm")]
    Info {
        /// Package name (e.g., lodash, express@4.18.0)
        package: String,

        /// Show all versions
        #[arg(long)]
        versions: bool,
    },

    // =========================================================================
    // Project Commands
    // =========================================================================
    /// Start an empty Viper project from a blank template
    Init {
        /// Package name (defaults to directory name)
        #[arg(short, long)]
        name: Option<String>,

        /// Skip interactive prompts
        #[arg(short, long)]
        yes: bool,
    },

    /// Start HTTP server
    #[cfg(feature = "server")]
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Hostname to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },

    /// Upgrade to latest version of Viper
    Upgrade,
}

/// Package manager subcommands
#[cfg(feature = "pm")]
#[derive(Subcommand)]
enum PmCommands {
    /// List installed packages
    #[command(alias = "ls")]
    List {
        /// Depth of dependencies to show (0 = top-level only)
        #[arg(short, long, default_value = "0")]
        depth: usize,

        /// Project root directory
        #[arg(short, long)]
        cwd: Option<PathBuf>,
    },

    /// Test connectivity to the npm registry
    Ping,

    /// Show the cache directory
    Cache,

    /// Clean the cache
    CacheClean,
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
        Some(Commands::Build {
            entry,
            outdir,
            outfile,
            format,
            minify,
            sourcemap,
        }) => {
            bundle_files(entry, outdir, outfile, &format, minify, sourcemap)?;
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
        #[cfg(feature = "pm")]
        Some(Commands::Install { cwd }) => {
            pm_install(cwd)?;
        }
        #[cfg(feature = "pm")]
        Some(Commands::Add { packages, dev, cwd }) => {
            pm_add(packages, dev, cwd)?;
        }
        #[cfg(feature = "pm")]
        Some(Commands::Remove { packages, cwd }) => {
            pm_remove(packages, cwd)?;
        }
        #[cfg(feature = "pm")]
        Some(Commands::Update { packages, cwd }) => {
            pm_update(packages, cwd)?;
        }
        #[cfg(feature = "pm")]
        Some(Commands::Link { package: _ }) => {
            eprintln!("{}: link command not yet implemented", "error".red());
            std::process::exit(1);
        }
        #[cfg(feature = "pm")]
        Some(Commands::Outdated { cwd: _ }) => {
            eprintln!("{}: outdated command not yet implemented", "error".red());
            std::process::exit(1);
        }
        #[cfg(feature = "pm")]
        Some(Commands::Info { package, versions }) => {
            pm_view(&package, versions)?;
        }
        #[cfg(feature = "pm")]
        Some(Commands::Pm { command }) => match command {
            PmCommands::List { depth, cwd } => {
                pm_list(depth, cwd)?;
            }
            PmCommands::Ping => {
                pm_ping()?;
            }
            PmCommands::Cache => {
                pm_cache()?;
            }
            PmCommands::CacheClean => {
                pm_cache_clean()?;
            }
        },
        Some(Commands::Init { name, yes }) => {
            #[cfg(feature = "pm")]
            pm_init(name, yes)?;
            #[cfg(not(feature = "pm"))]
            {
                let _ = (name, yes);
                eprintln!("{}: pm feature not enabled", "error".red());
                std::process::exit(1);
            }
        }
        Some(Commands::Upgrade) => {
            println!(
                "{}",
                "You are already on the latest version of Viper.".green()
            );
        }
        None => {
            if let Some(code) = cli.eval {
                if cli.print {
                    print_transpiled(&code, "eval.ts", cli.minify)?;
                } else {
                    eval_code(&code)?;
                }
            } else if let Some(file) = cli.file {
                if cli.print {
                    let source = std::fs::read_to_string(&file).into_diagnostic()?;
                    let filename = file
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("input.ts");
                    print_transpiled(&source, filename, cli.minify)?;
                } else {
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
                "warning".yellow(),
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

    let start = std::time::Instant::now();

    match simple_bundle(config) {
        Ok(bundled) => {
            std::fs::create_dir_all(&output).into_diagnostic()?;

            let out_filename = filename.unwrap_or_else(|| "bundle.js".to_string());
            let out_path = output.join(&out_filename);

            std::fs::write(&out_path, &bundled).into_diagnostic()?;

            let elapsed = start.elapsed();
            println!(
                "  {} {} {} [{:.2?}]",
                out_path.display(),
                format!("{} bytes", bundled.len()).dimmed(),
                "✓".green(),
                elapsed
            );

            Ok(())
        }
        Err(e) => {
            eprintln!("{}: {}", "error".red(), e);
            std::process::exit(1);
        }
    }
}

/// Start HTTP server
#[cfg(feature = "server")]
fn serve(port: u16, host: String) -> Result<()> {
    use std::sync::Arc;

    println!("{} Listening on {}:{}", "Started".green(), host, port);

    let config = server::ServerConfig {
        hostname: host,
        port,
        development: false,
        static_dir: None,
        request_timeout_ms: 30000,
        max_body_size: 10 * 1024 * 1024,
    };

    let handler: server::StaticHandler = Arc::new(|req| {
        println!("{} {}", req.method.cyan(), req.url);

        match req.url.as_str() {
            "/" => server::ResponseBuilder::new().html(
                r#"<!DOCTYPE html>
<html>
<head><title>Viper Server</title></head>
<body>
    <h1>Viper HTTP Server</h1>
    <p>A fast TypeScript runtime powered by Boa and OXC</p>
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
            eprintln!("{}: {}", "error".red(), e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Execute a TypeScript/JavaScript file with full event loop support
fn run_file_with_event_loop(path: &PathBuf) -> Result<()> {
    let base_path = std::env::current_dir()
        .unwrap_or_else(|_| path.parent().map(|p| p.to_path_buf()).unwrap_or_default());

    let config = RuntimeConfig {
        base_path,
        use_event_loop: true,
        ..Default::default()
    };

    let mut runtime = Runtime::with_config(config).into_diagnostic()?;

    match runtime.run_file(path) {
        Ok(_value) => {
            // Don't print return values for file execution (unlike REPL)
        }
        Err(e) => {
            eprintln!("{}: {}", "error".red(), e);
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
            eprintln!("{}: {}", "error".red(), e);
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
            eprintln!("{}: {}", "error".red(), e);
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
                    "{} {} -> {}",
                    "Transpiled".green(),
                    input.display(),
                    output_path.display()
                );
            } else {
                println!("{}", js);
            }
        }
        Err(e) => {
            eprintln!("{}: {}", "error".red(), e);
            std::process::exit(1);
        }
    }

    Ok(())
}

// =============================================================================
// Package Manager Commands
// =============================================================================

#[cfg(feature = "pm")]
fn pm_install(root: Option<PathBuf>) -> Result<()> {
    use viper::pm::InstallResult;

    let root_dir = root.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    println!("{} v{}", "viper install".cyan().bold(), VERSION.dimmed());

    let config = PackageManagerConfig::new(&root_dir);
    let pm = PackageManager::with_config(config);

    match pm.install() {
        Ok(result) => {
            println!();
            for pkg in &result.added_packages {
                println!("{} {}", "+".green(), pkg);
            }
            println!();
            let label = if result.extracted == 1 {
                "package"
            } else {
                "packages"
            };
            println!(
                "{} {} installed [{}]",
                result.extracted,
                label,
                InstallResult::format_duration(result.total_time)
            );
            println!();
            println!("{}", result.timing_summary().dimmed());
            Ok(())
        }
        Err(e) => {
            eprintln!("{}: {}", "error".red().bold(), e);
            std::process::exit(1);
        }
    }
}

#[cfg(feature = "pm")]
fn pm_add(packages: Vec<String>, dev: bool, root: Option<PathBuf>) -> Result<()> {
    use viper::pm::InstallResult;

    let root_dir = root.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    println!("{} v{}", "viper add".cyan().bold(), VERSION.dimmed());

    let config = PackageManagerConfig::new(&root_dir);
    let pm = PackageManager::with_config(config);

    let pkg_refs: Vec<&str> = packages.iter().map(|s| s.as_str()).collect();

    match pm.add(&pkg_refs, dev) {
        Ok(result) => {
            println!();
            for pkg in &packages {
                let pkg_name = pkg.split('@').next().unwrap_or(pkg);
                let resolved = result
                    .added_packages
                    .iter()
                    .find(|p| p.starts_with(&format!("{}@", pkg_name)))
                    .map(|s| s.as_str())
                    .unwrap_or(pkg);
                println!("{} {}", "installed".green(), resolved);
            }
            println!();
            let label = if result.extracted == 1 {
                "package"
            } else {
                "packages"
            };
            println!(
                "{} {} installed [{}]",
                result.extracted,
                label,
                InstallResult::format_duration(result.total_time)
            );
            println!();
            println!("{}", result.timing_summary().dimmed());
            Ok(())
        }
        Err(e) => {
            eprintln!("{}: {}", "error".red().bold(), e);
            std::process::exit(1);
        }
    }
}

#[cfg(feature = "pm")]
fn pm_remove(packages: Vec<String>, root: Option<PathBuf>) -> Result<()> {
    use viper::pm::InstallResult;

    let root_dir = root.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    println!("{} v{}", "viper remove".cyan().bold(), VERSION.dimmed());

    let config = PackageManagerConfig::new(&root_dir);
    let pm = PackageManager::with_config(config);

    let pkg_refs: Vec<&str> = packages.iter().map(|s| s.as_str()).collect();
    let start = std::time::Instant::now();

    match pm.remove(&pkg_refs) {
        Ok(()) => {
            let elapsed = start.elapsed();
            println!();
            for pkg in &packages {
                println!("{} {}", "-".red(), pkg);
            }
            let label = if packages.len() == 1 {
                "package"
            } else {
                "packages"
            };
            println!(
                "{} {} removed [{}]",
                packages.len(),
                label,
                InstallResult::format_duration(elapsed)
            );
            println!();
            Ok(())
        }
        Err(e) => {
            eprintln!("{}: {}", "error".red().bold(), e);
            std::process::exit(1);
        }
    }
}

#[cfg(feature = "pm")]
fn create_spinner(msg: &str) -> indicatif::ProgressBar {
    use indicatif::{ProgressBar, ProgressStyle};
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

#[cfg(feature = "pm")]
fn pm_view(package: &str, show_versions: bool) -> Result<()> {
    use viper::pm::InstallResult;

    let spinner = create_spinner(&format!("Fetching {}...", package));
    let start = std::time::Instant::now();
    let config = PackageManagerConfig::new(std::env::current_dir().unwrap_or_default());
    let pm = PackageManager::with_config(config);

    match pm.view(package) {
        Ok(info) => {
            spinner.finish_and_clear();
            println!(
                "\n{}@{}\n",
                info.name.cyan().bold(),
                info.latest_version.green()
            );

            if let Some(desc) = &info.description {
                println!("{}", desc);
                println!();
            }

            if let Some(license) = &info.license {
                println!("{}: {}", "license".dimmed(), license);
            }
            if let Some(homepage) = &info.homepage {
                println!("{}: {}", "homepage".dimmed(), homepage);
            }
            if let Some(repo) = &info.repository {
                println!("{}: {}", "repository".dimmed(), repo);
            }
            if let Some(author) = &info.author {
                println!("{}: {}", "author".dimmed(), author);
            }

            if !info.keywords.is_empty() {
                println!("{}: {}", "keywords".dimmed(), info.keywords.join(", "));
            }

            println!(
                "{}: {} dependencies, {} dev",
                "deps".dimmed(),
                info.dependencies_count,
                info.dev_dependencies_count
            );

            if !info.dist_tags.is_empty() {
                println!();
                println!("{}", "dist-tags:".dimmed());
                for (tag, version) in &info.dist_tags {
                    println!("  {}: {}", tag.yellow(), version);
                }
            }

            if show_versions {
                println!();
                println!("{} ({} total):", "versions".dimmed(), info.versions.len());
                let cols = 5;
                for (i, version) in info.versions.iter().enumerate() {
                    if i % cols == 0 && i > 0 {
                        println!();
                    }
                    print!("  {:15}", version);
                }
                println!();
            } else {
                println!(
                    "\n{}: {} (use --versions to show all)",
                    "versions".dimmed(),
                    info.versions.len()
                );
            }

            println!(
                "\n{}\n",
                format!(
                    "fetched in {}",
                    InstallResult::format_duration(start.elapsed())
                )
                .dimmed()
            );

            Ok(())
        }
        Err(e) => {
            spinner.finish_and_clear();
            eprintln!("{}: {}", "error".red().bold(), e);
            std::process::exit(1);
        }
    }
}

#[cfg(feature = "pm")]
fn pm_ping() -> Result<()> {
    use viper::pm::InstallResult;

    let config = PackageManagerConfig::new(std::env::current_dir().unwrap_or_default());
    let pm = PackageManager::with_config(config);

    let spinner = create_spinner(&format!("PING {} ...", viper::pm::DEFAULT_REGISTRY));

    match pm.ping() {
        Ok(duration) => {
            spinner.finish_and_clear();
            println!(
                "{} {} {} ({})",
                "PING".cyan(),
                viper::pm::DEFAULT_REGISTRY,
                "PONG".green().bold(),
                InstallResult::format_duration(duration)
            );
            Ok(())
        }
        Err(e) => {
            spinner.finish_and_clear();
            println!(
                "{} {} {}",
                "PING".cyan(),
                viper::pm::DEFAULT_REGISTRY,
                "FAILED".red().bold()
            );
            eprintln!("{}: {}", "error".red().bold(), e);
            std::process::exit(1);
        }
    }
}

#[cfg(feature = "pm")]
fn pm_init(name: Option<String>, _yes: bool) -> Result<()> {
    println!("{} v{}", "viper init".cyan().bold(), VERSION.dimmed());

    let config = PackageManagerConfig::new(std::env::current_dir().unwrap_or_default());
    let pm = PackageManager::with_config(config);

    match pm.init(name.as_deref(), false) {
        Ok(()) => {
            println!();
            println!("{} Created package.json", "+".green());
            println!();
            Ok(())
        }
        Err(e) => {
            eprintln!("{}: {}", "error".red().bold(), e);
            std::process::exit(1);
        }
    }
}

#[cfg(feature = "pm")]
fn pm_update(packages: Vec<String>, root: Option<PathBuf>) -> Result<()> {
    use viper::pm::InstallResult;

    let root_dir = root.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    println!("{} v{}", "viper update".cyan().bold(), VERSION.dimmed());

    let config = PackageManagerConfig::new(&root_dir);
    let pm = PackageManager::with_config(config);

    let pkg_refs: Option<Vec<&str>> = if packages.is_empty() {
        None
    } else {
        Some(packages.iter().map(|s| s.as_str()).collect())
    };

    match pm.update(pkg_refs.as_deref()) {
        Ok(result) => {
            println!();
            for pkg in &result.added_packages {
                println!("{} {}", "~".yellow(), pkg);
            }
            println!();
            let label = if result.extracted == 1 {
                "package"
            } else {
                "packages"
            };
            println!(
                "{} {} updated [{}]",
                result.extracted,
                label,
                InstallResult::format_duration(result.total_time)
            );
            println!();
            println!("{}", result.timing_summary().dimmed());
            Ok(())
        }
        Err(e) => {
            eprintln!("{}: {}", "error".red().bold(), e);
            std::process::exit(1);
        }
    }
}

#[cfg(feature = "pm")]
fn pm_list(depth: usize, root: Option<PathBuf>) -> Result<()> {
    let root_dir = root.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let package_json_path = root_dir.join("package.json");
    let project_name = if package_json_path.exists() {
        std::fs::read_to_string(&package_json_path)
            .ok()
            .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
            .and_then(|json| {
                let name = json.get("name")?.as_str()?;
                let version = json.get("version")?.as_str()?;
                Some(format!("{}@{}", name, version))
            })
            .unwrap_or_else(|| root_dir.display().to_string())
    } else {
        root_dir.display().to_string()
    };

    println!("{}", project_name.cyan().bold());

    let config = PackageManagerConfig::new(&root_dir);
    let pm = PackageManager::with_config(config);

    match pm.list(depth) {
        Ok(packages) => {
            if packages.is_empty() {
                println!("{}", "(no dependencies installed)".dimmed());
            } else {
                for pkg in &packages {
                    let indent = "  ".repeat(pkg.depth);
                    let prefix = if pkg.depth == 0 { "+-" } else { "+-" };
                    println!(
                        "{}{}{}@{}",
                        indent,
                        prefix.dimmed(),
                        pkg.name,
                        pkg.version.dimmed()
                    );
                }
                println!();
                let label = if packages.len() == 1 {
                    "package"
                } else {
                    "packages"
                };
                println!("{} {} listed", packages.len(), label);
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("{}: {}", "error".red().bold(), e);
            std::process::exit(1);
        }
    }
}

#[cfg(feature = "pm")]
fn pm_cache() -> Result<()> {
    println!("{}: ~/.viper/cache", "Cache directory".cyan());
    println!("{}: not yet implemented", "Cache size".dimmed());
    Ok(())
}

#[cfg(feature = "pm")]
fn pm_cache_clean() -> Result<()> {
    println!("{}", "Cache cleaning not yet implemented".yellow());
    Ok(())
}
