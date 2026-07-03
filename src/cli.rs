use clap::{Parser, Subcommand, ValueEnum};

/// Hardware sensor monitoring REST API with CLI
#[derive(Parser, Debug)]
#[command(name = "lm-sensors-api", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Override bind address (default 0.0.0.0)
    #[arg(short = 'H', long, default_value = "0.0.0.0")]
    pub host: String,

    /// Override listen port (default 47890)
    #[arg(short = 'p', long, default_value = "47890")]
    pub port: u16,

    /// Logging level
    #[arg(short, long, default_value = "info")]
    pub log_level: LogLevel,

    /// Path to JSON config file
    #[arg(short, long)]
    pub config: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Start the HTTP server
    Start {
        #[arg(short = 'H', long)]
        host: Option<String>,
        #[arg(short = 'p', long)]
        port: Option<u16>,
    },

    /// Install as a systemd service
    InstallService {
        /// Install as user service
        #[arg(long)]
        user: bool,
        /// Path to the binary
        #[arg(long)]
        binary: Option<String>,
        /// Path to config file
        #[arg(long, default_value = "/etc/lm-sensors-api/config.json")]
        config: String,
    },

    /// Uninstall the systemd service
    UninstallService {
        #[arg(long)]
        user: bool,
    },

    /// Start the service
    StartService {
        #[arg(long)]
        user: bool,
    },

    /// Stop the service
    StopService {
        #[arg(long)]
        user: bool,
    },

    /// Restart the service
    RestartService {
        #[arg(long)]
        user: bool,
    },

    /// Show service status
    StatusService,
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

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
    use clap::CommandFactory;

    #[test]
    fn test_cli_no_args() {
        let cli = Cli::parse_from(["lm-sensors-api"]);
        assert!(cli.command.is_none());
        assert_eq!(cli.host, "0.0.0.0");
        assert_eq!(cli.port, 47890);
    }

    #[test]
    fn test_cli_host_port() {
        let cli = Cli::parse_from(["lm-sensors-api", "-H", "127.0.0.1", "-p", "9090"]);
        assert_eq!(cli.host, "127.0.0.1");
        assert_eq!(cli.port, 9090);
    }

    #[test]
    fn test_cli_log_level() {
        let cli = Cli::parse_from(["lm-sensors-api", "--log-level", "debug"]);
        assert_eq!(cli.log_level.to_string(), "debug");
    }

    #[test]
    fn test_cli_config() {
        let cli = Cli::parse_from(["lm-sensors-api", "-c", "/tmp/c.json"]);
        assert_eq!(cli.config.as_deref(), Some("/tmp/c.json"));
    }

    #[test]
    fn test_cli_install() {
        let cli = Cli::parse_from(["lm-sensors-api", "install-service"]);
        assert!(matches!(cli.command, Some(Command::InstallService { .. })));
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Trace.to_string(), "trace");
    }
}
