use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use gtk4::cairo::{Context, LinearGradient};
use gtk4::prelude::*;
use image::DynamicImage;
use crate::config::ConfigManager;
use crate::engine::image_ops::{
    StudioBackgroundPreset, TargetAspectRatio, load_image, render_studio_background,
};
use crate::engine::naming::get_suffixed_output_path;

struct AddBgState {
    source_path: PathBuf,
    preview_img: DynamicImage,
    orig_w: u32,
    _orig_h: u32,
    preset: StudioBackgroundPreset,
    padding: u32,
    corner_radius: f32,
    shadow_intensity: f32,
    aspect_ratio: TargetAspectRatio,
}

pub fn open_add_bg_modal(
    app: Option<&gtk4::Application>,
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

    let (orig_w, orig_h) = (img.width(), img.height());
    let max_dim = (orig_w.max(orig_h) as f64).max(1.0);
    let preview_scale = (640.0 / max_dim).min(1.0);
    let preview_img = if preview_scale < 1.0 {
        img.resize(
            ((orig_w as f64) * preview_scale).round() as u32,
            ((orig_h as f64) * preview_scale).round() as u32,
            image::imageops::FilterType::Triangle,
        )
    } else {
        img
    };

    let state = Rc::new(RefCell::new(AddBgState {
        source_path: image_path.to_path_buf(),
        preview_img,
        orig_w,
        _orig_h: orig_h,
        preset: StudioBackgroundPreset::SunsetOrange,
        padding: 64,
        corner_radius: 28.0,
        shadow_intensity: 0.65,
        aspect_ratio: TargetAspectRatio::Auto,
    }));

    let window = gtk4::Window::builder()
        .title("Background Studio — Hyprdrop")
        .default_width(780)
        .default_height(860)
        .build();

    if let Some(a) = app {
        a.add_window(&window);
    }

    window.add_css_class("hyprdrop-surface");

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    root_box.add_css_class("hyprdrop-surface");

    // Header Bar
    let header_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    header_box.set_margin_start(20);
    header_box.set_margin_end(20);
    header_box.set_margin_top(16);
    header_box.set_margin_bottom(12);

    let title_icon = gtk4::Image::from_icon_name("image-x-generic-symbolic");
    title_icon.set_pixel_size(18);
    title_icon.add_css_class("hyprdrop-brand-icon");
    header_box.append(&title_icon);

    let title_lbl = gtk4::Label::new(Some("Background Studio"));
    title_lbl.add_css_class("hyprdrop-title");
    title_lbl.set_hexpand(true);
    title_lbl.set_halign(gtk4::Align::Start);
    header_box.append(&title_lbl);

    let right_close = gtk4::Button::from_icon_name("window-close-symbolic");
    right_close.add_css_class("hyprdrop-ghost");
    right_close.add_css_class("hyprdrop-ghost-danger");
    right_close.set_tooltip_text(Some("Close"));
    let win_close2 = window.clone();
    right_close.connect_clicked(move |_| win_close2.close());
    header_box.append(&right_close);

    root_box.append(&header_box);

    // 2. Main Live Preview Canvas (High-DPI Rounded Studio Card)
    let canvas_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    canvas_box.set_margin_start(20);
    canvas_box.set_margin_end(20);
    canvas_box.set_margin_bottom(16);
    canvas_box.set_size_request(740, 360);
    canvas_box.set_hexpand(true);

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

    // 3. Lower Controls Area
    let controls_box = gtk4::Box::new(gtk4::Orientation::Vertical, 14);
    controls_box.set_margin_start(24);
    controls_box.set_margin_end(24);
    controls_box.set_margin_bottom(20);

    // Section Title: "BACKGROUND STYLE"
    let style_lbl = gtk4::Label::new(Some("BACKGROUND STYLE"));
    style_lbl.add_css_class("mac-section-header");
    style_lbl.set_halign(gtk4::Align::Start);
    controls_box.append(&style_lbl);

    // 16 Curated Swatches Grid (2 rows of 8)
    let grid_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    let row1 = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    let row2 = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    row1.set_halign(gtk4::Align::Start);
    row2.set_halign(gtk4::Align::Start);

    let presets = StudioBackgroundPreset::ALL_PRESETS;
    let mut swatch_redraw_triggers: Vec<gtk4::DrawingArea> = Vec::new();

    for (idx, &preset_val) in presets.iter().enumerate() {
        let target_row = if idx < 8 { &row1 } else { &row2 };

        let swatch_area = gtk4::DrawingArea::builder()
            .content_width(34)
            .content_height(34)
            .focusable(true)
            .build();

        let state_swatch = state.clone();
        let swatch_preset = preset_val;
        swatch_area.set_draw_func(move |_, cr, w, h| {
            let s = state_swatch.borrow();
            let is_selected = s.preset == swatch_preset;
            draw_swatch(cr, w as f64, h as f64, swatch_preset, is_selected);
        });

        // Click Gesture on Swatch
        let click = gtk4::GestureClick::new();
        let state_click = state.clone();
        let canvas_click = canvas.clone();
        let all_swatches = swatch_redraw_triggers.clone();
        let swatch_this = swatch_area.clone();

        click.connect_pressed(move |_, _, _, _| {
            state_click.borrow_mut().preset = swatch_preset;
            canvas_click.queue_draw();
            swatch_this.queue_draw();
            for sw in &all_swatches {
                sw.queue_draw();
            }
        });
        swatch_area.add_controller(click);

        swatch_redraw_triggers.push(swatch_area.clone());
        target_row.append(&swatch_area);
    }

    // 16th Swatch: Custom Color Picker / Rainbow Swatch
    let custom_area = gtk4::DrawingArea::builder()
        .content_width(34)
        .content_height(34)
        .focusable(true)
        .build();

    let state_custom_draw = state.clone();
    custom_area.set_draw_func(move |_, cr, w, h| {
        let s = state_custom_draw.borrow();
        let is_selected = matches!(s.preset, StudioBackgroundPreset::Custom(..));
        draw_rainbow_swatch(cr, w as f64, h as f64, is_selected);
    });

    let custom_dialog = gtk4::ColorDialog::builder()
        .title("Custom Studio Backdrop")
        .build();
    let state_custom_click = state.clone();
    let canvas_custom_click = canvas.clone();
    let custom_area_click = custom_area.clone();
    let all_swatches_click = swatch_redraw_triggers.clone();

    let custom_gesture = gtk4::GestureClick::new();
    let win_dialog = window.clone();
    custom_gesture.connect_pressed(move |_, _, _, _| {
        let state_c = state_custom_click.clone();
        let canvas_c = canvas_custom_click.clone();
        let area_c = custom_area_click.clone();
        let all_sw = all_swatches_click.clone();

        let initial_rgba = gtk4::gdk::RGBA::new(0.04, 0.52, 1.0, 1.0);
        custom_dialog.choose_rgba(
            Some(&win_dialog),
            Some(&initial_rgba),
            gtk4::gio::Cancellable::NONE,
            move |res| {
                if let Ok(rgba) = res {
                    let r = (rgba.red() * 255.0).round() as u8;
                    let g = (rgba.green() * 255.0).round() as u8;
                    let b = (rgba.blue() * 255.0).round() as u8;
                    let r2 = ((rgba.red() * 1.3).min(1.0) * 255.0).round() as u8;
                    let g2 = ((rgba.green() * 1.3).min(1.0) * 255.0).round() as u8;
                    let b2 = ((rgba.blue() * 1.3).min(1.0) * 255.0).round() as u8;
                    state_c.borrow_mut().preset = StudioBackgroundPreset::Custom(r, g, b, r2, g2, b2);
                    canvas_c.queue_draw();
                    area_c.queue_draw();
                    for sw in &all_sw {
                        sw.queue_draw();
                    }
                }
            },
        );
    });
    custom_area.add_controller(custom_gesture);
    row2.append(&custom_area);

    grid_box.append(&row1);
    grid_box.append(&row2);
    controls_box.append(&grid_box);

    // 4. Sliders (Padding, Corners, Shadow)
    let sliders_card = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    sliders_card.set_margin_top(8);

    // Slider 1: Padding
    let pad_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 14);
    let pad_title = gtk4::Label::new(Some("Padding"));
    pad_title.add_css_class("mac-subtitle");
    pad_title.set_width_chars(9);
    pad_title.set_halign(gtk4::Align::Start);
    pad_row.append(&pad_title);

    let pad_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 600.0, 4.0);
    pad_scale.set_value(64.0);
    pad_scale.set_hexpand(true);
    pad_row.append(&pad_scale);

    let pad_val_lbl = gtk4::Label::new(Some("64 px"));
    pad_val_lbl.add_css_class("mac-mono");
    pad_val_lbl.set_width_chars(8);
    pad_val_lbl.set_halign(gtk4::Align::End);
    pad_row.append(&pad_val_lbl);

    let state_pad = state.clone();
    let canvas_pad = canvas.clone();
    let pad_lbl_clone = pad_val_lbl.clone();
    pad_scale.connect_value_changed(move |scale| {
        let val = scale.value().round() as u32;
        pad_lbl_clone.set_text(&format!("{} px", val));
        state_pad.borrow_mut().padding = val;
        canvas_pad.queue_draw();
    });
    sliders_card.append(&pad_row);

    // Slider 2: Corners
    let cr_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 14);
    let cr_title = gtk4::Label::new(Some("Corners"));
    cr_title.add_css_class("mac-subtitle");
    cr_title.set_width_chars(9);
    cr_title.set_halign(gtk4::Align::Start);
    cr_row.append(&cr_title);

    let cr_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 150.0, 2.0);
    cr_scale.set_value(28.0);
    cr_scale.set_hexpand(true);
    cr_row.append(&cr_scale);

    let cr_val_lbl = gtk4::Label::new(Some("28 px"));
    cr_val_lbl.add_css_class("mac-mono");
    cr_val_lbl.set_width_chars(8);
    cr_val_lbl.set_halign(gtk4::Align::End);
    cr_row.append(&cr_val_lbl);

    let state_cr = state.clone();
    let canvas_cr = canvas.clone();
    let cr_lbl_clone = cr_val_lbl.clone();
    cr_scale.connect_value_changed(move |scale| {
        let val = scale.value().round() as f32;
        cr_lbl_clone.set_text(&format!("{:.0} px", val));
        state_cr.borrow_mut().corner_radius = val;
        canvas_cr.queue_draw();
    });
    sliders_card.append(&cr_row);

    // Slider 3: Shadow
    let sh_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 14);
    let sh_title = gtk4::Label::new(Some("Shadow"));
    sh_title.add_css_class("mac-subtitle");
    sh_title.set_width_chars(9);
    sh_title.set_halign(gtk4::Align::Start);
    sh_row.append(&sh_title);

    let sh_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 1.0, 0.02);
    sh_scale.set_value(0.65);
    sh_scale.set_hexpand(true);
    sh_row.append(&sh_scale);

    let sh_val_lbl = gtk4::Label::new(Some("0.65"));
    sh_val_lbl.add_css_class("mac-mono");
    sh_val_lbl.set_width_chars(8);
    sh_val_lbl.set_halign(gtk4::Align::End);
    sh_row.append(&sh_val_lbl);

    let state_sh = state.clone();
    let canvas_sh = canvas.clone();
    let sh_lbl_clone = sh_val_lbl.clone();
    sh_scale.connect_value_changed(move |scale| {
        let val = scale.value() as f32;
        sh_lbl_clone.set_text(&format!("{:.2}", val));
        state_sh.borrow_mut().shadow_intensity = val;
        canvas_sh.queue_draw();
    });
    sliders_card.append(&sh_row);

    controls_box.append(&sliders_card);

    // 5. Bottom Bar (Aspect Ratio Pills on Left, Reset & Save on Right)
    let bottom_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    bottom_row.set_margin_top(12);

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
    bottom_row.append(&ratio_box);

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

    let b_spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    b_spacer.set_hexpand(true);
    bottom_row.append(&b_spacer);

    // Reset Button
    let reset_btn = gtk4::Button::with_label("Reset");
    reset_btn.add_css_class("mac-secondary");
    let state_reset = state.clone();
    let canvas_reset = canvas.clone();
    let pad_reset = pad_scale.clone();
    let cr_reset = cr_scale.clone();
    let sh_reset = sh_scale.clone();
    let all_r_reset = ratio_pills.clone();
    let all_swatches_reset = swatch_redraw_triggers.clone();
    let custom_area_reset = custom_area.clone();

    reset_btn.connect_clicked(move |_| {
        let mut s = state_reset.borrow_mut();
        s.preset = StudioBackgroundPreset::SunsetOrange;
        s.padding = 64;
        s.corner_radius = 28.0;
        s.shadow_intensity = 0.65;
        s.aspect_ratio = TargetAspectRatio::Auto;
        pad_reset.set_value(64.0);
        cr_reset.set_value(28.0);
        sh_reset.set_value(0.65);
        for (p, r) in &all_r_reset {
            if *r == TargetAspectRatio::Auto {
                p.add_css_class("active");
            } else {
                p.remove_css_class("active");
            }
        }
        drop(s);
        canvas_reset.queue_draw();
        custom_area_reset.queue_draw();
        for sw in &all_swatches_reset {
            sw.queue_draw();
        }
    });
    bottom_row.append(&reset_btn);

    // Save Button (macOS Electric Blue Glowing Pill)
    let save_btn = gtk4::Button::with_label("Save with Background");
    save_btn.add_css_class("mac-primary");
    let win_save = window.clone();
    let state_save = state.clone();
    let on_success_rc = Rc::new(on_success);

    save_btn.connect_clicked(move |_| {
        let s = state_save.borrow();
        if let Ok(full_img) = load_image(&s.source_path) {
            let rendered = render_studio_background(
                &full_img,
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
        }
        win_save.close();
        crate::engine::trim_memory();
    });
    bottom_row.append(&save_btn);

    controls_box.append(&bottom_row);
    root_box.append(&controls_box);

    window.connect_destroy(move |_| {
        crate::engine::trim_memory();
    });

    window.set_child(Some(&root_box));
    crate::hyprland::ensure_hyprland_floating_rules();
    window.present();
}

fn draw_swatch(cr: &Context, w: f64, h: f64, preset: StudioBackgroundPreset, is_selected: bool) {
    let ((r1, g1, b1), (r2, g2, b2)) = preset.colors();
    let radius = 8.0;

    // Draw rounded square path
    draw_rounded_rect(cr, 2.0, 2.0, w - 4.0, h - 4.0, radius);

    // Create gradient pattern
    let lg = LinearGradient::new(2.0, 2.0, w - 2.0, h - 2.0);
    lg.add_color_stop_rgb(0.0, r1 as f64 / 255.0, g1 as f64 / 255.0, b1 as f64 / 255.0);
    lg.add_color_stop_rgb(1.0, r2 as f64 / 255.0, g2 as f64 / 255.0, b2 as f64 / 255.0);

    let _ = cr.set_source(&lg);
    let _ = cr.fill_preserve();

    if is_selected {
        // Prominent outer selection ring (macOS Blue + White inner stroke)
        cr.set_source_rgba(0.04, 0.52, 1.0, 0.95);
        cr.set_line_width(2.5);
        let _ = cr.stroke_preserve();

        draw_rounded_rect(cr, 0.5, 0.5, w - 1.0, h - 1.0, radius + 1.0);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.9);
        cr.set_line_width(1.0);
        let _ = cr.stroke();
    } else {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.15);
        cr.set_line_width(1.0);
        let _ = cr.stroke();
    }
}

fn draw_rainbow_swatch(cr: &Context, w: f64, h: f64, is_selected: bool) {
    let radius = 8.0;
    draw_rounded_rect(cr, 2.0, 2.0, w - 4.0, h - 4.0, radius);

    // Multi-stop rainbow gradient
    let lg = LinearGradient::new(2.0, 2.0, w - 2.0, h - 2.0);
    lg.add_color_stop_rgb(0.0, 1.0, 0.2, 0.2);
    lg.add_color_stop_rgb(0.25, 1.0, 0.8, 0.1);
    lg.add_color_stop_rgb(0.5, 0.1, 0.9, 0.3);
    lg.add_color_stop_rgb(0.75, 0.1, 0.5, 1.0);
    lg.add_color_stop_rgb(1.0, 0.8, 0.1, 0.9);

    let _ = cr.set_source(&lg);
    let _ = cr.fill_preserve();

    if is_selected {
        cr.set_source_rgba(0.04, 0.52, 1.0, 0.95);
        cr.set_line_width(2.5);
        let _ = cr.stroke_preserve();

        draw_rounded_rect(cr, 0.5, 0.5, w - 1.0, h - 1.0, radius + 1.0);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.9);
        cr.set_line_width(1.0);
        let _ = cr.stroke();
    } else {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.25);
        cr.set_line_width(1.0);
        let _ = cr.stroke();
    }
}

fn draw_rounded_rect(cr: &Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(x + r, y + h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
    cr.arc(x + r, y + r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
    cr.close_path();
}

fn draw_preview_backdrop(cr: &Context, canvas_w: f64, canvas_h: f64, state: &AddBgState) {
    // Fill canvas background
    cr.set_source_rgb(0.07, 0.07, 0.09);
    let _ = cr.paint();

    // Draw outer preview card frame with rounded corners
    let card_x = 10.0;
    let card_y = 10.0;
    let card_w = canvas_w - 20.0;
    let card_h = canvas_h - 20.0;

    draw_rounded_rect(cr, card_x, card_y, card_w, card_h, 18.0);
    cr.set_source_rgb(0.09, 0.09, 0.11);
    let _ = cr.fill_preserve();
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.06);
    cr.set_line_width(1.0);
    let _ = cr.stroke();

    let pw = state.preview_img.width() as f64;
    let scale_factor = pw / (state.orig_w as f64).max(1.0);

    let p_padding = ((state.padding as f64) * scale_factor).round() as u32;
    let p_radius = (state.corner_radius as f64 * scale_factor) as f32;

    let rendered = render_studio_background(
        &state.preview_img,
        state.preset,
        p_padding,
        p_radius,
        state.shadow_intensity,
        state.aspect_ratio,
    );

    let rw = rendered.width() as f64;
    let rh = rendered.height() as f64;

    let scale_x = (card_w - 32.0) / rw;
    let scale_y = (card_h - 32.0) / rh;
    let fit_scale = scale_x.min(scale_y).clamp(0.01, 1.0);

    let final_w = rw * fit_scale;
    let final_h = rh * fit_scale;
    let vx = card_x + (card_w - final_w) / 2.0;
    let vy = card_y + (card_h - final_h) / 2.0;

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
        cairo::Format::ARgb32.stride_for_width(rw as u32).unwrap_or(rw as i32 * 4),
    ) {
        cr.save().ok();
        // Clip within card frame
        draw_rounded_rect(cr, card_x + 2.0, card_y + 2.0, card_w - 4.0, card_h - 4.0, 16.0);
        cr.clip();

        cr.translate(vx, vy);
        cr.scale(fit_scale, fit_scale);
        let _ = cr.set_source_surface(&surface, 0.0, 0.0);
        let _ = cr.paint();
        cr.restore().ok();
    }
}
