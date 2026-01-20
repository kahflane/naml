///
/// naml CLI - The naml programming language command-line interface
///
/// Provides commands for running, building, and checking naml code:
/// - naml run <file>: Transpile to Rust and execute
/// - naml build: Compile to native binary or WASM
/// - naml check: Type check without building
/// - naml init: Create a new project
///

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use namlc::{check, check_with_types, compile_and_run, parse, tokenize, AstArena, DiagnosticReporter, SourceFile};

#[derive(Parser)]
#[command(name = "naml")]
#[command(author, version, about = "The naml programming language", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a naml file (transpile to Rust and run)
    Run {
        /// The file to run
        file: PathBuf,

        /// Use cached compilation for faster startup
        #[arg(long)]
        cached: bool,
    },

    /// Build a naml project
    Build {
        /// Target platform (native, server, browser)
        #[arg(long, default_value = "native")]
        target: String,

        /// Build in release mode
        #[arg(long)]
        release: bool,
    },

    /// Type check without building
    Check {
        /// File or directory to check
        path: Option<PathBuf>,
    },

    /// Initialize a new naml project
    Init {
        /// Project name
        name: Option<String>,
    },

    /// Run tests
    Test {
        /// Filter tests by name
        filter: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file, cached } => {
            run_file(&file, cached);
        }
        Commands::Build { target, release } => {
            build_project(&target, release);
        }
        Commands::Check { path } => {
            check_code(path.as_deref());
        }
        Commands::Init { name } => {
            init_project(name.as_deref());
        }
        Commands::Test { filter } => {
            run_tests(filter.as_deref());
        }
    }
}

fn run_file(file: &PathBuf, cached: bool) {
    let source_text = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };

    let file_name = file.display().to_string();
    let source_file = SourceFile::new(file_name.clone(), source_text.clone());
    let (tokens, interner) = tokenize(&source_text);

    let arena = AstArena::new();
    let parse_result = parse(&tokens, &source_text, &arena);

    if !parse_result.errors.is_empty() {
        let reporter = DiagnosticReporter::new(&source_file);
        reporter.report_parse_errors(&parse_result.errors);
        std::process::exit(1);
    }

    let type_result = check_with_types(&parse_result.ast, &interner);

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
        &type_result.symbols,
    ) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Execution error: {}", e);
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

fn check_file(path: &std::path::Path) {
    let source_text = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };

    let file_name = path.display().to_string();
    let source_file = SourceFile::new(file_name.clone(), source_text.clone());
    let (tokens, interner) = tokenize(&source_text);

    let arena = AstArena::new();
    let parse_result = parse(&tokens, &source_text, &arena);
    let mut has_errors = false;

    if !parse_result.errors.is_empty() {
        let reporter = DiagnosticReporter::new(&source_file);
        reporter.report_parse_errors(&parse_result.errors);
        has_errors = true;
    }

    if !has_errors {
        let type_errors = check(&parse_result.ast, &interner);

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
    let mut checked = 0;
    let mut errors = 0;

    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let file_path = entry.path();
        if file_path.extension().map(|e| e == "naml").unwrap_or(false) {
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
            let (tokens, interner) = tokenize(&source_text);

            let arena = AstArena::new();
            let parse_result = parse(&tokens, &source_text, &arena);
            let mut file_has_errors = false;

            if !parse_result.errors.is_empty() {
                let reporter = DiagnosticReporter::new(&source_file);
                reporter.report_parse_errors(&parse_result.errors);
                file_has_errors = true;
            }

            if !file_has_errors {
                let type_errors = check(&parse_result.ast, &interner);
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

fn init_project(name: Option<&str>) {
    let name = name.unwrap_or("my-naml-project");
    println!("Initializing project: {}", name);
    println!("(init not yet implemented)");
}

fn run_tests(filter: Option<&str>) {
    if let Some(f) = filter {
        println!("Running tests matching: {}", f);
    } else {
        println!("Running all tests");
    }
    println!("(test not yet implemented)");
}
