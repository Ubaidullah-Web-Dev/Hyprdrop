use hyprdrop::config::AppConfig;
use hyprdrop::engine::perimeter_math::PerimeterMath;

#[test]
fn test_hotspot_default_config_fields() {
    let cfg = AppConfig::default();
    assert_eq!(cfg.hotspot_enabled, true);
    assert_eq!(cfg.hotspot_locked, false);
    assert_eq!(cfg.hotspot_length, 220);
    assert_eq!(cfg.hotspot_thickness, 8);
    assert_eq!(cfg.hotspot_margin, 0);
    assert_eq!(cfg.hotspot_corner_radius, 6);
    assert_eq!(cfg.hotspot_auto_hide, false);
    assert_eq!(cfg.hotspot_color, "#161618");
    assert_eq!(cfg.hotspot_hover_color, "#0A84FF");
    assert_eq!(cfg.hotspot_accent_color, "#FF5722");
}

#[test]
fn test_hotspot_perimeter_position_clamping() {
    let math = PerimeterMath::new(1920.0, 1080.0);

    // Cursor at (1920, 500) -> on Right edge
    let s = math.cursor_to_perimeter_s(1920.0, 500.0);
    assert_eq!(s, 1920.0 + 500.0);

    // Cursor at (500, 0) -> on Top edge
    let s_top = math.cursor_to_perimeter_s(500.0, 0.0);
    assert_eq!(s_top, 500.0);

    // Cursor at (500, 1080) -> on Bottom edge
    let s_bottom = math.cursor_to_perimeter_s(500.0, 1080.0);
    assert_eq!(s_bottom, 1920.0 + 1080.0 + (1920.0 - 500.0));
}
