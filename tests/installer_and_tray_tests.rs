use std::fs;
use tempfile::tempdir;
use hyprdrop::hyprland::installer::{
    ConfigFormat, CONF_CONFIG_SNIPPET, LUA_CONFIG_SNIPPET,
    configure_hyprland_in, find_hyprland_config_in,
};
use hyprdrop::ui::tray::generate_icon_pixmap;

#[test]
fn test_detect_lua_config() {
    let dir = tempdir().expect("Failed to create tempdir");
    let lua_file = dir.path().join("hyprland.lua");
    fs::write(&lua_file, "-- Hyprland Lua Config\nrequire('source.input')\n").unwrap();

    let (found_path, format) = find_hyprland_config_in(dir.path()).expect("Should find config");
    assert_eq!(found_path, lua_file);
    assert_eq!(format, ConfigFormat::Lua);
    assert_eq!(format.file_name(), "hyprland.lua");
    assert_eq!(format.display_name(), "Lua");
}

#[test]
fn test_detect_conf_config() {
    let dir = tempdir().expect("Failed to create tempdir");
    let conf_file = dir.path().join("hyprland.conf");
    fs::write(&conf_file, "# Hyprland Classic Config\nmonitor=,preferred,auto,1\n").unwrap();

    let (found_path, format) = find_hyprland_config_in(dir.path()).expect("Should find config");
    assert_eq!(found_path, conf_file);
    assert_eq!(format, ConfigFormat::Conf);
    assert_eq!(format.file_name(), "hyprland.conf");
    assert_eq!(format.display_name(), "Classic .conf");
}

#[test]
fn test_fallback_to_lua_on_empty_dir() {
    let dir = tempdir().expect("Failed to create tempdir");
    let (found_path, format) = find_hyprland_config_in(dir.path()).expect("Should default");
    assert_eq!(found_path, dir.path().join("hyprland.lua"));
    assert_eq!(format, ConfigFormat::Lua);
}

#[test]
fn test_configure_lua_hyprland_creates_backup_and_snippet() {
    let dir = tempdir().expect("Failed to create tempdir");
    let lua_file = dir.path().join("hyprland.lua");
    let original_content = "-- Original Hyprland Lua Config\nrequire('edit_here.hyprland')\n";
    fs::write(&lua_file, original_content).unwrap();

    let result = configure_hyprland_in(dir.path()).expect("configure should succeed");
    assert!(!result.already_configured);
    assert_eq!(result.format, ConfigFormat::Lua);
    assert_eq!(result.config_path, lua_file);

    // Backup check
    let backup_path = result.backup_path.expect("Backup should be created");
    assert!(backup_path.exists());
    let backup_content = fs::read_to_string(&backup_path).unwrap();
    assert_eq!(backup_content, original_content);

    // Content check
    let updated_content = fs::read_to_string(&lua_file).unwrap();
    assert!(updated_content.starts_with(original_content));
    assert!(updated_content.contains("hl.layer_rule"));
    assert!(updated_content.contains("namespace = \"hyprdrop\""));
    assert!(updated_content.contains("SHIFT + mouse:272"));
    assert!(updated_content.contains("hyprdrop trigger"));

    // Verify ONLY mouse bind is configured, NO keyboard bind
    assert!(!updated_content.contains("SUPER"));
    assert!(!updated_content.contains("SUPER, D"));

    // Second run should be idempotent
    let second_run = configure_hyprland_in(dir.path()).expect("second run should succeed");
    assert!(second_run.already_configured);
    assert!(second_run.backup_path.is_none());
}

#[test]
fn test_configure_conf_hyprland_creates_backup_and_snippet() {
    let dir = tempdir().expect("Failed to create tempdir");
    let conf_file = dir.path().join("hyprland.conf");
    let original_content = "# Original Hyprland Config\nexec-once = waybar\n";
    fs::write(&conf_file, original_content).unwrap();

    let result = configure_hyprland_in(dir.path()).expect("configure should succeed");
    assert!(!result.already_configured);
    assert_eq!(result.format, ConfigFormat::Conf);
    assert_eq!(result.config_path, conf_file);

    // Backup check
    let backup_path = result.backup_path.expect("Backup should be created");
    assert!(backup_path.exists());
    let backup_content = fs::read_to_string(&backup_path).unwrap();
    assert_eq!(backup_content, original_content);

    // Content check
    let updated_content = fs::read_to_string(&conf_file).unwrap();
    assert!(updated_content.starts_with(original_content));
    assert!(updated_content.contains("layerrule = blur, hyprdrop"));
    assert!(updated_content.contains("layerrule = ignorezero, hyprdrop"));
    assert!(updated_content.contains("bind = SHIFT, mouse:272, exec, hyprdrop trigger"));

    // Verify ONLY mouse bind is configured, NO keyboard bind
    assert!(!updated_content.contains("SUPER"));
    assert!(!updated_content.contains("SUPER, D"));

    // Second run should be idempotent
    let second_run = configure_hyprland_in(dir.path()).expect("second run should succeed");
    assert!(second_run.already_configured);
    assert!(second_run.backup_path.is_none());
}

#[test]
fn test_snippets_validity() {
    // Both snippets must have window rules, layer rules, and mouse bind
    assert!(LUA_CONFIG_SNIPPET.contains("hl.window_rule"));
    assert!(LUA_CONFIG_SNIPPET.contains("float = true"));
    assert!(LUA_CONFIG_SNIPPET.contains("hl.layer_rule"));
    assert!(LUA_CONFIG_SNIPPET.contains("namespace = \"hyprdrop\""));
    assert!(LUA_CONFIG_SNIPPET.contains("SHIFT + mouse:272"));
    assert!(!LUA_CONFIG_SNIPPET.contains("SUPER"));

    assert!(CONF_CONFIG_SNIPPET.contains("windowrulev2 = float"));
    assert!(CONF_CONFIG_SNIPPET.contains("layerrule = blur, hyprdrop"));
    assert!(CONF_CONFIG_SNIPPET.contains("bind = SHIFT, mouse:272, exec, hyprdrop trigger"));
    assert!(!CONF_CONFIG_SNIPPET.contains("SUPER"));
}

#[test]
fn test_tray_icon_pixmap_generation() {
    let (w, h, data) = generate_icon_pixmap(32);
    assert_eq!(w, 32);
    assert_eq!(h, 32);
    assert_eq!(data.len(), 32 * 32 * 4);

    // Center pixel at (15, 15) should be Studio Graphite #161618
    let center_idx = (15 * 32 + 15) * 4;
    assert_eq!(data[center_idx], 255); // Alpha
    assert_eq!(data[center_idx + 1], 22); // R
    assert_eq!(data[center_idx + 2], 22); // G
    assert_eq!(data[center_idx + 3], 24); // B

    // Outer pixel at (25, 15) should be Modern Electric Blue #0A84FF (distance ~9.5 from center 15.5)
    let ring_idx = (15 * 32 + 25) * 4;
    assert_eq!(data[ring_idx], 255); // Alpha
    assert_eq!(data[ring_idx + 1], 10); // R
    assert_eq!(data[ring_idx + 2], 132); // G
    assert_eq!(data[ring_idx + 3], 255); // B

    // Corner pixel at (0, 0) should be transparent
    assert_eq!(data[0], 0); // Alpha 0
}

#[test]
fn test_hotspot_state() {
    // Before spawn, should not be active
    assert!(!hyprdrop::ui::hotspot::HotspotWindow::is_active());
}

#[test]
fn test_is_hyprland_configured_hermetic() {
    let dir = tempdir().expect("Failed to create tempdir");
    // Initially false on empty directory
    assert!(!hyprdrop::hyprland::installer::is_hyprland_configured_in(dir.path()));

    // False with unconfigured lua file
    let lua_file = dir.path().join("hyprland.lua");
    fs::write(&lua_file, "-- Empty lua\n").unwrap();
    assert!(!hyprdrop::hyprland::installer::is_hyprland_configured_in(dir.path()));

    // True after configure_hyprland_in
    let _ = configure_hyprland_in(dir.path()).unwrap();
    assert!(hyprdrop::hyprland::installer::is_hyprland_configured_in(dir.path()));
}

#[test]
fn test_is_hyprland_configured_in_modular_subpath() {
    let dir = tempdir().expect("Failed to create tempdir");
    let keybinds_dir = dir.path().join("edit_here/source");
    fs::create_dir_all(&keybinds_dir).unwrap();
    fs::write(
        keybinds_dir.join("keybinds.lua"),
        "hl.bind('SHIFT + mouse:272', hl.dsp.exec_cmd('hyprdrop trigger'))",
    )
    .unwrap();

    assert!(hyprdrop::hyprland::installer::is_hyprland_configured_in(dir.path()));
}

#[test]
fn test_ensure_systemd_service_file_generation() {
    let dir = tempdir().expect("Failed to create tempdir");
    let service_file = hyprdrop::hyprland::autostart::ensure_systemd_service_file(dir.path())
        .expect("Should generate service file");

    assert!(service_file.exists());
    let content = fs::read_to_string(&service_file).expect("Should read service file");
    assert!(content.contains("[Unit]"));
    assert!(content.contains("PartOf=graphical-session.target"));
    assert!(content.contains("ExecStart="));
    assert!(content.contains("--daemon"));
    assert!(content.contains("[Install]"));
    assert!(content.contains("WantedBy=graphical-session.target"));
}

#[test]
fn test_config_autostart_default() {
    let cfg = hyprdrop::config::AppConfig::default();
    assert!(cfg.autostart, "Autostart should default to true");
}



