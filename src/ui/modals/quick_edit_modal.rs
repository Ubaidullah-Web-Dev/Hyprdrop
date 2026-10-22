use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use gtk4::cairo::Context;
use gtk4::prelude::*;
use image::DynamicImage;
use crate::config::ConfigManager;
use crate::engine::image_ops::{
    adjust_brightness_contrast, auto_trim_transparent, load_image, save_image_with_format,
};
use crate::engine::naming::get_suffixed_output_path;

#[derive(Clone, Copy, Debug)]
enum TransformOp {
    Rotate90,
    Rotate180,
    FlipH,
    FlipV,
    Scale(f64),
    AutoTrim,
}

struct EditState {
    source_path: PathBuf,
    initial_preview: DynamicImage,
    preview_image: DynamicImage,
    orig_w: u32,
    orig_h: u32,
    cur_full_w: u32,
    cur_full_h: u32,
    ops: Vec<TransformOp>,
    brightness: i32,
    contrast: f32,
}

pub fn open_quick_edit_modal(
    app: Option<&gtk4::Application>,
    image_path: &Path,
    on_success: impl Fn(PathBuf) + 'static,
) {
    let img = match load_image(image_path) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Error loading image for Quick Edit: {:?}", e);
            return;
        }
    };

    let (init_w, init_h) = (img.width(), img.height());
    let max_dim = (init_w.max(init_h) as f64).max(1.0);
    let preview_scale = (720.0 / max_dim).min(1.0);
    let preview_image = if preview_scale < 1.0 {
        img.resize(
            ((init_w as f64) * preview_scale).round() as u32,
            ((init_h as f64) * preview_scale).round() as u32,
            image::imageops::FilterType::Triangle,
        )
    } else {
        img
    };

    let state = Rc::new(RefCell::new(EditState {
        source_path: image_path.to_path_buf(),
        initial_preview: preview_image.clone(),
        preview_image,
        orig_w: init_w,
        orig_h: init_h,
        cur_full_w: init_w,
        cur_full_h: init_h,
        ops: Vec::new(),
        brightness: 0,
        contrast: 0.0,
    }));

    let window = gtk4::Window::builder()
        .title("Quick Edit Studio — Hyprdrop")
        .default_width(860)
        .default_height(600)
        .build();

    if let Some(a) = app {
        a.add_window(&window);
    }

    window.add_css_class("hyprdrop-surface");

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    root_box.add_css_class("hyprdrop-surface");

    // macOS Header
    let header_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    header_box.set_margin_start(18);
    header_box.set_margin_end(18);
    header_box.set_margin_top(14);
    header_box.set_margin_bottom(12);

    let title_icon = gtk4::Image::from_icon_name("preferences-color-symbolic");
    title_icon.set_pixel_size(18);
    title_icon.add_css_class("hyprdrop-brand-icon");
    header_box.append(&title_icon);

    let header_title = gtk4::Label::new(Some("Quick Adjustments Studio"));
    header_title.add_css_class("hyprdrop-title");
    header_title.set_hexpand(true);
    header_title.set_halign(gtk4::Align::Start);
    header_box.append(&header_title);

    let right_close = gtk4::Button::from_icon_name("window-close-symbolic");
    right_close.add_css_class("hyprdrop-ghost");
    right_close.add_css_class("hyprdrop-ghost-danger");
    right_close.set_tooltip_text(Some("Close"));
    let win_close2 = window.clone();
    right_close.connect_clicked(move |_| win_close2.close());
    header_box.append(&right_close);

    root_box.append(&header_box);

    let body_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    body_box.set_vexpand(true);
    body_box.set_hexpand(true);

    // Preview canvas
    let canvas_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    canvas_box.set_hexpand(true);
    canvas_box.set_vexpand(true);

    let canvas = gtk4::DrawingArea::builder()
        .hexpand(true)
        .vexpand(true)
        .build();

    let state_draw = state.clone();
    canvas.set_draw_func(move |_, cr, w, h| {
        let s = state_draw.borrow();
        draw_edit_canvas(cr, w as f64, h as f64, &s);
    });
    canvas_box.append(&canvas);
    body_box.append(&canvas_box);

    // Sidebar
    let sidebar = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    sidebar.set_width_request(280);
    sidebar.set_margin_start(16);
    sidebar.set_margin_end(16);
    sidebar.set_margin_top(16);
    sidebar.set_margin_bottom(16);
    sidebar.add_css_class("mac-card");

    let title_lbl = gtk4::Label::new(Some("Quick Adjustments"));
    title_lbl.add_css_class("hyprdrop-title");
    title_lbl.set_halign(gtk4::Align::Start);
    sidebar.append(&title_lbl);

    let dim_lbl = gtk4::Label::new(Some(&format!("{}×{} px", init_w, init_h)));
    dim_lbl.add_css_class("hyprdrop-mono");
    dim_lbl.set_hexpand(true);
    dim_lbl.set_halign(gtk4::Align::End);

    // Rotation & Flip buttons
    let transform_lbl = gtk4::Label::new(Some("Orientation"));
    transform_lbl.add_css_class("hyprdrop-subtitle");
    transform_lbl.set_halign(gtk4::Align::Start);
    sidebar.append(&transform_lbl);

    let t_row1 = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    let rot90_btn = gtk4::Button::with_label("Rotate 90°");
    rot90_btn.add_css_class("hyprdrop-secondary");
    rot90_btn.set_hexpand(true);
    let state_rot = state.clone();
    let canvas_rot = canvas.clone();
    let dim_lbl_rot = dim_lbl.clone();
    rot90_btn.connect_clicked(move |_| {
        let mut s = state_rot.borrow_mut();
        s.preview_image = s.preview_image.rotate90();
        s.ops.push(TransformOp::Rotate90);
        let tmp = s.cur_full_w;
        s.cur_full_w = s.cur_full_h;
        s.cur_full_h = tmp;
        dim_lbl_rot.set_text(&format!("{}×{} px", s.cur_full_w, s.cur_full_h));
        drop(s);
        canvas_rot.queue_draw();
    });
    t_row1.append(&rot90_btn);

    let rot180_btn = gtk4::Button::with_label("Rotate 180°");
    rot180_btn.add_css_class("hyprdrop-secondary");
    rot180_btn.set_hexpand(true);
    let state_rot180 = state.clone();
    let canvas_rot180 = canvas.clone();
    rot180_btn.connect_clicked(move |_| {
        let mut s = state_rot180.borrow_mut();
        s.preview_image = s.preview_image.rotate180();
        s.ops.push(TransformOp::Rotate180);
        drop(s);
        canvas_rot180.queue_draw();
    });
    t_row1.append(&rot180_btn);
    sidebar.append(&t_row1);

    let t_row2 = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    let fliph_btn = gtk4::Button::with_label("Flip Horiz");
    fliph_btn.add_css_class("hyprdrop-secondary");
    fliph_btn.set_hexpand(true);
    let state_fliph = state.clone();
    let canvas_fliph = canvas.clone();
    fliph_btn.connect_clicked(move |_| {
        let mut s = state_fliph.borrow_mut();
        s.preview_image = s.preview_image.fliph();
        s.ops.push(TransformOp::FlipH);
        drop(s);
        canvas_fliph.queue_draw();
    });
    t_row2.append(&fliph_btn);

    let flipv_btn = gtk4::Button::with_label("Flip Vert");
    flipv_btn.add_css_class("hyprdrop-secondary");
    flipv_btn.set_hexpand(true);
    let state_flipv = state.clone();
    let canvas_flipv = canvas.clone();
    flipv_btn.connect_clicked(move |_| {
        let mut s = state_flipv.borrow_mut();
        s.preview_image = s.preview_image.flipv();
        s.ops.push(TransformOp::FlipV);
        drop(s);
        canvas_flipv.queue_draw();
    });
    t_row2.append(&flipv_btn);
    sidebar.append(&t_row2);

    // Quick Resize Section
    let resize_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    let resize_lbl = gtk4::Label::new(Some("Quick Resize"));
    resize_lbl.add_css_class("hyprdrop-subtitle");
    resize_header.append(&resize_lbl);
    resize_header.append(&dim_lbl);
    sidebar.append(&resize_header);

    let resize_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    let scales = [
        ("25%", 0.25f64),
        ("50%", 0.50f64),
        ("75%", 0.75f64),
        ("150%", 1.50f64),
        ("200%", 2.00f64),
    ];
    for (label, factor) in scales {
        let btn = gtk4::Button::with_label(label);
        btn.add_css_class("hyprdrop-pill");
        let state_res = state.clone();
        let canvas_res = canvas.clone();
        let dim_lbl_res = dim_lbl.clone();
        btn.connect_clicked(move |_| {
            let mut s = state_res.borrow_mut();
            let new_pw = ((s.preview_image.width() as f64) * factor).round().max(16.0) as u32;
            let new_ph = ((s.preview_image.height() as f64) * factor).round().max(16.0) as u32;
            s.preview_image = s.preview_image.resize(new_pw, new_ph, image::imageops::FilterType::Lanczos3);
            s.ops.push(TransformOp::Scale(factor));
            s.cur_full_w = ((s.cur_full_w as f64) * factor).round().max(16.0) as u32;
            s.cur_full_h = ((s.cur_full_h as f64) * factor).round().max(16.0) as u32;
            dim_lbl_res.set_text(&format!("{}×{} px", s.cur_full_w, s.cur_full_h));
            drop(s);
            canvas_res.queue_draw();
        });
        resize_box.append(&btn);
    }
    sidebar.append(&resize_box);
    sidebar.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

    // Auto-Trim Transparent
    let trim_btn = gtk4::Button::with_label("Auto-Trim Transparent");
    trim_btn.add_css_class("hyprdrop-pill");
    let state_trim = state.clone();
    let canvas_trim = canvas.clone();
    let dim_lbl_trim = dim_lbl.clone();
    trim_btn.connect_clicked(move |_| {
        let mut s = state_trim.borrow_mut();
        let orig_pw = s.preview_image.width().max(1) as f64;
        let orig_ph = s.preview_image.height().max(1) as f64;
        let trimmed = auto_trim_transparent(&s.preview_image.to_rgba8());
        let factor_w = trimmed.width() as f64 / orig_pw;
        let factor_h = trimmed.height() as f64 / orig_ph;
        s.preview_image = DynamicImage::ImageRgba8(trimmed);
        s.ops.push(TransformOp::AutoTrim);
        s.cur_full_w = ((s.cur_full_w as f64) * factor_w).round().max(1.0) as u32;
        s.cur_full_h = ((s.cur_full_h as f64) * factor_h).round().max(1.0) as u32;
        dim_lbl_trim.set_text(&format!("{}×{} px", s.cur_full_w, s.cur_full_h));
        drop(s);
        canvas_trim.queue_draw();
    });
    sidebar.append(&trim_btn);

    // Brightness Slider
    let b_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    let b_title = gtk4::Label::new(Some("Brightness"));
    b_title.add_css_class("hyprdrop-subtitle");
    let b_val = gtk4::Label::new(Some("0"));
    b_val.add_css_class("hyprdrop-mono");
    b_val.set_hexpand(true);
    b_val.set_halign(gtk4::Align::End);
    b_header.append(&b_title);
    b_header.append(&b_val);
    sidebar.append(&b_header);

    let b_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, -100.0, 100.0, 5.0);
    b_scale.set_value(0.0);
    let state_b = state.clone();
    let canvas_b = canvas.clone();
    let b_val_clone = b_val.clone();
    b_scale.connect_value_changed(move |scale| {
        let val = scale.value().round() as i32;
        b_val_clone.set_text(&format!("{}", val));
        state_b.borrow_mut().brightness = val;
        canvas_b.queue_draw();
    });
    sidebar.append(&b_scale);

    // Contrast Slider
    let c_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    let c_title = gtk4::Label::new(Some("Contrast"));
    c_title.add_css_class("hyprdrop-subtitle");
    let c_val = gtk4::Label::new(Some("0.0"));
    c_val.add_css_class("hyprdrop-mono");
    c_val.set_hexpand(true);
    c_val.set_halign(gtk4::Align::End);
    c_header.append(&c_title);
    c_header.append(&c_val);
    sidebar.append(&c_header);

    let c_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, -50.0, 50.0, 2.0);
    c_scale.set_value(0.0);
    let state_c = state.clone();
    let canvas_c = canvas.clone();
    let c_val_clone = c_val.clone();
    c_scale.connect_value_changed(move |scale| {
        let val = scale.value() as f32;
        c_val_clone.set_text(&format!("{:.1}", val));
        state_c.borrow_mut().contrast = val;
        canvas_c.queue_draw();
    });
    sidebar.append(&c_scale);

    // Reset button
    let reset_btn = gtk4::Button::with_label("Reset All");
    reset_btn.add_css_class("hyprdrop-secondary");
    let state_reset = state.clone();
    let canvas_reset = canvas.clone();
    let b_scale_reset = b_scale.clone();
    let c_scale_reset = c_scale.clone();
    let dim_lbl_reset = dim_lbl.clone();
    reset_btn.connect_clicked(move |_| {
        let mut s = state_reset.borrow_mut();
        s.preview_image = s.initial_preview.clone();
        s.ops.clear();
        s.cur_full_w = s.orig_w;
        s.cur_full_h = s.orig_h;
        s.brightness = 0;
        s.contrast = 0.0;
        b_scale_reset.set_value(0.0);
        c_scale_reset.set_value(0.0);
        dim_lbl_reset.set_text(&format!("{}×{} px", s.orig_w, s.orig_h));
        drop(s);
        canvas_reset.queue_draw();
    });
    sidebar.append(&reset_btn);

    let spacer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    sidebar.append(&spacer);

    // Action buttons
    let act_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let cancel_btn = gtk4::Button::with_label("Cancel");
    cancel_btn.add_css_class("hyprdrop-secondary");
    cancel_btn.set_hexpand(true);
    let win_cancel = window.clone();
    cancel_btn.connect_clicked(move |_| {
        win_cancel.close();
    });
    act_row.append(&cancel_btn);

    let save_btn = gtk4::Button::with_label("Save Edits");
    save_btn.add_css_class("hyprdrop-primary");
    save_btn.set_hexpand(true);
    let win_save = window.clone();
    let state_save = state.clone();
    let on_success_rc = Rc::new(on_success);
    save_btn.connect_clicked(move |_| {
        let s = state_save.borrow();
        if let Ok(mut full_img) = load_image(&s.source_path) {
            for op in &s.ops {
                match op {
                    TransformOp::Rotate90 => full_img = full_img.rotate90(),
                    TransformOp::Rotate180 => full_img = full_img.rotate180(),
                    TransformOp::FlipH => full_img = full_img.fliph(),
                    TransformOp::FlipV => full_img = full_img.flipv(),
                    TransformOp::Scale(factor) => {
                        let nw = ((full_img.width() as f64) * factor).round().max(16.0) as u32;
                        let nh = ((full_img.height() as f64) * factor).round().max(16.0) as u32;
                        full_img = full_img.resize(nw, nh, image::imageops::FilterType::Lanczos3);
                    }
                    TransformOp::AutoTrim => {
                        let trimmed = auto_trim_transparent(&full_img.to_rgba8());
                        full_img = DynamicImage::ImageRgba8(trimmed);
                    }
                }
            }
            let final_img = adjust_brightness_contrast(&full_img, s.brightness, s.contrast);
            let out_path = get_suffixed_output_path(&s.source_path, "Edited", None);
            let ext = out_path.extension().and_then(|e| e.to_str()).unwrap_or("png");
            if let Ok(()) = save_image_with_format(&final_img, &out_path, ext) {
                let _ = ConfigManager::append_history("QUICK EDIT", &s.source_path, &out_path, None);
                on_success_rc(out_path);
            }
        }
        win_save.close();
        crate::engine::trim_memory();
    });
    act_row.append(&save_btn);
    sidebar.append(&act_row);

    body_box.append(&sidebar);
    root_box.append(&body_box);

    window.connect_destroy(move |_| {
        crate::engine::trim_memory();
    });

    window.set_child(Some(&root_box));
    crate::hyprland::ensure_hyprland_floating_rules();
    window.present();
}

fn draw_edit_canvas(cr: &Context, cw: f64, ch: f64, state: &EditState) {
    cr.set_source_rgb(0.06, 0.06, 0.07);
    let _ = cr.paint();

    let adjusted = adjust_brightness_contrast(&state.preview_image, state.brightness, state.contrast);
    let iw = adjusted.width() as f64;
    let ih = adjusted.height() as f64;

    let sx = (cw - 32.0) / iw;
    let sy = (ch - 32.0) / ih;
    let scale = sx.min(sy).clamp(0.01, 1.0);
    let vx = (cw - iw * scale) / 2.0;
    let vy = (ch - ih * scale) / 2.0;

    let rgba = adjusted.to_rgba8();
    let mut bgra = Vec::with_capacity((iw * ih * 4.0) as usize);
    for p in rgba.pixels() {
        bgra.push(p[2]);
        bgra.push(p[1]);
        bgra.push(p[0]);
        bgra.push(p[3]);
    }

    if let Ok(surface) = cairo::ImageSurface::create_for_data(
        bgra,
        cairo::Format::ARgb32,
        iw as i32,
        ih as i32,
        cairo::Format::ARgb32.stride_for_width(iw as u32).unwrap_or(iw as i32 * 4),
    ) {
        cr.save().ok();
        cr.translate(vx, vy);
        cr.scale(scale, scale);
        let _ = cr.set_source_surface(&surface, 0.0, 0.0);
        let _ = cr.paint();
        cr.restore().ok();
    }
}
