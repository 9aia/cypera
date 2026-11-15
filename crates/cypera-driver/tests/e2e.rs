#![allow(clippy::unwrap_used)]
//! End-to-end tests: run the real engine through
//! `tamias-driver-cypera run`/`cli` — the same entrypoints the
//! `cypera` bins[] shim resolves to — against a minimal fixture repo
//! (see `fixture_repo`).

use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_tamias-driver-cypera")
}

/// Minimal repo fixture for the passthrough tests — a single-package
/// workspace whose `cypera.toml` declares every output `gen
/// crate-map` writes (the docs splice target + the `out` page).
/// The tests must not run against this repo itself: `docs/` is a
/// separate git worktree and is absent in detached gate worktrees,
/// which would make the configured splice target unreadable.
fn fixture_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"fixture-crate\"\nversion = \"0.1.0\"\nedition = \"2021\"\ndescription = \"fixture crate\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "//! Fixture crate.\n\npub fn f() {}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("cypera.toml"),
        "[crate_map]\nout = \"docs/reference/crate-map.md\"\n\n[[crate_map.splices]]\ntarget = \"docs/explanation/architecture.md\"\nmarker = \"<!-- crate-map -->\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("docs/explanation")).unwrap();
    std::fs::create_dir_all(root.join("docs/reference")).unwrap();
    std::fs::write(
        root.join("docs/explanation/architecture.md"),
        "# Architecture\n\n<!-- crate-map -->\n",
    )
    .unwrap();
    dir
}

fn driver(args: &[&str]) -> std::process::Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("spawn tamias-driver-cypera")
}

fn stdout(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn manifest_and_capabilities_verbs() {
    let o = driver(&["manifest"]);
    assert!(o.status.success());
    let v: serde_json::Value = serde_json::from_str(&stdout(&o)).unwrap();
    assert_eq!(v["name"], "cypera");

    let o = driver(&["capabilities"]);
    assert!(o.status.success());
    let v: serde_json::Value = serde_json::from_str(&stdout(&o)).unwrap();
    assert_eq!(v["capabilities"], serde_json::json!(["pavo"]));
}

#[test]
fn ops_manifest_lists_engine_verbs() {
    let o = driver(&["ops"]);
    assert!(o.status.success());
    let v: serde_json::Value = serde_json::from_str(&stdout(&o)).unwrap();
    let ops = v["ops"].as_array().unwrap();
    let queries: Vec<&str> = ops.iter().filter_map(|o| o["query"].as_str()).collect();
    for expected in [
        "cli.check",
        "cli.gen.crate-map",
        "cli.gen.cli",
        "cli.gen.config",
    ] {
        assert!(queries.contains(&expected), "{expected} missing from ops");
    }
    // `check` is the only read-only op; the gens write via --out /
    // cargo build.
    let check = ops.iter().find(|o| o["query"] == "cli.check").unwrap();
    assert_eq!(check["mutates"], serde_json::json!(false));
}

#[test]
fn run_passthrough_runs_engine_argv() {
    let repo = fixture_repo();
    let root = repo.path().to_str().unwrap();
    // `run gen crate-map --repo <root>` ≡ `cypera gen crate-map`.
    let o = driver(&["run", "gen", "crate-map", "--repo", root]);
    assert!(
        o.status.success(),
        "run gen crate-map failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(stdout(&o).contains("crate-map"));
}

#[test]
fn cli_shim_strips_driver_flags() {
    // `tamias bin exec` prepends `--dir <payload> --name <pkg>` —
    // the shim must strip them and run the engine argv untouched.
    let repo = fixture_repo();
    let root = repo.path().to_str().unwrap();
    let o = driver(&[
        "cli",
        "--dir",
        "/tmp/fake-payload",
        "--name",
        "cypera",
        "gen",
        "crate-map",
        "--repo",
        root,
    ]);
    assert!(
        o.status.success(),
        "cli passthrough failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    // With `cypera.toml` discovered, `gen crate-map` writes its
    // declared outputs and reports the paths on stdout.
    assert!(stdout(&o).contains("crate map"));
}

#[test]
fn cli_shim_help_parity() {
    let o = driver(&["cli", "--help"]);
    assert!(o.status.success());
    assert!(stdout(&o).contains("Usage"));
}
