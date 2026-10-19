use std::cell::RefCell;
use std::f64::consts::PI;
use std::path::PathBuf;
use std::rc::Rc;
use gtk4::cairo::{Context, FontSlant, FontWeight};
use gtk4::glib;
use gtk4::prelude::*;
use crate::engine::{FileCategory, format_file_size, get_conversion_slices, get_quick_action_slices};

pub const RADIAL_SIZE: i32 = 380;
pub const INNER_RADIUS: f64 = 54.0;
pub const OUTER_RADIUS: f64 = 175.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadialMode {
    FormatConversion,
    QuickActions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitZone {
    CenterHub,
    Slice(usize),
    Outside,
}

/// Pure polar coordinate hit test calculation.
/// Given coordinate `(x, y)` relative to widget top-left,
/// computes whether the point hits center hub, a slice `0..slice_count`, or outside.
pub fn hit_test(
    x: f64,
    y: f64,
    center_x: f64,
    center_y: f64,
    inner_r: f64,
    outer_r: f64,
    slice_count: usize,
) -> HitZone {
    if slice_count == 0 {
        return HitZone::Outside;
    }

    let dx = x - center_x;
    let dy = y - center_y;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist < inner_r {
        return HitZone::CenterHub;
    }
    if dist > outer_r {
        return HitZone::Outside;
    }

    // Angle starting from 12 o'clock (-PI / 2) going clockwise
    let raw_angle = dy.atan2(dx);
    let angle_from_top = (raw_angle + PI / 2.0).rem_euclid(2.0 * PI);
    let slice_angle = (2.0 * PI) / (slice_count as f64);
    let idx = (angle_from_top / slice_angle).floor() as usize;

    HitZone::Slice(idx.min(slice_count - 1))
}

pub struct RadialState {
    pub mode: RadialMode,
    pub current_files: Vec<PathBuf>,
    pub file_category: FileCategory,
    pub hovered_zone: HitZone,
}

impl RadialState {
    pub fn current_file(&self) -> Option<&PathBuf> {
        self.current_files.first()
    }
}

pub struct RadialMenu {
    pub widget: gtk4::DrawingArea,
    pub state: Rc<RefCell<RadialState>>,
}

impl RadialMenu {
    pub fn new(initial_file: Option<PathBuf>) -> Self {
        let files = initial_file.map(|f| vec![f]).unwrap_or_default();
        Self::new_with_files(files)
    }

    pub fn new_with_files(initial_files: Vec<PathBuf>) -> Self {
        let category = match initial_files.first() {
            Some(path) => FileCategory::from_path(path),
            None => FileCategory::Image,
        };

        let state = Rc::new(RefCell::new(RadialState {
            mode: RadialMode::FormatConversion,
            current_files: initial_files,
            file_category: category,
            hovered_zone: HitZone::Outside,
        }));

        let drawing_area = gtk4::DrawingArea::builder()
            .content_width(RADIAL_SIZE)
            .content_height(RADIAL_SIZE)
            .focusable(true)
            .can_focus(true)
            .build();

        let state_clone = state.clone();
        drawing_area.set_draw_func(move |_, cr, width, height| {
            let s = state_clone.borrow();
            Self::draw_radial_menu(cr, width as f64, height as f64, &s);
        });

        // Motion controller for hover tracking
        let motion = gtk4::EventControllerMotion::new();
        let state_motion = state.clone();
        let area_motion = drawing_area.clone();
        motion.connect_motion(move |_, x, y| {
            let mut s = state_motion.borrow_mut();
            let slices = match s.mode {
                RadialMode::FormatConversion => get_conversion_slices(s.file_category),
                RadialMode::QuickActions => get_quick_action_slices(s.file_category),
            };
            let cx = (RADIAL_SIZE as f64) / 2.0;
            let cy = (RADIAL_SIZE as f64) / 2.0;
            let zone = hit_test(x, y, cx, cy, INNER_RADIUS, OUTER_RADIUS, slices.len());
            if s.hovered_zone != zone {
                s.hovered_zone = zone;
                drop(s);
                area_motion.queue_draw();
            }
        });

        let state_leave = state.clone();
        let area_leave = drawing_area.clone();
        motion.connect_leave(move |_| {
            let mut s = state_leave.borrow_mut();
            if s.hovered_zone != HitZone::Outside {
                s.hovered_zone = HitZone::Outside;
                drop(s);
                area_leave.queue_draw();
            }
        });
        drawing_area.add_controller(motion);

        // Scroll controller to flip mode
        let scroll = gtk4::EventControllerScroll::new(
            gtk4::EventControllerScrollFlags::VERTICAL | gtk4::EventControllerScrollFlags::DISCRETE,
        );
        let state_scroll = state.clone();
        let area_scroll = drawing_area.clone();
        scroll.connect_scroll(move |_, _, _| {
            let mut s = state_scroll.borrow_mut();
            s.mode = match s.mode {
                RadialMode::FormatConversion => RadialMode::QuickActions,
                RadialMode::QuickActions => RadialMode::FormatConversion,
            };
            drop(s);
            area_scroll.queue_draw();
            glib::Propagation::Stop
        });
        drawing_area.add_controller(scroll);

        Self {
            widget: drawing_area,
            state,
        }
    }

    pub fn set_file(&self, file_path: PathBuf) {
        self.set_files(vec![file_path]);
    }

    pub fn set_files(&self, files: Vec<PathBuf>) {
        let mut s = self.state.borrow_mut();
        if let Some(first) = files.first() {
            s.file_category = FileCategory::from_path(first);
        }
        s.current_files = files;
        drop(s);
        self.widget.queue_draw();
    }

    pub fn toggle_mode(&self) {
        let mut s = self.state.borrow_mut();
        s.mode = match s.mode {
            RadialMode::FormatConversion => RadialMode::QuickActions,
            RadialMode::QuickActions => RadialMode::FormatConversion,
        };
        drop(s);
        self.widget.queue_draw();
    }

    fn draw_radial_menu(cr: &Context, width: f64, height: f64, state: &RadialState) {
        let cx = width / 2.0;
        let cy = height / 2.0;

        let slices = match state.mode {
            RadialMode::FormatConversion => get_conversion_slices(state.file_category),
            RadialMode::QuickActions => get_quick_action_slices(state.file_category),
        };
        let slice_count = slices.len();
        let slice_angle = (2.0 * PI) / (slice_count as f64);

        // Background Disc shadow / border
        cr.arc(cx, cy, OUTER_RADIUS + 3.0, 0.0, 2.0 * PI);
        cr.set_source_rgba(0.08, 0.08, 0.09, 0.45);
        let _ = cr.fill();

        // Outer rim ring
        cr.arc(cx, cy, OUTER_RADIUS, 0.0, 2.0 * PI);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.06);
        cr.set_line_width(1.0);
        let _ = cr.stroke();

        // Render Slices
        for (i, &slice_label) in slices.iter().enumerate() {
            let start_angle = -PI / 2.0 + (i as f64) * slice_angle;
            let end_angle = start_angle + slice_angle;

            let is_hovered = match state.hovered_zone {
                HitZone::Slice(idx) => idx == i,
                _ => false,
            };

            // Slice arc path
            cr.new_sub_path();
            cr.arc(cx, cy, OUTER_RADIUS - 1.0, start_angle, end_angle);
            cr.arc_negative(cx, cy, INNER_RADIUS + 1.0, end_angle, start_angle);
            cr.close_path();

            if is_hovered {
                // Active/Hover slice: #FF5722 (International Warm Orange)
                cr.set_source_rgba(1.0, 0.34, 0.13, 0.98);
                let _ = cr.fill_preserve();
                cr.set_source_rgba(1.0, 0.47, 0.31, 0.8);
                cr.set_line_width(1.5);
                let _ = cr.stroke();
            } else {
                // Elevated card slice: #212124 (94% opacity)
                cr.set_source_rgba(0.129, 0.129, 0.141, 0.94);
                let _ = cr.fill_preserve();
                // 1px subtle divider: rgba(255, 255, 255, 0.08)
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.08);
                cr.set_line_width(1.0);
                let _ = cr.stroke();
            }

            // Draw label
            let mid_angle = start_angle + slice_angle / 2.0;
            let mid_r = (INNER_RADIUS + OUTER_RADIUS) / 2.0;
            let label_x = cx + mid_r * mid_angle.cos();
            let label_y = cy + mid_r * mid_angle.sin();

            cr.select_font_face("Sans", FontSlant::Normal, FontWeight::Bold);
            let font_size = if slice_label.len() > 8 { 10.0 } else { 12.0 };
            cr.set_font_size(font_size);

            if is_hovered {
                cr.set_source_rgb(1.0, 1.0, 1.0);
            } else {
                cr.set_source_rgb(0.95, 0.95, 0.96);
            }

            if let Ok(extents) = cr.text_extents(slice_label) {
                cr.move_to(
                    label_x - extents.width() / 2.0 - extents.x_bearing(),
                    label_y - extents.height() / 2.0 - extents.y_bearing(),
                );
                let _ = cr.show_text(slice_label);
            }
        }

        // Center Hub
        let is_center_hovered = state.hovered_zone == HitZone::CenterHub;

        cr.arc(cx, cy, INNER_RADIUS, 0.0, 2.0 * PI);
        if is_center_hovered {
            cr.set_source_rgba(0.18, 0.18, 0.20, 0.98);
        } else {
            cr.set_source_rgba(0.086, 0.086, 0.094, 0.95);
        }
        let _ = cr.fill_preserve();

        // Center hub border: orange if center hovered, subtle border otherwise
        if is_center_hovered {
            cr.set_source_rgba(1.0, 0.34, 0.13, 0.9);
            cr.set_line_width(2.0);
        } else {
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.12);
            cr.set_line_width(1.0);
        }
        let _ = cr.stroke();

        // Center Hub Typography
        match state.hovered_zone {
            HitZone::Slice(idx) if idx < slices.len() => {
                // Display hovered format or action in bright warm orange
                cr.select_font_face("Sans", FontSlant::Normal, FontWeight::Bold);
                let hover_text = slices[idx];
                let fsize = if hover_text.len() > 8 { 12.0 } else { 14.0 };
                cr.set_font_size(fsize);
                cr.set_source_rgb(1.0, 0.34, 0.13);

                if let Ok(ext) = cr.text_extents(hover_text) {
                    cr.move_to(
                        cx - ext.width() / 2.0 - ext.x_bearing(),
                        cy - ext.height() / 2.0 - ext.y_bearing() - 4.0,
                    );
                    let _ = cr.show_text(hover_text);
                }

                // Subtitle: Action hint
                cr.select_font_face("Monospace", FontSlant::Normal, FontWeight::Normal);
                cr.set_font_size(9.0);
                cr.set_source_rgb(0.62, 0.62, 0.66);
                let sub = match state.mode {
                    RadialMode::FormatConversion => "CONVERT",
                    RadialMode::QuickActions => "ACTION",
                };
                if let Ok(ext) = cr.text_extents(sub) {
                    cr.move_to(
                        cx - ext.width() / 2.0 - ext.x_bearing(),
                        cy + 14.0 - ext.y_bearing(),
                    );
                    let _ = cr.show_text(sub);
                }
            }
            _ => {
                // Default center: File size, Multiple files count, or Drop prompt
                let (main_text, sub_override) = match state.current_files.len() {
                    0 => ("HYPRDROP".to_string(), None),
                    1 => {
                        let path = &state.current_files[0];
                        let sz = match std::fs::metadata(path) {
                            Ok(meta) => format_file_size(meta.len()),
                            Err(_) => "FILE".to_string(),
                        };
                        (sz, None)
                    }
                    n => {
                        let total_bytes: u64 = state
                            .current_files
                            .iter()
                            .filter_map(|p| std::fs::metadata(p).ok().map(|m| m.len()))
                            .sum();
                        (format!("{} FILES", n), Some(format_file_size(total_bytes)))
                    }
                };

                cr.select_font_face("Monospace", FontSlant::Normal, FontWeight::Bold);
                cr.set_font_size(13.0);
                cr.set_source_rgb(0.96, 0.96, 0.97);

                if let Ok(ext) = cr.text_extents(&main_text) {
                    cr.move_to(
                        cx - ext.width() / 2.0 - ext.x_bearing(),
                        cy - ext.height() / 2.0 - ext.y_bearing() - 6.0,
                    );
                    let _ = cr.show_text(&main_text);
                }

                // Subtitle: Mode ("FORMATS" or "ACTIONS") or total size
                let mode_str = match state.mode {
                    RadialMode::FormatConversion => "FORMATS",
                    RadialMode::QuickActions => "ACTIONS",
                };
                let sub_display = sub_override.unwrap_or_else(|| mode_str.to_string());
                cr.select_font_face("Sans", FontSlant::Normal, FontWeight::Bold);
                cr.set_font_size(9.0);
                cr.set_source_rgba(1.0, 0.34, 0.13, 0.85);

                if let Ok(ext) = cr.text_extents(&sub_display) {
                    cr.move_to(
                        cx - ext.width() / 2.0 - ext.x_bearing(),
                        cy + 12.0 - ext.y_bearing(),
                    );
                    let _ = cr.show_text(&sub_display);
                }
            }
        }
    }
}
