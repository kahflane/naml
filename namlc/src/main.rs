///
/// naml CLI - The naml programming language command-line interface
///
/// Provides commands for running, building, and checking naml code:
/// - naml run <file>: Execute with JIT compilation
/// - naml build: Compile to native binary or WASM
/// - naml check: Type check without building
/// - naml init: Create a new project
///

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "naml")]
#[command(author, version, about = "The naml programming language", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a naml file with JIT compilation
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
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };

    let (tokens, _interner) = namlc::tokenize(&source);

    println!("Tokenized {} tokens from {}", tokens.len(), file.display());

    for token in &tokens {
        if !token.is_trivia() {
            println!("  {:?} @ {:?}", token.kind, token.span);
        }
    }

    if cached {
        println!("(cached mode not yet implemented)");
    }
}

fn build_project(target: &str, release: bool) {
    println!("Building for target: {} (release: {})", target, release);
    println!("(build not yet implemented)");
}

fn check_code(path: Option<&std::path::Path>) {
    let path_str = path.map(|p| p.display().to_string()).unwrap_or_else(|| ".".into());
    println!("Checking: {}", path_str);
    println!("(check not yet implemented)");
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
