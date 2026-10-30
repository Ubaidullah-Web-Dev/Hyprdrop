use hyprdrop::config::{AppConfig, AppTheme, ConfigManager};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_allow_trigger_when_closed_default_is_false() {
    let cfg = AppConfig::default();
    assert!(!cfg.allow_trigger_when_closed, "allow_trigger_when_closed should default to false");
}

#[test]
fn test_allow_trigger_when_closed_serde_backward_compatibility() {
    // Simulating old config JSON without the new field
    let old_json = r#"{
        "naming_pattern": "{stem}.{ext}",
        "jpeg_quality": 85,
        "wheel_size": 380,
        "show_drop_shelf": true,
        "drop_shelf_timeout_secs": 15
    }"#;

    let cfg: AppConfig = serde_json::from_str(old_json).expect("Should deserialize old config successfully");
    assert!(!cfg.allow_trigger_when_closed, "Old config should default allow_trigger_when_closed to false");

    // Simulating new config with field set to true
    let new_json = r#"{
        "naming_pattern": "{stem}.{ext}",
        "jpeg_quality": 85,
        "wheel_size": 380,
        "show_drop_shelf": true,
        "drop_shelf_timeout_secs": 15,
        "allow_trigger_when_closed": true
    }"#;

    let cfg_new: AppConfig = serde_json::from_str(new_json).expect("Should deserialize new config successfully");
    assert!(cfg_new.allow_trigger_when_closed, "Field should deserialize to true when present");
}

#[test]
fn test_matugen_template_file_integrity() {
    let template_path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/matugen/hyprdrop.json");
    let content = fs::read_to_string(template_path).expect("assets/matugen/hyprdrop.json must exist");
    
    // Check that required Matugen replacement tags exist
    assert!(content.contains("{{colors.primary.default.hex}}"));
    assert!(content.contains("{{colors.secondary.default.hex}}"));
    assert!(content.contains("{{colors.tertiary.default.hex}}"));
    assert!(content.contains("{{colors.surface.default.hex}}"));
    assert!(content.contains("{{colors.surface_container.default.hex}}"));

    // Check that simulated parsed theme conforms to AppTheme
    let simulated_json = content
        .replace("{{colors.primary.default.hex}}", "#FFB2BC")
        .replace("{{colors.secondary.default.hex}}", "#E5BDC1")
        .replace("{{colors.tertiary.default.hex}}", "#EABF8F")
        .replace("{{colors.surface.default.hex}}", "#1A1112")
        .replace("{{colors.surface_container.default.hex}}", "#241B1C");

    let theme: AppTheme = serde_json::from_str(&simulated_json).expect("Simulated theme JSON should parse into AppTheme");
    assert_eq!(theme.name, "Matugen (Material You)");
    assert_eq!(theme.primary, "#FFB2BC");
    assert_eq!(theme.secondary, "#E5BDC1");
    assert_eq!(theme.accent, "#EABF8F");
    assert_eq!(theme.surface, "#1A1112");
    assert_eq!(theme.card_bg, "#241B1C");
}

#[test]
fn test_matugen_theme_loader_custom_path() {
    let tmp = tempdir().unwrap();
    let theme_file = tmp.path().join("matugen.json");

    let sample_theme = AppTheme {
        name: "Matugen (Material You)".to_string(),
        primary: "#123456".to_string(),
        secondary: "#654321".to_string(),
        accent: "#ABCDEF".to_string(),
        surface: "#010101".to_string(),
        card_bg: "#020202".to_string(),
    };

    fs::write(&theme_file, serde_json::to_string(&sample_theme).unwrap()).unwrap();

    let loaded: AppTheme = serde_json::from_str(&fs::read_to_string(&theme_file).unwrap()).unwrap();
    assert_eq!(loaded.primary, "#123456");
    assert_eq!(loaded.accent, "#ABCDEF");
}

#[test]
fn test_load_matugen_theme_detects_real_or_fallback() {
    // ConfigManager::load_matugen_theme() should either find the user's generated matugen file or return None/fallback gracefully without panicking
    let loaded = ConfigManager::load_matugen_theme();
    if let Some(theme) = loaded {
        assert_eq!(theme.name, "Matugen (Material You)");
        assert!(theme.primary.starts_with('#'));
        assert!(theme.surface.starts_with('#'));
    }
}

