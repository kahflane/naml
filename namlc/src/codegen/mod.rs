///
/// Code Generation Module
///
/// This module handles transpilation of naml AST to Rust source code.
/// The generated Rust code is then compiled with cargo and executed.
///
/// Pipeline:
/// 1. Generate Rust source from AST
/// 2. Write to .naml_build/src/main.rs
/// 3. Generate Cargo.toml
/// 4. Run cargo build --release
/// 5. Execute the resulting binary
///

pub mod rust;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use lasso::Rodeo;
use thiserror::Error;

use crate::ast::SourceFile;

#[derive(Debug, Error)]
pub enum CodegenError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Cargo build failed: {0}")]
    CargoBuild(String),

    #[error("Execution failed: {0}")]
    Execution(String),

    #[error("Unsupported feature: {0}")]
    Unsupported(String),
}

pub struct BuildConfig {
    pub output_dir: PathBuf,
    pub release: bool,
    pub target: Target,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Native,
    Server,
    Browser,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from(".naml_build"),
            release: true,
            target: Target::Native,
        }
    }
}

pub fn compile_and_run(ast: &SourceFile<'_>, interner: &Rodeo) -> Result<(), CodegenError> {
    let config = BuildConfig::default();

    let rust_code = rust::generate(ast, interner)?;

    setup_build_directory(&config)?;

    let main_rs_path = config.output_dir.join("src").join("main.rs");
    fs::write(&main_rs_path, &rust_code)?;

    let cargo_toml = generate_cargo_toml("naml_program");
    let cargo_toml_path = config.output_dir.join("Cargo.toml");
    fs::write(&cargo_toml_path, cargo_toml)?;

    build_project(&config)?;

    run_binary(&config)?;

    Ok(())
}

pub fn compile_only(
    ast: &SourceFile<'_>,
    interner: &Rodeo,
    config: &BuildConfig,
) -> Result<PathBuf, CodegenError> {
    let rust_code = rust::generate(ast, interner)?;

    setup_build_directory(config)?;

    let main_rs_path = config.output_dir.join("src").join("main.rs");
    fs::write(&main_rs_path, &rust_code)?;

    let cargo_toml = generate_cargo_toml("naml_program");
    let cargo_toml_path = config.output_dir.join("Cargo.toml");
    fs::write(&cargo_toml_path, cargo_toml)?;

    build_project(config)?;

    let binary_name = if cfg!(windows) {
        "naml_program.exe"
    } else {
        "naml_program"
    };

    let binary_path = if config.release {
        config.output_dir.join("target").join("release").join(binary_name)
    } else {
        config.output_dir.join("target").join("debug").join(binary_name)
    };

    Ok(binary_path)
}

fn setup_build_directory(config: &BuildConfig) -> Result<(), CodegenError> {
    let src_dir = config.output_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    Ok(())
}

fn generate_cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[workspace]

[dependencies]
tokio = {{ version = "1", features = ["full"] }}
"#,
        name
    )
}

fn build_project(config: &BuildConfig) -> Result<(), CodegenError> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build");

    if config.release {
        cmd.arg("--release");
    }

    cmd.current_dir(&config.output_dir);

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CodegenError::CargoBuild(stderr.to_string()));
    }

    Ok(())
}

fn run_binary(config: &BuildConfig) -> Result<(), CodegenError> {
    let binary_name = if cfg!(windows) {
        "naml_program.exe"
    } else {
        "naml_program"
    };

    let binary_path = if config.release {
        config.output_dir.join("target").join("release").join(binary_name)
    } else {
        config.output_dir.join("target").join("debug").join(binary_name)
    };

    let output = Command::new(&binary_path).output()?;

    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));

    if !output.status.success() {
        if let Some(code) = output.status.code() {
            std::process::exit(code);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_cargo_toml() {
        let toml = generate_cargo_toml("test_project");
        assert!(toml.contains("name = \"test_project\""));
        assert!(toml.contains("edition = \"2021\""));
    }
}
