//! Build script that:
//! 1. Calls `build_dashboard.py` to minify CSS/JS and inline into a single HTML file
//! 2. Sets GIT_HASH and PKG_VERSION env vars for the /api/version endpoint
//!
//! The result is a single bundled HTML file embedded at compile time —
//! zero external asset requests at runtime.

use std::process::Command;

fn main() {
    // Run the Python build script to produce static/bundled.html.
    let status = Command::new("python3")
        .args(["build_dashboard.py"])
        .current_dir(".")
        .status()
        .expect("python3 not found — install python3 to build");

    if !status.success() {
        panic!("build_dashboard.py failed (exit code {})", status.code().unwrap_or(-1));
    }

    println!("cargo:rerun-if-changed=static/index.html");
    println!("cargo:rerun-if-changed=static/app.css");
    println!("cargo:rerun-if-changed=static/app.js");
    println!("cargo:rerun-if-changed=build_dashboard.py");
    println!("cargo:rerun-if-changed=build.rs");

    // Capture the short git commit hash.
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into());

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    println!("cargo:rustc-env=PKG_VERSION={}", env!("CARGO_PKG_VERSION"));
}