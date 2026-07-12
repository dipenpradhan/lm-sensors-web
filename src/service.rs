//! # Systemd service management module
//!
//! Provides utilities for installing, managing, and removing a
//! systemd service unit file for lm-sensors-web.
//!
//! # Operations
//!
//! | Method        | Action                                           |
//! |--------------|--------------------------------------------------|
//! | `install`    | Write unit file + `daemon-reload`                |
//! | `uninstall`  | Stop, disable, remove unit file + `daemon-reload`|
//! | `control`    | Delegate to `systemctl <action>` (start/stop/etc) |
//! | `status`     | Show service status via `systemctl status`        |
//!
//! # Unit file
//!
//! The generated unit file is a standard systemd `[Service]` unit with:
//! - `Restart=on-failure` for auto-recovery
//! - Journal logging for standard output/error
//! - Configurable binary path and config file via environment variables

use std::fs;
use std::path::PathBuf;
use tracing::warn;

/// Service name used in systemd unit file names.
const SVC_NAME: &str = "lm-sensors-web";

/// Systemd service manager.
///
/// Provides static methods for service lifecycle management.
/// All methods delegate to `systemctl` for the actual operations.
pub struct ServiceManager;

impl ServiceManager {
    /// Install the service by writing a unit file and reloading systemd.
    ///
    /// # Arguments
    /// * `binary` — absolute path to the compiled binary
    /// * `config` — absolute path to the config JSON file
    /// * `user` — `true` for user-level service, `false` for system-wide
    pub fn install(binary: &str, config: &str, user: bool) -> Result<(), String> {
        // Generate the systemd unit file content.
        let unit = Self::unit_file(binary, config);

        // Determine the target directory based on user vs system service.
        let dir = if user {
            dirs::config_dir()
                .map(|d| d.join("systemd/user"))
                .ok_or_else(|| String::from("Cannot determine user config directory"))?
        } else {
            PathBuf::from("/etc/systemd/system")
        };

        // Write the unit file.
        let path = dir.join(format!("{}.service", SVC_NAME));
        if let Some(p) = path.parent() {
            fs::create_dir_all(p).map_err(|e| format!("mkdir: {}", e))?;
        }
        fs::write(&path, &unit).map_err(|e| format!("write: {}", e))?;

        // Reload systemd to pick up the new unit file.
        Self::run_systemctl(user, "daemon-reload");
        tracing::info!("Installed: {}", path.display());
        Ok(())
    }

    /// Uninstall the service: stop, disable, remove the unit file.
    pub fn uninstall(user: bool) -> Result<(), String> {
        // Stop and disable the service before removing the file.
        Self::run_systemctl(user, "stop");
        Self::run_systemctl(user, "disable");

        // Determine and remove the unit file.
        let dir = if user {
            dirs::config_dir()
                .map(|d| d.join("systemd/user"))
                .ok_or_else(|| String::from("Cannot determine user config directory"))?
        } else {
            PathBuf::from("/etc/systemd/system")
        };
        let path = dir.join(format!("{}.service", SVC_NAME));
        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("unlink: {}", e))?;
        }
        Self::run_systemctl(user, "daemon-reload");
        tracing::info!("Uninstalled service");
        Ok(())
    }

    /// Control the service: start, stop, or restart via systemctl.
    pub fn control(action: &str, user: bool) -> Result<(), String> {
        Self::run_systemctl(user, action);
        Ok(())
    }

    /// Show the current service status.
    pub fn status(user: bool) -> Result<(), String> {
        Self::run_systemctl(user, "status");
        Ok(())
    }

    /// Run a `systemctl` command.
    ///
    /// Adds the `--user` flag for user-level services.
    /// Silently logs errors (systemctl failures are non-fatal).
    fn run_systemctl(user: bool, action: &str) {
        let mut args = Vec::new();
        if user {
            args.push("--user");
        }
        args.push(action);
        args.push(SVC_NAME);
        let _ = std::process::Command::new("systemctl")
            .args(&args)
            .output()
            .map_err(|e| warn!("systemctl {}: {}", action, e));
    }

    /// Generate a systemd unit file string.
    ///
    /// Produces a standard `[Unit] / [Service] / [Install]` unit file
    /// with auto-restart, journal logging, and environment variables.
    pub fn unit_file(binary: &str, config: &str) -> String {
        // Derive the working directory from the binary path so
        // relative paths (e.g. static/) resolve correctly.
        let working_dir = if let Some(parent) = std::path::Path::new(binary).parent() {
            parent.display().to_string()
        } else {
            "/".to_string()
        };
        format!(
            r#"[Unit]
Description=LM Sensors Web API Service
After=network.target

[Service]
Type=simple
WorkingDirectory={}
ExecStart={}
Environment=RUST_LOG=info
Environment=CONFIG_PATH={}
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
"#,
            working_dir, binary, config
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify the generated unit file contains all required sections.
    #[test]
    fn test_unit_file() {
        let u =
            ServiceManager::unit_file("/usr/bin/lm-sensors-web", "/etc/lm-sensors-web/config.json");
        assert!(u.contains("[Unit]"));
        assert!(u.contains("[Service]"));
        assert!(u.contains("[Install]"));
        assert!(u.contains("Type=simple"));
        assert!(u.contains("Restart=on-failure"));
        assert!(u.contains("WantedBy=multi-user.target"));
        assert!(u.contains("WorkingDirectory=/usr/bin"));
    }

    /// Verify the unit file contains the correct binary and config paths.
    #[test]
    fn test_unit_file_paths() {
        let u = ServiceManager::unit_file("/opt/my-bin", "/etc/my-config.json");
        assert!(u.contains("ExecStart=/opt/my-bin"));
        assert!(u.contains("CONFIG_PATH=/etc/my-config.json"));
        assert!(u.contains("WorkingDirectory=/opt"));
    }
}
