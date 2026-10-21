use std::path::PathBuf;
use std::process::Command;
use anyhow::{Context, Result};
use crate::ipc::IpcClient;

/// Checks whether Hyprdrop is configured to autostart on system boot / login.
pub fn is_autostart_enabled() -> bool {
    // 1. Check systemd user service status
    if let Ok(output) = Command::new("systemctl")
        .args(["--user", "is-enabled", "hyprdrop.service"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();
        if trimmed == "enabled" {
            return true;
        } else if trimmed == "disabled" {
            return false;
        }
    }

    // 2. Check XDG autostart desktop file
    if let Some(config_dir) = dirs::config_dir() {
        let autostart_desktop = config_dir.join("autostart").join("hyprdrop.desktop");
        if autostart_desktop.exists()
            && let Ok(content) = std::fs::read_to_string(&autostart_desktop)
        {
            return !content.contains("X-GNOME-Autostart-enabled=false");
        }
    }

    // 3. Fallback to persisted config preference
    crate::config::ConfigManager::load_config().autostart
}

/// Enables or disables autostart for the Hyprdrop daemon and app on system boot.
pub fn set_autostart(enable: bool) -> Result<()> {
    let config_dir = dirs::config_dir().context("Cannot locate user config directory")?;

    if enable {
        ensure_systemd_service_file(&config_dir)?;

        // Enable systemd user service
        let _ = Command::new("systemctl")
            .args(["--user", "enable", "hyprdrop.service"])
            .output();

        // Also create ~/.config/autostart/hyprdrop.desktop for non-systemd/XDG desktop environments
        let autostart_dir = config_dir.join("autostart");
        let _ = std::fs::create_dir_all(&autostart_dir);
        let autostart_file = autostart_dir.join("hyprdrop.desktop");
        let content = r#"[Desktop Entry]
Type=Application
Name=Hyprdrop Daemon
Comment=Hyprdrop background daemon
Exec=hyprdrop --daemon
Terminal=false
StartupNotify=false
Categories=Utility;
X-GNOME-Autostart-enabled=true
"#;
        let _ = std::fs::write(&autostart_file, content);
    } else {
        // Disable systemd user service
        let _ = Command::new("systemctl")
            .args(["--user", "disable", "hyprdrop.service"])
            .output();

        // Remove systemd wants symlink if lingering
        let wants_file = config_dir
            .join("systemd")
            .join("user")
            .join("graphical-session.target.wants")
            .join("hyprdrop.service");
        if wants_file.exists() {
            let _ = std::fs::remove_file(&wants_file);
        }

        // Remove ~/.config/autostart/hyprdrop.desktop
        let autostart_file = config_dir.join("autostart").join("hyprdrop.desktop");
        if autostart_file.exists() {
            let _ = std::fs::remove_file(&autostart_file);
        }
    }

    Ok(())
}

/// Ensures the user systemd service file exists at ~/.config/systemd/user/hyprdrop.service.
pub fn ensure_systemd_service_file(config_dir: &std::path::Path) -> Result<PathBuf> {
    let service_dir = config_dir.join("systemd").join("user");
    let service_file = service_dir.join("hyprdrop.service");

    if !service_file.exists() {
        std::fs::create_dir_all(&service_dir)?;
        let exe = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("/home/ubaid/.local/bin/hyprdrop"));
        let content = format!(
            r#"[Unit]
Description=Hyprdrop Background Daemon for Hyprland
Documentation=https://github.com/hyprland-community/hyprdrop
PartOf=graphical-session.target
After=graphical-session.target

[Service]
Type=simple
ExecStart={} --daemon
Restart=on-failure
RestartSec=2s
Slice=app.slice

[Install]
WantedBy=graphical-session.target
"#,
            exe.display()
        );
        std::fs::write(&service_file, content)?;
        let _ = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .output();
    }

    Ok(service_file)
}

/// Checks whether the background daemon is currently active and responding to IPC pings.
pub fn is_daemon_running() -> bool {
    IpcClient::ping().unwrap_or(false)
}

/// Ensures the background daemon is running.
/// If not running, launches it via systemd user service or spawns detached process.
/// Waits briefly until the daemon IPC socket is responsive.
pub fn ensure_daemon_running() -> bool {
    if is_daemon_running() {
        return true;
    }

    if let Some(config_dir) = dirs::config_dir() {
        let _ = ensure_systemd_service_file(&config_dir);
    }

    // 1. Try starting via systemctl user service
    let systemctl_started = Command::new("systemctl")
        .args(["--user", "start", "hyprdrop.service"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if systemctl_started {
        // Wait up to 3 seconds for systemd service to start up and bind IPC socket
        for _ in 0..60 {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if is_daemon_running() {
                return true;
            }
        }
    }

    // 2. Fallback only if systemd failed: spawn detached background process
    if !is_daemon_running() {
        let exe = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("/home/ubaid/.local/bin/hyprdrop"));

        let _ = Command::new(&exe)
            .arg("--daemon")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();

        for _ in 0..40 {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if is_daemon_running() {
                return true;
            }
        }
    }

    is_daemon_running()
}
