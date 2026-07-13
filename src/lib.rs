//! # lm-sensors-web
//!
//! A hardware sensor monitoring library and web application built in Rust.
//! Exposes Linux `libsensors` data via REST API, WebSocket live-feed,
//! webhooks, and a dark-mode web dashboard.
//!
//! # As a library
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! lm-sensors-web = "0.1"
//! ```
//!
//! ## Quick start
//!
//! ```no_run
//! use lm_sensors_web::prelude::*;
//!
//! // Read all sensor data
//! let sm = SensorManager::new().expect("sensors");
//! let readings = sm.read_all();
//! println!("Found {} devices", readings.devices.len());
//!
//! // Load config
//! let config = Config::default();
//! println!("Host: {}, Port: {}", config.server.host, config.server.port);
//! ```
//!
//! # As a binary
//!
//! ```bash
//! cargo install lm-sensors-web
//! lm-sensors-web -H 0.0.0.0 -p 47890
//! ```
//!
//! # Module overview
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`config`] | Runtime config (JSON file + defaults) |
//! | [`sensors`] | `lm-sensors` wrapper with safe `Send + Sync` |
//! | [`webhook`] | HTTP webhook engine (always / temperature / on-change) |
//! | [`websocket`] | WebSocket broadcast server for real-time feed |
//! | [`service`] | Systemd service install / manage utilities |
//! | [`server`] | Axum router construction (framework-specific) |
//! | [`state`] | Shared application state (framework-specific) |

// ── Convenience prelude ──────────────────────────────────────────────
/// Re-export the most commonly used types and traits so users can write:
///
/// ```no_run
/// use lm_sensors_web::prelude::*;
/// ```
pub mod prelude {
    #[doc(no_inline)]
    pub use super::Parser;
    #[doc(no_inline)]
    pub use super::config::*;
    #[doc(no_inline)]
    pub use super::sensors::*;
}

// ── Re-export key traits so users don't need to add extra deps ───────
pub use clap::Parser;

// ── Public modules ───────────────────────────────────────────────────
pub mod config;
pub mod sensors;
pub mod service;
pub mod webhook;
pub mod websocket;

// ── Framework-specific modules (useful only when embedding the server)
//
// These expose Axum/tower-http types. Library users who only need
// sensor data, webhooks, or WebSocket broadcasting should stick to
// the modules above.
// ──────────────────────────────────────────────────────────────────────
#[doc(hidden)]
pub mod api;
pub mod cli;
pub mod server;
pub mod state;
