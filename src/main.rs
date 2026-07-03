mod api;
mod cli;
mod config;
mod sensors;
mod server;
mod service;
mod state;
mod websocket;
mod webhook;

use std::sync::Arc;

use clap::Parser;
use cli::{Cli, Command};
use config::Config;
use sensors::SensorManager;
use service::ServiceManager;
use state::AppState;
use websocket::WebSocketServer;
use webhook::WebhookEngine;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();

    // Setup logging
    let log_level = match cli.log_level {
        cli::LogLevel::Trace => "trace",
        cli::LogLevel::Debug => "debug",
        cli::LogLevel::Info => "info",
        cli::LogLevel::Warn => "warn",
        cli::LogLevel::Error => "error",
    };
    tracing_subscriber::fmt()
        .with_max_level(log_level.parse().unwrap_or(tracing::level_filters::LevelFilter::INFO))
        .init();

    // Load config
    let config = match &cli.config {
        Some(path) => {
            unsafe { std::env::set_var("CONFIG_PATH", path.as_str()); }
            Config::load(std::path::Path::new(path))
                .unwrap_or_else(|e| {
                    info!("Config load failed, using defaults: {}", e);
                    Config::default()
                })
        }
        None => Config::default(),
    };

    // Handle service subcommands (no server needed)
    if let Some(cmd) = &cli.command {
        match cmd {
            Command::Start { .. } => {
                // Fall through to server start
            }
            Command::InstallService { user, binary, config: cfg } => {
                let binary = binary.clone()
                    .or_else(|| std::env::current_exe().ok().map(|p| p.display().to_string()))
                    .unwrap_or_else(|| "/usr/local/bin/lm-sensors-api".into());
                return ServiceManager::install(&binary, cfg, *user);
            }
            Command::UninstallService { user } => return ServiceManager::uninstall(*user),
            Command::StartService { user } => return ServiceManager::control("start", *user),
            Command::StopService { user } => return ServiceManager::control("stop", *user),
            Command::RestartService { user } => return ServiceManager::control("restart", *user),
            Command::StatusService => return ServiceManager::status(false),
        }
    }

    // Start server
    run_server(cli, config).await
}

async fn run_server(cli: Cli, config: Config) -> Result<(), String> {
    // Apply CLI overrides
    let mut config = config;
    config.server.host = cli.host.clone();
    config.server.port = cli.port;

    info!("Starting lm-sensors-api on {}:{}", config.server.host, config.server.port);

    // Initialize sensor manager
    let sensor_manager = Arc::new(
        SensorManager::new().unwrap_or_else(|e| {
            info!("Sensor init failed: {} (server will continue with empty sensor list)", e);
            SensorManager::new().unwrap_or_else(|_| unreachable!())
        }),
    );

    let config_rwlock = Arc::new(tokio::sync::RwLock::new(config.clone()));

    // WebSocket
    let ws_path = config.websocket.path.clone();
    let mut ws_server: Option<WebSocketServer> = None;
    if config.websocket.enabled {
        let ws = WebSocketServer::new(sensor_manager.clone(), config_rwlock.clone());
        ws.start_broadcast();
        ws_server = Some(ws);
    }

    // Webhooks
    if !config.webhooks.is_empty() {
        let engine = WebhookEngine::new(sensor_manager.clone(), config_rwlock.clone());
        engine.start();
    }

    // Build state
    let state = AppState {
        sensor_manager: sensor_manager.clone(),
        config: config_rwlock.clone(),
        ws_state: ws_server.as_ref().map(|s| Arc::clone(&s.state)),
    };

    // Build router
    let app = server::create_router(state, Some(ws_path), ws_server.as_ref());

    let addr = format!("{}:{}", config.server.host, config.server.port);
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Bind failed {}: {}", addr, e))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {}", e))?;

    Ok(())
}
