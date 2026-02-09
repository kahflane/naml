///
/// Package manager error types.
///
/// All errors that can occur during package operations: manifest parsing,
/// dependency resolution, Git downloads, and cache management.
///

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("Manifest not found at {path}")]
    ManifestNotFound { path: PathBuf },

    #[error("Failed to parse manifest at {path}: {reason}")]
    ManifestParse { path: PathBuf, reason: String },

    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("Invalid dependency '{name}': {reason}")]
    InvalidDependency { name: String, reason: String },

    #[error("Failed to clone Git repository {url}: {reason}")]
    GitCloneFailed { url: String, reason: String },

    #[error("Failed to checkout ref '{reference}' in {url}: {reason}")]
    GitCheckoutFailed {
        url: String,
        reference: String,
        reason: String,
    },

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Circular dependency detected: {}", format_cycle(cycle))]
    CircularDependency { cycle: Vec<String> },

    #[error("Package '{name}' not found in manifest")]
    PackageNotFound { name: String },

    #[error("Package '{name}' not downloaded. Run `naml pkg get` first.")]
    PackageNotDownloaded { name: String },

    #[error("Dependency conflict for '{name}': {reason}")]
    DependencyConflict { name: String, reason: String },

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
}

fn format_cycle(cycle: &[String]) -> String {
    cycle.join(" -> ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_messages() {
        let err = PackageError::ManifestNotFound {
            path: PathBuf::from("/tmp/naml.toml"),
        };
        assert!(err.to_string().contains("Manifest not found"));
        assert!(err.to_string().contains("/tmp/naml.toml"));

        let err = PackageError::CircularDependency {
            cycle: vec!["a".to_string(), "b".to_string(), "a".to_string()],
        };
        assert!(err.to_string().contains("Circular dependency"));
        assert!(err.to_string().contains("a -> b -> a"));

        let err = PackageError::PackageNotDownloaded {
            name: "json".to_string(),
        };
        assert!(err.to_string().contains("json"));
        assert!(err.to_string().contains("not downloaded"));

        let err = PackageError::InvalidManifest("missing field".to_string());
        assert!(err.to_string().contains("Invalid manifest"));
        assert!(err.to_string().contains("missing field"));

        let err = PackageError::InvalidDependency {
            name: "utils".to_string(),
            reason: "missing source".to_string(),
        };
        assert!(err.to_string().contains("Invalid dependency"));
        assert!(err.to_string().contains("utils"));
        assert!(err.to_string().contains("missing source"));

        let err = PackageError::GitCloneFailed {
            url: "https://github.com/test/repo".to_string(),
            reason: "network error".to_string(),
        };
        assert!(err.to_string().contains("Failed to clone"));
        assert!(err.to_string().contains("https://github.com/test/repo"));
        assert!(err.to_string().contains("network error"));

        let err = PackageError::GitCheckoutFailed {
            url: "https://github.com/test/repo".to_string(),
            reference: "v1.0.0".to_string(),
            reason: "ref not found".to_string(),
        };
        assert!(err.to_string().contains("Failed to checkout"));
        assert!(err.to_string().contains("v1.0.0"));
        assert!(err.to_string().contains("ref not found"));

        let err = PackageError::CacheError("failed to create dir".to_string());
        assert!(err.to_string().contains("Cache error"));
        assert!(err.to_string().contains("failed to create dir"));

        let err = PackageError::PackageNotFound {
            name: "missing".to_string(),
        };
        assert!(err.to_string().contains("missing"));
        assert!(err.to_string().contains("not found"));

        let err = PackageError::DependencyConflict {
            name: "utils".to_string(),
            reason: "version mismatch".to_string(),
        };
        assert!(err.to_string().contains("Dependency conflict"));
        assert!(err.to_string().contains("utils"));
        assert!(err.to_string().contains("version mismatch"));
    }
}
