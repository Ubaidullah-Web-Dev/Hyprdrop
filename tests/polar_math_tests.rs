use std::f64::consts::PI;
use hyprdrop::engine::FileCategory;
use hyprdrop::ui::radial_menu::{
    HitZone, INNER_RADIUS, OUTER_RADIUS, RadialMode, get_slice_support_status, hit_test,
};

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

#[test]
fn test_video_quick_actions_grayed_out() {
    let cat = FileCategory::AudioVideo;
    let mode = RadialMode::QuickActions;

    // Video should NOT allow image-only studios
    let (edit_ok, reason_edit) = get_slice_support_status(cat, mode, "EDIT", Some("mp4"));
    assert!(!edit_ok);
    assert_eq!(reason_edit, Some("Unavailable for Video"));

    let (crop_ok, reason_crop) = get_slice_support_status(cat, mode, "CROP", Some("mp4"));
    assert!(!crop_ok);
    assert_eq!(reason_crop, Some("Unavailable for Video"));

    let (bg_ok, reason_bg) = get_slice_support_status(cat, mode, "ADD BG", Some("mp4"));
    assert!(!bg_ok);
    assert_eq!(reason_bg, Some("Unavailable for Video"));

    let (trim_ok, reason_trim) = get_slice_support_status(cat, mode, "AUTO TRIM", Some("mp4"));
    assert!(!trim_ok);
    assert_eq!(reason_trim, Some("Unavailable for Video"));

    let (redact_ok, reason_redact) = get_slice_support_status(cat, mode, "REDACT", Some("mp4"));
    assert!(!redact_ok);
    assert_eq!(reason_redact, Some("Unavailable for Video"));

    let (ocr_ok, reason_ocr) = get_slice_support_status(cat, mode, "OCR / COPY", Some("mp4"));
    assert!(!ocr_ok);
    assert_eq!(reason_ocr, Some("Unavailable for Video"));

    // Video SHOULD allow compress and metadata
    let (compress_ok, reason_comp) = get_slice_support_status(cat, mode, "COMPRESS", Some("mp4"));
    assert!(compress_ok);
    assert!(reason_comp.is_none());

    let (meta_ok, reason_meta) = get_slice_support_status(cat, mode, "METADATA", Some("mp4"));
    assert!(meta_ok);
    assert!(reason_meta.is_none());
}

#[test]
fn test_image_all_quick_actions_enabled() {
    let cat = FileCategory::Image;
    let mode = RadialMode::QuickActions;

    let actions = ["CROP", "ADD BG", "COMPRESS", "REDACT", "AUTO TRIM", "METADATA", "EDIT", "OCR / COPY"];
    for act in actions {
        let (ok, reason) = get_slice_support_status(cat, mode, act, Some("png"));
        assert!(ok, "Expected {} to be enabled for image", act);
        assert!(reason.is_none());
    }
}

#[test]
fn test_format_conversion_already_in_format() {
    let cat = FileCategory::Image;
    let mode = RadialMode::FormatConversion;

    // PNG -> PNG is redundant
    let (png_ok, reason_png) = get_slice_support_status(cat, mode, "PNG", Some("png"));
    assert!(!png_ok);
    assert_eq!(reason_png, Some("Already In Format"));

    // PNG -> JPG is allowed
    let (jpg_ok, reason_jpg) = get_slice_support_status(cat, mode, "JPG", Some("png"));
    assert!(jpg_ok);
    assert!(reason_jpg.is_none());

    // MP4 -> MP4 on video is redundant
    let (mp4_ok, reason_mp4) = get_slice_support_status(FileCategory::AudioVideo, mode, "MP4", Some("mp4"));
    assert!(!mp4_ok);
    assert_eq!(reason_mp4, Some("Already In Format"));

    // MP4 -> WEBM on video is allowed
    let (webm_ok, reason_webm) = get_slice_support_status(FileCategory::AudioVideo, mode, "WEBM", Some("mp4"));
    assert!(webm_ok);
    assert!(reason_webm.is_none());
}
