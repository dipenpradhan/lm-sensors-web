//! # Command-line interface
//!
//! Uses `clap` derive macros for argument parsing. Supports two modes:
//!
//! 1. **Server mode** (default): parses `-H`/`-p`/`-c`/`-l` flags and starts the HTTP server.
//! 2. **Service mode**: delegates to subcommands (`install-service`, `start-service`, etc.)
//!    which manage systemd unit files without starting the server.

use clap::{Parser, Subcommand, ValueEnum};

/// Top-level CLI parser.
///
/// When no subcommand is given, the server starts. Subcommands like
/// `install-service` perform service management and exit early.
#[derive(Parser, Debug)]
#[command(
    name = "lm-sensors-web",
    version,
    about = "Hardware sensor monitoring REST API with CLI"
)]
pub struct Cli {
    /// Subcommand (service management). `None` = start the server.
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Bind address override (default: 127.0.0.1 — localhost only).
    ///
    /// Use `0.0.0.0` to expose on all interfaces (requires caution).
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    pub host: String,

    /// Listen port override (default: 47890 — outside well-known range).
    #[arg(short = 'p', long, default_value = "47890")]
    pub port: u16,

    /// Logging verbosity (trace is most detailed, error is least).
    #[arg(short, long, default_value = "info")]
    pub log_level: LogLevel,

    /// Path to JSON config file. Falls back to built-in defaults if missing.
    #[arg(short, long)]
    pub config: Option<String>,
}

/// Service-management subcommands. Each delegates to `ServiceManager`
/// for systemd unit file manipulation and `systemctl` calls.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Explicit start subcommand (same as no subcommand).
    Start {
        #[arg(short = 'H', long)]
        host: Option<String>,
        #[arg(short = 'p', long)]
        port: Option<u16>,
    },
    /// Write a systemd unit file and run `daemon-reload`.
    InstallService {
        /// User-level service (vs. system-wide).
        #[arg(long)]
        user: bool,
        /// Absolute path to the compiled binary.
        #[arg(long)]
        binary: Option<String>,
        /// Config file path baked into the unit file.
        #[arg(long, default_value = "/etc/lm-sensors-web/config.json")]
        config: String,
    },
    /// Stop, disable, and remove the systemd unit file.
    UninstallService {
        #[arg(long)]
        user: bool,
    },
    /// Start the service via `systemctl start`.
    StartService {
        #[arg(long)]
        user: bool,
    },
    /// Stop the service via `systemctl stop`.
    StopService {
        #[arg(long)]
        user: bool,
    },
    /// Restart the service via `systemctl restart`.
    RestartService {
        #[arg(long)]
        user: bool,
    },
    /// Show service status via `systemctl status`.
    StatusService,
}

/// Logging levels matching `tracing` levels.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

/// Map enum variants to lowercase strings for `tracing` configuration.
impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Error => write!(f, "error"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Trace => write!(f, "trace"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Default values when no arguments are passed.
    #[test]
    fn test_cli_no_args() {
        let cli = Cli::parse_from(["lm-sensors-web"]);
        assert!(cli.command.is_none());
        assert_eq!(cli.host, "127.0.0.1");
        assert_eq!(cli.port, 47890);
    }

    /// Host and port overrides.
    #[test]
    fn test_cli_host_port() {
        let cli = Cli::parse_from(["lm-sensors-web", "-H", "127.0.0.1", "-p", "9090"]);
        assert_eq!(cli.host, "127.0.0.1");
        assert_eq!(cli.port, 9090);
    }

    /// Log-level parsing and display.
    #[test]
    fn test_cli_log_level() {
        let cli = Cli::parse_from(["lm-sensors-web", "--log-level", "debug"]);
        assert_eq!(cli.log_level.to_string(), "debug");
    }

    /// Config path flag.
    #[test]
    fn test_cli_config() {
        let cli = Cli::parse_from(["lm-sensors-web", "-c", "/tmp/c.json"]);
        assert_eq!(cli.config.as_deref(), Some("/tmp/c.json"));
    }

    /// Install subcommand recognition.
    #[test]
    fn test_cli_install() {
        let cli = Cli::parse_from(["lm-sensors-web", "install-service"]);
        assert!(matches!(cli.command, Some(Command::InstallService { .. })));
    }

    /// LogLevel::Display round-trip.
    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Trace.to_string(), "trace");
    }

    /// Start subcommand parsing.
    #[test]
    fn test_cli_start_subcommand() {
        let cli = Cli::parse_from(["lm-sensors-web", "start"]);
        assert!(matches!(cli.command, Some(Command::Start { .. })));
    }

    /// Uninstall subcommand parsing.
    #[test]
    fn test_cli_uninstall() {
        let cli = Cli::parse_from(["lm-sensors-web", "uninstall-service"]);
        assert!(matches!(
            cli.command,
            Some(Command::UninstallService { .. })
        ));
    }

    /// Service control subcommands.
    #[test]
    fn test_cli_service_control() {
        for subcmd in ["start-service", "stop-service", "restart-service"] {
            let cli = Cli::parse_from(["lm-sensors-web", subcmd]);
            assert!(
                matches!(
                    cli.command,
                    Some(Command::StartService { .. })
                        | Some(Command::StopService { .. })
                        | Some(Command::RestartService { .. })
                ),
                "Expected service control command for '{}'",
                subcmd
            );
        }
    }

    /// Status service subcommand.
    #[test]
    fn test_cli_status_service() {
        let cli = Cli::parse_from(["lm-sensors-web", "status-service"]);
        assert!(matches!(cli.command, Some(Command::StatusService)));
    }

    /// LogLevel Default is Info.
    #[test]
    fn test_log_level_default() {
        let default = LogLevel::default();
        assert_eq!(default.to_string(), "info");
    }

    /// All LogLevel variants serialise to correct strings.
    #[test]
    fn test_all_log_levels_display() {
        assert_eq!(LogLevel::Error.to_string(), "error");
        assert_eq!(LogLevel::Warn.to_string(), "warn");
        assert_eq!(LogLevel::Info.to_string(), "info");
        assert_eq!(LogLevel::Debug.to_string(), "debug");
        assert_eq!(LogLevel::Trace.to_string(), "trace");
    }

    /// Install service with --user flag.
    #[test]
    fn test_cli_install_with_user() {
        let cli = Cli::parse_from(["lm-sensors-web", "install-service", "--user"]);
        match cli.command {
            Some(Command::InstallService { user, .. }) => assert!(user),
            _ => panic!("Expected InstallService"),
        }
    }

    /// Combined flags: host, port, config, log-level.
    #[test]
    fn test_cli_all_flags() {
        let cli = Cli::parse_from([
            "lm-sensors-web",
            "-H",
            "10.0.0.1",
            "-p",
            "8080",
            "-c",
            "/etc/app.json",
            "--log-level",
            "trace",
        ]);
        assert_eq!(cli.host, "10.0.0.1");
        assert_eq!(cli.port, 8080);
        assert_eq!(cli.config.as_deref(), Some("/etc/app.json"));
        assert_eq!(cli.log_level.to_string(), "trace");
    }
}
