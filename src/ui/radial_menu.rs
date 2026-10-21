use std::cell::RefCell;
use std::f64::consts::PI;
use std::path::PathBuf;
use std::rc::Rc;
use gtk4::cairo::{Context, FontSlant, FontWeight, Operator};
use gtk4::glib;
use gtk4::prelude::*;
use crate::engine::{FileCategory, format_file_size, get_conversion_slices, get_quick_action_slices};

pub const RADIAL_SIZE: i32 = 400;
pub const INNER_RADIUS: f64 = 56.0;
pub const OUTER_RADIUS: f64 = 176.0;

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
            hovered_zone: HitZone::CenterHub,
        }));

        let drawing_area = gtk4::DrawingArea::builder()
            .content_width(RADIAL_SIZE)
            .content_height(RADIAL_SIZE)
            .focusable(true)
            .can_focus(true)
            .build();

        drawing_area.add_css_class("hyprdrop-radial-canvas");

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
        // Ensure 100% transparent canvas background
        cr.save().ok();
        cr.set_operator(Operator::Clear);
        let _ = cr.paint();
        cr.restore().ok();

        let cx = width / 2.0;
        let cy = height / 2.0;

        let slices = match state.mode {
            RadialMode::FormatConversion => get_conversion_slices(state.file_category),
            RadialMode::QuickActions => get_quick_action_slices(state.file_category),
        };
        let slice_count = slices.len();
        if slice_count == 0 {
            return;
        }

        let slice_angle = (2.0 * PI) / (slice_count as f64);
        let gap_angle = 0.055; // Crisp gap between modern curved sectors

        // Render Slices (Modern macOS Island Sectors with Rounded Corners & Pop-Out Hover)
        for (i, &slice_label) in slices.iter().enumerate() {
            let start_angle = -PI / 2.0 + (i as f64) * slice_angle;
            let end_angle = start_angle + slice_angle;

            let is_hovered = match state.hovered_zone {
                HitZone::Slice(idx) => idx == i,
                _ => false,
            };

            let mid_angle = start_angle + slice_angle / 2.0;

            // When hovered: pop-out displacement outward along radial angle + expansion
            let (pop_dx, pop_dy, r_in, r_out) = if is_hovered {
                let pop_dist = 8.0;
                (
                    pop_dist * mid_angle.cos(),
                    pop_dist * mid_angle.sin(),
                    INNER_RADIUS + 4.0,
                    OUTER_RADIUS + 4.0,
                )
            } else {
                (0.0, 0.0, INNER_RADIUS + 6.0, OUTER_RADIUS - 2.0)
            };

            let sector_cx = cx + pop_dx;
            let sector_cy = cy + pop_dy;

            let a_start = start_angle + gap_angle / 2.0;
            let a_end = end_angle - gap_angle / 2.0;

            // Draw rounded annular sector
            Self::draw_rounded_sector(cr, sector_cx, sector_cy, r_in, r_out, a_start, a_end, 7.0);

            if is_hovered {
                // macOS Electric Blue with subtle gradient / glow
                cr.set_source_rgba(0.04, 0.52, 1.0, 0.96); // #0A84FF
                let _ = cr.fill_preserve();
                cr.set_source_rgba(0.40, 0.72, 1.0, 0.85);
                cr.set_line_width(1.8);
                let _ = cr.stroke();
            } else {
                // Sleek macOS dark frosted card slice
                cr.set_source_rgba(0.11, 0.11, 0.14, 0.88);
                let _ = cr.fill_preserve();
                // Subtle 1px frosted border
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.10);
                cr.set_line_width(1.0);
                let _ = cr.stroke();
            }

            // Draw Slice Label (centered within the sector)
            let mid_r = (r_in + r_out) / 2.0;
            let label_x = sector_cx + mid_r * mid_angle.cos();
            let label_y = sector_cy + mid_r * mid_angle.sin();

            cr.select_font_face("Sans", FontSlant::Normal, FontWeight::Bold);
            let font_size = if slice_label.len() > 7 { 10.5 } else { 12.0 };
            cr.set_font_size(font_size);

            if is_hovered {
                cr.set_source_rgb(1.0, 1.0, 1.0);
            } else {
                cr.set_source_rgb(0.92, 0.92, 0.96);
            }

            if let Ok(extents) = cr.text_extents(slice_label) {
                cr.new_sub_path();
                cr.move_to(
                    label_x - extents.width() / 2.0 - extents.x_bearing(),
                    label_y - extents.height() / 2.0 - extents.y_bearing(),
                );
                let _ = cr.show_text(slice_label);
            }
        }

        // --- Center Hub ---
        // Clean sub path to PREVENT any diagonal lines
        cr.new_sub_path();

        let is_center_hovered = state.hovered_zone == HitZone::CenterHub;

        cr.arc(cx, cy, INNER_RADIUS, 0.0, 2.0 * PI);
        if is_center_hovered {
            cr.set_source_rgba(0.14, 0.14, 0.18, 0.98);
        } else {
            cr.set_source_rgba(0.08, 0.08, 0.10, 0.94);
        }
        let _ = cr.fill_preserve();

        // Center hub border: macOS Blue if hovered, translucent rim otherwise
        if is_center_hovered {
            cr.set_source_rgba(0.04, 0.52, 1.0, 0.90);
            cr.set_line_width(2.0);
        } else {
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.14);
            cr.set_line_width(1.0);
        }
        let _ = cr.stroke();

        // Center Hub Typography
        match state.hovered_zone {
            HitZone::Slice(idx) if idx < slices.len() => {
                // Display hovered format or action in bright macOS Electric Blue
                cr.select_font_face("Sans", FontSlant::Normal, FontWeight::Bold);
                let hover_text = slices[idx];
                let fsize = if hover_text.len() > 8 { 12.0 } else { 14.0 };
                cr.set_font_size(fsize);
                cr.set_source_rgb(0.18, 0.60, 1.0); // Electric Blue text

                if let Ok(ext) = cr.text_extents(hover_text) {
                    cr.new_sub_path();
                    cr.move_to(
                        cx - ext.width() / 2.0 - ext.x_bearing(),
                        cy - ext.height() / 2.0 - ext.y_bearing() - 4.0,
                    );
                    let _ = cr.show_text(hover_text);
                }

                // Subtitle: Action hint
                cr.select_font_face("Monospace", FontSlant::Normal, FontWeight::Normal);
                cr.set_font_size(9.0);
                cr.set_source_rgb(0.60, 0.60, 0.66);
                let sub = match state.mode {
                    RadialMode::FormatConversion => "CONVERT",
                    RadialMode::QuickActions => "ACTION",
                };
                if let Ok(ext) = cr.text_extents(sub) {
                    cr.new_sub_path();
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
                    0 => ("HYPRDROP".to_string(), Some("DROP FILE".to_string())),
                    1 => {
                        let path = &state.current_files[0];
                        let sz = match std::fs::metadata(path) {
                            Ok(meta) => format_file_size(meta.len()),
                            Err(_) => "FILE".to_string(),
                        };
                        let file_name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("");
                        let display_name = if file_name.len() > 11 {
                            format!("{}…", &file_name[..9])
                        } else {
                            file_name.to_string()
                        };
                        (sz, Some(display_name))
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
                cr.set_font_size(12.5);
                cr.set_source_rgb(0.96, 0.96, 0.98);

                if let Ok(ext) = cr.text_extents(&main_text) {
                    cr.new_sub_path();
                    cr.move_to(
                        cx - ext.width() / 2.0 - ext.x_bearing(),
                        cy - ext.height() / 2.0 - ext.y_bearing() - 5.0,
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
                cr.set_source_rgba(0.18, 0.60, 1.0, 0.90); // macOS Blue

                if let Ok(ext) = cr.text_extents(&sub_display) {
                    cr.new_sub_path();
                    cr.move_to(
                        cx - ext.width() / 2.0 - ext.x_bearing(),
                        cy + 13.0 - ext.y_bearing(),
                    );
                    let _ = cr.show_text(&sub_display);
                }
            }
        }
    }

    /// Draws an annular sector with rounded corners on all 4 vertices.
    #[allow(clippy::too_many_arguments)]
    fn draw_rounded_sector(
        cr: &Context,
        cx: f64,
        cy: f64,
        r_in: f64,
        r_out: f64,
        a_start: f64,
        a_end: f64,
        corner_r: f64,
    ) {
        let span = a_end - a_start;
        let da_in = (corner_r / r_in).min(span / 4.0);
        let da_out = (corner_r / r_out).min(span / 4.0);
        let dr = corner_r.min((r_out - r_in) / 3.0);

        cr.new_sub_path();

        // 1. Inner arc from a_start + da_in to a_end - da_in
        cr.arc(cx, cy, r_in, a_start + da_in, a_end - da_in);

        // 2. Corner to straight outer edge at a_end
        let x_in_end = cx + (r_in + dr) * a_end.cos();
        let y_in_end = cy + (r_in + dr) * a_end.sin();
        cr.line_to(x_in_end, y_in_end);

        // 3. Straight edge to r_out - dr at a_end
        let x_out_end = cx + (r_out - dr) * a_end.cos();
        let y_out_end = cy + (r_out - dr) * a_end.sin();
        cr.line_to(x_out_end, y_out_end);

        // 4. Outer arc from a_end - da_out back to a_start + da_out
        cr.arc_negative(cx, cy, r_out, a_end - da_out, a_start + da_out);

        // 5. Straight edge at a_start from r_out - dr down to r_in + dr
        let x_in_start = cx + (r_in + dr) * a_start.cos();
        let y_in_start = cy + (r_in + dr) * a_start.sin();
        cr.line_to(x_in_start, y_in_start);

        cr.close_path();
    }
}
