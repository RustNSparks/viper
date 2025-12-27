//! Package manager core implementation using Orogene's node-maintainer

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use async_std::task;
use indicatif::{ProgressBar, ProgressStyle};
use node_maintainer::NodeMaintainerOptions;
use oro_common::CorgiManifest;
use url::Url;

use super::error::{PmError, PmResult};
use super::{DEFAULT_CONCURRENCY, DEFAULT_REGISTRY};

/// Create a Bun-style spinner progress bar
fn create_spinner(msg: &str) -> ProgressBar {
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
        let resolved_count = Arc::new(AtomicUsize::new(0));
        let last_package = Arc::new(Mutex::new(String::new()));

        // Create spinner for resolving phase
        let spinner = if self.config.progress {
            Some(create_spinner("Resolving dependencies..."))
        } else {
            None
        };

        // Build node-maintainer options
        let resolved_clone = resolved_count.clone();
        let last_pkg_clone = last_package.clone();
        let spinner_clone = spinner.clone();

        // on_resolve_progress takes |&Package, Duration|
        let opts = NodeMaintainerOptions::new()
            .root(&self.config.root)
            .registry(self.config.registry.clone())
            .concurrency(self.config.concurrency)
            .hoisted(self.config.hoisted)
            .on_resolve_progress(move |pkg: &_, _duration: Duration| {
                let count = resolved_clone.fetch_add(1, Ordering::SeqCst) + 1;
                let pkg_name = pkg.name().to_string();
                if let Ok(mut last) = last_pkg_clone.lock() {
                    *last = pkg_name.clone();
                }
                if let Some(ref pb) = spinner_clone {
                    pb.set_message(format!("Resolving: {} ({})", pkg_name, count));
                }
            });

        // Resolve dependencies
        let resolve_start = Instant::now();
        let maintainer = opts
            .resolve_manifest(manifest)
            .await
            .map_err(|e| PmError::Resolution(e.to_string()))?;
        let resolve_time = resolve_start.elapsed();

        let total_resolved = resolved_count.load(Ordering::SeqCst);
        if let Some(ref pb) = spinner {
            pb.set_message(format!("Resolved {} packages", total_resolved));
        }

        // Update spinner for extraction phase
        if let Some(ref pb) = spinner {
            pb.set_message("Extracting packages...");
        }

        // extract() takes no arguments in this version
        let extract_start = Instant::now();
        let extracted = maintainer
            .extract()
            .await
            .map_err(|e| PmError::Extraction(e.to_string()))?;
        let extract_time = extract_start.elapsed();

        // Finish spinner
        if let Some(ref pb) = spinner {
            pb.finish_and_clear();
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

        // Get resolved versions from lockfile for display
        // Parse the KDL lockfile to find actual resolved versions
        let mut version_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        if let Ok(lock_content) = async_std::fs::read_to_string(&lockfile_path).await {
            // Simple parsing: find "pkg "name" { version "x.y.z" }" patterns
            for line in lock_content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("pkg \"") {
                    // Extract package name
                    if let Some(end) = trimmed[5..].find('"') {
                        let pkg_name = &trimmed[5..5 + end];
                        // Look for version in next few lines (simplified parsing)
                        let remaining = &lock_content[lock_content.find(trimmed).unwrap_or(0)..];
                        for version_line in remaining.lines().take(5) {
                            let vl = version_line.trim();
                            if vl.starts_with("version \"") {
                                if let Some(vend) = vl[9..].find('"') {
                                    let version = &vl[9..9 + vend];
                                    version_map.insert(pkg_name.to_string(), version.to_string());
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Read package.json to get top-level dependency names, then lookup versions
        let mut added_packages = Vec::new();
        if let Ok(pkg_content) = async_std::fs::read_to_string(&package_json_path).await {
            if let Ok(pkg_json) = serde_json::from_str::<serde_json::Value>(&pkg_content) {
                if let Some(deps) = pkg_json.get("dependencies").and_then(|d| d.as_object()) {
                    for (name, _) in deps {
                        let version = version_map
                            .get(name)
                            .cloned()
                            .unwrap_or_else(|| "*".to_string());
                        added_packages.push(format!("{}@{}", name, version));
                    }
                }
                if let Some(deps) = pkg_json.get("devDependencies").and_then(|d| d.as_object()) {
                    for (name, _) in deps {
                        let version = version_map
                            .get(name)
                            .cloned()
                            .unwrap_or_else(|| "*".to_string());
                        added_packages.push(format!("{}@{}", name, version));
                    }
                }
            }
        }

        Ok(InstallResult {
            resolved: resolved_count.load(Ordering::SeqCst),
            extracted,
            resolve_time,
            extract_time,
            total_time,
            added_packages,
            removed_packages: Vec::new(),
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

/// Information about a package from the registry
#[derive(Debug, Clone)]
pub struct PackageInfo {
    /// Package name
    pub name: String,
    /// Latest version
    pub latest_version: String,
    /// Description
    pub description: Option<String>,
    /// License
    pub license: Option<String>,
    /// Homepage URL
    pub homepage: Option<String>,
    /// Repository URL
    pub repository: Option<String>,
    /// Author
    pub author: Option<String>,
    /// Keywords
    pub keywords: Vec<String>,
    /// All available versions
    pub versions: Vec<String>,
    /// Dist-tags (latest, next, etc.)
    pub dist_tags: std::collections::HashMap<String, String>,
    /// Dependencies count
    pub dependencies_count: usize,
    /// Dev dependencies count
    pub dev_dependencies_count: usize,
}

impl PackageManager {
    /// View package information from the registry
    pub fn view(&self, package: &str) -> PmResult<PackageInfo> {
        task::block_on(self.view_async(package))
    }

    /// View package information asynchronously
    pub async fn view_async(&self, package: &str) -> PmResult<PackageInfo> {
        use nassun::NassunOpts;

        let nassun = NassunOpts::new()
            .registry(self.config.registry.clone())
            .build();

        let pkg = nassun
            .resolve(package)
            .await
            .map_err(|e| PmError::PackageNotFound(format!("{}: {}", package, e)))?;

        let packument = pkg
            .packument()
            .await
            .map_err(|e| PmError::Network(e.to_string()))?;

        let metadata = pkg
            .metadata()
            .await
            .map_err(|e| PmError::Network(e.to_string()))?;

        // Get latest version from dist-tags
        let latest_version = packument
            .tags
            .get("latest")
            .map(|v| v.to_string())
            .unwrap_or_else(|| {
                metadata
                    .manifest
                    .version
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_default()
            });

        // Get all versions sorted (descending)
        let mut versions: Vec<String> = packument.versions.keys().map(|v| v.to_string()).collect();
        versions.sort_by(|a, b| b.cmp(a));

        // Get dist-tags
        let dist_tags: std::collections::HashMap<String, String> = packument
            .tags
            .iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect();

        // Extract repository URL
        let repository = metadata.manifest.repository.as_ref().and_then(|r| match r {
            oro_common::Repository::Str(s) => Some(s.clone()),
            oro_common::Repository::Obj { url, .. } => url.clone(),
        });

        // Extract author
        let author = metadata.manifest.author.as_ref().map(|a| match a {
            oro_common::PersonField::Str(s) => s.clone(),
            oro_common::PersonField::Obj(person) => {
                let name = person.name.as_deref().unwrap_or("");
                match &person.email {
                    Some(email) => format!("{} <{}>", name, email),
                    None => name.to_string(),
                }
            }
        });

        Ok(PackageInfo {
            name: metadata
                .manifest
                .name
                .clone()
                .unwrap_or_else(|| package.to_string()),
            latest_version,
            description: metadata.manifest.description.clone(),
            license: metadata.manifest.license.clone(),
            homepage: metadata.manifest.homepage.clone(),
            repository,
            author,
            keywords: metadata.manifest.keywords.clone(),
            versions,
            dist_tags,
            dependencies_count: metadata.manifest.dependencies.len(),
            dev_dependencies_count: metadata.manifest.dev_dependencies.len(),
        })
    }

    /// Ping the npm registry to check connectivity
    pub fn ping(&self) -> PmResult<Duration> {
        task::block_on(self.ping_async())
    }

    /// Ping the registry asynchronously
    pub async fn ping_async(&self) -> PmResult<Duration> {
        use nassun::NassunOpts;

        let start = Instant::now();

        let nassun = NassunOpts::new()
            .registry(self.config.registry.clone())
            .build();

        // Try to resolve a well-known package to test connectivity
        nassun
            .resolve("lodash@latest")
            .await
            .map_err(|e| PmError::Network(format!("Registry ping failed: {}", e)))?;

        Ok(start.elapsed())
    }

    /// Initialize a new package.json
    pub fn init(&self, name: Option<&str>, interactive: bool) -> PmResult<()> {
        task::block_on(self.init_async(name, interactive))
    }

    /// Initialize package.json asynchronously
    pub async fn init_async(&self, name: Option<&str>, _interactive: bool) -> PmResult<()> {
        let package_json_path = self.config.root.join("package.json");

        if package_json_path.exists() {
            return Err(PmError::Other("package.json already exists".to_string()));
        }

        // Get the directory name as default package name
        let default_name = self
            .config
            .root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my-project")
            .to_string();

        let package_name = name.map(|s| s.to_string()).unwrap_or(default_name);

        let manifest = serde_json::json!({
            "name": package_name,
            "version": "1.0.0",
            "description": "",
            "main": "index.js",
            "scripts": {
                "test": "echo \"Error: no test specified\" && exit 1"
            },
            "keywords": [],
            "author": "",
            "license": "ISC"
        });

        let manifest_str = serde_json::to_string_pretty(&manifest)
            .map_err(|e| PmError::ManifestParse(e.to_string()))?;

        async_std::fs::write(&package_json_path, manifest_str)
            .await
            .map_err(|e| PmError::Io(e))?;

        Ok(())
    }

    /// List installed packages (synchronous - no async needed for local fs)
    pub fn list(&self, depth: usize) -> PmResult<Vec<InstalledPackage>> {
        let node_modules = self.config.root.join("node_modules");

        if !node_modules.exists() {
            return Ok(Vec::new());
        }

        let mut packages = Vec::new();
        self.scan_node_modules(&node_modules, 0, depth, &mut packages)?;

        Ok(packages)
    }

    /// Recursively scan node_modules (synchronous)
    fn scan_node_modules(
        &self,
        dir: &std::path::Path,
        current_depth: usize,
        max_depth: usize,
        packages: &mut Vec<InstalledPackage>,
    ) -> PmResult<()> {
        if current_depth > max_depth {
            return Ok(());
        }

        let entries = std::fs::read_dir(dir).map_err(PmError::Io)?;

        for entry in entries {
            let entry = entry.map_err(PmError::Io)?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Handle scoped packages (@org/pkg)
            if name.starts_with('@') {
                let scope_entries = std::fs::read_dir(&path).map_err(PmError::Io)?;

                for scope_entry in scope_entries {
                    let scope_entry = scope_entry.map_err(PmError::Io)?;
                    let scope_path = scope_entry.path();

                    if scope_path.is_dir() {
                        let pkg_name = scope_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("");
                        let full_name = format!("{}/{}", name, pkg_name);

                        if let Some(pkg) =
                            self.read_package_info(&scope_path, &full_name, current_depth)
                        {
                            packages.push(pkg);
                        }

                        // Scan nested node_modules
                        let nested = scope_path.join("node_modules");
                        if nested.exists() {
                            self.scan_node_modules(
                                &nested,
                                current_depth + 1,
                                max_depth,
                                packages,
                            )?;
                        }
                    }
                }
            } else if name != ".bin" && name != ".package-lock.json" {
                if let Some(pkg) = self.read_package_info(&path, &name, current_depth) {
                    packages.push(pkg);
                }

                // Scan nested node_modules
                let nested = path.join("node_modules");
                if nested.exists() {
                    self.scan_node_modules(&nested, current_depth + 1, max_depth, packages)?;
                }
            }
        }

        Ok(())
    }

    /// Read package info from a package directory (synchronous)
    fn read_package_info(
        &self,
        path: &std::path::Path,
        name: &str,
        depth: usize,
    ) -> Option<InstalledPackage> {
        let package_json = path.join("package.json");
        if !package_json.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&package_json).ok()?;
        let manifest: serde_json::Value = serde_json::from_str(&content).ok()?;

        let version = manifest
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();

        Some(InstalledPackage {
            name: name.to_string(),
            version,
            depth,
        })
    }

    /// Update packages to their latest versions
    pub fn update(&self, packages: Option<&[&str]>) -> PmResult<InstallResult> {
        task::block_on(self.update_async(packages))
    }

    /// Update packages asynchronously
    pub async fn update_async(&self, packages: Option<&[&str]>) -> PmResult<InstallResult> {
        use nassun::NassunOpts;

        let package_json_path = self.config.root.join("package.json");

        if !package_json_path.exists() {
            return Err(PmError::ManifestParse("package.json not found".to_string()));
        }

        let content = async_std::fs::read_to_string(&package_json_path)
            .await
            .map_err(|e| PmError::ManifestParse(e.to_string()))?;

        let mut manifest: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| PmError::ManifestParse(e.to_string()))?;

        let nassun = NassunOpts::new()
            .registry(self.config.registry.clone())
            .build();

        // Collect packages to update
        let mut to_update: Vec<(String, bool)> = Vec::new(); // (name, is_dev)

        if let Some(pkgs) = packages {
            // Update specific packages
            for &pkg in pkgs {
                // Check if it's in dependencies or devDependencies
                let in_deps = manifest
                    .get("dependencies")
                    .and_then(|d| d.get(pkg))
                    .is_some();
                let in_dev = manifest
                    .get("devDependencies")
                    .and_then(|d| d.get(pkg))
                    .is_some();

                if in_deps {
                    to_update.push((pkg.to_string(), false));
                } else if in_dev {
                    to_update.push((pkg.to_string(), true));
                }
            }
        } else {
            // Update all packages
            if let Some(deps) = manifest.get("dependencies").and_then(|d| d.as_object()) {
                for name in deps.keys() {
                    to_update.push((name.clone(), false));
                }
            }
            if let Some(deps) = manifest.get("devDependencies").and_then(|d| d.as_object()) {
                for name in deps.keys() {
                    to_update.push((name.clone(), true));
                }
            }
        }

        // Fetch latest versions and update manifest
        for (name, is_dev) in &to_update {
            if let Ok(pkg) = nassun.resolve(&format!("{}@latest", name)).await {
                if let Ok(metadata) = pkg.metadata().await {
                    if let Some(version) = &metadata.manifest.version {
                        let dep_key = if *is_dev {
                            "devDependencies"
                        } else {
                            "dependencies"
                        };
                        manifest[dep_key][name] =
                            serde_json::Value::String(format!("^{}", version));
                    }
                }
            }
        }

        // Write updated manifest
        let manifest_str = serde_json::to_string_pretty(&manifest)
            .map_err(|e| PmError::ManifestParse(e.to_string()))?;

        async_std::fs::write(&package_json_path, manifest_str)
            .await
            .map_err(|e| PmError::Io(e))?;

        // Run install to apply changes
        self.install_async().await
    }
}

/// Information about an installed package
#[derive(Debug, Clone)]
pub struct InstalledPackage {
    /// Package name
    pub name: String,
    /// Installed version
    pub version: String,
    /// Depth in the dependency tree (0 = top-level)
    pub depth: usize,
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
    /// Top-level packages that were added (name@version)
    pub added_packages: Vec<String>,
    /// Packages that were removed
    pub removed_packages: Vec<String>,
}

impl InstallResult {
    /// Format duration in a human-readable way (like Bun: [584.00ms] or [11.34s])
    pub fn format_duration(d: Duration) -> String {
        let millis = d.as_millis();
        if millis < 1000 {
            format!("{:.2}ms", millis as f64)
        } else {
            format!("{:.2}s", d.as_secs_f64())
        }
    }

    /// Get the package count label (singular or plural)
    fn package_label(count: usize) -> &'static str {
        if count == 1 { "package" } else { "packages" }
    }

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
