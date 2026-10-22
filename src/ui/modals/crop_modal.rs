use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use gtk4::cairo::Context;
use gtk4::prelude::*;

use crate::config::ConfigManager;
use crate::engine::image_ops::{crop_image, load_image, save_image_with_format};
use crate::engine::naming::get_suffixed_output_path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AspectPreset {
    Free,
    Ratio1x1,
    Ratio16x9,
    Ratio4x3,
    Ratio3x2,
    Ratio9x16,
}

impl AspectPreset {
    pub fn ratio(&self) -> Option<f64> {
        match self {
            Self::Free => None,
            Self::Ratio1x1 => Some(1.0),
            Self::Ratio16x9 => Some(16.0 / 9.0),
            Self::Ratio4x3 => Some(4.0 / 3.0),
            Self::Ratio3x2 => Some(3.0 / 2.0),
            Self::Ratio9x16 => Some(9.0 / 16.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DragHandle {
    None,
    Inside,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Top,
    Bottom,
    Left,
    Right,
}

struct CropState {
    source_path: PathBuf,
    preview_surface: cairo::ImageSurface,
    preview_w: f64,
    preview_h: f64,
    orig_w: f64,
    orig_h: f64,
    preset: AspectPreset,
    crop_x: f64,
    crop_y: f64,
    crop_w: f64,
    crop_h: f64,
    active_handle: DragHandle,
    drag_start_x: f64,
    drag_start_y: f64,
    init_crop_x: f64,
    init_crop_y: f64,
    init_crop_w: f64,
    init_crop_h: f64,
    is_dragging: bool,
}

pub fn open_crop_modal(
    app: Option<&gtk4::Application>,
    image_path: &Path,
    on_success: impl Fn(PathBuf) + 'static,
) {
    let img = match load_image(image_path) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Error opening image for crop: {:?}", e);
            return;
        }
    };

    let (orig_w, orig_h) = (img.width() as f64, img.height() as f64);
    let init_w = orig_w * 0.8;
    let init_h = orig_h * 0.8;
    let init_x = (orig_w - init_w) / 2.0;
    let init_y = (orig_h - init_h) / 2.0;

    let max_dim = orig_w.max(orig_h);
    let preview_scale = (800.0 / max_dim).min(1.0);
    let preview_img = if preview_scale < 1.0 {
        img.resize(
            (orig_w * preview_scale).round() as u32,
            (orig_h * preview_scale).round() as u32,
            image::imageops::FilterType::Triangle,
        )
    } else {
        img
    };

    let preview_w = preview_img.width() as f64;
    let preview_h = preview_img.height() as f64;
    let rgba = preview_img.to_rgba8();
    let mut bgra = Vec::with_capacity((preview_w * preview_h * 4.0) as usize);
    for p in rgba.pixels() {
        bgra.push(p[2]);
        bgra.push(p[1]);
        bgra.push(p[0]);
        bgra.push(p[3]);
    }
    let preview_surface = cairo::ImageSurface::create_for_data(
        bgra,
        cairo::Format::ARgb32,
        preview_w as i32,
        preview_h as i32,
        cairo::Format::ARgb32.stride_for_width(preview_w as u32).unwrap_or(preview_w as i32 * 4),
    ).expect("Failed to create preview surface");

    let state = Rc::new(RefCell::new(CropState {
        source_path: image_path.to_path_buf(),
        preview_surface,
        preview_w,
        preview_h,
        orig_w,
        orig_h,
        preset: AspectPreset::Free,
        crop_x: init_x,
        crop_y: init_y,
        crop_w: init_w,
        crop_h: init_h,
        active_handle: DragHandle::None,
        drag_start_x: 0.0,
        drag_start_y: 0.0,
        init_crop_x: init_x,
        init_crop_y: init_y,
        init_crop_w: init_w,
        init_crop_h: init_h,
        is_dragging: false,
    }));

    let window = gtk4::Window::builder()
        .title("Crop Image — Hyprdrop")
        .default_width(880)
        .default_height(640)
        .build();

    if let Some(a) = app {
        a.add_window(&window);
    }

    window.add_css_class("hyprdrop-surface");

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    root_box.add_css_class("hyprdrop-surface");

    // Header Toolbar
    let header_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    header_box.set_margin_start(18);
    header_box.set_margin_end(18);
    header_box.set_margin_top(14);
    header_box.set_margin_bottom(12);

    let title_icon = gtk4::Image::from_icon_name("transform-crop-and-rotate-symbolic");
    title_icon.set_pixel_size(18);
    title_icon.add_css_class("hyprdrop-brand-icon");
    header_box.append(&title_icon);

    let title_lbl = gtk4::Label::new(Some("Crop Studio"));
    title_lbl.add_css_class("hyprdrop-title");
    header_box.append(&title_lbl);

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    header_box.append(&spacer);

    // Aspect Ratio Presets
    let presets = [
        ("Free", AspectPreset::Free),
        ("1:1", AspectPreset::Ratio1x1),
        ("16:9", AspectPreset::Ratio16x9),
        ("4:3", AspectPreset::Ratio4x3),
        ("3:2", AspectPreset::Ratio3x2),
        ("9:16", AspectPreset::Ratio9x16),
    ];

    let preset_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    let mut pill_buttons = Vec::new();

    for (name, preset_val) in presets {
        let btn = gtk4::Button::with_label(name);
        btn.add_css_class("hyprdrop-pill");
        if preset_val == AspectPreset::Free {
            btn.add_css_class("active");
        }
        preset_box.append(&btn);
        pill_buttons.push((btn, preset_val));
    }
    header_box.append(&preset_box);

    // Live editable pixel dimensions (Width x Height)
    let dim_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    dim_box.set_valign(gtk4::Align::Center);

    let w_lbl = gtk4::Label::new(Some("W:"));
    w_lbl.add_css_class("hyprdrop-subtitle");
    dim_box.append(&w_lbl);

    let w_spin = gtk4::SpinButton::with_range(20.0, orig_w, 1.0);
    w_spin.set_value(init_w.round());
    w_spin.add_css_class("hyprdrop-mono");
    dim_box.append(&w_spin);

    let x_lbl = gtk4::Label::new(Some("×"));
    x_lbl.add_css_class("hyprdrop-subtitle");
    dim_box.append(&x_lbl);

    let h_lbl = gtk4::Label::new(Some("H:"));
    h_lbl.add_css_class("hyprdrop-subtitle");
    dim_box.append(&h_lbl);

    let h_spin = gtk4::SpinButton::with_range(20.0, orig_h, 1.0);
    h_spin.set_value(init_h.round());
    h_spin.add_css_class("hyprdrop-mono");
    dim_box.append(&h_spin);

    let px_lbl = gtk4::Label::new(Some("px"));
    px_lbl.add_css_class("hyprdrop-subtitle");
    dim_box.append(&px_lbl);

    header_box.append(&dim_box);

    let close_btn = gtk4::Button::from_icon_name("window-close-symbolic");
    close_btn.add_css_class("hyprdrop-ghost");
    close_btn.add_css_class("hyprdrop-ghost-danger");
    close_btn.set_tooltip_text(Some("Close"));
    let win_close = window.clone();
    close_btn.connect_clicked(move |_| win_close.close());
    header_box.append(&close_btn);

    root_box.append(&header_box);

    // Preview Drawing Area
    let canvas = gtk4::DrawingArea::builder()
        .vexpand(true)
        .hexpand(true)
        .focusable(true)
        .build();

    let state_draw = state.clone();
    canvas.set_draw_func(move |_, cr, w, h| {
        let s = state_draw.borrow();
        draw_crop_canvas(cr, w as f64, h as f64, &s);
    });

    // Gesture Click for handle grabbing
    let click = gtk4::GestureClick::new();
    let state_click = state.clone();
    let canvas_click = canvas.clone();
    click.connect_pressed(move |_, _, x, y| {
        let mut s = state_click.borrow_mut();
        let (view_x, view_y, scale) = get_canvas_transform(
            canvas_click.width() as f64,
            canvas_click.height() as f64,
            s.orig_w,
            s.orig_h,
        );

        let img_x = (x - view_x) / scale;
        let img_y = (y - view_y) / scale;

        s.active_handle = get_hit_handle(img_x, img_y, s.crop_x, s.crop_y, s.crop_w, s.crop_h, 14.0 / scale);
        s.drag_start_x = img_x;
        s.drag_start_y = img_y;
        s.init_crop_x = s.crop_x;
        s.init_crop_y = s.crop_y;
        s.init_crop_w = s.crop_w;
        s.init_crop_h = s.crop_h;
        s.is_dragging = s.active_handle != DragHandle::None;
        drop(s);
        canvas_click.queue_draw();
    });

    let state_release = state.clone();
    let canvas_release = canvas.clone();
    click.connect_released(move |_, _, _, _| {
        let mut s = state_release.borrow_mut();
        s.active_handle = DragHandle::None;
        s.is_dragging = false;
        drop(s);
        canvas_release.queue_draw();
    });
    canvas.add_controller(click);

    // Drag Motion
    let motion = gtk4::EventControllerMotion::new();
    let state_motion = state.clone();
    let canvas_motion = canvas.clone();
    let w_spin_motion = w_spin.clone();
    let h_spin_motion = h_spin.clone();
    motion.connect_motion(move |_, x, y| {
        let mut s = state_motion.borrow_mut();
        if !s.is_dragging || s.active_handle == DragHandle::None {
            return;
        }

        let (view_x, view_y, scale) = get_canvas_transform(
            canvas_motion.width() as f64,
            canvas_motion.height() as f64,
            s.orig_w,
            s.orig_h,
        );

        let img_x = (x - view_x) / scale;
        let img_y = (y - view_y) / scale;
        let dx = img_x - s.drag_start_x;
        let dy = img_y - s.drag_start_y;
        let img_w = s.orig_w;
        let img_h = s.orig_h;

        update_crop_rect(&mut s, dx, dy, img_w, img_h);
        let cur_w = s.crop_w.round();
        let cur_h = s.crop_h.round();
        drop(s);
        if (w_spin_motion.value() - cur_w).abs() >= 1.0 {
            w_spin_motion.set_value(cur_w);
        }
        if (h_spin_motion.value() - cur_h).abs() >= 1.0 {
            h_spin_motion.set_value(cur_h);
        }
        canvas_motion.queue_draw();
    });
    canvas.add_controller(motion);

    let is_updating_spin = Rc::new(std::cell::Cell::new(false));

    // Spin Button Listeners for live user pixel inputs
    let state_w = state.clone();
    let canvas_w = canvas.clone();
    let h_spin_w = h_spin.clone();
    let is_updating_spin_w = is_updating_spin.clone();
    w_spin.connect_value_changed(move |spin| {
        if is_updating_spin_w.get() {
            return;
        }
        is_updating_spin_w.set(true);
        let ratio_h = {
            let mut s = state_w.borrow_mut();
            if s.is_dragging {
                is_updating_spin_w.set(false);
                return;
            }
            let max_w = s.orig_w - s.crop_x;
            s.crop_w = spin.value().clamp(20.0, max_w);
            s.preset.ratio().map(|ratio| {
                let max_h = s.orig_h - s.crop_y;
                let target_h = (s.crop_w / ratio).clamp(20.0, max_h);
                s.crop_h = target_h;
                target_h
            })
        };
        if let Some(h) = ratio_h {
            h_spin_w.set_value(h.round());
        }
        is_updating_spin_w.set(false);
        canvas_w.queue_draw();
    });

    let state_h = state.clone();
    let canvas_h = canvas.clone();
    let w_spin_h = w_spin.clone();
    let is_updating_spin_h = is_updating_spin.clone();
    h_spin.connect_value_changed(move |spin| {
        if is_updating_spin_h.get() {
            return;
        }
        is_updating_spin_h.set(true);
        let ratio_w = {
            let mut s = state_h.borrow_mut();
            if s.is_dragging {
                is_updating_spin_h.set(false);
                return;
            }
            let max_h = s.orig_h - s.crop_y;
            s.crop_h = spin.value().clamp(20.0, max_h);
            s.preset.ratio().map(|ratio| {
                let max_w = s.orig_w - s.crop_x;
                let target_w = (s.crop_h * ratio).clamp(20.0, max_w);
                s.crop_w = target_w;
                target_w
            })
        };
        if let Some(w) = ratio_w {
            w_spin_h.set_value(w.round());
        }
        is_updating_spin_h.set(false);
        canvas_h.queue_draw();
    });

    root_box.append(&canvas);

    // Preset Pill Connections
    for (btn, preset_val) in pill_buttons.clone() {
        let state_p = state.clone();
        let canvas_p = canvas.clone();
        let all_pills = pill_buttons.clone();
        let b = btn.clone();
        let h_spin_p = h_spin.clone();
        let is_updating_spin_p = is_updating_spin.clone();
        btn.connect_clicked(move |_| {
            for (p_btn, _) in &all_pills {
                p_btn.remove_css_class("active");
            }
            b.add_css_class("active");

            is_updating_spin_p.set(true);
            let target_h = {
                let mut s = state_p.borrow_mut();
                s.preset = preset_val;
                s.preset.ratio().map(|ratio| {
                    let max_h = s.orig_h - s.crop_y;
                    let h = (s.crop_w / ratio).min(max_h).max(20.0);
                    s.crop_h = h;
                    h
                })
            };
            if let Some(h) = target_h {
                h_spin_p.set_value(h.round());
            }
            is_updating_spin_p.set(false);
            canvas_p.queue_draw();
        });
    }

    // Bottom Action Bar
    let bottom_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    bottom_box.set_margin_start(16);
    bottom_box.set_margin_end(16);
    bottom_box.set_margin_top(12);
    bottom_box.set_margin_bottom(12);

    let reset_btn = gtk4::Button::with_label("Reset");
    reset_btn.add_css_class("hyprdrop-secondary");
    let state_reset = state.clone();
    let canvas_reset = canvas.clone();
    let w_spin_reset = w_spin.clone();
    let h_spin_reset = h_spin.clone();
    let is_updating_spin_reset = is_updating_spin.clone();
    reset_btn.connect_clicked(move |_| {
        is_updating_spin_reset.set(true);
        let (w_val, h_val) = {
            let mut s = state_reset.borrow_mut();
            let (orig_w, orig_h) = (s.orig_w, s.orig_h);
            s.crop_w = orig_w * 0.8;
            s.crop_h = orig_h * 0.8;
            s.crop_x = (orig_w - s.crop_w) / 2.0;
            s.crop_y = (orig_h - s.crop_h) / 2.0;
            (s.crop_w, s.crop_h)
        };
        w_spin_reset.set_value(w_val.round());
        h_spin_reset.set_value(h_val.round());
        is_updating_spin_reset.set(false);
        canvas_reset.queue_draw();
    });
    bottom_box.append(&reset_btn);

    let b_spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    b_spacer.set_hexpand(true);
    bottom_box.append(&b_spacer);

    let cancel_btn = gtk4::Button::with_label("Cancel");
    cancel_btn.add_css_class("hyprdrop-secondary");
    let win_cancel = window.clone();
    cancel_btn.connect_clicked(move |_| {
        win_cancel.close();
    });
    bottom_box.append(&cancel_btn);

    let apply_btn = gtk4::Button::with_label("Apply Crop");
    apply_btn.add_css_class("hyprdrop-primary");
    let win_apply = window.clone();
    let state_apply = state.clone();
    let on_success_rc = Rc::new(on_success);
    apply_btn.connect_clicked(move |_| {
        let s = state_apply.borrow();
        if let Ok(full_img) = load_image(&s.source_path) {
            let cropped = crop_image(
                &full_img,
                s.crop_x.round() as u32,
                s.crop_y.round() as u32,
                s.crop_w.round() as u32,
                s.crop_h.round() as u32,
            );

            let out_path = get_suffixed_output_path(&s.source_path, "Cropped", None);
            let ext = out_path.extension().and_then(|e| e.to_str()).unwrap_or("png");
            if let Ok(()) = save_image_with_format(&cropped, &out_path, ext) {
                let _ = ConfigManager::append_history("CROP", &s.source_path, &out_path, None);
                on_success_rc(out_path);
            }
        }
        win_apply.close();
        crate::engine::trim_memory();
    });
    bottom_box.append(&apply_btn);

    root_box.append(&bottom_box);

    window.connect_destroy(move |_| {
        crate::engine::trim_memory();
    });

    window.set_child(Some(&root_box));
    crate::hyprland::ensure_hyprland_floating_rules();
    window.present();
}

fn get_canvas_transform(canvas_w: f64, canvas_h: f64, img_w: f64, img_h: f64) -> (f64, f64, f64) {
    let scale_x = (canvas_w - 40.0) / img_w;
    let scale_y = (canvas_h - 40.0) / img_h;
    let scale = scale_x.min(scale_y).clamp(0.01, 1.0);
    let view_w = img_w * scale;
    let view_h = img_h * scale;
    let view_x = (canvas_w - view_w) / 2.0;
    let view_y = (canvas_h - view_h) / 2.0;
    (view_x, view_y, scale)
}

fn get_hit_handle(
    x: f64,
    y: f64,
    cx: f64,
    cy: f64,
    cw: f64,
    ch: f64,
    threshold: f64,
) -> DragHandle {
    let near = |px: f64, py: f64| -> bool {
        (x - px).abs() <= threshold && (y - py).abs() <= threshold
    };

    if near(cx, cy) { return DragHandle::TopLeft; }
    if near(cx + cw, cy) { return DragHandle::TopRight; }
    if near(cx, cy + ch) { return DragHandle::BottomLeft; }
    if near(cx + cw, cy + ch) { return DragHandle::BottomRight; }
    if near(cx + cw / 2.0, cy) { return DragHandle::Top; }
    if near(cx + cw / 2.0, cy + ch) { return DragHandle::Bottom; }
    if near(cx, cy + ch / 2.0) { return DragHandle::Left; }
    if near(cx + cw, cy + ch / 2.0) { return DragHandle::Right; }

    if x >= cx && x <= cx + cw && y >= cy && y <= cy + ch {
        return DragHandle::Inside;
    }
    DragHandle::None
}

fn update_crop_rect(s: &mut CropState, dx: f64, dy: f64, img_w: f64, img_h: f64) {
    let min_dim = 24.0;
    match s.active_handle {
        DragHandle::Inside => {
            s.crop_x = (s.init_crop_x + dx).clamp(0.0, img_w - s.crop_w);
            s.crop_y = (s.init_crop_y + dy).clamp(0.0, img_h - s.crop_h);
        }
        DragHandle::TopLeft => {
            let new_x = (s.init_crop_x + dx).clamp(0.0, s.init_crop_x + s.init_crop_w - min_dim);
            let new_y = (s.init_crop_y + dy).clamp(0.0, s.init_crop_y + s.init_crop_h - min_dim);
            s.crop_w = s.init_crop_w - (new_x - s.init_crop_x);
            s.crop_h = s.init_crop_h - (new_y - s.init_crop_y);
            s.crop_x = new_x;
            s.crop_y = new_y;
        }
        DragHandle::BottomRight => {
            s.crop_w = (s.init_crop_w + dx).clamp(min_dim, img_w - s.crop_x);
            s.crop_h = (s.init_crop_h + dy).clamp(min_dim, img_h - s.crop_y);
        }
        DragHandle::TopRight => {
            let new_y = (s.init_crop_y + dy).clamp(0.0, s.init_crop_y + s.init_crop_h - min_dim);
            s.crop_w = (s.init_crop_w + dx).clamp(min_dim, img_w - s.crop_x);
            s.crop_h = s.init_crop_h - (new_y - s.init_crop_y);
            s.crop_y = new_y;
        }
        DragHandle::BottomLeft => {
            let new_x = (s.init_crop_x + dx).clamp(0.0, s.init_crop_x + s.init_crop_w - min_dim);
            s.crop_w = s.init_crop_w - (new_x - s.init_crop_x);
            s.crop_h = (s.init_crop_h + dy).clamp(min_dim, img_h - s.crop_y);
            s.crop_x = new_x;
        }
        DragHandle::Top => {
            let new_y = (s.init_crop_y + dy).clamp(0.0, s.init_crop_y + s.init_crop_h - min_dim);
            s.crop_h = s.init_crop_h - (new_y - s.init_crop_y);
            s.crop_y = new_y;
        }
        DragHandle::Bottom => {
            s.crop_h = (s.init_crop_h + dy).clamp(min_dim, img_h - s.crop_y);
        }
        DragHandle::Left => {
            let new_x = (s.init_crop_x + dx).clamp(0.0, s.init_crop_x + s.init_crop_w - min_dim);
            s.crop_w = s.init_crop_w - (new_x - s.init_crop_x);
            s.crop_x = new_x;
        }
        DragHandle::Right => {
            s.crop_w = (s.init_crop_w + dx).clamp(min_dim, img_w - s.crop_x);
        }
        DragHandle::None => {}
    }

    if let Some(ratio) = s.preset.ratio() {
        s.crop_h = (s.crop_w / ratio).min(img_h - s.crop_y);
    }
}

fn draw_crop_canvas(cr: &Context, canvas_w: f64, canvas_h: f64, state: &CropState) {
    let (vx, vy, scale) = get_canvas_transform(canvas_w, canvas_h, state.orig_w, state.orig_h);

    // Dark void canvas background
    cr.set_source_rgb(0.05, 0.05, 0.06);
    let _ = cr.paint();

    // Draw scaled preview surface
    cr.save().ok();
    cr.translate(vx, vy);
    let surf_scale_x = (state.orig_w * scale) / state.preview_w;
    let surf_scale_y = (state.orig_h * scale) / state.preview_h;
    cr.scale(surf_scale_x, surf_scale_y);
    let _ = cr.set_source_surface(&state.preview_surface, 0.0, 0.0);
    let _ = cr.paint();
    cr.restore().ok();

    // Screen coordinates for crop box
    let scx = vx + state.crop_x * scale;
    let scy = vy + state.crop_y * scale;
    let scw = state.crop_w * scale;
    let sch = state.crop_h * scale;

    // Darkened exterior mask (rgba(0, 0, 0, 0.65))
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.65);
    // Top
    cr.rectangle(vx, vy, state.orig_w * scale, (scy - vy).max(0.0));
    // Bottom
    cr.rectangle(vx, scy + sch, state.orig_w * scale, (vy + state.orig_h * scale - (scy + sch)).max(0.0));
    // Left
    cr.rectangle(vx, scy, (scx - vx).max(0.0), sch);
    // Right
    cr.rectangle(scx + scw, scy, (vx + state.orig_w * scale - (scx + scw)).max(0.0), sch);
    let _ = cr.fill();

    // Rule-of-thirds gridlines
    if state.is_dragging {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.25);
        cr.set_line_width(1.0);
        for i in 1..=2 {
            let gx = scx + (scw / 3.0) * (i as f64);
            let gy = scy + (sch / 3.0) * (i as f64);
            cr.move_to(gx, scy);
            cr.line_to(gx, scy + sch);
            cr.move_to(scx, gy);
            cr.line_to(scx + scw, gy);
        }
        let _ = cr.stroke();
    }

    // macOS Electric Blue crop boundary box: #0A84FF
    cr.rectangle(scx, scy, scw, sch);
    cr.set_source_rgb(0.04, 0.52, 1.0);
    cr.set_line_width(2.0);
    let _ = cr.stroke();

    // 8 Draggable Handles
    let hr = 5.0;
    let handles = [
        (scx, scy),
        (scx + scw, scy),
        (scx, scy + sch),
        (scx + scw, scy + sch),
        (scx + scw / 2.0, scy),
        (scx + scw / 2.0, scy + sch),
        (scx, scy + sch / 2.0),
        (scx + scw, scy + sch / 2.0),
    ];

    for (hx, hy) in handles {
        cr.arc(hx, hy, hr, 0.0, 2.0 * std::f64::consts::PI);
        cr.set_source_rgb(1.0, 1.0, 1.0);
        let _ = cr.fill_preserve();
        cr.set_source_rgb(0.04, 0.52, 1.0);
        cr.set_line_width(2.0);
        let _ = cr.stroke();
    }
}
