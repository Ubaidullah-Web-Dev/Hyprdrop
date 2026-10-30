use std::fs;
use std::path::PathBuf;
use std::process::Command;
use anyhow::{Context, Result};
use chrono::Utc;

use crate::hyprland::installer::unconfigure_hyprland;

/// Detailed breakdown of uninstallation operations and backups created.
#[derive(Debug, Default, Clone)]
pub struct UninstallReport {
    pub hyprland_reverted: bool,
    pub hyprland_backup: Option<PathBuf>,
    pub matugen_reverted: bool,
    pub matugen_backup: Option<PathBuf>,
    pub service_stopped: bool,
    pub service_file_removed: bool,
    pub desktop_entry_removed: bool,
    pub icons_removed: bool,
    pub binaries_removed: bool,
    pub messages: Vec<String>,
}

/// Safely removes the [templates.hyprdrop] section from a Matugen config.toml string.
pub fn strip_matugen_hyprdrop_section(content: &str) -> String {
    let mut result: Vec<String> = Vec::new();
    let mut in_hyprdrop_section = false;

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Detect Hyprdrop header comment or table definition
        if trimmed.contains("Hyprdrop Radial Utility") || trimmed.starts_with("[templates.hyprdrop]") {
            in_hyprdrop_section = true;
            // Backtrack to remove any decorative comment banner lines preceding this section
            while let Some(prev) = result.last() {
                let prev_trimmed = prev.trim();
                if prev_trimmed.starts_with('#') || prev_trimmed.is_empty() {
                    result.pop();
                } else {
                    break;
                }
            }
            i += 1;
            continue;
        }

        if in_hyprdrop_section {
            // Check if next section begins
            if trimmed.starts_with('[') && !trimmed.starts_with("[templates.hyprdrop") {
                in_hyprdrop_section = false;
                result.push(line.to_string());
            }
            // Otherwise skip lines inside [templates.hyprdrop]
        } else {
            result.push(line.to_string());
        }
        i += 1;
    }

    // Clean up trailing empty lines
    while let Some(last) = result.last() {
        if last.trim().is_empty() {
            result.pop();
        } else {
            break;
        }
    }

    let mut out = result.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

/// Unregisters Matugen integration safely: removes template files and strips config.toml.
pub fn unconfigure_matugen() -> Result<Option<PathBuf>> {
    let Some(home) = dirs::home_dir() else {
        return Ok(None);
    };

    // 1. Remove ~/.config/matugen/templates/hyprdrop.json
    let matugen_template = home.join(".config/matugen/templates/hyprdrop.json");
    if matugen_template.exists() {
        let _ = fs::remove_file(&matugen_template);
    }

    // 2. Remove ~/.config/hyprdrop/matugen.json (generated palette)
    let generated_theme = home.join(".config/hyprdrop/matugen.json");
    if generated_theme.exists() {
        let _ = fs::remove_file(&generated_theme);
    }

    // 3. Revert ~/.config/matugen/config.toml
    let config_path = home.join(".config/matugen/config.toml");
    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if content.contains("[templates.hyprdrop]") {
                let backup_name = format!("config.toml.bak.{}", Utc::now().format("%Y%m%d%H%M%S"));
                let backup_path = home.join(".config/matugen").join(&backup_name);
                fs::copy(&config_path, &backup_path)
                    .with_context(|| format!("Failed to create Matugen backup at {}", backup_path.display()))?;

                let cleaned = strip_matugen_hyprdrop_section(&content);
                fs::write(&config_path, cleaned)?;
                return Ok(Some(backup_path));
            }
        }
    }

    Ok(None)
}

/// Completely and safely uninstalls Hyprdrop, reverting all integrations.
pub fn perform_uninstall(remove_binaries: bool) -> Result<UninstallReport> {
    let mut report = UninstallReport::default();
    let home = dirs::home_dir().context("Cannot determine home directory")?;

    // 1. Stop and disable systemd user daemon
    let _ = Command::new("systemctl")
        .args(["--user", "stop", "hyprdrop.service"])
        .output();
    let _ = Command::new("systemctl")
        .args(["--user", "disable", "hyprdrop.service"])
        .output();
    report.service_stopped = true;

    let service_file = home.join(".config/systemd/user/hyprdrop.service");
    if service_file.exists() {
        let _ = fs::remove_file(&service_file);
        report.service_file_removed = true;
        let _ = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .output();
        report.messages.push("Removed ~/.config/systemd/user/hyprdrop.service".to_string());
    }

    // 2. Revert Hyprland configuration (window rules, blur layer rules & Shift+Click keybind)
    match unconfigure_hyprland() {
        Ok(res) => {
            if res.was_configured {
                report.hyprland_reverted = true;
                report.hyprland_backup = res.backup_path.clone();
                if let Some(ref bp) = res.backup_path {
                    report.messages.push(format!("Reverted Hyprland rules (Backup: {})", bp.display()));
                } else {
                    report.messages.push("Reverted Hyprland rules successfully".to_string());
                }
            }
        }
        Err(e) => {
            report.messages.push(format!("Warning: Could not unconfigure Hyprland: {}", e));
        }
    }

    // 3. Revert Matugen template and configuration
    match unconfigure_matugen() {
        Ok(backup_opt) => {
            report.matugen_reverted = true;
            report.matugen_backup = backup_opt.clone();
            if let Some(ref bp) = backup_opt {
                report.messages.push(format!("Reverted Matugen config.toml (Backup: {})", bp.display()));
            } else {
                report.messages.push("Removed Matugen template and generated palettes".to_string());
            }
        }
        Err(e) => {
            report.messages.push(format!("Warning: Could not unconfigure Matugen: {}", e));
        }
    }

    // 4. Remove Desktop Entries
    let app_dir = home.join(".local/share/applications");
    let desktop_file = app_dir.join("hyprdrop.desktop");
    if desktop_file.exists() {
        let _ = fs::remove_file(&desktop_file);
        report.desktop_entry_removed = true;
        report.messages.push("Removed desktop entry ~/.local/share/applications/hyprdrop.desktop".to_string());
    }

    let _ = Command::new("update-desktop-database")
        .arg(&app_dir)
        .output();

    // 5. Remove Icons & Data Dir
    let icon_dir = home.join(".local/share/icons/hicolor");
    let svg_icon = icon_dir.join("scalable/apps/hyprdrop.svg");
    if svg_icon.exists() {
        let _ = fs::remove_file(&svg_icon);
        report.icons_removed = true;
    }

    for size in &[16, 24, 32, 48, 64, 128, 256, 512] {
        let png_icon = icon_dir.join(format!("{}x{}/apps/hyprdrop.png", size, size));
        if png_icon.exists() {
            let _ = fs::remove_file(&png_icon);
            report.icons_removed = true;
        }
    }

    let data_dir = home.join(".local/share/hyprdrop");
    if data_dir.exists() {
        let _ = fs::remove_dir_all(&data_dir);
        report.messages.push("Removed shared assets in ~/.local/share/hyprdrop".to_string());
    }

    let _ = Command::new("gtk-update-icon-cache")
        .args(["-q", "-t"])
        .arg(&icon_dir)
        .output();

    // 6. Optionally remove installed executables
    if remove_binaries {
        let bin_dir = home.join(".local/bin");
        let main_bin = bin_dir.join("hyprdrop");
        let helper_bin = bin_dir.join("hyprdrop-detect-selection");

        if helper_bin.exists() {
            let _ = fs::remove_file(&helper_bin);
        }
        if main_bin.exists() {
            let _ = fs::remove_file(&main_bin);
            report.binaries_removed = true;
            report.messages.push("Removed executables from ~/.local/bin".to_string());
        }
    }

    Ok(report)
}
