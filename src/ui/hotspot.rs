// ==============================================================================
// Hyprdrop Dynamic Draggable Edge Hotspot & Corner Morphing Studio
// ==============================================================================

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use gtk4::cairo::Context;
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};

use crate::config::{AppConfig, ConfigManager};
use crate::engine::perimeter_math::{PerimeterMath, ScreenEdge};
use crate::hyprland::{get_cursor_position, get_focused_monitor};
use crate::ui::overlay_window::{OverlayWindow, extract_paths_from_drop_value};

pub fn parse_hex_color(hex: &str, default_alpha: f64) -> (f64, f64, f64, f64) {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&clean[0..2], 16),
            u8::from_str_radix(&clean[2..4], 16),
            u8::from_str_radix(&clean[4..6], 16),
        ) {
            return (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, default_alpha);
        }
    }
    (0.09, 0.09, 0.10, default_alpha)
}

#[derive(Clone)]
pub struct HotspotState {
    pub config: AppConfig,
    pub monitor_w: f64,
    pub monitor_h: f64,
    pub is_hovered: bool,
    pub is_drop_active: bool,
    pub is_dragging: bool,
    pub drag_start_x: f64,
    pub drag_start_y: f64,
}

#[derive(Clone)]
pub struct HotspotController {
    pub window: gtk4::ApplicationWindow,
    pub drawing_area: gtk4::DrawingArea,
    pub state: Rc<RefCell<HotspotState>>,
    pub is_fullscreen_hidden: Rc<Cell<bool>>,
}

impl HotspotController {
    pub fn set_fullscreen_hidden(&self, hidden: bool) {
        if hidden == self.is_fullscreen_hidden.get() {
            return;
        }
        self.is_fullscreen_hidden.set(hidden);

        if hidden {
            self.window.set_visible(false);
        } else {
            let s = self.state.borrow();
            if s.config.hotspot_enabled {
                self.window.set_visible(true);
            }
        }
    }

    /// Repositions and resizes the LayerShell window based on current perimeter coordinate.
    pub fn update_geometry(&self) {
        let s = self.state.borrow();
        let math = PerimeterMath::new(s.monitor_w, s.monitor_h);
        let current_s = math.t_to_s(s.config.hotspot_perimeter_pos);
        let span = math.calculate_span(current_s, s.config.hotspot_length as f64);
        let thickness = s.config.hotspot_thickness as i32;
        let margin = s.config.hotspot_margin;

        let win = &self.window;

        // Reset all anchors
        win.set_anchor(Edge::Top, false);
        win.set_anchor(Edge::Right, false);
        win.set_anchor(Edge::Bottom, false);
        win.set_anchor(Edge::Left, false);

        match span.edge {
            ScreenEdge::Top => {
                let win_w = (s.config.hotspot_length as i32).max(thickness + 12);
                let win_h = thickness + 8;
                let left_offset = (span.start_s as i32).clamp(0, (s.monitor_w as i32) - win_w);

                win.set_anchor(Edge::Top, true);
                win.set_anchor(Edge::Left, true);
                win.set_margin(Edge::Top, margin);
                win.set_margin(Edge::Left, left_offset);
                win.set_default_size(win_w, win_h);
                self.drawing_area.set_size_request(win_w, win_h);
            }
            ScreenEdge::Right => {
                let win_w = thickness + 8;
                let win_h = (s.config.hotspot_length as i32).max(thickness + 12);
                let top_offset = ((span.start_s - s.monitor_w) as i32).clamp(0, (s.monitor_h as i32) - win_h);

                win.set_anchor(Edge::Right, true);
                win.set_anchor(Edge::Top, true);
                win.set_margin(Edge::Right, margin);
                win.set_margin(Edge::Top, top_offset);
                win.set_default_size(win_w, win_h);
                self.drawing_area.set_size_request(win_w, win_h);
            }
            ScreenEdge::Bottom => {
                let win_w = (s.config.hotspot_length as i32).max(thickness + 12);
                let win_h = thickness + 8;
                let bottom_dist = span.start_s - s.monitor_w - s.monitor_h;
                let left_offset = ((s.monitor_w - bottom_dist - (s.config.hotspot_length as f64)) as i32)
                    .clamp(0, (s.monitor_w as i32) - win_w);

                win.set_anchor(Edge::Bottom, true);
                win.set_anchor(Edge::Left, true);
                win.set_margin(Edge::Bottom, margin);
                win.set_margin(Edge::Left, left_offset);
                win.set_default_size(win_w, win_h);
                self.drawing_area.set_size_request(win_w, win_h);
            }
            ScreenEdge::Left => {
                let win_w = thickness + 8;
                let win_h = (s.config.hotspot_length as i32).max(thickness + 12);
                let left_dist = span.start_s - 2.0 * s.monitor_w - s.monitor_h;
                let top_offset = ((s.monitor_h - left_dist - (s.config.hotspot_length as f64)) as i32)
                    .clamp(0, (s.monitor_h as i32) - win_h);

                win.set_anchor(Edge::Left, true);
                win.set_anchor(Edge::Top, true);
                win.set_margin(Edge::Left, margin);
                win.set_margin(Edge::Top, top_offset);
                win.set_default_size(win_w, win_h);
                self.drawing_area.set_size_request(win_w, win_h);
            }
            ScreenEdge::CornerTopRight { len_top, len_right } => {
                let win_w = ((len_top as i32) + thickness + 8).max(thickness + 12);
                let win_h = ((len_right as i32) + thickness + 8).max(thickness + 12);

                win.set_anchor(Edge::Top, true);
                win.set_anchor(Edge::Right, true);
                win.set_margin(Edge::Top, margin);
                win.set_margin(Edge::Right, margin);
                win.set_default_size(win_w, win_h);
                self.drawing_area.set_size_request(win_w, win_h);
            }
            ScreenEdge::CornerBottomRight { len_right, len_bottom } => {
                let win_w = ((len_bottom as i32) + thickness + 8).max(thickness + 12);
                let win_h = ((len_right as i32) + thickness + 8).max(thickness + 12);

                win.set_anchor(Edge::Bottom, true);
                win.set_anchor(Edge::Right, true);
                win.set_margin(Edge::Bottom, margin);
                win.set_margin(Edge::Right, margin);
                win.set_default_size(win_w, win_h);
                self.drawing_area.set_size_request(win_w, win_h);
            }
            ScreenEdge::CornerBottomLeft { len_bottom, len_left } => {
                let win_w = ((len_bottom as i32) + thickness + 8).max(thickness + 12);
                let win_h = ((len_left as i32) + thickness + 8).max(thickness + 12);

                win.set_anchor(Edge::Bottom, true);
                win.set_anchor(Edge::Left, true);
                win.set_margin(Edge::Bottom, margin);
                win.set_margin(Edge::Left, margin);
                win.set_default_size(win_w, win_h);
                self.drawing_area.set_size_request(win_w, win_h);
            }
            ScreenEdge::CornerTopLeft { len_left, len_top } => {
                let win_w = ((len_top as i32) + thickness + 8).max(thickness + 12);
                let win_h = ((len_left as i32) + thickness + 8).max(thickness + 12);

                win.set_anchor(Edge::Top, true);
                win.set_anchor(Edge::Left, true);
                win.set_margin(Edge::Top, margin);
                win.set_margin(Edge::Left, margin);
                win.set_default_size(win_w, win_h);
                self.drawing_area.set_size_request(win_w, win_h);
            }
        }

        self.drawing_area.queue_draw();
    }
}

thread_local! {
    static ACTIVE_HOTSPOT: RefCell<Option<HotspotController>> = const { RefCell::new(None) };
}

static LISTENER_INITIALIZED: AtomicBool = AtomicBool::new(false);

pub struct HotspotWindow;

impl HotspotWindow {
    /// Spawns or updates the dynamic perimeter hotspot widget.
    pub fn spawn(app: &gtk4::Application) {
        if !gtk4_layer_shell::is_supported() {
            return;
        }

        let cfg = ConfigManager::load_config();
        if !cfg.hotspot_enabled {
            Self::close();
            return;
        }

        Self::close();

        let (mon_w, mon_h) = if let Some(mon) = get_focused_monitor() {
            let scale = mon.scale.max(0.1);
            ((mon.width as f64 / scale), (mon.height as f64 / scale))
        } else {
            (1920.0, 1080.0)
        };

        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("Hyprdrop Edge Hotspot")
            .resizable(false)
            .decorated(false)
            .build();

        window.add_css_class("hyprdrop-hotspot");
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_namespace(Some("hyprdrop-hotspot"));

        let state = Rc::new(RefCell::new(HotspotState {
            config: cfg.clone(),
            monitor_w: mon_w,
            monitor_h: mon_h,
            is_hovered: false,
            is_drop_active: false,
            is_dragging: false,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
        }));

        let drawing_area = gtk4::DrawingArea::builder()
            .focusable(false)
            .can_focus(false)
            .build();

        drawing_area.add_css_class("hyprdrop-hotspot-canvas");

        let state_draw = state.clone();
        drawing_area.set_draw_func(move |_, cr, width, height| {
            let s = state_draw.borrow();
            Self::draw_hotspot(cr, width as f64, height as f64, &s);
        });

        // Click vs Drag Gesture Discrimination
        let drag_gesture = gtk4::GestureDrag::new();
        let state_drag = state.clone();
        let app_drag = app.clone();

        drag_gesture.connect_drag_begin(move |gesture, x, y| {
            let mut s = state_drag.borrow_mut();
            s.is_dragging = false;
            s.drag_start_x = x;
            s.drag_start_y = y;
            gesture.set_state(gtk4::EventSequenceState::Claimed);
        });

        let state_drag_update = state.clone();
        drag_gesture.connect_drag_update(move |_, offset_x, offset_y| {
            let mut s = state_drag_update.borrow_mut();
            let dist = (offset_x * offset_x + offset_y * offset_y).sqrt();

            if !s.config.hotspot_locked && dist >= 6.0 {
                s.is_dragging = true;

                let cur = get_cursor_position();
                let math = PerimeterMath::new(s.monitor_w, s.monitor_h);
                let new_s = math.cursor_to_perimeter_s(cur.x as f64, cur.y as f64);
                let new_t = math.s_to_t(new_s);

                s.config.hotspot_perimeter_pos = new_t;
                drop(s);

                ACTIVE_HOTSPOT.with(|cell| {
                    if let Some(ctrl) = cell.borrow().as_ref() {
                        ctrl.update_geometry();
                    }
                });
            }
        });

        let state_drag_end = state.clone();
        let app_drag_end = app_drag.clone();
        drag_gesture.connect_drag_end(move |_, offset_x, offset_y| {
            let dist = (offset_x * offset_x + offset_y * offset_y).sqrt();
            let mut s = state_drag_end.borrow_mut();

            if s.is_dragging && dist >= 6.0 && !s.config.hotspot_locked {
                // Drag completed: Save new perimeter position
                let _ = ConfigManager::save_config(&s.config);
                s.is_dragging = false;
            } else {
                // It was a click! Spawn Hyprdrop radial overlay under cursor
                s.is_dragging = false;
                drop(s);
                let files = crate::ui::overlay_window::detect_selected_files();
                OverlayWindow::spawn_with_files(&app_drag_end, files);
            }
        });

        drawing_area.add_controller(drag_gesture);

        // Hover & Motion Controller (Auto-Hide & Feedback)
        let motion = gtk4::EventControllerMotion::new();
        let state_motion = state.clone();
        let da_motion = drawing_area.clone();
        motion.connect_enter(move |_, _, _| {
            let mut s = state_motion.borrow_mut();
            s.is_hovered = true;
            drop(s);
            da_motion.queue_draw();
        });

        let state_motion_leave = state.clone();
        let da_motion_leave = drawing_area.clone();
        motion.connect_leave(move |_| {
            let mut s = state_motion_leave.borrow_mut();
            s.is_hovered = false;
            drop(s);
            da_motion_leave.queue_draw();
        });
        drawing_area.add_controller(motion);

        // Drop Target for file drop onto the hotspot
        let drop_target = gtk4::DropTarget::new(
            glib::types::Type::INVALID,
            gdk::DragAction::COPY | gdk::DragAction::MOVE,
        );
        drop_target.set_preload(true);
        drop_target.set_types(&[
            gdk::FileList::static_type(),
            gtk4::gio::File::static_type(),
            glib::types::Type::STRING,
        ]);

        let state_drop_enter = state.clone();
        let da_drop_enter = drawing_area.clone();
        drop_target.connect_enter(move |_, _, _| {
            let mut s = state_drop_enter.borrow_mut();
            s.is_drop_active = true;
            drop(s);
            da_drop_enter.queue_draw();
            gdk::DragAction::COPY
        });

        let state_drop_leave = state.clone();
        let da_drop_leave = drawing_area.clone();
        drop_target.connect_leave(move |_| {
            let mut s = state_drop_leave.borrow_mut();
            s.is_drop_active = false;
            drop(s);
            da_drop_leave.queue_draw();
        });

        let state_drop = state.clone();
        let da_drop = drawing_area.clone();
        let app_drop = app.clone();
        drop_target.connect_drop(move |_, val, _, _| {
            let mut s = state_drop.borrow_mut();
            s.is_drop_active = false;
            drop(s);
            da_drop.queue_draw();

            let paths = extract_paths_from_drop_value(val);
            if paths.is_empty() {
                return false;
            }
            OverlayWindow::spawn_with_files(&app_drop, paths);
            true
        });

        drawing_area.add_controller(drop_target);

        window.set_child(Some(&drawing_area));

        let is_fs_initial = crate::hyprland::check_is_fullscreen();
        let ctrl = HotspotController {
            window: window.clone(),
            drawing_area: drawing_area.clone(),
            state,
            is_fullscreen_hidden: Rc::new(Cell::new(is_fs_initial)),
        };

        ctrl.update_geometry();

        let win_weak = window.downgrade();
        window.connect_destroy(move |_| {
            ACTIVE_HOTSPOT.with(|cell| {
                if let Ok(mut opt) = cell.try_borrow_mut()
                    && let Some(ref c) = *opt
                {
                    if let Some(curr) = win_weak.upgrade() {
                        if c.window == curr {
                            *opt = None;
                        }
                    } else {
                        *opt = None;
                    }
                }
            });
        });

        ACTIVE_HOTSPOT.with(|cell| {
            *cell.borrow_mut() = Some(ctrl.clone());
        });

        if !is_fs_initial {
            window.present();
        }

        // Initialize background Hyprland socket2 fullscreen event listener
        if !LISTENER_INITIALIZED.swap(true, Ordering::SeqCst) {
            crate::hyprland::listen_hyprland_fullscreen_events(|is_fs| {
                glib::idle_add_once(move || {
                    Self::set_fullscreen_hidden(is_fs);
                });
            });
        }
    }

    /// Reloads the hotspot with the latest configuration from disk.
    pub fn reload(app: &gtk4::Application) {
        Self::spawn(app);
    }

    /// Sets whether the hotspot should be hidden because a fullscreen window is active.
    pub fn set_fullscreen_hidden(hidden: bool) {
        ACTIVE_HOTSPOT.with(|cell| {
            if let Some(ctrl) = cell.borrow().as_ref() {
                ctrl.set_fullscreen_hidden(hidden);
            }
        });
    }

    /// Closes the active edge hotspot if running.
    pub fn close() {
        let ctrl_opt = ACTIVE_HOTSPOT.with(|cell| cell.borrow_mut().take());
        if let Some(ctrl) = ctrl_opt {
            ctrl.window.close();
        }
    }

    /// Toggles the edge drop target hotspot on or off. Returns whether it is now active.
    pub fn toggle(app: &gtk4::Application) -> bool {
        let existing = ACTIVE_HOTSPOT.with(|cell| cell.borrow_mut().take());
        if let Some(ctrl) = existing {
            ctrl.window.close();
            false
        } else {
            Self::spawn(app);
            true
        }
    }

    /// Live hot-reloads the active hotspot colors without window recreate.
    pub fn update_theme_live(color: &str, hover: &str, accent: &str) {
        ACTIVE_HOTSPOT.with(|cell| {
            if let Some(ctrl) = cell.borrow().as_ref() {
                let mut s = ctrl.state.borrow_mut();
                s.config.hotspot_color = color.to_string();
                s.config.hotspot_hover_color = hover.to_string();
                s.config.hotspot_accent_color = accent.to_string();
                ctrl.drawing_area.queue_draw();
            }
        });
    }

    /// Checks if the edge drop target hotspot is currently active.
    pub fn is_active() -> bool {
        ACTIVE_HOTSPOT.with(|cell| cell.borrow().is_some())
    }

    /// Draws the morphing hotspot pill or bent-corner arc with Cairo.
    fn draw_hotspot(cr: &Context, width: f64, height: f64, state: &HotspotState) {
        let _ = cr.save();

        let math = PerimeterMath::new(state.monitor_w, state.monitor_h);
        let s = math.t_to_s(state.config.hotspot_perimeter_pos);
        let span = math.calculate_span(s, state.config.hotspot_length as f64);

        let alpha = if state.is_drop_active {
            1.0
        } else if state.is_hovered {
            0.98
        } else if state.config.hotspot_auto_hide {
            0.12 // Proximity auto-hide idle opacity
        } else {
            0.88
        };

        let (base_r, base_g, base_b, _) = parse_hex_color(&state.config.hotspot_color, alpha);
        let (hover_r, hover_g, hover_b, _) = parse_hex_color(&state.config.hotspot_hover_color, 0.98);
        let (act_r, act_g, act_b, _) = parse_hex_color(&state.config.hotspot_accent_color, 1.0);

        let (r, g, b) = if state.is_drop_active {
            (act_r, act_g, act_b)
        } else if state.is_hovered {
            (hover_r, hover_g, hover_b)
        } else {
            (base_r, base_g, base_b)
        };

        let thickness = state.config.hotspot_thickness as f64;
        let radius = (state.config.hotspot_corner_radius as f64).min(thickness / 2.0);

        match span.edge {
            ScreenEdge::Top => {
                // Horizontal Island Pill flush against top screen edge
                let pill_w = width - 4.0;
                let pill_h = thickness;
                let x = 2.0;
                let y = 0.0;

                Self::draw_rounded_rect(cr, x, y, pill_w, pill_h, radius);
                cr.set_source_rgba(r, g, b, alpha);
                let _ = cr.fill_preserve();

                // Subtle rim border
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.18);
                cr.set_line_width(1.0);
                let _ = cr.stroke();

                // Tactile grip dots if unlocked
                if !state.config.hotspot_locked {
                    Self::draw_grip_dots(cr, width / 2.0, y + pill_h / 2.0, true);
                }
            }
            ScreenEdge::Bottom => {
                // Horizontal Island Pill flush against bottom screen edge
                let pill_w = width - 4.0;
                let pill_h = thickness;
                let x = 2.0;
                let y = height - pill_h;

                Self::draw_rounded_rect(cr, x, y, pill_w, pill_h, radius);
                cr.set_source_rgba(r, g, b, alpha);
                let _ = cr.fill_preserve();

                // Subtle rim border
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.18);
                cr.set_line_width(1.0);
                let _ = cr.stroke();

                // Tactile grip dots if unlocked
                if !state.config.hotspot_locked {
                    Self::draw_grip_dots(cr, width / 2.0, y + pill_h / 2.0, true);
                }
            }
            ScreenEdge::Right => {
                // Vertical Island Pill flush against right screen edge
                let pill_w = thickness;
                let pill_h = height - 4.0;
                let x = width - pill_w;
                let y = 2.0;

                Self::draw_rounded_rect(cr, x, y, pill_w, pill_h, radius);
                cr.set_source_rgba(r, g, b, alpha);
                let _ = cr.fill_preserve();

                // Subtle rim border
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.18);
                cr.set_line_width(1.0);
                let _ = cr.stroke();

                // Tactile grip dots if unlocked
                if !state.config.hotspot_locked {
                    Self::draw_grip_dots(cr, x + pill_w / 2.0, height / 2.0, false);
                }
            }
            ScreenEdge::Left => {
                // Vertical Island Pill flush against left screen edge
                let pill_w = thickness;
                let pill_h = height - 4.0;
                let x = 0.0;
                let y = 2.0;

                Self::draw_rounded_rect(cr, x, y, pill_w, pill_h, radius);
                cr.set_source_rgba(r, g, b, alpha);
                let _ = cr.fill_preserve();

                // Subtle rim border
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.18);
                cr.set_line_width(1.0);
                let _ = cr.stroke();

                // Tactile grip dots if unlocked
                if !state.config.hotspot_locked {
                    Self::draw_grip_dots(cr, x + pill_w / 2.0, height / 2.0, false);
                }
            }
            ScreenEdge::CornerTopRight { .. } => {
                // Morphing L-Shape Bent Pill in Top-Right
                Self::draw_corner_l_shape(cr, width, height, thickness, radius, true, false);
                cr.set_source_rgba(r, g, b, alpha);
                let _ = cr.fill_preserve();

                cr.set_source_rgba(1.0, 1.0, 1.0, 0.22);
                cr.set_line_width(1.0);
                let _ = cr.stroke();
            }
            ScreenEdge::CornerBottomRight { .. } => {
                // Morphing L-Shape Bent Pill in Bottom-Right
                Self::draw_corner_l_shape(cr, width, height, thickness, radius, true, true);
                cr.set_source_rgba(r, g, b, alpha);
                let _ = cr.fill_preserve();

                cr.set_source_rgba(1.0, 1.0, 1.0, 0.22);
                cr.set_line_width(1.0);
                let _ = cr.stroke();
            }
            ScreenEdge::CornerBottomLeft { .. } => {
                // Morphing L-Shape Bent Pill in Bottom-Left
                Self::draw_corner_l_shape(cr, width, height, thickness, radius, false, true);
                cr.set_source_rgba(r, g, b, alpha);
                let _ = cr.fill_preserve();

                cr.set_source_rgba(1.0, 1.0, 1.0, 0.22);
                cr.set_line_width(1.0);
                let _ = cr.stroke();
            }
            ScreenEdge::CornerTopLeft { .. } => {
                // Morphing L-Shape Bent Pill in Top-Left
                Self::draw_corner_l_shape(cr, width, height, thickness, radius, false, false);
                cr.set_source_rgba(r, g, b, alpha);
                let _ = cr.fill_preserve();

                cr.set_source_rgba(1.0, 1.0, 1.0, 0.22);
                cr.set_line_width(1.0);
                let _ = cr.stroke();
            }
        }

        let _ = cr.restore();
    }

    fn draw_rounded_rect(cr: &Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
        use std::f64::consts::PI;
        cr.new_sub_path();
        cr.arc(x + w - r, y + r, r, -PI / 2.0, 0.0);
        cr.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
        cr.arc(x + r, y + h - r, r, PI / 2.0, PI);
        cr.arc(x + r, y + r, r, PI, 3.0 * PI / 2.0);
        cr.close_path();
    }

    fn draw_corner_l_shape(
        cr: &Context,
        width: f64,
        height: f64,
        thickness: f64,
        radius: f64,
        is_right: bool,
        is_bottom: bool,
    ) {
        use std::f64::consts::PI;
        cr.new_sub_path();

        let x0 = 0.0;
        let y0 = 0.0;
        let x1 = width;
        let y1 = height;

        if is_right && !is_bottom {
            // Top-Right Corner flush against top & right bezel
            cr.move_to(x0, y0 + thickness);
            cr.line_to(x0, y0);
            cr.arc(x1 - radius, y0 + radius, radius, -PI / 2.0, 0.0);
            cr.line_to(x1, y1);
            cr.line_to(x1 - thickness, y1);
            cr.line_to(x1 - thickness, y0 + thickness);
            cr.close_path();
        } else if is_right && is_bottom {
            // Bottom-Right Corner flush against bottom & right bezel
            cr.move_to(x1, y0);
            cr.line_to(x1, y1 - radius);
            cr.arc(x1 - radius, y1 - radius, radius, 0.0, PI / 2.0);
            cr.line_to(x0, y1);
            cr.line_to(x0, y1 - thickness);
            cr.line_to(x1 - thickness, y1 - thickness);
            cr.line_to(x1 - thickness, y0);
            cr.close_path();
        } else if !is_right && is_bottom {
            // Bottom-Left Corner flush against bottom & left bezel
            cr.move_to(x0, y0);
            cr.line_to(x0, y1 - radius);
            cr.arc(x0 + radius, y1 - radius, radius, PI, PI / 2.0);
            cr.line_to(x1, y1);
            cr.line_to(x1, y1 - thickness);
            cr.line_to(x0 + thickness, y1 - thickness);
            cr.line_to(x0 + thickness, y0);
            cr.close_path();
        } else {
            // Top-Left Corner flush against top & left bezel
            cr.move_to(x0, y1);
            cr.line_to(x0, y0 + radius);
            cr.arc(x0 + radius, y0 + radius, radius, PI, 3.0 * PI / 2.0);
            cr.line_to(x1, y0);
            cr.line_to(x1, y0 + thickness);
            cr.line_to(x0 + thickness, y0 + thickness);
            cr.line_to(x0 + thickness, y1);
            cr.close_path();
        }
    }

    fn draw_grip_dots(cr: &Context, cx: f64, cy: f64, horizontal: bool) {
        use std::f64::consts::PI;
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.55);
        let dot_r = 1.2;
        let spacing = 5.0;

        for i in -1..=1 {
            let (dx, dy) = if horizontal {
                (cx + (i as f64) * spacing, cy)
            } else {
                (cx, cy + (i as f64) * spacing)
            };
            cr.new_sub_path();
            cr.arc(dx, dy, dot_r, 0.0, 2.0 * PI);
            let _ = cr.fill();
        }
    }
}
