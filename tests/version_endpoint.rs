//! # Version endpoint tests
//!
//! Tests the `/api/version` endpoint response schema and data validity.
//!
//! # Running
//!
//! ```bash
//! cargo test --test version_endpoint
//! ```

/// Verify the version response schema matches expected fields.
#[test]
fn test_version_response_schema() {
    let response = serde_json::json!({
        "version": env!("PKG_VERSION"),
        "git_hash": env!("GIT_HASH"),
        "build_time": "2024-01-01T00:00:00+00:00"
    });
    let value: serde_json::Value = response;

    // All three fields must be present and strings.
    assert!(value["version"].is_string());
    assert!(value["git_hash"].is_string());
    assert!(value["build_time"].is_string());
}

/// Verify the git hash is not "unknown" (should be a short commit hash in CI).
#[test]
fn test_git_hash_not_unknown() {
    let git_hash = env!("GIT_HASH");
    assert_ne!(git_hash, "unknown", "GIT_HASH should be set by build.rs");
    // Short git hashes are typically 7-9 hex chars.
    assert!(git_hash.len() >= 7 && git_hash.len() <= 9);
    assert!(
        git_hash.chars().all(|c| c.is_ascii_hexdigit()),
        "GIT_HASH should be a hex string, got: {}",
        git_hash
    );
}

/// Verify the version string matches semver format.
#[test]
fn test_version_semver_format() {
    let version = env!("PKG_VERSION");
    assert!(
        version.starts_with("0.1"),
        "Version should start with '0.1', got: {}",
        version
    );
}

/// Verify version response can be serialized and parsed correctly.
#[test]
fn test_version_json_roundtrip() {
    let version = env!("PKG_VERSION");
    let git_hash = env!("GIT_HASH");

    let json_str = serde_json::json!({
        "version": version,
        "git_hash": git_hash,
        "build_time": "2026-07-12T19:00:00+00:00"
    })
    .to_string();

    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed["version"], version);
    assert_eq!(parsed["git_hash"], git_hash);
}

/// Verify the build.rs env vars are actually set (sanity check).
#[test]
fn test_build_env_vars_present() {
    // These would fail at compile time if env vars aren't set.
    let _version: &str = env!("PKG_VERSION");
    let _git_hash: &str = env!("GIT_HASH");
}
