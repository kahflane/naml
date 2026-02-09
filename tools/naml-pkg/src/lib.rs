///
/// # naml-pkg — Package manager library for the naml programming language
///
/// This crate provides the library API used by `naml pkg` subcommands
/// and the compiler's type checker for resolving package imports.
///
/// ## Library Usage
///
/// ```rust,ignore
/// use naml_pkg::{PackageManager, find_project_root};
///
/// if let Some(root) = find_project_root(source_dir) {
///     let mut pm = PackageManager::from_manifest_path(&root.join("naml.toml"))?;
///     pm.ensure_all_downloaded()?;
/// }
/// ```
///
/// ## CLI
///
/// ```sh
/// naml pkg get          # Download all dependencies from naml.toml
/// naml pkg init [name]  # Create a new naml project
/// ```
///

pub mod cache;
pub mod downloader;
pub mod errors;
pub mod init;
pub mod manifest;
pub mod manager;
pub mod resolver;

pub use cache::find_project_root;
pub use errors::PackageError;
pub use init::init_project;
pub use manager::PackageManager;
pub use manifest::{Dependency, DependencySource, GitRef, Manifest, PackageMetadata};
