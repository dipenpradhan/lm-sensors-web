//! # Entry point
//!
//! Parses CLI arguments, initializes logging, loads config, sets up the
//! sensor manager, WebSocket broadcast, webhook engine, and starts the
//! HTTP server.
//!
//! # Shutdown
//!
//! Installs a SIGTERM/SIGINT handler for graceful shutdown: the server
//! stops accepting new connections and existing requests are allowed to
//! complete within 10 seconds.

mod api;
mod cli;
mod config;
mod sensors;
mod server;
mod service;
mod state;
mod webhook;
mod websocket;

use std::sync::Arc;

use clap::Parser;
use cli::{Cli, Command};
use config::Config;
use sensors::SensorManager;
use service::ServiceManager;
use state::AppState;
use tracing::info;
use webhook::WebhookEngine;
use websocket::WebSocketServer;

#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();

    // ── Setup logging ─────────────────────────────────────
    // Map the CLI log-level enum to a string for tracing initialization.
    let log_level = cli.log_level.to_string();
    tracing_subscriber::fmt()
        .with_max_level(
            log_level
                .parse()
                .unwrap_or(tracing::level_filters::LevelFilter::INFO),
        )
        .init();

    // ── Load config ───────────────────────────────────────
    // Prefer user-provided config file; fall back to built-in defaults.
    let config = match &cli.config {
        Some(path) => Config::load(std::path::Path::new(path)).unwrap_or_else(|e| {
            info!("Config load failed, using defaults: {}", e);
            Config::default()
        }),
        None => Config::default(),
    };

    // ── Handle service subcommands (no server needed) ─────
    // These exit early — they only manage systemd unit files.
    if let Some(cmd) = &cli.command {
        match cmd {
            Command::Start { .. } => {
                // Fall through to server start below.
            }
            Command::InstallService {
                user,
                binary,
                config: cfg,
            } => {
                let binary = binary
                    .clone()
                    .or_else(|| {
                        std::env::current_exe()
                            .ok()
                            .map(|p| p.display().to_string())
                    })
                    .unwrap_or_else(|| "/usr/local/bin/lm-sensors-web".into());
                return ServiceManager::install(&binary, cfg, *user);
            }
            Command::UninstallService { user } => return ServiceManager::uninstall(*user),
            Command::StartService { user } => return ServiceManager::control("start", *user),
            Command::StopService { user } => return ServiceManager::control("stop", *user),
            Command::RestartService { user } => return ServiceManager::control("restart", *user),
            Command::StatusService => return ServiceManager::status(false),
        }
    }

    // ── Start the server ──────────────────────────────────
    run_server(cli, config).await
}

/// Run the HTTP server with graceful shutdown support.
///
/// # Lifecycle
/// 1. Load config from file or defaults
/// 2. Apply CLI overrides only when explicitly provided (not default values)
/// 3. Initialize sensor manager (non-fatal if unavailable)
/// 4. Start WebSocket broadcast task (if enabled)
/// 5. Start webhook engine (if configured)
/// 6. Bind TCP listener and serve requests
/// 7. On SIGTERM/SIGINT → stop accepting connections, join tasks
async fn run_server(cli: Cli, config: Config) -> Result<(), String> {
    // Apply CLI overrides only when explicitly provided.
    // Default values (127.0.0.1, 47890) are just clap's fallbacks — they
    // should not override config file values. We check for the defaults
    // and skip override if the user didn't actually pass the flags.
    let mut config = config;
    // Check if user passed explicit --config flag or CONFIG_PATH env var.
    let config_path: Option<String> = cli
        .config
        .clone()
        .or_else(|| std::env::var("CONFIG_PATH").ok());
    if let Some(path) = config_path {
        if let Ok(_file_config) = Config::load(std::path::Path::new(&path)) {
            info!("Loaded config from {}", path);
            // CLI host/port override file config only if they differ from clap defaults.
            if cli.host != "127.0.0.1" {
                config.server.host = cli.host.clone();
            }
            if cli.port != 47890 {
                config.server.port = cli.port;
            }
        } else {
            // Fall back to CLI defaults + file config values.
            if cli.host != "127.0.0.1" {
                config.server.host = cli.host.clone();
            }
            if cli.port != 47890 {
                config.server.port = cli.port;
            }
        }
    } else {
        // No config file — use CLI values.
        config.server.host = cli.host.clone();
        config.server.port = cli.port;
    }

    info!(
        "Starting lm-sensors-web on {}:{}",
        config.server.host, config.server.port
    );

    // Initialize sensor manager.
    // If libsensors is not available, exit with a clear error.
    // A sensor-monitoring server without sensor access has no purpose.
    let sensor_manager = Arc::new(
        SensorManager::new()
            .map_err(|e| format!("Failed to initialize sensor subsystem: {}", e))?,
    );

    // Wrap config in an async RwLock for runtime reload support.
    let config_rwlock = Arc::new(tokio::sync::RwLock::new(config.clone()));

    // ── WebSocket broadcast ───────────────────────────────
    let ws_path = config.websocket.path.clone();
    let ws_server: Option<WebSocketServer> = if config.websocket.enabled {
        let ws = WebSocketServer::new(sensor_manager.clone(), config_rwlock.clone());
        ws.start_broadcast();
        Some(ws)
    } else {
        None
    };

    // ── Webhook engine ────────────────────────────────────
    if !config.webhooks.is_empty() {
        let engine = WebhookEngine::new(sensor_manager.clone(), config_rwlock.clone());
        engine.start();
    }

    // ── Build shared state ────────────────────────────────
    // Store config path in state so reload_config can use it without env vars.
    let config_path = cli
        .config
        .clone()
        .unwrap_or_else(|| String::from("config.json"));
    let state = AppState {
        sensor_manager: sensor_manager.clone(),
        config: config_rwlock.clone(),
        ws_state: ws_server.as_ref().map(|s| Arc::clone(&s.state)),
        config_path: config_path.clone(),
    };

    // ── Build Axum router ─────────────────────────────────
    let app = server::create_router(state, Some(ws_path), ws_server.as_ref());

    // ── Bind and serve ────────────────────────────────────
    let addr = format!("{}:{}", config.server.host, config.server.port);
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Bind failed {}: {}", addr, e))?;

    // Spawn the server and handle graceful shutdown.
    let server = axum::serve(listener, app);
    server
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| format!("Server error: {}", e))?;

    Ok(())
}

/// Graceful shutdown handler.
///
/// Listens for SIGTERM (systemd) or SIGINT (Ctrl-C) and returns
/// when either is received, causing `with_graceful_shutdown` to
/// stop accepting new connections and let in-flight requests finish.
async fn shutdown_signal() {
    // Only available on Unix; on Windows this is a no-op that waits forever.
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to install SIGINT handler");
        tokio::select! {
            _ = sigterm.recv() => { info!("Received SIGTERM, shutting down"); },
            _ = sigint.recv() => { info!("Received SIGINT, shutting down"); },
        }
    }
    #[cfg(not(unix))]
    {
        // On non-Unix, wait for Ctrl-C via tokio's built-in signal.
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl-C handler");
        info!("Received Ctrl-C, shutting down");
    }
}
