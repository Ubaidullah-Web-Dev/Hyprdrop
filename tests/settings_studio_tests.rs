use hyprdrop::config::AppConfig;
use hyprdrop::engine::perimeter_math::{PerimeterMath, PerimeterSpan, ScreenEdge};

#[test]
fn test_screen_edge_names() {
    assert_eq!(ScreenEdge::Top.name(), "Top Edge");
    assert_eq!(ScreenEdge::Right.name(), "Right Edge");
    assert_eq!(ScreenEdge::Bottom.name(), "Bottom Edge");
    assert_eq!(ScreenEdge::Left.name(), "Left Edge");
    assert_eq!(
        ScreenEdge::CornerTopRight { len_top: 50.0, len_right: 50.0 }.name(),
        "Top-Right Corner"
    );
    assert_eq!(
        ScreenEdge::CornerBottomRight { len_right: 50.0, len_bottom: 50.0 }.name(),
        "Bottom-Right Corner"
    );
    assert_eq!(
        ScreenEdge::CornerBottomLeft { len_bottom: 50.0, len_left: 50.0 }.name(),
        "Bottom-Left Corner"
    );
    assert_eq!(
        ScreenEdge::CornerTopLeft { len_left: 50.0, len_top: 50.0 }.name(),
        "Top-Left Corner"
    );
}

#[test]
fn test_perimeter_edge_center_coord() {
    let math = PerimeterMath::new(1920.0, 1080.0);

    // Top Edge: s = 960.0 -> center_coord = 960.0
    let span_top = PerimeterSpan {
        edge: ScreenEdge::Top,
        center_s: 960.0,
        length: 200.0,
        start_s: 860.0,
        end_s: 1060.0,
    };
    assert!((math.edge_center_coord(&span_top) - 960.0).abs() < 1e-4);

    // Right Edge: s = 1920.0 + 540.0 = 2460.0 -> center_coord = 540.0
    let span_right = PerimeterSpan {
        edge: ScreenEdge::Right,
        center_s: 2460.0,
        length: 200.0,
        start_s: 2360.0,
        end_s: 2560.0,
    };
    assert!((math.edge_center_coord(&span_right) - 540.0).abs() < 1e-4);
}

#[test]
fn test_config_hotspot_defaults_and_mutation() {
    let mut cfg = AppConfig::default();

    assert!(cfg.hotspot_enabled);
    assert!(!cfg.hotspot_locked);
    assert_eq!(cfg.hotspot_perimeter_pos, 0.41); // Right edge center on 16:9 monitor
    assert_eq!(cfg.hotspot_length, 220);
    assert_eq!(cfg.hotspot_thickness, 8);
    assert_eq!(cfg.hotspot_margin, 0);
    assert_eq!(cfg.hotspot_corner_radius, 6);
    assert!(!cfg.hotspot_auto_hide);

    // Mutate parameters
    cfg.hotspot_locked = true;
    cfg.hotspot_perimeter_pos = 0.25; // Top-Right corner
    cfg.hotspot_length = 260;
    cfg.hotspot_thickness = 12;
    cfg.hotspot_margin = 8;
    cfg.hotspot_corner_radius = 8;
    cfg.hotspot_auto_hide = true;
    cfg.hotspot_color = "#BB9AF7".to_string(); // Tokyo Night
    cfg.hotspot_hover_color = "#C0CAF5".to_string();
    cfg.hotspot_accent_color = "#7AA2F7".to_string();

    assert!(cfg.hotspot_locked);
    assert_eq!(cfg.hotspot_perimeter_pos, 0.25);
    assert_eq!(cfg.hotspot_length, 260);
    assert_eq!(cfg.hotspot_thickness, 12);
    assert_eq!(cfg.hotspot_margin, 8);
    assert_eq!(cfg.hotspot_corner_radius, 8);
    assert!(cfg.hotspot_auto_hide);
    assert_eq!(cfg.hotspot_color, "#BB9AF7");
}

#[test]
fn test_hex_color_validation() {
    let swatches = [
        "#00D2FF",
        "#38BDF8",
        "#0284C7",
        "#BB9AF7",
        "#C0CAF5",
        "#7AA2F7",
        "#89B4FA",
        "#CBA6F7",
        "#F5C2E7",
        "#10B981",
        "#34D399",
        "#059669",
        "#F59E0B",
        "#FBBF24",
        "#EA580C",
        "#E2E8F0",
        "#FFFFFF",
        "#64748B",
    ];

    for hex in swatches {
        assert!(hex.starts_with('#'));
        assert_eq!(hex.len(), 7);
        let num = u32::from_str_radix(&hex[1..], 16);
        assert!(num.is_ok(), "Invalid hex: {}", hex);
    }
}
