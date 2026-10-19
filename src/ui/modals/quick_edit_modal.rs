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

struct EditState {
    source_path: PathBuf,
    original_image: DynamicImage,
    current_image: DynamicImage,
    brightness: i32,
    contrast: f32,
}

pub fn open_quick_edit_modal(
    parent: &impl IsA<gtk4::Window>,
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

    let state = Rc::new(RefCell::new(EditState {
        source_path: image_path.to_path_buf(),
        original_image: img.clone(),
        current_image: img,
        brightness: 0,
        contrast: 0.0,
    }));

    let window = gtk4::Window::builder()
        .title("Quick Edit — Hyprdrop")
        .transient_for(parent)
        .modal(true)
        .default_width(860)
        .default_height(600)
        .build();

    let root_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    root_box.add_css_class("hyprdrop-surface");

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
    root_box.append(&canvas_box);

    // Sidebar
    let sidebar = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    sidebar.set_width_request(280);
    sidebar.set_margin_start(16);
    sidebar.set_margin_end(16);
    sidebar.set_margin_top(16);
    sidebar.set_margin_bottom(16);
    sidebar.add_css_class("hyprdrop-card");

    let title_lbl = gtk4::Label::new(Some("Quick Adjustments"));
    title_lbl.add_css_class("hyprdrop-title");
    title_lbl.set_halign(gtk4::Align::Start);
    sidebar.append(&title_lbl);

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
    rot90_btn.connect_clicked(move |_| {
        let mut s = state_rot.borrow_mut();
        s.current_image = s.current_image.rotate90();
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
        s.current_image = s.current_image.rotate180();
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
        s.current_image = s.current_image.fliph();
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
        s.current_image = s.current_image.flipv();
        drop(s);
        canvas_flipv.queue_draw();
    });
    t_row2.append(&flipv_btn);
    sidebar.append(&t_row2);

    // Quick Resize Section
    let resize_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    let resize_lbl = gtk4::Label::new(Some("Quick Resize"));
    resize_lbl.add_css_class("hyprdrop-subtitle");
    let dim_lbl = gtk4::Label::new(Some(&format!("{}×{} px", init_w, init_h)));
    dim_lbl.add_css_class("hyprdrop-mono");
    dim_lbl.set_hexpand(true);
    dim_lbl.set_halign(gtk4::Align::End);
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
            let new_w = ((s.current_image.width() as f64) * factor).round().max(16.0) as u32;
            let new_h = ((s.current_image.height() as f64) * factor).round().max(16.0) as u32;
            s.current_image = s.current_image.resize(new_w, new_h, image::imageops::FilterType::Lanczos3);
            dim_lbl_res.set_text(&format!("{}×{} px", s.current_image.width(), s.current_image.height()));
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
        let trimmed = auto_trim_transparent(&s.current_image.to_rgba8());
        s.current_image = DynamicImage::ImageRgba8(trimmed);
        dim_lbl_trim.set_text(&format!("{}×{} px", s.current_image.width(), s.current_image.height()));
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
        s.current_image = s.original_image.clone();
        s.brightness = 0;
        s.contrast = 0.0;
        b_scale_reset.set_value(0.0);
        c_scale_reset.set_value(0.0);
        dim_lbl_reset.set_text(&format!("{}×{} px", s.original_image.width(), s.original_image.height()));
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
        let final_img = adjust_brightness_contrast(&s.current_image, s.brightness, s.contrast);
        let out_path = get_suffixed_output_path(&s.source_path, "Edited", None);
        let ext = out_path.extension().and_then(|e| e.to_str()).unwrap_or("png");
        if let Ok(()) = save_image_with_format(&final_img, &out_path, ext) {
            let _ = ConfigManager::append_history("QUICK EDIT", &s.source_path, &out_path, None);
            on_success_rc(out_path);
        }
        win_save.close();
    });
    act_row.append(&save_btn);
    sidebar.append(&act_row);

    root_box.append(&sidebar);
    window.set_child(Some(&root_box));
    window.present();
}

fn draw_edit_canvas(cr: &Context, cw: f64, ch: f64, state: &EditState) {
    cr.set_source_rgb(0.06, 0.06, 0.07);
    let _ = cr.paint();

    let adjusted = adjust_brightness_contrast(&state.current_image, state.brightness, state.contrast);
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
