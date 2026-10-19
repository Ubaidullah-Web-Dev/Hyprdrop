use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use gtk4::cairo::Context;
use gtk4::prelude::*;
use image::DynamicImage;
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
    image: DynamicImage,
    bgra_cache: Vec<u8>,
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
    parent: &impl IsA<gtk4::Window>,
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

    let rgba = img.to_rgba8();
    let mut bgra_cache = Vec::with_capacity((orig_w * orig_h * 4.0) as usize);
    for p in rgba.pixels() {
        bgra_cache.push(p[2]);
        bgra_cache.push(p[1]);
        bgra_cache.push(p[0]);
        bgra_cache.push(p[3]);
    }

    let state = Rc::new(RefCell::new(CropState {
        source_path: image_path.to_path_buf(),
        image: img,
        bgra_cache,
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
        .transient_for(parent)
        .modal(true)
        .default_width(880)
        .default_height(640)
        .build();

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    root_box.add_css_class("hyprdrop-surface");

    // Header Toolbar
    let header_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    header_box.set_margin_start(16);
    header_box.set_margin_end(16);
    header_box.set_margin_top(12);
    header_box.set_margin_bottom(12);

    let title_lbl = gtk4::Label::new(Some("Crop Image"));
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
            s.image.width() as f64,
            s.image.height() as f64,
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
            s.image.width() as f64,
            s.image.height() as f64,
        );

        let img_x = (x - view_x) / scale;
        let img_y = (y - view_y) / scale;
        let dx = img_x - s.drag_start_x;
        let dy = img_y - s.drag_start_y;
        let img_w = s.image.width() as f64;
        let img_h = s.image.height() as f64;

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

    // Spin Button Listeners for live user pixel inputs
    let state_w = state.clone();
    let canvas_w = canvas.clone();
    let h_spin_w = h_spin.clone();
    w_spin.connect_value_changed(move |spin| {
        let mut s = state_w.borrow_mut();
        if s.is_dragging {
            return;
        }
        let max_w = s.image.width() as f64 - s.crop_x;
        s.crop_w = spin.value().clamp(20.0, max_w);
        if let Some(ratio) = s.preset.ratio() {
            let max_h = s.image.height() as f64 - s.crop_y;
            s.crop_h = (s.crop_w / ratio).clamp(20.0, max_h);
            h_spin_w.set_value(s.crop_h.round());
        }
        drop(s);
        canvas_w.queue_draw();
    });

    let state_h = state.clone();
    let canvas_h = canvas.clone();
    let w_spin_h = w_spin.clone();
    h_spin.connect_value_changed(move |spin| {
        let mut s = state_h.borrow_mut();
        if s.is_dragging {
            return;
        }
        let max_h = s.image.height() as f64 - s.crop_y;
        s.crop_h = spin.value().clamp(20.0, max_h);
        if let Some(ratio) = s.preset.ratio() {
            let max_w = s.image.width() as f64 - s.crop_x;
            s.crop_w = (s.crop_h * ratio).clamp(20.0, max_w);
            w_spin_h.set_value(s.crop_w.round());
        }
        drop(s);
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
        btn.connect_clicked(move |_| {
            for (p_btn, _) in &all_pills {
                p_btn.remove_css_class("active");
            }
            b.add_css_class("active");

            let mut s = state_p.borrow_mut();
            s.preset = preset_val;
            if let Some(ratio) = preset_val.ratio() {
                // Adjust crop_h to match ratio
                let max_h = s.image.height() as f64 - s.crop_y;
                let target_h = (s.crop_w / ratio).min(max_h);
                s.crop_h = target_h.max(20.0);
                h_spin_p.set_value(s.crop_h.round());
            }
            drop(s);
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
    reset_btn.connect_clicked(move |_| {
        let mut s = state_reset.borrow_mut();
        let (orig_w, orig_h) = (s.image.width() as f64, s.image.height() as f64);
        s.crop_w = orig_w * 0.8;
        s.crop_h = orig_h * 0.8;
        s.crop_x = (orig_w - s.crop_w) / 2.0;
        s.crop_y = (orig_h - s.crop_h) / 2.0;
        w_spin_reset.set_value(s.crop_w.round());
        h_spin_reset.set_value(s.crop_h.round());
        drop(s);
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
        let cropped = crop_image(
            &s.image,
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
        win_apply.close();
    });
    bottom_box.append(&apply_btn);

    root_box.append(&bottom_box);
    window.set_child(Some(&root_box));
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
    let img_w = state.image.width() as f64;
    let img_h = state.image.height() as f64;
    let (vx, vy, scale) = get_canvas_transform(canvas_w, canvas_h, img_w, img_h);

    // Dark void canvas background
    cr.set_source_rgb(0.05, 0.05, 0.06);
    let _ = cr.paint();

    // Draw scaled image using cached BGRA
    let bgra = state.bgra_cache.clone();

    if let Ok(surface) = cairo::ImageSurface::create_for_data(
        bgra,
        cairo::Format::ARgb32,
        img_w as i32,
        img_h as i32,
        cairo::Format::ARgb32.stride_for_width(img_w as u32).unwrap_or(img_w as i32 * 4),
    ) {
        cr.save().ok();
        cr.translate(vx, vy);
        cr.scale(scale, scale);
        let _ = cr.set_source_surface(&surface, 0.0, 0.0);
        let _ = cr.paint();
        cr.restore().ok();
    }

    // Screen coordinates for crop box
    let scx = vx + state.crop_x * scale;
    let scy = vy + state.crop_y * scale;
    let scw = state.crop_w * scale;
    let sch = state.crop_h * scale;

    // Darkened exterior mask (rgba(0, 0, 0, 0.65))
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.65);
    // Top
    cr.rectangle(vx, vy, img_w * scale, (scy - vy).max(0.0));
    // Bottom
    cr.rectangle(vx, scy + sch, img_w * scale, (vy + img_h * scale - (scy + sch)).max(0.0));
    // Left
    cr.rectangle(vx, scy, (scx - vx).max(0.0), sch);
    // Right
    cr.rectangle(scx + scw, scy, (vx + img_w * scale - (scx + scw)).max(0.0), sch);
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

    // Orange crop boundary box: #FF5722
    cr.rectangle(scx, scy, scw, sch);
    cr.set_source_rgb(1.0, 0.34, 0.13);
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
        cr.set_source_rgb(1.0, 0.34, 0.13);
        cr.set_line_width(2.0);
        let _ = cr.stroke();
    }
}
