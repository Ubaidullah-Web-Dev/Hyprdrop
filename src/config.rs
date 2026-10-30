use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

fn default_hotspot_length() -> u32 {
    220
}

fn default_hotspot_thickness() -> u32 {
    8
}

fn default_hotspot_corner_radius() -> u32 {
    6
}

fn default_hotspot_pos() -> f64 {
    0.41
}

fn default_hotspot_color() -> String {
    "#0A84FF".to_string()
}

fn default_hotspot_hover_color() -> String {
    "#38BDF8".to_string()
}

fn default_hotspot_accent_color() -> String {
    "#0062CC".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppTheme {
    pub name: String,
    pub primary: String,
    pub secondary: String,
    pub accent: String,
    pub surface: String,
    pub card_bg: String,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self {
            name: "Electric Blue (Default)".to_string(),
            primary: "#0A84FF".to_string(),
            secondary: "#38BDF8".to_string(),
            accent: "#0062CC".to_string(),
            surface: "#0D0E12".to_string(),
            card_bg: "#14161F".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub output_dir: Option<PathBuf>,
    pub naming_pattern: String,
    pub jpeg_quality: u8,
    pub wheel_size: u32,
    pub show_drop_shelf: bool,
    pub drop_shelf_timeout_secs: u32,
    #[serde(default = "default_true")]
    pub autostart: bool,
    #[serde(default = "default_true")]
    pub hotspot_enabled: bool,
    #[serde(default)]
    pub hotspot_locked: bool,
    #[serde(default = "default_hotspot_pos")]
    pub hotspot_perimeter_pos: f64,
    #[serde(default = "default_hotspot_length")]
    pub hotspot_length: u32,
    #[serde(default = "default_hotspot_thickness")]
    pub hotspot_thickness: u32,
    #[serde(default)]
    pub hotspot_margin: i32,
    #[serde(default = "default_hotspot_corner_radius")]
    pub hotspot_corner_radius: u32,
    #[serde(default)]
    pub hotspot_auto_hide: bool,
    #[serde(default = "default_hotspot_color")]
    pub hotspot_color: String,
    #[serde(default = "default_hotspot_hover_color")]
    pub hotspot_hover_color: String,
    #[serde(default = "default_hotspot_accent_color")]
    pub hotspot_accent_color: String,
    #[serde(default)]
    pub allow_trigger_when_closed: bool,
    #[serde(default)]
    pub theme: AppTheme,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            output_dir: None,
            naming_pattern: "{stem} ({counter}).{ext}".to_string(),
            jpeg_quality: 82,
            wheel_size: 380,
            show_drop_shelf: true,
            drop_shelf_timeout_secs: 15,
            autostart: true,
            hotspot_enabled: true,
            hotspot_locked: false,
            hotspot_perimeter_pos: 0.41,
            hotspot_length: 220,
            hotspot_thickness: 8,
            hotspot_margin: 0,
            hotspot_corner_radius: 6,
            hotspot_auto_hide: false,
            hotspot_color: "#0A84FF".to_string(),
            hotspot_hover_color: "#38BDF8".to_string(),
            hotspot_accent_color: "#0062CC".to_string(),
            allow_trigger_when_closed: false,
            theme: AppTheme::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryRecord {
    pub id: String,
    pub timestamp: String,
    pub action: String,
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub original_size_bytes: u64,
    pub output_size_bytes: u64,
    pub savings_pct: Option<f64>,
}

pub struct ConfigManager;

impl ConfigManager {
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("hyprdrop")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    pub fn history_path() -> PathBuf {
        Self::config_dir().join("history.json")
    }

    pub fn socket_path() -> PathBuf {
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            PathBuf::from(runtime_dir).join("hyprdrop.sock")
        } else {
            std::env::temp_dir().join(format!("hyprdrop-{}.sock", users_uid()))
        }
    }

    pub fn matugen_theme_path() -> PathBuf {
        Self::config_dir().join("matugen.json")
    }

    pub fn load_matugen_theme() -> Option<AppTheme> {
        // 1. Try ~/.config/hyprdrop/matugen.json
        let p = Self::matugen_theme_path();
        if p.exists() {
            if let Ok(content) = fs::read_to_string(&p) {
                if let Ok(theme) = serde_json::from_str::<AppTheme>(&content) {
                    return Some(theme);
                }
            }
        }

        // 2. Try ~/.config/hyprdrop/theme.json
        let alt = Self::config_dir().join("theme.json");
        if alt.exists() {
            if let Ok(content) = fs::read_to_string(&alt) {
                if let Ok(theme) = serde_json::from_str::<AppTheme>(&content) {
                    return Some(theme);
                }
            }
        }

        // 3. Fallback: Parse from system Matugen generated palettes if present
        if let Some(home) = dirs::home_dir() {
            let qs_colors = home.join(".local/state/quickshell/user/generated/colors.json");
            if qs_colors.exists() {
                if let Ok(content) = fs::read_to_string(&qs_colors) {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                        let primary = v.get("primary").and_then(|s| s.as_str()).unwrap_or("#FFB2BC");
                        let secondary = v.get("secondary").and_then(|s| s.as_str()).unwrap_or("#E5BDC1");
                        let accent = v.get("tertiary").and_then(|s| s.as_str()).unwrap_or("#EABF8F");
                        let surface = v.get("surface").or_else(|| v.get("background")).and_then(|s| s.as_str()).unwrap_or("#1A1112");
                        let card_bg = v.get("surface_container").or_else(|| v.get("surface_variant")).and_then(|s| s.as_str()).unwrap_or("#241B1C");
                        return Some(AppTheme {
                            name: "Matugen (Material You)".to_string(),
                            primary: primary.to_string(),
                            secondary: secondary.to_string(),
                            accent: accent.to_string(),
                            surface: surface.to_string(),
                            card_bg: card_bg.to_string(),
                        });
                    }
                }
            }

            let dusky_tui = home.join(".config/matugen/generated/dusky_tui.json");
            if dusky_tui.exists() {
                if let Ok(content) = fs::read_to_string(&dusky_tui) {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                        let primary = v.get("accent").and_then(|s| s.as_str()).unwrap_or("#FFB2BC");
                        let secondary = v.get("warning").and_then(|s| s.as_str()).unwrap_or("#E5BDC1");
                        let accent = v.get("success").and_then(|s| s.as_str()).unwrap_or("#EABF8F");
                        let surface = v.get("bg").and_then(|s| s.as_str()).unwrap_or("#1A1112");
                        return Some(AppTheme {
                            name: "Matugen (Material You)".to_string(),
                            primary: primary.to_string(),
                            secondary: secondary.to_string(),
                            accent: accent.to_string(),
                            surface: surface.to_string(),
                            card_bg: "#241B1C".to_string(),
                        });
                    }
                }
            }
        }

        None
    }

    pub fn load_config() -> AppConfig {
        let path = Self::config_path();
        let mut cfg = if let Ok(content) = fs::read_to_string(&path)
            && let Ok(cfg) = serde_json::from_str::<AppConfig>(&content) {
                cfg
            } else {
                AppConfig::default()
            };

        // If theme is set to Matugen, dynamically refresh colors from matugen.json if available
        if cfg.theme.name.to_lowercase().contains("matugen") {
            if let Some(m_theme) = Self::load_matugen_theme() {
                cfg.theme = m_theme;
            }
        }
        cfg
    }

    pub fn save_config(cfg: &AppConfig) -> Result<()> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir)?;
        let json = serde_json::to_string_pretty(cfg)?;
        fs::write(Self::config_path(), json)?;
        Ok(())
    }

    pub fn load_history() -> Vec<HistoryRecord> {
        let path = Self::history_path();
        if let Ok(content) = fs::read_to_string(&path)
            && let Ok(records) = serde_json::from_str(&content) {
                return records;
            }
        Vec::new()
    }

    pub fn append_history(
        action: &str,
        input_path: &Path,
        output_path: &Path,
        savings_pct: Option<f64>,
    ) -> Result<()> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir)?;

        let mut history = Self::load_history();
        let original_size = fs::metadata(input_path).map(|m| m.len()).unwrap_or(0);
        let output_size = fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);

        let record = HistoryRecord {
            id: format!("{}-{}", Utc::now().timestamp_millis(), history.len()),
            timestamp: Utc::now().to_rfc3339(),
            action: action.to_string(),
            input_path: input_path.to_path_buf(),
            output_path: output_path.to_path_buf(),
            original_size_bytes: original_size,
            output_size_bytes: output_size,
            savings_pct,
        };

        history.insert(0, record);
        if history.len() > 100 {
            history.truncate(100);
        }

        let json = serde_json::to_string_pretty(&history)?;
        fs::write(Self::history_path(), json)?;
        Ok(())
    }

    pub fn clear_history() -> Result<()> {
        let path = Self::history_path();
        if path.exists() {
            fs::write(&path, "[]")?;
        }
        Ok(())
    }
}

fn users_uid() -> u32 {
    unsafe { libc::getuid() }
}
