use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
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

    pub fn load_config() -> AppConfig {
        let path = Self::config_path();
        if let Ok(content) = fs::read_to_string(&path)
            && let Ok(cfg) = serde_json::from_str(&content) {
                return cfg;
            }
        AppConfig::default()
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
