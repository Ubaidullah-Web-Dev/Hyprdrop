use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Context, Result, anyhow};
use chrono::Utc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Lua,
    Conf,
}

impl ConfigFormat {
    pub fn file_name(&self) -> &'static str {
        match self {
            ConfigFormat::Lua => "hyprland.lua",
            ConfigFormat::Conf => "hyprland.conf",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ConfigFormat::Lua => "Lua",
            ConfigFormat::Conf => "Classic .conf",
        }
    }
}

pub struct InstallResult {
    pub config_path: PathBuf,
    pub format: ConfigFormat,
    pub backup_path: Option<PathBuf>,
    pub already_configured: bool,
    pub reloaded: bool,
}

#[derive(Debug, Clone)]
pub struct UninstallResult {
    pub config_path: PathBuf,
    pub backup_path: Option<PathBuf>,
    pub was_configured: bool,
    pub reloaded: bool,
}

pub const LUA_CONFIG_SNIPPET: &str = r#"
-- --- Hyprdrop Radial Utility Configuration ---
hl.window_rule({
    name = "hyprdrop_float_class",
    match = { class = ".*hyprdrop.*" },
    float = true,
    center = true,
    size = "660 660",
})

hl.window_rule({
    name = "hyprdrop_float_title",
    match = { title = ".*Hyprdrop.*" },
    float = true,
    center = true,
    size = "660 660",
})

hl.layer_rule({
    name = "hyprdrop_hotspot",
    match = { namespace = "hyprdrop-hotspot" },
    blur = false,
    ignore_alpha = 0.0,
    animation = "none",
})

hl.layer_rule({
    name = "hyprdrop",
    match = { namespace = "hyprdrop" },
    blur = true,
    ignore_alpha = 0.1,
    animation = "popin 80%",
})

hl.bind(
    "SHIFT + mouse:272",
    hl.dsp.exec_cmd("hyprdrop trigger"),
    { description = "Hyprdrop Radial Trigger", mouse = true }
)
"#;

pub const CONF_CONFIG_SNIPPET: &str = r#"
# --- Hyprdrop Radial Utility Configuration ---
windowrulev2 = float, class:^(.*hyprdrop.*)$
windowrulev2 = center, class:^(.*hyprdrop.*)$
windowrulev2 = size 660 660, class:^(.*hyprdrop.*)$
windowrulev2 = float, title:^(.*Hyprdrop.*)$
windowrulev2 = center, title:^(.*Hyprdrop.*)$
windowrulev2 = size 660 660, title:^(.*Hyprdrop.*)$
layerrule = blur, hyprdrop
layerrule = ignorezero, hyprdrop
layerrule = animation popin 80%, hyprdrop
bind = SHIFT, mouse:272, exec, hyprdrop trigger
"#;

/// Locates the Hyprland configuration file in a specified directory, detecting Lua or Conf format.
pub fn find_hyprland_config_in(hypr_dir: &Path) -> Result<(PathBuf, ConfigFormat)> {
    let lua_path = hypr_dir.join("hyprland.lua");
    let conf_path = hypr_dir.join("hyprland.conf");

    // Prefer hyprland.lua (Hyprland's updated syntax) if present, then hyprland.conf
    if lua_path.exists() {
        Ok((lua_path, ConfigFormat::Lua))
    } else if conf_path.exists() {
        Ok((conf_path, ConfigFormat::Conf))
    } else if hypr_dir.exists() {
        // Default to hyprland.lua for modern Hyprland installations
        Ok((lua_path, ConfigFormat::Lua))
    } else {
        Err(anyhow!(
            "Hyprland configuration directory not found at {}",
            hypr_dir.display()
        ))
    }
}

/// Locates the user's active Hyprland configuration file in ~/.config/hypr/.
pub fn find_hyprland_config() -> Result<(PathBuf, ConfigFormat)> {
    let hypr_dir = dirs::config_dir()
        .ok_or_else(|| anyhow!("Cannot locate user config directory"))?
        .join("hypr");
    find_hyprland_config_in(&hypr_dir)
}

/// Returns configuration status, path, and format for the settings UI.
pub fn get_hyprland_config_info() -> (bool, PathBuf, ConfigFormat) {
    let (path, format) = find_hyprland_config().unwrap_or_else(|_| {
        let p = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("hypr")
            .join("hyprland.lua");
        (p, ConfigFormat::Lua)
    });
    let configured = is_hyprland_configured();
    (configured, path, format)
}

/// Safely configures Hyprland for Hyprdrop in a specified directory.
pub fn configure_hyprland_in(hypr_dir: &Path) -> Result<InstallResult> {
    if !hypr_dir.exists() {
        fs::create_dir_all(hypr_dir)
            .with_context(|| format!("Failed to create directory {}", hypr_dir.display()))?;
    }

    let (config_path, format) = find_hyprland_config_in(hypr_dir)?;

    let content = if config_path.exists() {
        fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read {}", config_path.display()))?
    } else {
        String::new()
    };

    // Check if hyprdrop is already configured
    if content.contains("hyprdrop") {
        return Ok(InstallResult {
            config_path,
            format,
            backup_path: None,
            already_configured: true,
            reloaded: false,
        });
    }

    // Create timestamped backup if file exists and has content
    let backup_path = if config_path.exists() && !content.trim().is_empty() {
        let file_name = config_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("hyprland.config");
        let backup_name = format!("{}.bak.{}", file_name, Utc::now().format("%Y%m%d%H%M%S"));
        let bpath = hypr_dir.join(&backup_name);
        fs::copy(&config_path, &bpath)
            .with_context(|| format!("Failed to create backup at {}", bpath.display()))?;
        Some(bpath)
    } else {
        None
    };

    // Choose snippet matching syntax
    let snippet = match format {
        ConfigFormat::Lua => LUA_CONFIG_SNIPPET,
        ConfigFormat::Conf => CONF_CONFIG_SNIPPET,
    };

    let mut new_content = content;
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    new_content.push_str(snippet);

    fs::write(&config_path, new_content)
        .with_context(|| format!("Failed to write to {}", config_path.display()))?;

    // Reload Hyprland config if hyprctl is available
    let reloaded = Command::new("hyprctl")
        .arg("reload")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    Ok(InstallResult {
        config_path,
        format,
        backup_path,
        already_configured: false,
        reloaded,
    })
}

/// Safely configures Hyprland for Hyprdrop by creating a backup and appending rules.
pub fn configure_hyprland() -> Result<InstallResult> {
    let hypr_dir = dirs::config_dir()
        .ok_or_else(|| anyhow!("Cannot locate user config directory"))?
        .join("hypr");

    configure_hyprland_in(&hypr_dir)
}

/// Checks whether hyprdrop rules are configured in a specified Hyprland configuration directory.
pub fn is_hyprland_configured_in(hypr_dir: &Path) -> bool {
    let Ok((config_path, _)) = find_hyprland_config_in(hypr_dir) else {
        return false;
    };

    if let Ok(content) = fs::read_to_string(&config_path)
        && content.contains("hyprdrop")
    {
        return true;
    }

    // Also check modular include/overlay files in hypr_dir if present
    for sub in &[
        "edit_here/source/keybinds.lua",
        "edit_here/source/window_rules.lua",
        "source/keybinds.lua",
        "source/window_rules.lua",
    ] {
        let p = hypr_dir.join(sub);
        if let Ok(content) = fs::read_to_string(p)
            && content.contains("hyprdrop")
        {
            return true;
        }
    }

    false
}

/// Checks whether hyprdrop rules are configured in the user's active Hyprland config.
pub fn is_hyprland_configured() -> bool {
    let Some(hypr_dir) = dirs::config_dir().map(|d| d.join("hypr")) else {
        return false;
    };
    is_hyprland_configured_in(&hypr_dir)
}

/// Safely removes Hyprdrop configuration blocks and individual rules from a config string.
pub fn strip_hyprdrop_rules(content: &str) -> String {
    let mut result = Vec::new();
    let mut in_hyprdrop_block = false;
    let mut paren_depth = 0i32;
    let mut brace_depth = 0i32;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.contains("--- Hyprdrop Radial Utility Configuration ---") {
            in_hyprdrop_block = true;
            paren_depth = 0;
            brace_depth = 0;
            continue;
        }

        if in_hyprdrop_block {
            for c in trimmed.chars() {
                match c {
                    '(' => paren_depth += 1,
                    ')' => paren_depth = (paren_depth - 1).max(0),
                    '{' => brace_depth += 1,
                    '}' => brace_depth = (brace_depth - 1).max(0),
                    _ => {}
                }
            }

            // Check if another user section begins or if block has naturally closed
            if paren_depth == 0 && brace_depth == 0 {
                if (trimmed.starts_with("# ---") || trimmed.starts_with("-- ---"))
                    && !trimmed.contains("Hyprdrop")
                {
                    in_hyprdrop_block = false;
                    result.push(line);
                    continue;
                }
                if !trimmed.is_empty()
                    && !trimmed.contains("hyprdrop")
                    && !trimmed.starts_with(')')
                    && !trimmed.starts_with('}')
                    && !trimmed.starts_with("hl.")
                    && !trimmed.starts_with("windowrule")
                    && !trimmed.starts_with("layerrule")
                    && !trimmed.starts_with("bind")
                {
                    in_hyprdrop_block = false;
                    result.push(line);
                    continue;
                }
            }
            continue;
        }

        // Catch any stray hyprdrop rules outside the official block
        if trimmed.contains("hyprdrop") {
            continue;
        }

        result.push(line);
    }

    // Trim trailing empty lines nicely
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

/// Safely unconfigures Hyprland by removing Hyprdrop rules and restoring a clean configuration.
pub fn unconfigure_hyprland_in(hypr_dir: &Path) -> Result<UninstallResult> {
    let Ok((config_path, _)) = find_hyprland_config_in(hypr_dir) else {
        return Ok(UninstallResult {
            config_path: hypr_dir.join("hyprland.conf"),
            backup_path: None,
            was_configured: false,
            reloaded: false,
        });
    };

    if !config_path.exists() {
        return Ok(UninstallResult {
            config_path,
            backup_path: None,
            was_configured: false,
            reloaded: false,
        });
    }

    let content = fs::read_to_string(&config_path)?;
    if !content.contains("hyprdrop") {
        return Ok(UninstallResult {
            config_path,
            backup_path: None,
            was_configured: false,
            reloaded: false,
        });
    }

    // Create timestamped backup before modifying user configuration
    let file_name = config_path.file_name().and_then(|s| s.to_str()).unwrap_or("hyprland.config");
    let backup_name = format!("{}.bak.{}", file_name, Utc::now().format("%Y%m%d%H%M%S"));
    let bpath = hypr_dir.join(&backup_name);
    fs::copy(&config_path, &bpath)
        .with_context(|| format!("Failed to create backup at {}", bpath.display()))?;

    let cleaned = strip_hyprdrop_rules(&content);
    fs::write(&config_path, cleaned)?;

    // Also check modular include/overlay files in hypr_dir if present
    for sub in &[
        "edit_here/source/keybinds.lua",
        "edit_here/source/window_rules.lua",
        "source/keybinds.lua",
        "source/window_rules.lua",
    ] {
        let p = hypr_dir.join(sub);
        if p.exists() {
            if let Ok(c) = fs::read_to_string(&p) {
                if c.contains("hyprdrop") {
                    let sub_backup = p.with_extension(format!("lua.bak.{}", Utc::now().format("%Y%m%d%H%M%S")));
                    let _ = fs::copy(&p, sub_backup);
                    let sub_cleaned = strip_hyprdrop_rules(&c);
                    let _ = fs::write(&p, sub_cleaned);
                }
            }
        }
    }

    let reloaded = Command::new("hyprctl")
        .arg("reload")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    Ok(UninstallResult {
        config_path,
        backup_path: Some(bpath),
        was_configured: true,
        reloaded,
    })
}

/// Safely unconfigures Hyprland for Hyprdrop in the user's active configuration directory.
pub fn unconfigure_hyprland() -> Result<UninstallResult> {
    let hypr_dir = dirs::config_dir()
        .ok_or_else(|| anyhow!("Cannot locate user config directory"))?
        .join("hypr");

    unconfigure_hyprland_in(&hypr_dir)
}
