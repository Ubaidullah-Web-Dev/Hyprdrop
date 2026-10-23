use hyprdrop::engine::perimeter_math::{PerimeterMath, ScreenEdge};

#[test]
fn test_perimeter_total_length() {
    let math = PerimeterMath::new(1920.0, 1080.0);
    assert_eq!(math.total_perimeter(), 2.0 * (1920.0 + 1080.0));
    assert_eq!(math.total_perimeter(), 6000.0);
}

#[test]
fn test_pure_edge_classification() {
    let math = PerimeterMath::new(1920.0, 1080.0);

    // Center of Top edge (s = 960)
    let span = math.calculate_span(960.0, 200.0);
    assert_eq!(span.edge, ScreenEdge::Top);
    assert_eq!(span.is_corner(), false);

    // Center of Right edge (s = 1920 + 540 = 2460)
    let span = math.calculate_span(2460.0, 200.0);
    assert_eq!(span.edge, ScreenEdge::Right);
    assert_eq!(span.is_corner(), false);

    // Center of Bottom edge (s = 1920 + 1080 + 960 = 3960)
    let span = math.calculate_span(3960.0, 200.0);
    assert_eq!(span.edge, ScreenEdge::Bottom);
    assert_eq!(span.is_corner(), false);

    // Center of Left edge (s = 1920 + 1080 + 1920 + 540 = 5460)
    let span = math.calculate_span(5460.0, 200.0);
    assert_eq!(span.edge, ScreenEdge::Left);
    assert_eq!(span.is_corner(), false);
}

#[test]
fn test_corner_spanning_top_right_split() {
    let math = PerimeterMath::new(1920.0, 1080.0);
    // Top-Right corner is at s = 1920.0
    // If center is at s = 1900.0 with length 100.0:
    // Span goes from 1850.0 to 1950.0
    // On Top: 1920 - 1850 = 70.0 (70%)
    // On Right: 1950 - 1920 = 30.0 (30%)
    let span = math.calculate_span(1900.0, 100.0);
    match span.edge {
        ScreenEdge::CornerTopRight { len_top, len_right } => {
            assert!((len_top - 70.0).abs() < 1e-4, "Expected len_top ~ 70, got {}", len_top);
            assert!((len_right - 30.0).abs() < 1e-4, "Expected len_right ~ 30, got {}", len_right);
        }
        other => panic!("Expected CornerTopRight, got {:?}", other),
    }
}

#[test]
fn test_corner_spanning_bottom_right() {
    let math = PerimeterMath::new(1920.0, 1080.0);
    // Bottom-Right corner is at s = 1920 + 1080 = 3000.0
    let span = math.calculate_span(3000.0, 100.0);
    match span.edge {
        ScreenEdge::CornerBottomRight { len_right, len_bottom } => {
            assert!((len_right - 50.0).abs() < 1e-4);
            assert!((len_bottom - 50.0).abs() < 1e-4);
        }
        other => panic!("Expected CornerBottomRight, got {:?}", other),
    }
}

#[test]
fn test_normalized_position_conversion() {
    let math = PerimeterMath::new(1920.0, 1080.0);
    // Top edge center: t = 960 / 6000 = 0.16
    assert!((math.t_to_s(0.16) - 960.0).abs() < 1e-4);
    assert!((math.s_to_t(960.0) - 0.16).abs() < 1e-4);
}

#[test]
fn test_dynamic_snap_points_1366x768() {
    let math = PerimeterMath::new(1366.0, 768.0);
    let p = math.total_perimeter();
    assert_eq!(p, 4268.0);

    // Top center: 1366 / 2 = 683
    let t_top = math.snap_top_edge();
    assert!((t_top - 683.0 / 4268.0).abs() < 1e-4);

    // Top-right corner: 1366
    let t_tr = math.snap_corner_top_right();
    assert!((t_tr - 1366.0 / 4268.0).abs() < 1e-4);

    // Right center: 1366 + 384 = 1750
    let t_right = math.snap_right_edge();
    assert!((t_right - 1750.0 / 4268.0).abs() < 1e-4);

    // Bottom-right corner: 1366 + 768 = 2134
    let t_br = math.snap_corner_bottom_right();
    assert!((t_br - 2134.0 / 4268.0).abs() < 1e-4);

    // Bottom center: 2134 + 683 = 2817
    let t_bot = math.snap_bottom_edge();
    assert!((t_bot - 2817.0 / 4268.0).abs() < 1e-4);

    // Bottom-left corner: 2134 + 1366 = 3500 -> ~0.820056 (NOT 0.750!)
    let t_bl = math.snap_corner_bottom_left();
    assert!((t_bl - 3500.0 / 4268.0).abs() < 1e-4);
    assert!((t_bl - 0.820056).abs() < 1e-4);

    // Left center: 3500 + 384 = 3884
    let t_left = math.snap_left_edge();
    assert!((t_left - 3884.0 / 4268.0).abs() < 1e-4);

    // Top-left corner: 0.0
    let t_tl = math.snap_corner_top_left();
    assert_eq!(t_tl, 0.0);
}
