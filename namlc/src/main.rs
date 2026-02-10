//!
//! naml CLI - The naml programming language command-line interface
//!
//! Provides commands for running, building, and checking naml code:
//! - naml run <file>: JIT compile and execute
//! - naml build: Compile to native binary or WASM
//! - naml check: Type check without building
//! - naml pkg init: Create a new project
//! - naml pkg get: Download all dependencies
//!

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use namlc::{check_with_types, compile_and_run, parse, tokenize, AstArena, DiagnosticReporter, SourceFile};

#[derive(Parser)]
#[command(name = "naml")]
#[command(author, version, about = "The naml programming language", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        file: PathBuf,
        #[arg(long)]
        cached: bool,
        #[arg(long, help = "Release mode: disable shadow stack for better performance")]
        release: bool,
        #[arg(long, help = "Unsafe mode: disable array bounds checking for maximum performance")]
        r#unsafe: bool,
    },
    Build {
        #[arg(long, default_value = "native")]
        target: String,
        #[arg(long)]
        release: bool,
    },
    Check {
        path: Option<PathBuf>,
    },
    Test {
        filter: Option<String>,
    },
    #[command(about = "Package manager commands")]
    Pkg {
        #[command(subcommand)]
        command: PkgCommands,
    },
}

#[derive(Subcommand)]
enum PkgCommands {
    #[command(about = "Create a new naml project")]
    Init {
        #[arg(default_value = "my-naml-project")]
        name: String,
    },
    #[command(about = "Download all dependencies from naml.toml")]
    Get,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file, cached, release, r#unsafe } => {
            run_file(&file, cached, release, r#unsafe);
        }
        Commands::Build { target, release } => {
            build_project(&target, release);
        }
        Commands::Check { path } => {
            check_code(path.as_deref());
        }
        Commands::Test { filter } => {
            run_tests(filter.as_deref());
        }
        Commands::Pkg { command } => match command {
            PkgCommands::Init { name } => pkg_init(&name),
            PkgCommands::Get => pkg_get(),
        },
    }
}

fn run_file(file: &PathBuf, cached: bool, release: bool, unsafe_mode: bool) {
    if file.extension().map(|e| e != "nm").unwrap_or(true) {
        eprintln!("Error: expected a .nm file, got '{}'", file.display());
        std::process::exit(1);
    }
    let source_text = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };

    let file_name = file.display().to_string();
    let source_file = SourceFile::new(file_name.clone(), source_text.clone());
    let (tokens, mut interner) = tokenize(&source_text);

    let arena = AstArena::new();
    let parse_result = parse(&tokens, &source_text, &arena);

    if !parse_result.errors.is_empty() {
        let reporter = DiagnosticReporter::new(&source_file);
        reporter.report_parse_errors(&parse_result.errors);
        std::process::exit(1);
    }

    let source_dir = std::path::Path::new(&file_name).parent().map(|p| p.to_path_buf());

    let pkg_manager = create_package_manager(source_dir.as_deref());

    let type_result = check_with_types(
        &parse_result.ast,
        &mut interner,
        source_dir,
        pkg_manager.as_ref(),
    );

    if !type_result.errors.is_empty() {
        let reporter = DiagnosticReporter::new(&source_file);
        reporter.report_type_errors(&type_result.errors);
        std::process::exit(1);
    }

    if cached {
        eprintln!("(cached mode not yet implemented)");
    }

    match compile_and_run(
        &parse_result.ast,
        &interner,
        &type_result.annotations,
        &type_result.imported_modules,
        &source_file,
        release,
        unsafe_mode,
    ) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn build_project(target: &str, release: bool) {
    println!("Building for target: {} (release: {})", target, release);
    println!("(build not yet implemented)");
}

fn check_code(path: Option<&std::path::Path>) {
    let path = path.unwrap_or(std::path::Path::new("."));

    if path.is_file() {
        check_file(path);
    } else if path.is_dir() {
        check_directory(path);
    } else {
        eprintln!("Error: {} does not exist", path.display());
        std::process::exit(1);
    }
}

fn create_package_manager(source_dir: Option<&std::path::Path>) -> Option<naml_pkg::PackageManager> {
    let root = naml_pkg::find_project_root(source_dir?)?;
    let manifest_path = root.join("naml.toml");
    match naml_pkg::PackageManager::from_manifest_path(&manifest_path) {
        Ok(mut pm) => {
            if pm.has_dependencies() {
                if let Err(e) = pm.ensure_all_downloaded() {
                    eprintln!("Warning: failed to resolve packages: {}", e);
                }
            }
            Some(pm)
        }
        Err(e) => {
            eprintln!("Warning: failed to load manifest: {}", e);
            None
        }
    }
}

fn check_file(path: &std::path::Path) {
    if path.extension().map(|e| e != "nm").unwrap_or(true) {
        eprintln!("Error: expected a .nm file, got '{}'", path.display());
        std::process::exit(1);
    }
    let source_text = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };

    let file_name = path.display().to_string();
    let source_file = SourceFile::new(file_name.clone(), source_text.clone());
    let (tokens, mut interner) = tokenize(&source_text);

    let arena = AstArena::new();
    let parse_result = parse(&tokens, &source_text, &arena);
    let mut has_errors = false;

    if !parse_result.errors.is_empty() {
        let reporter = DiagnosticReporter::new(&source_file);
        reporter.report_parse_errors(&parse_result.errors);
        has_errors = true;
    }

    if !has_errors {
        let source_dir = path.parent().map(|p| p.to_path_buf());
        let pkg_manager = create_package_manager(source_dir.as_deref());
        let type_errors = check_with_types(
            &parse_result.ast,
            &mut interner,
            source_dir,
            pkg_manager.as_ref(),
        ).errors;

        if !type_errors.is_empty() {
            let reporter = DiagnosticReporter::new(&source_file);
            reporter.report_type_errors(&type_errors);
            has_errors = true;
        }
    }

    if has_errors {
        std::process::exit(1);
    } else {
        println!("No errors in {}", file_name);
    }
}

fn check_directory(path: &std::path::Path) {
    let pkg_manager = create_package_manager(Some(path));
    let mut checked = 0;
    let mut errors = 0;

    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let file_path = entry.path();
        if file_path.extension().map(|e| e == "nm").unwrap_or(false) {
            let source_text = match std::fs::read_to_string(file_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error reading {}: {}", file_path.display(), e);
                    errors += 1;
                    continue;
                }
            };

            let file_name = file_path.display().to_string();
            let source_file = SourceFile::new(file_name.clone(), source_text.clone());
            let (tokens, mut interner) = tokenize(&source_text);

            let arena = AstArena::new();
            let parse_result = parse(&tokens, &source_text, &arena);
            let mut file_has_errors = false;

            if !parse_result.errors.is_empty() {
                let reporter = DiagnosticReporter::new(&source_file);
                reporter.report_parse_errors(&parse_result.errors);
                file_has_errors = true;
            }

            if !file_has_errors {
                let source_dir = file_path.parent().map(|p| p.to_path_buf());
                let type_errors = check_with_types(
                    &parse_result.ast,
                    &mut interner,
                    source_dir,
                    pkg_manager.as_ref(),
                ).errors;
                if !type_errors.is_empty() {
                    let reporter = DiagnosticReporter::new(&source_file);
                    reporter.report_type_errors(&type_errors);
                    file_has_errors = true;
                }
            }

            if file_has_errors {
                errors += 1;
            }
            checked += 1;
        }
    }

    println!("Checked {} files, {} with errors", checked, errors);

    if errors > 0 {
        std::process::exit(1);
    }
}

fn pkg_init(name: &str) {
    let dir = PathBuf::from(name);
    match naml_pkg::init_project(name, &dir) {
        Ok(()) => {
            println!("Created project '{}'", name);
            println!("  cd {}", name);
            println!("  naml run main.nm");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn pkg_get() {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let project_root = match naml_pkg::find_project_root(&cwd) {
        Some(r) => r,
        None => {
            eprintln!("Error: no naml.toml found in {} or any parent directory", cwd.display());
            std::process::exit(1);
        }
    };

    let manifest_path = project_root.join("naml.toml");
    println!("Found manifest at {}", manifest_path.display());

    match naml_pkg::PackageManager::from_manifest_path(&manifest_path) {
        Ok(mut pm) => {
            if let Err(e) = pm.ensure_all_downloaded() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            println!("All dependencies downloaded successfully.");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_tests(filter: Option<&str>) {
    if let Some(f) = filter {
        println!("Running tests matching: {}", f);
    } else {
        println!("Running all tests");
    }
    println!("(test not yet implemented)");
}
