///
/// # Project Initialization Module
///
/// This module provides scaffolding for new naml projects. It creates
/// the necessary directory structure and default files including:
///
/// - `naml.toml` - Project manifest with dependencies and metadata
/// - `main.nm` - Hello world entry point
///
/// ## Usage
///
/// ```rust,ignore
/// use std::path::Path;
/// use naml_pkg::init::init_project;
///
/// init_project("my-project", Path::new("./my-project"))?;
/// ```
///
/// ## Error Handling
///
/// Returns `PackageError::ManifestParse` if the directory already contains
/// a `naml.toml` file. All I/O errors are wrapped and propagated via the
/// `From<std::io::Error>` implementation.
///

use std::path::Path;
use crate::errors::PackageError;
use crate::manifest::default_manifest;

const HELLO_WORLD: &str = r#"fn main() {
    println("Hello, world!");
}
"#;

pub fn init_project(name: &str, dir: &Path) -> Result<(), PackageError> {
    std::fs::create_dir_all(dir)?;

    let manifest_path = dir.join("naml.toml");
    if manifest_path.exists() {
        return Err(PackageError::InvalidManifest(
            format!("Project already initialized at {}", manifest_path.display()),
        ));
    }

    let manifest_content = default_manifest(name);
    std::fs::write(&manifest_path, manifest_content)?;

    let main_path = dir.join("main.nm");
    std::fs::write(&main_path, HELLO_WORLD)?;

    println!("Created project '{}' in {}", name, dir.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::manifest::parse_manifest_str;

    #[test]
    fn test_init_project_creates_files() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("test-project");

        init_project("test-project", &project_dir).unwrap();

        assert!(project_dir.join("naml.toml").exists());
        assert!(project_dir.join("main.nm").exists());
    }

    #[test]
    fn test_init_project_manifest_content() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("test-project");

        init_project("test-project", &project_dir).unwrap();

        let manifest_path = project_dir.join("naml.toml");
        let content = std::fs::read_to_string(&manifest_path).unwrap();

        assert!(content.contains("test-project"));

        let manifest = parse_manifest_str(&content).unwrap();
        assert_eq!(manifest.package.name, "test-project");
        assert_eq!(manifest.package.version, "0.1.0");
    }

    #[test]
    fn test_init_project_already_exists() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("test-project");

        init_project("test-project", &project_dir).unwrap();

        let result = init_project("test-project", &project_dir);
        assert!(result.is_err());
        match result {
            Err(PackageError::InvalidManifest(msg)) => {
                assert!(msg.contains("already initialized"));
            }
            _ => panic!("Expected InvalidManifest error"),
        }
    }

    #[test]
    fn test_init_project_main_content() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("test-project");

        init_project("test-project", &project_dir).unwrap();

        let main_path = project_dir.join("main.nm");
        let content = std::fs::read_to_string(&main_path).unwrap();

        assert!(content.contains("Hello, world!"));
        assert!(content.contains("fn main()"));
    }
}
