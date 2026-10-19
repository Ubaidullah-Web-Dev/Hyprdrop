use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use gtk4::cairo::Context;
use gtk4::prelude::*;
use image::DynamicImage;
use crate::config::ConfigManager;
use crate::engine::image_ops::{
    StudioBackgroundPreset, TargetAspectRatio, load_image, render_studio_background,
};
use crate::engine::naming::get_suffixed_output_path;

struct AddBgState {
    source_path: PathBuf,
    image: DynamicImage,
    preset: StudioBackgroundPreset,
    padding: u32,
    corner_radius: f32,
    shadow_intensity: f32,
    aspect_ratio: TargetAspectRatio,
}

pub fn open_add_bg_modal(
    parent: &impl IsA<gtk4::Window>,
    image_path: &Path,
    on_success: impl Fn(PathBuf) + 'static,
) {
    let img = match load_image(image_path) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Error loading image for Add BG: {:?}", e);
            return;
        }
    };

    let state = Rc::new(RefCell::new(AddBgState {
        source_path: image_path.to_path_buf(),
        image: img,
        preset: StudioBackgroundPreset::DeepStudio,
        padding: 64,
        corner_radius: 24.0,
        shadow_intensity: 0.65,
        aspect_ratio: TargetAspectRatio::Auto,
    }));

    let window = gtk4::Window::builder()
        .title("Add Studio Background — Hyprdrop")
        .transient_for(parent)
        .modal(true)
        .default_width(940)
        .default_height(680)
        .build();

    let root_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    root_box.add_css_class("hyprdrop-surface");

    // Left Preview Canvas Area
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
        draw_preview_backdrop(cr, w as f64, h as f64, &s);
    });
    canvas_box.append(&canvas);
    root_box.append(&canvas_box);

    // Right Sidebar Controls
    let sidebar = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    sidebar.set_width_request(320);
    sidebar.set_margin_start(16);
    sidebar.set_margin_end(16);
    sidebar.set_margin_top(16);
    sidebar.set_margin_bottom(16);
    sidebar.add_css_class("hyprdrop-card");

    let sidebar_title = gtk4::Label::new(Some("Studio Backdrop"));
    sidebar_title.add_css_class("hyprdrop-title");
    sidebar_title.set_halign(gtk4::Align::Start);
    sidebar.append(&sidebar_title);

    // 2-Row Palette of 16 Curated Presets
    let palette_lbl = gtk4::Label::new(Some("Color Palette"));
    palette_lbl.add_css_class("hyprdrop-subtitle");
    palette_lbl.set_halign(gtk4::Align::Start);
    sidebar.append(&palette_lbl);

    let palette_flow = gtk4::FlowBox::new();
    palette_flow.set_max_children_per_line(8);
    palette_flow.set_min_children_per_line(8);
    palette_flow.set_selection_mode(gtk4::SelectionMode::None);

    let presets = StudioBackgroundPreset::ALL_PRESETS;
    for (idx, &p) in presets.iter().enumerate() {
        let color_btn = gtk4::Button::new();
        color_btn.set_size_request(28, 28);
        color_btn.set_tooltip_text(Some(p.name()));
        color_btn.add_css_class(&format!("bg-preset-{}", idx));

        let state_p = state.clone();
        let canvas_p = canvas.clone();
        color_btn.connect_clicked(move |_| {
            state_p.borrow_mut().preset = p;
            canvas_p.queue_draw();
        });

        palette_flow.append(&color_btn);
    }
    sidebar.append(&palette_flow);

    // Separator
    sidebar.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

    // Sliders
    // 1. Padding (0 - 300 px)
    let pad_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    let pad_title = gtk4::Label::new(Some("Padding"));
    pad_title.add_css_class("hyprdrop-subtitle");
    let pad_val_lbl = gtk4::Label::new(Some("64 px"));
    pad_val_lbl.add_css_class("hyprdrop-mono");
    pad_val_lbl.set_hexpand(true);
    pad_val_lbl.set_halign(gtk4::Align::End);
    pad_header.append(&pad_title);
    pad_header.append(&pad_val_lbl);
    sidebar.append(&pad_header);

    let pad_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 300.0, 4.0);
    pad_scale.set_value(64.0);
    let state_pad = state.clone();
    let canvas_pad = canvas.clone();
    let pad_lbl_clone = pad_val_lbl.clone();
    pad_scale.connect_value_changed(move |scale| {
        let val = scale.value().round() as u32;
        pad_lbl_clone.set_text(&format!("{} px", val));
        state_pad.borrow_mut().padding = val;
        canvas_pad.queue_draw();
    });
    sidebar.append(&pad_scale);

    // 2. Corner Radius (0 - 150 px)
    let cr_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    let cr_title = gtk4::Label::new(Some("Corners"));
    cr_title.add_css_class("hyprdrop-subtitle");
    let cr_val_lbl = gtk4::Label::new(Some("24 px"));
    cr_val_lbl.add_css_class("hyprdrop-mono");
    cr_val_lbl.set_hexpand(true);
    cr_val_lbl.set_halign(gtk4::Align::End);
    cr_header.append(&cr_title);
    cr_header.append(&cr_val_lbl);
    sidebar.append(&cr_header);

    let cr_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 150.0, 2.0);
    cr_scale.set_value(24.0);
    let state_cr = state.clone();
    let canvas_cr = canvas.clone();
    let cr_lbl_clone = cr_val_lbl.clone();
    cr_scale.connect_value_changed(move |scale| {
        let val = scale.value().round() as f32;
        cr_lbl_clone.set_text(&format!("{:.0} px", val));
        state_cr.borrow_mut().corner_radius = val;
        canvas_cr.queue_draw();
    });
    sidebar.append(&cr_scale);

    // 3. Drop Shadow (0.0 - 1.0)
    let sh_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    let sh_title = gtk4::Label::new(Some("Shadow"));
    sh_title.add_css_class("hyprdrop-subtitle");
    let sh_val_lbl = gtk4::Label::new(Some("0.65"));
    sh_val_lbl.add_css_class("hyprdrop-mono");
    sh_val_lbl.set_hexpand(true);
    sh_val_lbl.set_halign(gtk4::Align::End);
    sh_header.append(&sh_title);
    sh_header.append(&sh_val_lbl);
    sidebar.append(&sh_header);

    let sh_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 1.0, 0.05);
    sh_scale.set_value(0.65);
    let state_sh = state.clone();
    let canvas_sh = canvas.clone();
    let sh_lbl_clone = sh_val_lbl.clone();
    sh_scale.connect_value_changed(move |scale| {
        let val = scale.value() as f32;
        sh_lbl_clone.set_text(&format!("{:.2}", val));
        state_sh.borrow_mut().shadow_intensity = val;
        canvas_sh.queue_draw();
    });
    sidebar.append(&sh_scale);

    // Aspect Ratio Pills
    let ratio_lbl = gtk4::Label::new(Some("Output Aspect Ratio"));
    ratio_lbl.add_css_class("hyprdrop-subtitle");
    ratio_lbl.set_halign(gtk4::Align::Start);
    sidebar.append(&ratio_lbl);

    let ratio_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    let ratio_options = [
        ("Auto", TargetAspectRatio::Auto),
        ("1:1", TargetAspectRatio::Ratio1x1),
        ("16:9", TargetAspectRatio::Ratio16x9),
        ("4:3", TargetAspectRatio::Ratio4x3),
        ("9:16", TargetAspectRatio::Ratio9x16),
    ];

    let mut ratio_pills = Vec::new();
    for (name, r_val) in ratio_options {
        let pill = gtk4::Button::with_label(name);
        pill.add_css_class("hyprdrop-pill");
        if r_val == TargetAspectRatio::Auto {
            pill.add_css_class("active");
        }
        ratio_box.append(&pill);
        ratio_pills.push((pill, r_val));
    }
    sidebar.append(&ratio_box);

    for (pill, r_val) in ratio_pills.clone() {
        let state_r = state.clone();
        let canvas_r = canvas.clone();
        let all_r = ratio_pills.clone();
        let pill_btn = pill.clone();
        pill.connect_clicked(move |_| {
            for (p, _) in &all_r {
                p.remove_css_class("active");
            }
            pill_btn.add_css_class("active");
            state_r.borrow_mut().aspect_ratio = r_val;
            canvas_r.queue_draw();
        });
    }

    // Bottom Spacer & Buttons
    let v_spacer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    v_spacer.set_vexpand(true);
    sidebar.append(&v_spacer);

    let btn_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let cancel_btn = gtk4::Button::with_label("Cancel");
    cancel_btn.add_css_class("hyprdrop-secondary");
    cancel_btn.set_hexpand(true);
    let win_cancel = window.clone();
    cancel_btn.connect_clicked(move |_| {
        win_cancel.close();
    });
    btn_row.append(&cancel_btn);

    let save_btn = gtk4::Button::with_label("Save with Background");
    save_btn.add_css_class("hyprdrop-primary");
    save_btn.set_hexpand(true);
    let win_save = window.clone();
    let state_save = state.clone();
    let on_success_rc = Rc::new(on_success);
    save_btn.connect_clicked(move |_| {
        let s = state_save.borrow();
        let rendered = render_studio_background(
            &s.image,
            s.preset,
            s.padding,
            s.corner_radius,
            s.shadow_intensity,
            s.aspect_ratio,
        );

        let out_path = get_suffixed_output_path(&s.source_path, "With Background", Some("png"));
        if let Ok(()) = rendered.save(&out_path) {
            let _ = ConfigManager::append_history("ADD BG", &s.source_path, &out_path, None);
            on_success_rc(out_path);
        }
        win_save.close();
    });
    btn_row.append(&save_btn);
    sidebar.append(&btn_row);

    root_box.append(&sidebar);
    window.set_child(Some(&root_box));
    window.present();
}

fn draw_preview_backdrop(cr: &Context, canvas_w: f64, canvas_h: f64, state: &AddBgState) {
    // Fill canvas background
    cr.set_source_rgb(0.06, 0.06, 0.07);
    let _ = cr.paint();

    // Generate smaller preview rendering for fluid interaction
    let img_w = state.image.width() as f64;
    let img_h = state.image.height() as f64;
    let preview_scale = 480.0 / img_w.max(img_h).max(1.0);
    let p_img = if preview_scale < 1.0 {
        state.image.resize(
            (img_w * preview_scale).round() as u32,
            (img_h * preview_scale).round() as u32,
            image::imageops::FilterType::Nearest,
        )
    } else {
        state.image.clone()
    };

    let p_padding = ((state.padding as f64) * preview_scale).round() as u32;
    let p_radius = (state.corner_radius as f64 * preview_scale) as f32;

    let rendered = render_studio_background(
        &p_img,
        state.preset,
        p_padding,
        p_radius,
        state.shadow_intensity,
        state.aspect_ratio,
    );

    let rw = rendered.width() as f64;
    let rh = rendered.height() as f64;

    let scale_x = (canvas_w - 48.0) / rw;
    let scale_y = (canvas_h - 48.0) / rh;
    let fit_scale = scale_x.min(scale_y).min(1.0).max(0.01);

    let final_w = rw * fit_scale;
    let final_h = rh * fit_scale;
    let vx = (canvas_w - final_w) / 2.0;
    let vy = (canvas_h - final_h) / 2.0;

    let mut bgra = Vec::with_capacity((rw * rh * 4.0) as usize);
    for p in rendered.pixels() {
        bgra.push(p[2]);
        bgra.push(p[1]);
        bgra.push(p[0]);
        bgra.push(p[3]);
    }

    if let Ok(surface) = cairo::ImageSurface::create_for_data(
        bgra,
        cairo::Format::ARgb32,
        rw as i32,
        rh as i32,
        cairo::Format::ARgb32.stride_for_width(rw as u32).unwrap_or(0),
    ) {
        cr.save().ok();
        cr.translate(vx, vy);
        cr.scale(fit_scale, fit_scale);
        let _ = cr.set_source_surface(&surface, 0.0, 0.0);
        let _ = cr.paint();
        cr.restore().ok();
    }
}
