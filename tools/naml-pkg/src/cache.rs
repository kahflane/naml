///
/// # Package Cache Management
///
/// This module manages the global package cache directory and project root discovery.
///
/// ## Cache Directory Structure
///
/// The global cache stores downloaded packages in a platform-appropriate location:
/// - **Linux**: `~/.cache/naml/packages/`
/// - **macOS**: `~/Library/Caches/naml/packages/`
/// - **Windows**: `{LOCALAPPDATA}/naml/packages/`
///
/// Each package is stored in a subdirectory named `{package_name}-{url_hash}/` where
/// the hash is the first 16 characters of the BLAKE3 hash of the package URL. This
/// prevents path collisions when different URLs provide packages with the same name.
///
/// ## Project Root Discovery
///
/// The `find_project_root()` function walks up the directory tree from a starting
/// point looking for `naml.toml`. This is used to resolve relative paths in the
/// manifest and to determine where to store local build artifacts.
///
/// ## Local Path Resolution
///
/// Local path dependencies (e.g., `path = "../common"`) are resolved relative to
/// the directory containing the manifest file, not the current working directory.
///

use std::path::{Path, PathBuf};
use crate::errors::PackageError;

pub fn cache_dir() -> Result<PathBuf, PackageError> {
    let base = dirs::cache_dir()
        .ok_or(PackageError::CacheError(
            "Could not determine platform cache directory".to_string()
        ))?;

    let naml_cache = base.join("naml").join("packages");

    std::fs::create_dir_all(&naml_cache)
        .map_err(|e| PackageError::CacheError(
            format!("Failed to create cache directory: {}", e)
        ))?;

    Ok(naml_cache)
}

pub fn package_cache_path(name: &str, url: &str) -> Result<PathBuf, PackageError> {
    let base = cache_dir()?;

    let hash = blake3::hash(url.as_bytes());
    let hash_hex = hash.to_hex();
    let short_hash = &hash_hex.as_str()[..16];

    let dir_name = format!("{}-{}", name, short_hash);
    Ok(base.join(dir_name))
}

pub fn is_cached(name: &str, url: &str) -> Result<bool, PackageError> {
    let path = package_cache_path(name, url)?;

    if !path.exists() {
        return Ok(false);
    }

    let has_files = std::fs::read_dir(&path)
        .map_err(|e| PackageError::CacheError(
            format!("Failed to read cache directory: {}", e)
        ))?
        .next()
        .is_some();

    Ok(has_files)
}

pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start;

    loop {
        let manifest_path = current.join("naml.toml");
        if manifest_path.exists() {
            return Some(current.to_path_buf());
        }

        current = current.parent()?;
    }
}

pub fn local_package_path(manifest_dir: &Path, relative_path: &str) -> PathBuf {
    let joined = manifest_dir.join(relative_path);

    joined.canonicalize().unwrap_or(joined)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_cache_dir_ends_with_naml_packages() {
        let dir = cache_dir().expect("cache_dir should return a path");
        let path_str = dir.to_string_lossy();

        assert!(path_str.ends_with("naml/packages") ||
                path_str.ends_with("naml\\packages"),
                "Cache dir should end with naml/packages, got: {}", path_str);
    }

    #[test]
    fn test_package_cache_path_deterministic() {
        let name = "test-pkg";
        let url = "https://github.com/user/repo.git";

        let path1 = package_cache_path(name, url).unwrap();
        let path2 = package_cache_path(name, url).unwrap();

        assert_eq!(path1, path2, "Same name and URL should produce same path");
    }

    #[test]
    fn test_package_cache_path_different_urls() {
        let name = "test-pkg";
        let url1 = "https://github.com/user/repo1.git";
        let url2 = "https://github.com/user/repo2.git";

        let path1 = package_cache_path(name, url1).unwrap();
        let path2 = package_cache_path(name, url2).unwrap();

        assert_ne!(path1, path2, "Different URLs should produce different paths");
    }

    #[test]
    fn test_is_cached_empty_dir() {
        let name = "test-pkg";
        let url = "https://example.com/test.git";

        let path = package_cache_path(name, url).unwrap();

        if path.exists() {
            fs::remove_dir_all(&path).ok();
        }

        fs::create_dir_all(&path).unwrap();

        let cached = is_cached(name, url).unwrap();
        assert!(!cached, "Empty directory should not count as cached");

        fs::remove_dir_all(&path).ok();
    }

    #[test]
    fn test_is_cached_with_files() {
        let name = "test-pkg";
        let url = "https://example.com/test2.git";

        let path = package_cache_path(name, url).unwrap();

        if path.exists() {
            fs::remove_dir_all(&path).ok();
        }

        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("test.txt"), "content").unwrap();

        let cached = is_cached(name, url).unwrap();
        assert!(cached, "Directory with files should count as cached");

        fs::remove_dir_all(&path).ok();
    }

    #[test]
    fn test_find_project_root_no_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let nested = temp_dir.path().join("a").join("b").join("c");
        fs::create_dir_all(&nested).unwrap();

        let root = find_project_root(&nested);
        assert!(root.is_none(), "Should return None when no naml.toml exists");
    }

    #[test]
    fn test_find_project_root_finds_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("naml.toml");
        fs::write(&manifest_path, "").unwrap();

        let nested = temp_dir.path().join("src").join("nested");
        fs::create_dir_all(&nested).unwrap();

        let root = find_project_root(&nested);
        assert_eq!(root, Some(temp_dir.path().to_path_buf()),
                   "Should find project root with naml.toml");
    }

    #[test]
    fn test_local_package_path_joins_correctly() {
        let manifest_dir = Path::new("/project/subdir");
        let relative = "../common";

        let joined = local_package_path(manifest_dir, relative);

        let joined_str = joined.to_string_lossy();
        assert!(joined_str.contains("common"),
                "Should join paths correctly, got: {}", joined_str);
    }

    #[test]
    fn test_local_package_path_absolute() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_dir = temp_dir.path();
        let common_dir = temp_dir.path().join("common");
        fs::create_dir_all(&common_dir).unwrap();

        let relative = "./common";
        let joined = local_package_path(manifest_dir, relative);

        assert!(joined.ends_with("common"),
                "Should resolve to common directory");
    }
}
