//! # Version endpoint
//!
//! - `GET /api/version` — returns version info including build timestamp and git hash

use axum::Json;
use chrono::Local;
use serde_json::json;

/// Return version metadata for debugging.
///
/// Returns:
/// ```json
/// {
///   "version": "0.1.0",
///   "git_hash": "abc1234",
///   "build_time": "2026-07-12T19:15:00+00:00"
/// }
/// ```
pub async fn version() -> Json<serde_json::Value> {
    Json(json!({
        "version": env!("PKG_VERSION"),
        "git_hash": env!("GIT_HASH"),
        "build_time": Local::now().to_rfc3339()
    }))
}