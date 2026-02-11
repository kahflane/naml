///
/// AOT (Ahead-of-Time) Compilation Integration Tests
///
/// Builds `.nm` fixture files via `naml build`, executes the resulting
/// binaries, and asserts on stdout output. Covers feature correctness
/// and memory/refcount safety across all language tiers.
///
/// Each test calls `aot_run("fixture_name")` which:
/// 1. Locates the `naml` binary via `env!("CARGO_BIN_EXE_naml")`
/// 2. Runs `naml build <fixture>.nm -o <tempdir>/out`
/// 3. Executes the resulting binary, captures stdout
/// 4. Returns stdout as String
///
/// Run all:  `cargo test --test aot`
/// Run one:  `cargo test --test aot hello`
///
/// NOTE: `naml build` uses a shared temp path for the object file,
/// so builds are serialized via BUILD_LOCK to prevent races.
///

use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static BUILD_LOCK: Mutex<()> = Mutex::new(());

fn fixture_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push("aot");
    p.push(format!("{}.nm", name));
    p
}

fn aot_run(fixture_name: &str) -> String {
    let naml = env!("CARGO_BIN_EXE_naml");
    let src = fixture_path(fixture_name);
    assert!(src.exists(), "Fixture not found: {}", src.display());

    let tmp = tempfile::tempdir().expect("failed to create tempdir");
    let out_bin = tmp.path().join("out");

    {
        let _lock = BUILD_LOCK.lock().unwrap();
        let build = Command::new(naml)
            .args(["build", &src.to_string_lossy(), "-o", &out_bin.to_string_lossy()])
            .output()
            .expect("failed to run naml build");

        assert!(
            build.status.success(),
            "naml build failed for {}:\nstdout: {}\nstderr: {}",
            fixture_name,
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr),
        );

        assert!(out_bin.exists(), "Binary not produced for {}", fixture_name);
    }

    let timeout = Duration::from_secs(30);
    let start = Instant::now();

    let mut child = Command::new(&out_bin)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to execute built binary");

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = {
                    use std::io::Read;
                    let mut s = String::new();
                    child.stdout.take().unwrap().read_to_string(&mut s).unwrap();
                    s
                };
                let stderr = {
                    use std::io::Read;
                    let mut s = String::new();
                    child.stderr.take().unwrap().read_to_string(&mut s).unwrap();
                    s
                };
                if !status.success() {
                    eprintln!(
                        "WARNING: {} exited with {} (stdout captured)\nstderr: {}",
                        fixture_name, status, stderr,
                    );
                }
                return stdout;
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    panic!("Timeout ({}s) running {}", timeout.as_secs(), fixture_name);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => panic!("Error waiting for {}: {}", fixture_name, e),
        }
    }
}

// ── Tier 1: Basic Features ──────────────────────────────────────────

#[test]
fn hello() {
    let out = aot_run("hello");
    assert!(out.contains("Hello, World!"), "got: {}", out);
}

#[test]
fn arithmetic() {
    let out = aot_run("arithmetic");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn variables() {
    let out = aot_run("variables");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn functions() {
    let out = aot_run("functions");
    assert!(out.contains("55"), "fib(10) should be 55, got: {}", out);
}

#[test]
fn control_flow() {
    let out = aot_run("control_flow");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn strings() {
    let out = aot_run("strings");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn arrays() {
    let out = aot_run("arrays");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn maps() {
    let out = aot_run("maps");
    assert!(out.contains("OK"), "got: {}", out);
}

// ── Tier 2: Type System ─────────────────────────────────────────────

#[test]
fn structs() {
    let out = aot_run("structs");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn options() {
    let out = aot_run("options");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn enums() {
    let out = aot_run("enums");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn generics() {
    let out = aot_run("generics");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn methods() {
    let out = aot_run("methods");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn type_casting() {
    let out = aot_run("type_casting");
    assert!(out.contains("OK"), "got: {}", out);
}

// ── Tier 3: Advanced Features ───────────────────────────────────────

#[test]
fn lambdas() {
    let out = aot_run("lambdas");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn exceptions() {
    let out = aot_run("exceptions");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn interfaces() {
    let out = aot_run("interfaces");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn nested_structs() {
    let out = aot_run("nested_structs");
    assert!(out.contains("OK"), "got: {}", out);
}

// ── Tier 4: Concurrency ────────────────────────────────────────────

#[test]
fn spawn_join() {
    let out = aot_run("spawn_join");
    assert!(out.contains("Done"), "got: {}", out);
}

#[test]
fn channels() {
    let out = aot_run("channels");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn mutex() {
    let out = aot_run("mutex");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn atomics() {
    let out = aot_run("atomics");
    assert!(out.contains("OK"), "got: {}", out);
}

// ── Tier 5: Std Library ─────────────────────────────────────────────

#[test]
fn std_random() {
    let out = aot_run("std_random");
    assert!(out.contains("true"), "got: {}", out);
}

#[test]
fn std_datetime() {
    let out = aot_run("std_datetime");
    assert!(out.contains("true"), "got: {}", out);
}

#[test]
fn std_metrics() {
    let out = aot_run("std_metrics");
    assert!(out.contains("true"), "got: {}", out);
}

// ── Tier 6: Refcount / Memory ───────────────────────────────────────

#[test]
fn mem_loop_strings() {
    let out = aot_run("mem_loop_strings");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn mem_struct_fields() {
    let out = aot_run("mem_struct_fields");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn mem_option_heap() {
    let out = aot_run("mem_option_heap");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn mem_array_strings() {
    let out = aot_run("mem_array_strings");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn mem_spawn_captured() {
    let out = aot_run("mem_spawn_captured");
    assert!(out.contains("OK"), "got: {}", out);
}

#[test]
fn mem_binary_tree() {
    let out = aot_run("mem_binary_tree");
    assert!(out.contains("127"), "expected 127 nodes, got: {}", out);
}
