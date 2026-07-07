//! # CLI integration tests
//!
//! Tests CLI argument parsing, subcommand handling, and flag combinations.
//!
//! # Running
//!
//! ```bash
//! cargo test --test cli_integration
//! ```

use clap::Parser;
use lm_sensors_web::cli::{Cli, Command};

// ── Default Behavior Tests ──────────────────────────────────────

/// No arguments: defaults to server mode.
#[test]
fn test_no_args_defaults() {
    let cli = Cli::parse_from(["lm-sensors-web"]);
    assert!(cli.command.is_none());
    assert_eq!(cli.host, "0.0.0.0");
    assert_eq!(cli.port, 47890);
    assert!(cli.config.is_none());
}

/// --help flag returns error (not panic).
#[test]
fn test_help_flag() {
    let result = Cli::try_parse_from(["lm-sensors-web", "--help"]);
    assert!(result.is_err());
}

// ── Host/Port Tests ────────────────────────────────────────────

/// Host override with long flag.
#[test]
fn test_host_long_flag() {
    let cli = Cli::parse_from(["lm-sensors-web", "--host", "127.0.0.1"]);
    assert_eq!(cli.host, "127.0.0.1");
}

/// Host override with short flag.
#[test]
fn test_host_short_flag() {
    let cli = Cli::parse_from(["lm-sensors-web", "-H", "10.0.0.1"]);
    assert_eq!(cli.host, "10.0.0.1");
}

/// Port override with long flag.
#[test]
fn test_port_long_flag() {
    let cli = Cli::parse_from(["lm-sensors-web", "--port", "8080"]);
    assert_eq!(cli.port, 8080);
}

/// Port override with short flag.
#[test]
fn test_port_short_flag() {
    let cli = Cli::parse_from(["lm-sensors-web", "-p", "3000"]);
    assert_eq!(cli.port, 3000);
}

/// Both host and port overrides.
#[test]
fn test_host_and_port() {
    let cli = Cli::parse_from(["lm-sensors-web", "-H", "192.168.1.1", "-p", "9090"]);
    assert_eq!(cli.host, "192.168.1.1");
    assert_eq!(cli.port, 9090);
}

// ── Log Level Tests ────────────────────────────────────────────

/// Log level with long flag.
#[test]
fn test_log_level_long_flag() {
    let cli = Cli::parse_from(["lm-sensors-web", "--log-level", "debug"]);
    assert_eq!(cli.log_level.to_string(), "debug");
}

/// Log level with short flag.
#[test]
fn test_log_level_short_flag() {
    let cli = Cli::parse_from(["lm-sensors-web", "-l", "trace"]);
    assert_eq!(cli.log_level.to_string(), "trace");
}

/// All log level variants.
#[test]
fn test_all_log_levels() {
    for level in ["error", "warn", "info", "debug", "trace"] {
        let cli = Cli::parse_from(["lm-sensors-web", "--log-level", level]);
        assert_eq!(cli.log_level.to_string(), level);
    }
}

/// Invalid log level should fail.
#[test]
fn test_invalid_log_level() {
    let result = Cli::try_parse_from(["lm-sensors-web", "--log-level", "verbose"]);
    assert!(result.is_err());
}

// ── Config Tests ──────────────────────────────────────────────

/// Config with long flag.
#[test]
fn test_config_long_flag() {
    let cli = Cli::parse_from(["lm-sensors-web", "--config", "/etc/app/config.json"]);
    assert_eq!(cli.config.as_deref(), Some("/etc/app/config.json"));
}

/// Config with short flag.
#[test]
fn test_config_short_flag() {
    let cli = Cli::parse_from(["lm-sensors-web", "-c", "~/config.json"]);
    assert_eq!(cli.config.as_deref(), Some("~/config.json"));
}

// ── Subcommand Tests ─────────────────────────────────────────

/// Start subcommand.
#[test]
fn test_start_subcommand() {
    let cli = Cli::parse_from(["lm-sensors-web", "start"]);
    assert!(matches!(cli.command, Some(Command::Start { .. })));
}

/// Install service subcommand.
#[test]
fn test_install_service() {
    let cli = Cli::parse_from([
        "lm-sensors-web",
        "install-service",
        "--binary",
        "/usr/local/bin/lm-sensors-web",
        "--config",
        "/etc/lm-sensors-web/config.json",
    ]);
    assert!(matches!(cli.command, Some(Command::InstallService { .. })));
}

/// Install service with --user flag.
#[test]
fn test_install_service_user() {
    let cli = Cli::parse_from(["lm-sensors-web", "install-service", "--user"]);
    match cli.command {
        Some(Command::InstallService { user, .. }) => assert!(user),
        _ => panic!("Expected InstallService"),
    }
}

/// Uninstall service.
#[test]
fn test_uninstall_service() {
    let cli = Cli::parse_from(["lm-sensors-web", "uninstall-service"]);
    assert!(matches!(
        cli.command,
        Some(Command::UninstallService { .. })
    ));
}

/// Service control subcommands.
#[test]
fn test_service_control() {
    for cmd in ["start-service", "stop-service", "restart-service"] {
        let cli = Cli::parse_from(["lm-sensors-web", cmd]);
        assert!(
            matches!(
                cli.command,
                Some(Command::StartService { .. })
                    | Some(Command::StopService { .. })
                    | Some(Command::RestartService { .. })
            ),
            "Expected service control for '{}'",
            cmd
        );
    }
}

/// Status service.
#[test]
fn test_status_service() {
    let cli = Cli::parse_from(["lm-sensors-web", "status-service"]);
    assert!(matches!(cli.command, Some(Command::StatusService)));
}

// ── Combined Flag Tests ──────────────────────────────────────

/// All flags combined.
#[test]
fn test_all_flags_combined() {
    let cli = Cli::parse_from([
        "lm-sensors-web",
        "-H",
        "10.0.0.1",
        "-p",
        "8080",
        "-c",
        "/etc/app.json",
        "-l",
        "trace",
    ]);
    assert_eq!(cli.host, "10.0.0.1");
    assert_eq!(cli.port, 8080);
    assert_eq!(cli.config.as_deref(), Some("/etc/app.json"));
    assert_eq!(cli.log_level.to_string(), "trace");
}

/// Flags with subcommand.
#[test]
fn test_flags_with_subcommand() {
    let cli = Cli::parse_from([
        "lm-sensors-web",
        "-H",
        "127.0.0.1",
        "-p",
        "8080",
        "status-service",
    ]);
    assert_eq!(cli.host, "127.0.0.1");
    assert_eq!(cli.port, 8080);
    assert!(matches!(cli.command, Some(Command::StatusService)));
}

// ── Edge Cases ────────────────────────────────────────────────

/// Port 0 (random port).
#[test]
fn test_port_zero() {
    let cli = Cli::parse_from(["lm-sensors-web", "-p", "0"]);
    assert_eq!(cli.port, 0);
}

/// Port 65535 (max).
#[test]
fn test_port_max() {
    let cli = Cli::parse_from(["lm-sensors-web", "-p", "65535"]);
    assert_eq!(cli.port, 65535);
}

/// IPv6 address.
#[test]
fn test_ipv6_address() {
    let cli = Cli::parse_from(["lm-sensors-web", "-H", "::1"]);
    assert_eq!(cli.host, "::1");
}

/// Config path with spaces (should still work).
#[test]
fn test_config_path_with_spaces() {
    let cli = Cli::parse_from(["lm-sensors-web", "-c", "/path/with spaces/config.json"]);
    assert_eq!(cli.config.as_deref(), Some("/path/with spaces/config.json"));
}
