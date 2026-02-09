///
/// # Manifest Parsing
///
/// This module provides types and functions for parsing `naml.toml` manifest files.
/// It handles package metadata and dependency specifications, supporting both git
/// and local path dependencies.
///
/// ## Dependency Resolution
///
/// Dependencies can be specified in two formats:
/// - **Simple**: Just a version string (reserved for future registry support)
/// - **Detailed**: An object with `git` or `path` fields
///
/// Git dependencies support `tag`, `branch`, or `rev` references. If none are
/// specified, the default branch is used.
///
/// ## Example naml.toml
///
/// ```toml
/// [package]
/// name = "my-project"
/// version = "0.1.0"
/// description = "A naml project"
/// authors = ["Author Name"]
/// license = "MIT"
///
/// [dependencies]
/// json = { git = "https://github.com/naml-lang/json", tag = "v0.1.0" }
/// utils = { path = "../shared/utils" }
/// http = { git = "https://github.com/naml-lang/http", branch = "main" }
/// crypto = { git = "https://github.com/naml-lang/crypto", rev = "abc123" }
/// ```
///
/// ## Internal Representation
///
/// The module parses TOML into `Manifest` structs, then normalizes dependency
/// specifications into `Dependency` structs with `DependencySource` enums.
/// This normalization validates that each dependency has exactly one source
/// (git or path) and resolves git references.
///

use serde::{Deserialize, Serialize};
use indexmap::IndexMap;
use std::path::{Path, PathBuf};
use crate::errors::PackageError;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    pub package: PackageMetadata,
    #[serde(default)]
    pub dependencies: IndexMap<String, DependencySpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DependencySpec {
    Simple(String),
    Detailed(DetailedDependency),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DetailedDependency {
    #[serde(default)]
    pub git: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub rev: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GitRef {
    Tag(String),
    Branch(String),
    Rev(String),
    Default,
}

#[derive(Debug, Clone)]
pub enum DependencySource {
    Git { url: String, git_ref: GitRef },
    Local { path: PathBuf },
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub source: DependencySource,
}

pub fn parse_manifest(path: &Path) -> Result<Manifest, PackageError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| PackageError::Io(e))?;
    parse_manifest_str(&content)
}

pub fn parse_manifest_str(content: &str) -> Result<Manifest, PackageError> {
    toml::from_str(content)
        .map_err(|e| PackageError::InvalidManifest(e.to_string()))
}

pub fn default_manifest(name: &str) -> String {
    format!(
        r#"[package]
name = "{}"
version = "0.1.0"
description = ""
authors = []
license = ""

[dependencies]
"#,
        name
    )
}

impl Manifest {
    pub fn dependencies(&self) -> Result<Vec<Dependency>, PackageError> {
        let mut deps = Vec::new();

        for (name, spec) in &self.dependencies {
            let source = match spec {
                DependencySpec::Simple(_version) => {
                    return Err(PackageError::InvalidManifest(
                        format!("Registry dependencies not yet supported for '{}'", name)
                    ));
                }
                DependencySpec::Detailed(detailed) => {
                    resolve_detailed_dependency(detailed)?
                }
            };

            deps.push(Dependency {
                name: name.clone(),
                source,
            });
        }

        Ok(deps)
    }
}

fn resolve_detailed_dependency(dep: &DetailedDependency) -> Result<DependencySource, PackageError> {
    let has_git = dep.git.is_some();
    let has_path = dep.path.is_some();

    if !has_git && !has_path {
        return Err(PackageError::InvalidManifest(
            "Dependency must specify either 'git' or 'path'".to_string()
        ));
    }

    if has_git && has_path {
        return Err(PackageError::InvalidManifest(
            "Dependency cannot specify both 'git' and 'path'".to_string()
        ));
    }

    if let Some(git_url) = &dep.git {
        let git_ref = resolve_git_ref(dep)?;
        Ok(DependencySource::Git {
            url: git_url.clone(),
            git_ref,
        })
    } else if let Some(path_str) = &dep.path {
        Ok(DependencySource::Local {
            path: PathBuf::from(path_str),
        })
    } else {
        unreachable!()
    }
}

fn resolve_git_ref(dep: &DetailedDependency) -> Result<GitRef, PackageError> {
    let ref_count = [&dep.tag, &dep.branch, &dep.rev]
        .iter()
        .filter(|r| r.is_some())
        .count();

    if ref_count > 1 {
        return Err(PackageError::InvalidManifest(
            "Dependency can only specify one of 'tag', 'branch', or 'rev'".to_string()
        ));
    }

    if let Some(tag) = &dep.tag {
        Ok(GitRef::Tag(tag.clone()))
    } else if let Some(branch) = &dep.branch {
        Ok(GitRef::Branch(branch.clone()))
    } else if let Some(rev) = &dep.rev {
        Ok(GitRef::Rev(rev.clone()))
    } else {
        Ok(GitRef::Default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_manifest_with_dependencies() {
        let toml_content = r#"
[package]
name = "test-project"
version = "0.2.0"
description = "A test project"
authors = ["Alice", "Bob"]
license = "MIT"

[dependencies]
json = { git = "https://github.com/naml-lang/json", tag = "v0.1.0" }
utils = { path = "../shared/utils" }
http = { git = "https://github.com/naml-lang/http", branch = "main" }
crypto = { git = "https://github.com/naml-lang/crypto", rev = "abc123def" }
local-lib = { path = "./libs/local" }
"#;

        let manifest = parse_manifest_str(toml_content).expect("Failed to parse manifest");

        assert_eq!(manifest.package.name, "test-project");
        assert_eq!(manifest.package.version, "0.2.0");
        assert_eq!(manifest.package.description, Some("A test project".to_string()));
        assert_eq!(manifest.package.authors, vec!["Alice", "Bob"]);
        assert_eq!(manifest.package.license, Some("MIT".to_string()));
        assert_eq!(manifest.dependencies.len(), 5);
    }

    #[test]
    fn test_parse_minimal_manifest() {
        let toml_content = r#"
[package]
name = "minimal"
version = "0.1.0"
"#;

        let manifest = parse_manifest_str(toml_content).expect("Failed to parse manifest");

        assert_eq!(manifest.package.name, "minimal");
        assert_eq!(manifest.package.version, "0.1.0");
        assert_eq!(manifest.package.description, None);
        assert_eq!(manifest.package.authors.len(), 0);
        assert_eq!(manifest.package.license, None);
        assert_eq!(manifest.dependencies.len(), 0);
    }

    #[test]
    fn test_dependencies_conversion() {
        let toml_content = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
json = { git = "https://github.com/naml-lang/json", tag = "v0.1.0" }
utils = { path = "../utils" }
http = { git = "https://github.com/naml-lang/http" }
"#;

        let manifest = parse_manifest_str(toml_content).expect("Failed to parse manifest");
        let deps = manifest.dependencies().expect("Failed to convert dependencies");

        assert_eq!(deps.len(), 3);

        let json_dep = deps.iter().find(|d| d.name == "json").unwrap();
        match &json_dep.source {
            DependencySource::Git { url, git_ref } => {
                assert_eq!(url, "https://github.com/naml-lang/json");
                assert_eq!(*git_ref, GitRef::Tag("v0.1.0".to_string()));
            }
            _ => panic!("Expected git dependency"),
        }

        let utils_dep = deps.iter().find(|d| d.name == "utils").unwrap();
        match &utils_dep.source {
            DependencySource::Local { path } => {
                assert_eq!(path, &PathBuf::from("../utils"));
            }
            _ => panic!("Expected local dependency"),
        }

        let http_dep = deps.iter().find(|d| d.name == "http").unwrap();
        match &http_dep.source {
            DependencySource::Git { url, git_ref } => {
                assert_eq!(url, "https://github.com/naml-lang/http");
                assert_eq!(*git_ref, GitRef::Default);
            }
            _ => panic!("Expected git dependency"),
        }
    }

    #[test]
    fn test_default_manifest_is_valid() {
        let content = default_manifest("my-project");
        let manifest = parse_manifest_str(&content).expect("Default manifest should be valid TOML");

        assert_eq!(manifest.package.name, "my-project");
        assert_eq!(manifest.package.version, "0.1.0");
        assert_eq!(manifest.dependencies.len(), 0);
    }

    #[test]
    fn test_error_on_dependency_with_no_source() {
        let toml_content = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
invalid = { tag = "v0.1.0" }
"#;

        let manifest = parse_manifest_str(toml_content).expect("Failed to parse manifest");
        let result = manifest.dependencies();

        assert!(result.is_err());
        match result {
            Err(PackageError::InvalidManifest(msg)) => {
                assert!(msg.contains("either 'git' or 'path'"));
            }
            _ => panic!("Expected InvalidManifest error"),
        }
    }

    #[test]
    fn test_error_on_dependency_with_both_git_and_path() {
        let toml_content = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
invalid = { git = "https://github.com/test/repo", path = "../local" }
"#;

        let manifest = parse_manifest_str(toml_content).expect("Failed to parse manifest");
        let result = manifest.dependencies();

        assert!(result.is_err());
        match result {
            Err(PackageError::InvalidManifest(msg)) => {
                assert!(msg.contains("cannot specify both"));
            }
            _ => panic!("Expected InvalidManifest error"),
        }
    }

    #[test]
    fn test_error_on_multiple_git_refs() {
        let toml_content = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
invalid = { git = "https://github.com/test/repo", tag = "v0.1.0", branch = "main" }
"#;

        let manifest = parse_manifest_str(toml_content).expect("Failed to parse manifest");
        let result = manifest.dependencies();

        assert!(result.is_err());
        match result {
            Err(PackageError::InvalidManifest(msg)) => {
                assert!(msg.contains("only specify one"));
            }
            _ => panic!("Expected InvalidManifest error"),
        }
    }

    #[test]
    fn test_git_ref_variants() {
        let toml_content = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
with-tag = { git = "https://github.com/test/tag", tag = "v1.0" }
with-branch = { git = "https://github.com/test/branch", branch = "develop" }
with-rev = { git = "https://github.com/test/rev", rev = "abc123" }
default = { git = "https://github.com/test/default" }
"#;

        let manifest = parse_manifest_str(toml_content).expect("Failed to parse manifest");
        let deps = manifest.dependencies().expect("Failed to convert dependencies");

        let tag_dep = deps.iter().find(|d| d.name == "with-tag").unwrap();
        match &tag_dep.source {
            DependencySource::Git { git_ref, .. } => {
                assert_eq!(*git_ref, GitRef::Tag("v1.0".to_string()));
            }
            _ => panic!("Expected git dependency"),
        }

        let branch_dep = deps.iter().find(|d| d.name == "with-branch").unwrap();
        match &branch_dep.source {
            DependencySource::Git { git_ref, .. } => {
                assert_eq!(*git_ref, GitRef::Branch("develop".to_string()));
            }
            _ => panic!("Expected git dependency"),
        }

        let rev_dep = deps.iter().find(|d| d.name == "with-rev").unwrap();
        match &rev_dep.source {
            DependencySource::Git { git_ref, .. } => {
                assert_eq!(*git_ref, GitRef::Rev("abc123".to_string()));
            }
            _ => panic!("Expected git dependency"),
        }

        let default_dep = deps.iter().find(|d| d.name == "default").unwrap();
        match &default_dep.source {
            DependencySource::Git { git_ref, .. } => {
                assert_eq!(*git_ref, GitRef::Default);
            }
            _ => panic!("Expected git dependency"),
        }
    }

    #[test]
    fn test_error_on_simple_dependency_spec() {
        let toml_content = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
registry-dep = "0.1.0"
"#;

        let manifest = parse_manifest_str(toml_content).expect("Failed to parse manifest");
        let result = manifest.dependencies();

        assert!(result.is_err());
        match result {
            Err(PackageError::InvalidManifest(msg)) => {
                assert!(msg.contains("Registry dependencies not yet supported"));
            }
            _ => panic!("Expected InvalidManifest error"),
        }
    }
}
