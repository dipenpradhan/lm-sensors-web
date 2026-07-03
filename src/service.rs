use std::fs;
use std::path::PathBuf;
use tracing::warn;

const SVC_NAME: &str = "lm-sensors-api";

pub struct ServiceManager;

impl ServiceManager {
    pub fn install(binary: &str, config: &str, user: bool) -> Result<(), String> {
        let unit = Self::unit_file(binary, config);
        let dir = if user {
            dirs::config_dir()
                .map(|d| d.join("systemd/user"))
                .ok_or_else(|| String::from("Cannot determine user config directory"))?
        } else {
            PathBuf::from("/etc/systemd")
        };

        let path = dir.join(format!("{}.service", SVC_NAME));
        if let Some(p) = path.parent() {
            fs::create_dir_all(p).map_err(|e| format!("mkdir: {}", e))?;
        }
        fs::write(&path, &unit).map_err(|e| format!("write: {}", e))?;

        Self::run_systemctl(user, "daemon-reload");
        tracing::info!("Installed: {}", path.display());
        Ok(())
    }

    pub fn uninstall(user: bool) -> Result<(), String> {
        Self::run_systemctl(user, "stop");
        Self::run_systemctl(user, "disable");

        let dir = if user {
            dirs::config_dir()
                .map(|d| d.join("systemd/user"))
                .ok_or_else(|| String::from("Cannot determine user config directory"))?
        } else {
            PathBuf::from("/etc/systemd")
        };
        let path = dir.join(format!("{}.service", SVC_NAME));
        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("unlink: {}", e))?;
        }
        Self::run_systemctl(user, "daemon-reload");
        tracing::info!("Uninstalled service");
        Ok(())
    }

    pub fn control(action: &str, user: bool) -> Result<(), String> {
        Self::run_systemctl(user, action);
        Ok(())
    }

    pub fn status(user: bool) -> Result<(), String> {
        Self::run_systemctl(user, "status");
        Ok(())
    }

    fn run_systemctl(user: bool, action: &str) {
        let flag = if user { "--user" } else { "" };
        let _ = std::process::Command::new("systemctl")
            .args(&[flag, action, SVC_NAME])
            .output()
            .map_err(|e| warn!("systemctl {}: {}", action, e));
    }

    pub fn unit_file(binary: &str, config: &str) -> String {
        format!(
            r#"[Unit]
Description=LM Sensors API Service
After=network.target

[Service]
Type=simple
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
            binary, config
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_file() {
        let u = ServiceManager::unit_file("/usr/bin/lm-sensors-api", "/etc/lm-sensors-api/config.json");
        assert!(u.contains("[Unit]"));
        assert!(u.contains("[Service]"));
        assert!(u.contains("[Install]"));
        assert!(u.contains("Type=simple"));
        assert!(u.contains("Restart=on-failure"));
        assert!(u.contains("WantedBy=multi-user.target"));
    }
}
