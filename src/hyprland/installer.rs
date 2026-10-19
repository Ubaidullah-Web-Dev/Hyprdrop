use std::fs;
use std::path::PathBuf;
use std::process::Command;
use anyhow::{Context, Result, anyhow};
use chrono::Utc;

pub struct InstallResult {
    pub config_path: PathBuf,
    pub backup_path: Option<PathBuf>,
    pub already_configured: bool,
    pub reloaded: bool,
}

/// Safely configures Hyprland for Hyprdrop by creating a backup and appending rules.
pub fn configure_hyprland() -> Result<InstallResult> {
    let hypr_dir = dirs::config_dir()
        .ok_or_else(|| anyhow!("Cannot locate user config directory"))?
        .join("hypr");

    let config_path = hypr_dir.join("hyprland.conf");
    if !config_path.exists() {
        return Err(anyhow!("Hyprland config not found at {}", config_path.display()));
    }

    let content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;

    // Check if hyprdrop is already configured
    if content.contains("hyprdrop") {
        return Ok(InstallResult {
            config_path,
            backup_path: None,
            already_configured: true,
            reloaded: false,
        });
    }

    // Create timestamped backup
    let backup_name = format!("hyprland.conf.bak.{}", Utc::now().format("%Y%m%d%H%M%S"));
    let backup_path = hypr_dir.join(&backup_name);
    fs::copy(&config_path, &backup_path)
        .with_context(|| format!("Failed to create backup at {}", backup_path.display()))?;

    // Append Hyprdrop configuration
    let rules_snippet = r#"
# --- Hyprdrop Radial Utility Configuration ---
layerrule = blur, hyprdrop
layerrule = ignorezero, hyprdrop
layerrule = animation popin 80%, hyprdrop
bind = SHIFT, mouse:272, exec, hyprdrop trigger
"#;

    let mut new_content = content;
    new_content.push_str(rules_snippet);
    fs::write(&config_path, new_content)
        .with_context(|| format!("Failed to update {}", config_path.display()))?;

    // Reload Hyprland config
    let reloaded = Command::new("hyprctl")
        .arg("reload")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    Ok(InstallResult {
        config_path,
        backup_path: Some(backup_path),
        already_configured: false,
        reloaded,
    })
}

/// Checks whether hyprdrop rules are configured in Hyprland config.
pub fn is_hyprland_configured() -> bool {
    let config_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hypr")
        .join("hyprland.conf");

    if let Ok(content) = fs::read_to_string(config_path) {
        content.contains("hyprdrop")
    } else {
        false
    }
}
