///
/// # Package Manager Orchestrator
///
/// The `PackageManager` struct is the main entry point for both the CLI binary
/// and the compiler's type checker integration. It coordinates manifest parsing,
/// dependency resolution, and package source directory lookups.
///
/// ## CLI Usage
///
/// The `naml pkg get` command creates a `PackageManager` from the project's
/// `naml.toml` and calls `ensure_all_downloaded()` to resolve and cache all
/// transitive dependencies.
///
/// ## Compiler Integration
///
/// The naml compiler creates a `PackageManager` during `naml run` and passes it
/// to the type checker. The type checker calls `is_package()` and
/// `package_source_dir()` to resolve `use` statements to cached package files.
///

use std::path::{Path, PathBuf};

use crate::errors::PackageError;
use crate::manifest::{parse_manifest, Manifest};
use crate::resolver::{resolve, DependencyGraph, ResolvedPackage};

pub struct PackageManager {
    manifest: Manifest,
    manifest_dir: PathBuf,
    graph: Option<DependencyGraph>,
}

impl PackageManager {
    pub fn from_manifest_path(path: &Path) -> Result<Self, PackageError> {
        let manifest = parse_manifest(path)?;
        let manifest_dir = path
            .parent()
            .ok_or_else(|| PackageError::ManifestNotFound {
                path: path.to_path_buf(),
            })?
            .to_path_buf();

        Ok(Self {
            manifest,
            manifest_dir,
            graph: None,
        })
    }

    pub fn from_manifest(manifest: Manifest, manifest_dir: PathBuf) -> Self {
        Self {
            manifest,
            manifest_dir,
            graph: None,
        }
    }

    pub fn resolve(&mut self) -> Result<(), PackageError> {
        let graph = resolve(&self.manifest, &self.manifest_dir)?;
        self.graph = Some(graph);
        Ok(())
    }

    pub fn ensure_all_downloaded(&mut self) -> Result<(), PackageError> {
        if self.graph.is_none() {
            self.resolve()?;
        }
        Ok(())
    }

    pub fn is_package(&self, name: &str) -> bool {
        if let Some(ref graph) = self.graph {
            graph.packages.contains_key(name)
        } else {
            self.manifest.dependencies.contains_key(name)
        }
    }

    pub fn resolve_package(&self, name: &str) -> Option<&ResolvedPackage> {
        self.graph.as_ref()?.packages.get(name)
    }

    pub fn package_source_dir(&self, name: &str) -> Option<PathBuf> {
        let pkg = self.resolve_package(name)?;
        Some(pkg.cache_path.clone())
    }

    pub fn all_packages(&self) -> Vec<&ResolvedPackage> {
        match self.graph {
            Some(ref graph) => graph.packages.values().collect(),
            None => Vec::new(),
        }
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn manifest_dir(&self) -> &Path {
        &self.manifest_dir
    }

    pub fn has_dependencies(&self) -> bool {
        !self.manifest.dependencies.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::parse_manifest_str;

    #[test]
    fn test_from_manifest() {
        let toml_content = r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
"#;
        let manifest = parse_manifest_str(toml_content).unwrap();
        let manager = PackageManager::from_manifest(manifest, PathBuf::from("/tmp/test"));

        assert_eq!(manager.manifest().package.name, "test-project");
        assert_eq!(manager.manifest().package.version, "0.1.0");
        assert_eq!(manager.manifest_dir(), Path::new("/tmp/test"));
    }

    #[test]
    fn test_has_dependencies_empty() {
        let toml_content = r#"
[package]
name = "no-deps"
version = "0.1.0"

[dependencies]
"#;
        let manifest = parse_manifest_str(toml_content).unwrap();
        let manager = PackageManager::from_manifest(manifest, PathBuf::from("/tmp/test"));

        assert!(!manager.has_dependencies());
    }

    #[test]
    fn test_has_dependencies_with_deps() {
        let toml_content = r#"
[package]
name = "with-deps"
version = "0.1.0"

[dependencies]
json = { path = "../json" }
"#;
        let manifest = parse_manifest_str(toml_content).unwrap();
        let manager = PackageManager::from_manifest(manifest, PathBuf::from("/tmp/test"));

        assert!(manager.has_dependencies());
    }

    #[test]
    fn test_is_package_before_resolve() {
        let toml_content = r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
utils = { path = "../utils" }
json = { path = "../json" }
"#;
        let manifest = parse_manifest_str(toml_content).unwrap();
        let manager = PackageManager::from_manifest(manifest, PathBuf::from("/tmp/test"));

        assert!(manager.is_package("utils"));
        assert!(manager.is_package("json"));
        assert!(!manager.is_package("nonexistent"));
    }

    #[test]
    fn test_package_source_dir_not_resolved() {
        let toml_content = r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
utils = { path = "../utils" }
"#;
        let manifest = parse_manifest_str(toml_content).unwrap();
        let manager = PackageManager::from_manifest(manifest, PathBuf::from("/tmp/test"));

        assert!(manager.package_source_dir("utils").is_none());
    }
}
