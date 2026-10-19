use std::f64::consts::PI;
use hyprdrop::ui::radial_menu::{HitZone, INNER_RADIUS, OUTER_RADIUS, hit_test};

#[test]
fn test_center_hub_hit() {
    let cx = 190.0;
    let cy = 190.0;
    let slice_count = 8;

    // Exact center
    assert_eq!(
        hit_test(cx, cy, cx, cy, INNER_RADIUS, OUTER_RADIUS, slice_count),
        HitZone::CenterHub
    );

    // Just inside inner radius
    assert_eq!(
        hit_test(cx + INNER_RADIUS - 1.0, cy, cx, cy, INNER_RADIUS, OUTER_RADIUS, slice_count),
        HitZone::CenterHub
    );

    assert_eq!(
        hit_test(cx, cy - INNER_RADIUS + 1.0, cx, cy, INNER_RADIUS, OUTER_RADIUS, slice_count),
        HitZone::CenterHub
    );
}

#[test]
fn test_outside_hit() {
    let cx = 190.0;
    let cy = 190.0;
    let slice_count = 8;

    // Far away
    assert_eq!(
        hit_test(0.0, 0.0, cx, cy, INNER_RADIUS, OUTER_RADIUS, slice_count),
        HitZone::Outside
    );

    // Just outside outer radius
    assert_eq!(
        hit_test(cx + OUTER_RADIUS + 2.0, cy, cx, cy, INNER_RADIUS, OUTER_RADIUS, slice_count),
        HitZone::Outside
    );
}

#[test]
fn test_4_slices_hit_testing() {
    let cx = 190.0;
    let cy = 190.0;
    let slice_count = 4;
    let mid_r = (INNER_RADIUS + OUTER_RADIUS) / 2.0;

    // Slice 0: Top (12 o'clock area, from -PI/2 to 0 -> in our top-centered layout)
    // angle_from_top = 0 to PI/2
    // Angle at 12:30 o'clock: angle_from_top = PI/4
    let x_0 = cx + mid_r * (PI / 4.0 - PI / 2.0).cos();
    let y_0 = cy + mid_r * (PI / 4.0 - PI / 2.0).sin();
    assert_eq!(
        hit_test(x_0, y_0, cx, cy, INNER_RADIUS, OUTER_RADIUS, slice_count),
        HitZone::Slice(0)
    );

    // Slice 1: Right quadrant (angle_from_top = 3*PI/4)
    let x_1 = cx + mid_r * (3.0 * PI / 4.0 - PI / 2.0).cos();
    let y_1 = cy + mid_r * (3.0 * PI / 4.0 - PI / 2.0).sin();
    assert_eq!(
        hit_test(x_1, y_1, cx, cy, INNER_RADIUS, OUTER_RADIUS, slice_count),
        HitZone::Slice(1)
    );

    // Slice 2: Bottom quadrant (angle_from_top = 5*PI/4)
    let x_2 = cx + mid_r * (5.0 * PI / 4.0 - PI / 2.0).cos();
    let y_2 = cy + mid_r * (5.0 * PI / 4.0 - PI / 2.0).sin();
    assert_eq!(
        hit_test(x_2, y_2, cx, cy, INNER_RADIUS, OUTER_RADIUS, slice_count),
        HitZone::Slice(2)
    );

    // Slice 3: Left quadrant (angle_from_top = 7*PI/4)
    let x_3 = cx + mid_r * (7.0 * PI / 4.0 - PI / 2.0).cos();
    let y_3 = cy + mid_r * (7.0 * PI / 4.0 - PI / 2.0).sin();
    assert_eq!(
        hit_test(x_3, y_3, cx, cy, INNER_RADIUS, OUTER_RADIUS, slice_count),
        HitZone::Slice(3)
    );
}

#[test]
fn test_7_slices_hit_testing() {
    let cx = 190.0;
    let cy = 190.0;
    let slice_count = 7;
    let mid_r = (INNER_RADIUS + OUTER_RADIUS) / 2.0;
    let slice_angle = (2.0 * PI) / 7.0;

    for i in 0..7 {
        let angle_from_top = (i as f64 + 0.5) * slice_angle;
        let actual_angle = angle_from_top - PI / 2.0;
        let px = cx + mid_r * actual_angle.cos();
        let py = cy + mid_r * actual_angle.sin();

        assert_eq!(
            hit_test(px, py, cx, cy, INNER_RADIUS, OUTER_RADIUS, slice_count),
            HitZone::Slice(i),
            "Failed hit test for slice index {} of 7",
            i
        );
    }
}

#[test]
fn test_8_slices_hit_testing() {
    let cx = 190.0;
    let cy = 190.0;
    let slice_count = 8;
    let mid_r = (INNER_RADIUS + OUTER_RADIUS) / 2.0;
    let slice_angle = (2.0 * PI) / 8.0;

    for i in 0..8 {
        let angle_from_top = (i as f64 + 0.5) * slice_angle;
        let actual_angle = angle_from_top - PI / 2.0;
        let px = cx + mid_r * actual_angle.cos();
        let py = cy + mid_r * actual_angle.sin();

        assert_eq!(
            hit_test(px, py, cx, cy, INNER_RADIUS, OUTER_RADIUS, slice_count),
            HitZone::Slice(i),
            "Failed hit test for slice index {} of 8",
            i
        );
    }
}

#[test]
fn test_edge_zero_slices() {
    assert_eq!(
        hit_test(190.0, 190.0, 190.0, 190.0, INNER_RADIUS, OUTER_RADIUS, 0),
        HitZone::Outside
    );
}
