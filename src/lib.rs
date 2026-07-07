//! # lm-sensors-web library crate
//!
//! A hardware sensor monitoring REST API that exposes Linux `lm-sensors` data
//! via HTTP endpoints, WebSocket live-feed, and webhooks.
//!
//! # Module overview
//!
//! | Module      | Purpose                                              |
//! |------------|------------------------------------------------------|
//! | `api`      | Axum route handlers (health, sensors, devices)       |
//! | `cli`      | Command-line parsing via `clap`                      |
//! | `config`   | Runtime config (JSON file + defaults)                |
//! | `sensors`  | `lm-sensors` wrapper with safe `Send + Sync`         |
//! | `server`   | Axum router + static file serving                    |
//! | `service`  | Systemd service install / manage utilities            |
//! | `state`    | Shared app state (injected via Axum `State<T>`)     |
//! | `websocket`| WebSocket broadcast server for real-time feed         |
//! | `webhook`  | HTTP webhook engine (always / temperature / on-change)|

pub mod api;
pub mod cli;
pub mod config;
pub mod sensors;
pub mod server;
pub mod service;
pub mod state;
pub mod webhook;
pub mod websocket;
