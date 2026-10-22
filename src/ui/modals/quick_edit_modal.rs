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
    let has_transparency = img.color().has_alpha();
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
        .default_height(580)
        .build();

    if let Some(a) = app {
        a.add_window(&window);
    }

    window.add_css_class("hyprdrop-surface");

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    root_box.add_css_class("hyprdrop-surface");

    // 1. Header Bar
    let header_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    header_box.set_margin_start(20);
    header_box.set_margin_end(20);
    header_box.set_margin_top(16);
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

    // 2. Fixed Image Preview Canvas at Top
    let canvas_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    canvas_box.set_margin_start(20);
    canvas_box.set_margin_end(20);
    canvas_box.set_margin_bottom(12);
    canvas_box.set_size_request(-1, 220);
    canvas_box.set_hexpand(true);

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
    root_box.append(&canvas_box);

    // 3. Scrollable Controls Area
    let controls_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    controls_box.set_margin_start(24);
    controls_box.set_margin_end(24);
    controls_box.set_margin_bottom(20);

    // Dynamic resolution label
    let dim_lbl = gtk4::Label::new(Some(&format!("{}×{} px", init_w, init_h)));
    dim_lbl.add_css_class("hyprdrop-mono");
    dim_lbl.set_hexpand(true);
    dim_lbl.set_halign(gtk4::Align::End);

    // Section 1: Orientation & Geometry
    let transform_lbl = gtk4::Label::new(Some("ORIENTATION & GEOMETRY"));
    transform_lbl.add_css_class("mac-section-header");
    transform_lbl.set_halign(gtk4::Align::Start);
    controls_box.append(&transform_lbl);

    let t_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);

    let rot90_btn = gtk4::Button::with_label("Rotate 90°");
    rot90_btn.add_css_class("hyprdrop-pill");
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
    t_row.append(&rot90_btn);

    let rot180_btn = gtk4::Button::with_label("Rotate 180°");
    rot180_btn.add_css_class("hyprdrop-pill");
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
    t_row.append(&rot180_btn);

    let fliph_btn = gtk4::Button::with_label("Flip Horiz");
    fliph_btn.add_css_class("hyprdrop-pill");
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
    t_row.append(&fliph_btn);

    let flipv_btn = gtk4::Button::with_label("Flip Vert");
    flipv_btn.add_css_class("hyprdrop-pill");
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
    t_row.append(&flipv_btn);

    let trim_btn = gtk4::Button::with_label("Auto-Trim");
    trim_btn.add_css_class("hyprdrop-pill");
    trim_btn.set_hexpand(true);
    if !has_transparency {
        trim_btn.set_sensitive(false);
        trim_btn.set_tooltip_text(Some("Auto-Trim is only available for images with alpha transparency (PNG/WEBP)"));
    } else {
        trim_btn.set_tooltip_text(Some("Auto-Trim transparent borders"));
    }
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
    t_row.append(&trim_btn);

    controls_box.append(&t_row);

    // Section 2: Quick Scale & Dimensions
    let resize_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    let resize_lbl = gtk4::Label::new(Some("QUICK SCALE & RESOLUTION"));
    resize_lbl.add_css_class("mac-section-header");
    resize_header.append(&resize_lbl);
    resize_header.append(&dim_lbl);
    controls_box.append(&resize_header);

    let resize_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
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
        btn.set_hexpand(true);
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
    controls_box.append(&resize_box);

    // Section 3: Color & Lighting
    let color_lbl = gtk4::Label::new(Some("COLOR & LIGHTING"));
    color_lbl.add_css_class("mac-section-header");
    color_lbl.set_halign(gtk4::Align::Start);
    controls_box.append(&color_lbl);

    let sliders_box = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    sliders_box.add_css_class("mac-card");

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
    sliders_box.append(&b_header);

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
    sliders_box.append(&b_scale);

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
    sliders_box.append(&c_header);

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
    sliders_box.append(&c_scale);

    controls_box.append(&sliders_box);

    // Section 4: Action Buttons Row
    let act_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    act_row.set_margin_top(8);

    let reset_btn = gtk4::Button::with_label("Reset All");
    reset_btn.add_css_class("hyprdrop-secondary");
    let state_reset = state.clone();
    let canvas_reset = canvas.clone();
    let b_scale_reset = b_scale.clone();
    let c_scale_reset = c_scale.clone();
    let b_val_reset = b_val.clone();
    let c_val_reset = c_val.clone();
    let dim_lbl_reset = dim_lbl.clone();
    reset_btn.connect_clicked(move |_| {
        let (orig_w, orig_h) = {
            let mut s = state_reset.borrow_mut();
            s.preview_image = s.initial_preview.clone();
            s.ops.clear();
            s.cur_full_w = s.orig_w;
            s.cur_full_h = s.orig_h;
            s.brightness = 0;
            s.contrast = 0.0;
            (s.orig_w, s.orig_h)
        };
        b_scale_reset.set_value(0.0);
        c_scale_reset.set_value(0.0);
        b_val_reset.set_text("0");
        c_val_reset.set_text("0.0");
        dim_lbl_reset.set_text(&format!("{}×{} px", orig_w, orig_h));
        canvas_reset.queue_draw();
    });
    act_row.append(&reset_btn);

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    act_row.append(&spacer);

    let cancel_btn = gtk4::Button::with_label("Cancel");
    cancel_btn.add_css_class("hyprdrop-secondary");
    let win_cancel = window.clone();
    cancel_btn.connect_clicked(move |_| {
        win_cancel.close();
    });
    act_row.append(&cancel_btn);

    let save_btn = gtk4::Button::with_label("Save Edits");
    save_btn.add_css_class("hyprdrop-primary");
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

    controls_box.append(&act_row);

    // 4. Wrap controls in ScrolledWindow beneath the fixed top preview
    let scrolled_controls = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .child(&controls_box)
        .build();
    root_box.append(&scrolled_controls);

    window.connect_destroy(move |_| {
        crate::engine::trim_memory();
    });

    window.set_child(Some(&root_box));
    crate::hyprland::ensure_hyprland_floating_rules();
    window.present();
}

fn draw_rounded_rect(cr: &Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(x + r, y + h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
    cr.arc(x + r, y + r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
    cr.close_path();
}

fn draw_edit_canvas(cr: &Context, cw: f64, ch: f64, state: &EditState) {
    // Fill canvas background
    cr.set_source_rgb(0.07, 0.07, 0.09);
    let _ = cr.paint();

    // Outer card frame
    let card_x = 8.0;
    let card_y = 6.0;
    let card_w = (cw - 16.0).max(10.0);
    let card_h = (ch - 12.0).max(10.0);

    draw_rounded_rect(cr, card_x, card_y, card_w, card_h, 14.0);
    cr.set_source_rgb(0.09, 0.09, 0.12);
    let _ = cr.fill_preserve();
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.06);
    cr.set_line_width(1.0);
    let _ = cr.stroke();

    let adjusted = adjust_brightness_contrast(&state.preview_image, state.brightness, state.contrast);
    let iw = adjusted.width() as f64;
    let ih = adjusted.height() as f64;

    let sx = (card_w - 24.0) / iw;
    let sy = (card_h - 20.0) / ih;
    let scale = sx.min(sy).clamp(0.01, 1.0);
    let fw = iw * scale;
    let fh = ih * scale;
    let vx = card_x + (card_w - fw) / 2.0;
    let vy = card_y + (card_h - fh) / 2.0;

    // Soft drop shadow behind image
    draw_rounded_rect(cr, vx + 2.0, vy + 4.0, fw, fh, 6.0);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.4);
    let _ = cr.fill();

    // Checkerboard pattern for transparency behind the image
    let checker_size = 12.0;
    cr.save().ok();
    draw_rounded_rect(cr, vx, vy, fw, fh, 6.0);
    let _ = cr.clip();
    let num_x = (fw / checker_size).ceil() as i32;
    let num_y = (fh / checker_size).ceil() as i32;
    for ix in 0..num_x {
        for iy in 0..num_y {
            if (ix + iy) % 2 == 0 {
                cr.set_source_rgb(0.13, 0.13, 0.16);
            } else {
                cr.set_source_rgb(0.18, 0.18, 0.22);
            }
            cr.rectangle(vx + ix as f64 * checker_size, vy + iy as f64 * checker_size, checker_size, checker_size);
            let _ = cr.fill();
        }
    }

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
    cr.restore().ok();

    // Border around image
    draw_rounded_rect(cr, vx, vy, fw, fh, 6.0);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.12);
    cr.set_line_width(1.0);
    let _ = cr.stroke();
}
