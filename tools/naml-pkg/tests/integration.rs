///
/// # Integration Tests for naml-pkg
///
/// End-to-end tests covering complete workflows including project initialization,
/// dependency resolution, local path dependencies, transitive dependencies, and
/// circular dependency detection.
///

use std::fs;
use std::path::Path;
use tempfile::TempDir;

use naml_pkg::{find_project_root, init_project, Manifest, PackageError, PackageManager};

fn create_manifest_file(dir: &Path, name: &str, deps: &str) -> std::io::Result<()> {
    let content = format!(
        r#"[package]
name = "{}"
version = "0.1.0"

[dependencies]
{}"#,
        name, deps
    );
    fs::write(dir.join("naml.toml"), content)
}

fn create_minimal_manifest(dir: &Path, name: &str) -> std::io::Result<()> {
    let content = format!(
        r#"[package]
name = "{}"
version = "0.1.0"

[dependencies]
"#,
        name
    );
    fs::write(dir.join("naml.toml"), content)
}

fn create_main_nm(dir: &Path) -> std::io::Result<()> {
    fs::write(
        dir.join("main.nm"),
        r#"fn main() {
    println("Hello from package!");
}
"#,
    )
}

#[test]
fn test_init_and_parse_workflow() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_dir = temp_dir.path().join("test-project");

    init_project("test-project", &project_dir).expect("Failed to init project");

    assert!(project_dir.exists(), "Project directory should be created");
    assert!(
        project_dir.join("naml.toml").exists(),
        "naml.toml should be created"
    );
    assert!(
        project_dir.join("main.nm").exists(),
        "main.nm should be created"
    );

    let manifest_content = fs::read_to_string(project_dir.join("naml.toml"))
        .expect("Failed to read naml.toml");
    let manifest: Manifest =
        toml::from_str(&manifest_content).expect("Failed to parse naml.toml");

    assert_eq!(
        manifest.package.name, "test-project",
        "Package name should match"
    );
    assert_eq!(
        manifest.package.version, "0.1.0",
        "Version should be 0.1.0"
    );
    assert_eq!(
        manifest.dependencies.len(),
        0,
        "Should have no dependencies"
    );
}

#[test]
fn test_init_project_then_add_dependency() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_dir = temp_dir.path().join("my-project");
    let lib_dir = project_dir.join("libs").join("mylib");

    init_project("my-project", &project_dir).expect("Failed to init project");

    fs::create_dir_all(&lib_dir).expect("Failed to create lib directory");
    create_minimal_manifest(&lib_dir, "mylib").expect("Failed to create lib manifest");
    create_main_nm(&lib_dir).expect("Failed to create lib main.nm");

    create_manifest_file(
        &project_dir,
        "my-project",
        r#"mylib = { path = "./libs/mylib" }"#,
    )
    .expect("Failed to update manifest");

    let manifest_path = project_dir.join("naml.toml");
    let mut pm = PackageManager::from_manifest_path(&manifest_path)
        .expect("Failed to create PackageManager");

    pm.resolve().expect("Failed to resolve dependencies");

    assert!(
        pm.is_package("mylib"),
        "mylib should be recognized as a package"
    );

    let source_dir = pm
        .package_source_dir("mylib")
        .expect("mylib source directory should be found");

    assert!(
        source_dir.ends_with("mylib"),
        "Source directory should point to mylib"
    );
    assert!(
        source_dir.exists(),
        "Source directory should exist: {:?}",
        source_dir
    );
}

#[test]
fn test_local_dependency_resolution() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_dir = temp_dir.path().join("project");
    let lib_dir = project_dir.join("libs").join("mylib");

    fs::create_dir_all(&project_dir).expect("Failed to create project directory");
    fs::create_dir_all(&lib_dir).expect("Failed to create lib directory");

    create_manifest_file(
        &project_dir,
        "project",
        r#"mylib = { path = "./libs/mylib" }"#,
    )
    .expect("Failed to create project manifest");

    create_minimal_manifest(&lib_dir, "mylib").expect("Failed to create lib manifest");
    create_main_nm(&lib_dir).expect("Failed to create lib main.nm");

    let manifest_path = project_dir.join("naml.toml");
    let mut pm = PackageManager::from_manifest_path(&manifest_path)
        .expect("Failed to create PackageManager");

    pm.resolve().expect("Failed to resolve dependencies");

    assert!(pm.is_package("mylib"), "mylib should be a package");

    let source_dir = pm
        .package_source_dir("mylib")
        .expect("mylib source directory should be found");

    assert!(
        source_dir.exists(),
        "mylib source directory should exist"
    );
    assert!(
        source_dir.join("naml.toml").exists(),
        "mylib should have naml.toml"
    );
    assert!(
        source_dir.join("main.nm").exists(),
        "mylib should have main.nm"
    );
}

#[test]
fn test_transitive_local_dependencies() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let root_dir = temp_dir.path().join("root");
    let liba_dir = root_dir.join("liba");
    let libb_dir = root_dir.join("libb");

    fs::create_dir_all(&root_dir).expect("Failed to create root directory");
    fs::create_dir_all(&liba_dir).expect("Failed to create liba directory");
    fs::create_dir_all(&libb_dir).expect("Failed to create libb directory");

    create_manifest_file(&root_dir, "root", r#"liba = { path = "./liba" }"#)
        .expect("Failed to create root manifest");

    create_manifest_file(&liba_dir, "liba", r#"libb = { path = "../libb" }"#)
        .expect("Failed to create liba manifest");
    create_main_nm(&liba_dir).expect("Failed to create liba main.nm");

    create_minimal_manifest(&libb_dir, "libb").expect("Failed to create libb manifest");
    create_main_nm(&libb_dir).expect("Failed to create libb main.nm");

    let manifest_path = root_dir.join("naml.toml");
    let mut pm = PackageManager::from_manifest_path(&manifest_path)
        .expect("Failed to create PackageManager");

    pm.resolve()
        .expect("Failed to resolve transitive dependencies");

    let all_packages = pm.all_packages();
    let package_names: Vec<&str> = all_packages.iter().map(|p| p.name.as_str()).collect();

    assert!(
        package_names.contains(&"liba"),
        "liba should be in all_packages"
    );
    assert!(
        package_names.contains(&"libb"),
        "libb should be in all_packages (transitive)"
    );
    assert_eq!(
        all_packages.len(),
        2,
        "Should have exactly 2 packages: liba and libb"
    );

    assert!(pm.is_package("liba"), "liba should be a package");
    assert!(pm.is_package("libb"), "libb should be a package");

    let liba_dir_resolved = pm
        .package_source_dir("liba")
        .expect("liba source directory should be found");
    let libb_dir_resolved = pm
        .package_source_dir("libb")
        .expect("libb source directory should be found");

    assert!(liba_dir_resolved.exists(), "liba directory should exist");
    assert!(libb_dir_resolved.exists(), "libb directory should exist");
}

#[test]
fn test_circular_local_dependency_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let root_dir = temp_dir.path().join("root");
    let pkg_a_dir = root_dir.join("pkg_a");
    let pkg_b_dir = root_dir.join("pkg_b");

    fs::create_dir_all(&root_dir).expect("Failed to create root directory");
    fs::create_dir_all(&pkg_a_dir).expect("Failed to create pkg_a directory");
    fs::create_dir_all(&pkg_b_dir).expect("Failed to create pkg_b directory");

    create_manifest_file(&root_dir, "root", r#"pkg_a = { path = "./pkg_a" }"#)
        .expect("Failed to create root manifest");

    create_manifest_file(&pkg_a_dir, "pkg_a", r#"pkg_b = { path = "../pkg_b" }"#)
        .expect("Failed to create pkg_a manifest");
    create_main_nm(&pkg_a_dir).expect("Failed to create pkg_a main.nm");

    create_manifest_file(&pkg_b_dir, "pkg_b", r#"pkg_a = { path = "../pkg_a" }"#)
        .expect("Failed to create pkg_b manifest");
    create_main_nm(&pkg_b_dir).expect("Failed to create pkg_b main.nm");

    let manifest_path = root_dir.join("naml.toml");
    let mut pm = PackageManager::from_manifest_path(&manifest_path)
        .expect("Failed to create PackageManager");

    let result = pm.resolve();

    assert!(
        result.is_err(),
        "Should fail to resolve circular dependencies"
    );

    match result {
        Err(PackageError::CircularDependency { cycle }) => {
            assert!(
                !cycle.is_empty(),
                "Cycle should contain at least one package"
            );
            let cycle_str = cycle.join(" -> ");
            assert!(
                cycle_str.contains("pkg_a") || cycle_str.contains("pkg_b"),
                "Cycle should mention pkg_a or pkg_b, got: {}",
                cycle_str
            );
        }
        Err(e) => panic!("Expected CircularDependency error, got: {:?}", e),
        Ok(_) => panic!("Expected error, got Ok"),
    }
}

#[test]
fn test_find_project_root_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_root = temp_dir.path().join("my-project");
    let nested_dir = project_root.join("src").join("modules").join("deep");

    fs::create_dir_all(&nested_dir).expect("Failed to create nested directory");

    create_minimal_manifest(&project_root, "my-project")
        .expect("Failed to create manifest at root");

    let found_root = find_project_root(&nested_dir).expect("Should find project root");

    assert_eq!(
        found_root, project_root,
        "Should find the correct project root"
    );

    let manifest_path = found_root.join("naml.toml");
    assert!(
        manifest_path.exists(),
        "Found root should contain naml.toml"
    );
}

#[test]
fn test_package_manager_no_dependencies() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_dir = temp_dir.path().join("empty-project");

    fs::create_dir_all(&project_dir).expect("Failed to create project directory");
    create_minimal_manifest(&project_dir, "empty-project")
        .expect("Failed to create manifest");

    let manifest_path = project_dir.join("naml.toml");
    let mut pm = PackageManager::from_manifest_path(&manifest_path)
        .expect("Failed to create PackageManager");

    assert!(
        !pm.has_dependencies(),
        "Should report no dependencies before resolve"
    );

    pm.resolve().expect("Should resolve successfully");

    let all_packages = pm.all_packages();
    assert!(
        all_packages.is_empty(),
        "all_packages() should be empty when no dependencies"
    );

    assert!(
        !pm.has_dependencies(),
        "Should still report no dependencies after resolve"
    );

    assert!(
        !pm.is_package("nonexistent"),
        "Should return false for nonexistent package"
    );
}

#[test]
fn test_manifest_from_manifest_direct() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_dir = temp_dir.path().join("test-project");

    fs::create_dir_all(&project_dir).expect("Failed to create project directory");
    create_minimal_manifest(&project_dir, "test-project")
        .expect("Failed to create manifest");

    let manifest_path = project_dir.join("naml.toml");
    let manifest_content = fs::read_to_string(&manifest_path).expect("Failed to read manifest");
    let manifest: Manifest =
        toml::from_str(&manifest_content).expect("Failed to parse manifest");

    let pm = PackageManager::from_manifest(manifest.clone(), project_dir.clone());

    assert_eq!(pm.manifest().package.name, "test-project");
    assert_eq!(pm.manifest_dir(), project_dir.as_path());
    assert!(!pm.has_dependencies());
}

#[test]
fn test_multiple_local_dependencies() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let root_dir = temp_dir.path().join("root");
    let lib1_dir = root_dir.join("lib1");
    let lib2_dir = root_dir.join("lib2");
    let lib3_dir = root_dir.join("lib3");

    fs::create_dir_all(&root_dir).expect("Failed to create root directory");
    fs::create_dir_all(&lib1_dir).expect("Failed to create lib1 directory");
    fs::create_dir_all(&lib2_dir).expect("Failed to create lib2 directory");
    fs::create_dir_all(&lib3_dir).expect("Failed to create lib3 directory");

    create_manifest_file(
        &root_dir,
        "root",
        r#"lib1 = { path = "./lib1" }
lib2 = { path = "./lib2" }
lib3 = { path = "./lib3" }"#,
    )
    .expect("Failed to create root manifest");

    create_minimal_manifest(&lib1_dir, "lib1").expect("Failed to create lib1 manifest");
    create_main_nm(&lib1_dir).expect("Failed to create lib1 main.nm");

    create_minimal_manifest(&lib2_dir, "lib2").expect("Failed to create lib2 manifest");
    create_main_nm(&lib2_dir).expect("Failed to create lib2 main.nm");

    create_minimal_manifest(&lib3_dir, "lib3").expect("Failed to create lib3 manifest");
    create_main_nm(&lib3_dir).expect("Failed to create lib3 main.nm");

    let manifest_path = root_dir.join("naml.toml");
    let mut pm =
        PackageManager::from_manifest_path(&manifest_path).expect("Failed to create PackageManager");

    pm.resolve().expect("Failed to resolve dependencies");

    assert!(pm.has_dependencies(), "Should have dependencies");

    let all_packages = pm.all_packages();
    assert_eq!(all_packages.len(), 3, "Should have 3 packages");

    assert!(pm.is_package("lib1"), "lib1 should be a package");
    assert!(pm.is_package("lib2"), "lib2 should be a package");
    assert!(pm.is_package("lib3"), "lib3 should be a package");

    let lib1_dir_resolved = pm
        .package_source_dir("lib1")
        .expect("lib1 source directory should be found");
    let lib2_dir_resolved = pm
        .package_source_dir("lib2")
        .expect("lib2 source directory should be found");
    let lib3_dir_resolved = pm
        .package_source_dir("lib3")
        .expect("lib3 source directory should be found");

    assert!(lib1_dir_resolved.exists());
    assert!(lib2_dir_resolved.exists());
    assert!(lib3_dir_resolved.exists());
}

#[test]
fn test_diamond_dependency_deduplication() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let root_dir = temp_dir.path().join("root");
    let liba_dir = root_dir.join("liba");
    let libb_dir = root_dir.join("libb");
    let common_dir = root_dir.join("common");

    fs::create_dir_all(&root_dir).expect("Failed to create root directory");
    fs::create_dir_all(&liba_dir).expect("Failed to create liba directory");
    fs::create_dir_all(&libb_dir).expect("Failed to create libb directory");
    fs::create_dir_all(&common_dir).expect("Failed to create common directory");

    create_manifest_file(
        &root_dir,
        "root",
        r#"liba = { path = "./liba" }
libb = { path = "./libb" }"#,
    )
    .expect("Failed to create root manifest");

    create_manifest_file(&liba_dir, "liba", r#"common = { path = "../common" }"#)
        .expect("Failed to create liba manifest");
    create_main_nm(&liba_dir).expect("Failed to create liba main.nm");

    create_manifest_file(&libb_dir, "libb", r#"common = { path = "../common" }"#)
        .expect("Failed to create libb manifest");
    create_main_nm(&libb_dir).expect("Failed to create libb main.nm");

    create_minimal_manifest(&common_dir, "common").expect("Failed to create common manifest");
    create_main_nm(&common_dir).expect("Failed to create common main.nm");

    let manifest_path = root_dir.join("naml.toml");
    let mut pm =
        PackageManager::from_manifest_path(&manifest_path).expect("Failed to create PackageManager");

    pm.resolve()
        .expect("Failed to resolve diamond dependencies");

    let all_packages = pm.all_packages();
    let package_names: Vec<&str> = all_packages.iter().map(|p| p.name.as_str()).collect();

    assert!(package_names.contains(&"liba"), "Should have liba");
    assert!(package_names.contains(&"libb"), "Should have libb");
    assert!(package_names.contains(&"common"), "Should have common");

    let common_count = package_names.iter().filter(|&&n| n == "common").count();
    assert_eq!(
        common_count, 1,
        "common should appear exactly once (deduplication)"
    );
}

#[test]
fn test_resolve_package_returns_metadata() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let root_dir = temp_dir.path().join("root");
    let lib_dir = root_dir.join("mylib");

    fs::create_dir_all(&root_dir).expect("Failed to create root directory");
    fs::create_dir_all(&lib_dir).expect("Failed to create lib directory");

    create_manifest_file(&root_dir, "root", r#"mylib = { path = "./mylib" }"#)
        .expect("Failed to create root manifest");

    create_minimal_manifest(&lib_dir, "mylib").expect("Failed to create lib manifest");
    create_main_nm(&lib_dir).expect("Failed to create lib main.nm");

    let manifest_path = root_dir.join("naml.toml");
    let mut pm =
        PackageManager::from_manifest_path(&manifest_path).expect("Failed to create PackageManager");

    pm.resolve().expect("Failed to resolve dependencies");

    let resolved = pm
        .resolve_package("mylib")
        .expect("Should resolve mylib package");

    assert_eq!(resolved.name, "mylib");
    assert!(resolved.cache_path.exists());
    assert!(
        resolved.manifest.is_some(),
        "Resolved package should have manifest"
    );

    if let Some(ref manifest) = resolved.manifest {
        assert_eq!(manifest.package.name, "mylib");
    }
}

#[test]
fn test_ensure_all_downloaded_calls_resolve() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_dir = temp_dir.path().join("test-project");

    fs::create_dir_all(&project_dir).expect("Failed to create project directory");
    create_minimal_manifest(&project_dir, "test-project").expect("Failed to create manifest");

    let manifest_path = project_dir.join("naml.toml");
    let mut pm =
        PackageManager::from_manifest_path(&manifest_path).expect("Failed to create PackageManager");

    pm.ensure_all_downloaded()
        .expect("ensure_all_downloaded should succeed");

    let all_packages = pm.all_packages();
    assert!(
        all_packages.is_empty(),
        "Should have no packages after ensure_all_downloaded with no deps"
    );
}
