//! Package manager core implementation using Orogene's node-maintainer

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use async_std::task;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use node_maintainer::NodeMaintainerOptions;
use oro_common::CorgiManifest;
use url::Url;

use super::error::{PmError, PmResult};
use super::{DEFAULT_CONCURRENCY, DEFAULT_REGISTRY};

/// Package manager configuration
#[derive(Debug, Clone)]
pub struct PackageManagerConfig {
    /// Project root directory
    pub root: PathBuf,
    /// npm registry URL
    pub registry: Url,
    /// Number of concurrent operations
    pub concurrency: usize,
    /// Use hoisted (flat) node_modules layout
    pub hoisted: bool,
    /// Show progress bars
    pub progress: bool,
}

impl Default for PackageManagerConfig {
    fn default() -> Self {
        Self {
            root: std::env::current_dir().unwrap_or_default(),
            registry: Url::parse(DEFAULT_REGISTRY).unwrap(),
            concurrency: DEFAULT_CONCURRENCY,
            hoisted: true,
            progress: true,
        }
    }
}

impl PackageManagerConfig {
    /// Create a new config with the given root directory
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            ..Default::default()
        }
    }

    /// Set the npm registry URL
    pub fn registry(mut self, url: &str) -> PmResult<Self> {
        self.registry = Url::parse(url)
            .map_err(|e| PmError::Registry(format!("Invalid registry URL: {}", e)))?;
        Ok(self)
    }

    /// Set concurrency level
    pub fn concurrency(mut self, n: usize) -> Self {
        self.concurrency = n;
        self
    }

    /// Enable/disable hoisted layout
    pub fn hoisted(mut self, hoisted: bool) -> Self {
        self.hoisted = hoisted;
        self
    }

    /// Enable/disable progress bars
    pub fn progress(mut self, show: bool) -> Self {
        self.progress = show;
        self
    }
}

/// Viper Package Manager
pub struct PackageManager {
    config: PackageManagerConfig,
}

impl PackageManager {
    /// Create a new package manager with default configuration
    pub fn new() -> Self {
        Self {
            config: PackageManagerConfig::default(),
        }
    }

    /// Create a package manager with custom configuration
    pub fn with_config(config: PackageManagerConfig) -> Self {
        Self { config }
    }

    /// Install all dependencies from package.json
    pub fn install(&self) -> PmResult<InstallResult> {
        task::block_on(self.install_async())
    }

    /// Install all dependencies asynchronously
    pub async fn install_async(&self) -> PmResult<InstallResult> {
        let total_start = Instant::now();
        let package_json_path = self.config.root.join("package.json");

        if !package_json_path.exists() {
            return Err(PmError::ManifestParse(
                "package.json not found in current directory".to_string(),
            ));
        }

        // Read and parse package.json
        let manifest_str = async_std::fs::read_to_string(&package_json_path)
            .await
            .map_err(|e| PmError::ManifestParse(format!("Failed to read package.json: {}", e)))?;

        let manifest: CorgiManifest = serde_json::from_str(&manifest_str)
            .map_err(|e| PmError::ManifestParse(format!("Failed to parse package.json: {}", e)))?;

        // Setup progress tracking
        let multi_progress = if self.config.progress {
            Some(MultiProgress::new())
        } else {
            None
        };

        let resolved_count = Arc::new(AtomicUsize::new(0));

        // Create progress bars
        let resolve_pb = multi_progress.as_ref().map(|mp| {
            let pb = mp.add(ProgressBar::new_spinner());
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );
            pb.set_message("Resolving dependencies...");
            pb
        });

        let extract_pb = multi_progress.as_ref().map(|mp| {
            let pb = mp.add(ProgressBar::new_spinner());
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap(),
            );
            pb
        });

        // Build node-maintainer options
        let resolved_clone = resolved_count.clone();
        let resolve_pb_clone = resolve_pb.clone();

        // on_resolve_progress takes |&Package, Duration|
        let opts = NodeMaintainerOptions::new()
            .root(&self.config.root)
            .registry(self.config.registry.clone())
            .concurrency(self.config.concurrency)
            .hoisted(self.config.hoisted)
            .on_resolve_progress(move |_pkg: &_, _duration: Duration| {
                let count = resolved_clone.fetch_add(1, Ordering::SeqCst) + 1;
                if let Some(ref pb) = resolve_pb_clone {
                    pb.set_message(format!("Resolved {} packages...", count));
                }
            });

        // Resolve dependencies
        let resolve_start = Instant::now();
        let maintainer = opts
            .resolve_manifest(manifest)
            .await
            .map_err(|e| PmError::Resolution(e.to_string()))?;
        let resolve_time = resolve_start.elapsed();

        if let Some(ref pb) = resolve_pb {
            pb.finish_with_message(format!(
                "Resolved {} packages",
                resolved_count.load(Ordering::SeqCst)
            ));
        }

        // Extract packages to node_modules
        if let Some(ref pb) = extract_pb {
            pb.set_message("Extracting packages...");
        }

        // extract() takes no arguments in this version
        let extract_start = Instant::now();
        let extracted = maintainer
            .extract()
            .await
            .map_err(|e| PmError::Extraction(e.to_string()))?;
        let extract_time = extract_start.elapsed();

        if let Some(ref pb) = extract_pb {
            pb.finish_with_message(format!("Extracted {} packages", extracted));
        }

        // Write lockfile using to_lockfile() and convert to KDL format
        let lockfile_path = self.config.root.join("viper.lock");
        let lockfile = maintainer
            .to_kdl()
            .map_err(|e| PmError::Lockfile(e.to_string()))?;

        let lockfile_str = lockfile.to_string();

        async_std::fs::write(&lockfile_path, lockfile_str)
            .await
            .map_err(|e| PmError::Lockfile(format!("Failed to write lockfile: {}", e)))?;

        let total_time = total_start.elapsed();

        Ok(InstallResult {
            resolved: resolved_count.load(Ordering::SeqCst),
            extracted,
            resolve_time,
            extract_time,
            total_time,
        })
    }

    /// Add a package to dependencies
    pub fn add(&self, packages: &[&str], dev: bool) -> PmResult<InstallResult> {
        task::block_on(self.add_async(packages, dev))
    }

    /// Add packages asynchronously
    pub async fn add_async(&self, packages: &[&str], dev: bool) -> PmResult<InstallResult> {
        let package_json_path = self.config.root.join("package.json");

        // Read existing manifest or create new one
        let mut manifest: serde_json::Value = if package_json_path.exists() {
            let content = async_std::fs::read_to_string(&package_json_path)
                .await
                .map_err(|e| PmError::ManifestParse(e.to_string()))?;
            serde_json::from_str(&content).map_err(|e| PmError::ManifestParse(e.to_string()))?
        } else {
            serde_json::json!({
                "name": "viper-project",
                "version": "1.0.0"
            })
        };

        // Add packages to the appropriate section
        let dep_key = if dev {
            "devDependencies"
        } else {
            "dependencies"
        };

        if manifest.get(dep_key).is_none() {
            manifest[dep_key] = serde_json::json!({});
        }

        for pkg in packages {
            // Parse package spec (e.g., "lodash@^4.0.0" or just "lodash")
            let (name, version) = if let Some(at_pos) = pkg.rfind('@') {
                if at_pos > 0 {
                    (&pkg[..at_pos], &pkg[at_pos + 1..])
                } else {
                    (*pkg, "*")
                }
            } else {
                (*pkg, "*")
            };

            manifest[dep_key][name] = serde_json::Value::String(version.to_string());
        }

        // Write updated manifest
        let manifest_str = serde_json::to_string_pretty(&manifest)
            .map_err(|e| PmError::ManifestParse(e.to_string()))?;

        async_std::fs::write(&package_json_path, manifest_str)
            .await
            .map_err(|e| PmError::Io(e))?;

        // Run install
        self.install_async().await
    }

    /// Remove packages from dependencies
    pub fn remove(&self, packages: &[&str]) -> PmResult<()> {
        task::block_on(self.remove_async(packages))
    }

    /// Remove packages asynchronously
    pub async fn remove_async(&self, packages: &[&str]) -> PmResult<()> {
        let package_json_path = self.config.root.join("package.json");

        if !package_json_path.exists() {
            return Err(PmError::ManifestParse("package.json not found".to_string()));
        }

        let content = async_std::fs::read_to_string(&package_json_path)
            .await
            .map_err(|e| PmError::ManifestParse(e.to_string()))?;

        let mut manifest: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| PmError::ManifestParse(e.to_string()))?;

        // Remove from both dependencies and devDependencies
        for pkg in packages {
            if let Some(deps) = manifest.get_mut("dependencies") {
                if let Some(obj) = deps.as_object_mut() {
                    obj.remove(*pkg);
                }
            }
            if let Some(deps) = manifest.get_mut("devDependencies") {
                if let Some(obj) = deps.as_object_mut() {
                    obj.remove(*pkg);
                }
            }
        }

        // Write updated manifest
        let manifest_str = serde_json::to_string_pretty(&manifest)
            .map_err(|e| PmError::ManifestParse(e.to_string()))?;

        async_std::fs::write(&package_json_path, manifest_str)
            .await
            .map_err(|e| PmError::Io(e))?;

        // Reinstall to update node_modules
        self.install_async().await?;

        Ok(())
    }
}

impl Default for PackageManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of an install operation
#[derive(Debug, Clone)]
pub struct InstallResult {
    /// Number of packages resolved
    pub resolved: usize,
    /// Number of packages extracted
    pub extracted: usize,
    /// Time spent resolving dependencies
    pub resolve_time: Duration,
    /// Time spent extracting packages
    pub extract_time: Duration,
    /// Total time for the entire operation
    pub total_time: Duration,
}

impl InstallResult {
    /// Format duration in a human-readable way (like Bun)
    fn format_duration(d: Duration) -> String {
        let millis = d.as_millis();
        if millis < 1000 {
            format!("{}ms", millis)
        } else {
            let secs = d.as_secs_f64();
            if secs < 60.0 {
                format!("{:.2}s", secs)
            } else {
                let mins = (secs / 60.0).floor();
                let remaining_secs = secs - (mins * 60.0);
                format!("{}m {:.1}s", mins as u64, remaining_secs)
            }
        }
    }
}

impl std::fmt::Display for InstallResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Installed {} packages in {}",
            self.extracted,
            Self::format_duration(self.total_time)
        )
    }
}

/// Detailed timing information for install operations
impl InstallResult {
    /// Get a detailed timing breakdown
    pub fn timing_summary(&self) -> String {
        format!(
            "  Resolve: {}\n  Extract: {}\n  Total:   {}",
            Self::format_duration(self.resolve_time),
            Self::format_duration(self.extract_time),
            Self::format_duration(self.total_time)
        )
    }
}
