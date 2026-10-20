use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use gtk4::cairo::Context;
use gtk4::prelude::*;
use image::DynamicImage;
use crate::config::ConfigManager;
use crate::engine::image_ops::{load_image, save_image_with_format};
use crate::engine::naming::get_suffixed_output_path;
use crate::engine::redact_annotate::{
    AnnotationElement, RedactionKind, RedactionRegion, apply_redactions_and_annotations,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolKind {
    Blackout,
    Blur,
    Pixelate,
    Arrow,
    Rectangle,
    TextCallout,
}

struct StudioState {
    source_path: PathBuf,
    image: DynamicImage,
    active_tool: ToolKind,
    redactions: Vec<RedactionRegion>,
    annotations: Vec<AnnotationElement>,
    is_drawing: bool,
    start_x: f64,
    start_y: f64,
    current_x: f64,
    current_y: f64,
    callout_text: String,
}

pub fn open_redact_annotate_modal(
    parent: &impl IsA<gtk4::Window>,
    image_path: &Path,
    on_success: impl Fn(PathBuf) + 'static,
) {
    let img = match load_image(image_path) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Error loading image for redact/annotate: {:?}", e);
            return;
        }
    };

    let state = Rc::new(RefCell::new(StudioState {
        source_path: image_path.to_path_buf(),
        image: img,
        active_tool: ToolKind::Blackout,
        redactions: Vec::new(),
        annotations: Vec::new(),
        is_drawing: false,
        start_x: 0.0,
        start_y: 0.0,
        current_x: 0.0,
        current_y: 0.0,
        callout_text: "NOTE".to_string(),
    }));

    let window = gtk4::Window::builder()
        .title("Redact & Annotate — Hyprdrop")
        .transient_for(parent)
        .modal(true)
        .default_width(900)
        .default_height(650)
        .build();

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    root_box.add_css_class("hyprdrop-surface");

    // Toolbar
    let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    toolbar.set_margin_start(18);
    toolbar.set_margin_end(18);
    toolbar.set_margin_top(14);
    toolbar.set_margin_bottom(12);

    let traffic_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    traffic_box.set_valign(gtk4::Align::Center);

    let dot_close = gtk4::Button::new();
    dot_close.add_css_class("mac-traffic-dot");
    dot_close.add_css_class("mac-dot-close");
    let win_close = window.clone();
    dot_close.connect_clicked(move |_| win_close.close());

    let dot_min = gtk4::Button::new();
    dot_min.add_css_class("mac-traffic-dot");
    dot_min.add_css_class("mac-dot-min");

    let dot_max = gtk4::Button::new();
    dot_max.add_css_class("mac-traffic-dot");
    dot_max.add_css_class("mac-dot-max");

    traffic_box.append(&dot_close);
    traffic_box.append(&dot_min);
    traffic_box.append(&dot_max);
    toolbar.append(&traffic_box);

    let title_lbl = gtk4::Label::new(Some("Redact & Annotate"));
    title_lbl.add_css_class("mac-title");
    toolbar.append(&title_lbl);

    let tools = [
        ("Blackout", ToolKind::Blackout),
        ("Blur", ToolKind::Blur),
        ("Pixelate", ToolKind::Pixelate),
        ("Arrow", ToolKind::Arrow),
        ("Box", ToolKind::Rectangle),
        ("Callout", ToolKind::TextCallout),
    ];

    let mut tool_buttons = Vec::new();
    for (name, tool) in tools {
        let btn = gtk4::Button::with_label(name);
        btn.add_css_class("hyprdrop-pill");
        if tool == ToolKind::Blackout {
            btn.add_css_class("active");
        }
        toolbar.append(&btn);
        tool_buttons.push((btn, tool));
    }

    let callout_lbl = gtk4::Label::new(Some("Callout:"));
    callout_lbl.add_css_class("hyprdrop-subtitle");
    toolbar.append(&callout_lbl);

    let callout_entry = gtk4::Entry::builder()
        .placeholder_text("Callout text...")
        .text("NOTE")
        .width_chars(12)
        .build();
    let state_entry = state.clone();
    callout_entry.connect_changed(move |e| {
        state_entry.borrow_mut().callout_text = e.text().to_string();
    });
    toolbar.append(&callout_entry);

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    toolbar.append(&spacer);

    let undo_btn = gtk4::Button::with_label("Undo");
    undo_btn.add_css_class("hyprdrop-secondary");
    toolbar.append(&undo_btn);

    root_box.append(&toolbar);

    // Canvas
    let canvas = gtk4::DrawingArea::builder()
        .hexpand(true)
        .vexpand(true)
        .build();

    let state_draw = state.clone();
    canvas.set_draw_func(move |_, cr, w, h| {
        let s = state_draw.borrow();
        draw_redact_canvas(cr, w as f64, h as f64, &s);
    });

    // Tool switching
    for (btn, tool) in tool_buttons.clone() {
        let state_t = state.clone();
        let all_btns = tool_buttons.clone();
        let b_btn = btn.clone();
        btn.connect_clicked(move |_| {
            for (b, _) in &all_btns {
                b.remove_css_class("active");
            }
            b_btn.add_css_class("active");
            state_t.borrow_mut().active_tool = tool;
        });
    }

    // Gesture Click
    let click = gtk4::GestureClick::new();
    let state_click = state.clone();
    let canvas_click = canvas.clone();
    click.connect_pressed(move |_, _, x, y| {
        let mut s = state_click.borrow_mut();
        let (vx, vy, scale) = get_transform(
            canvas_click.width() as f64,
            canvas_click.height() as f64,
            s.image.width() as f64,
            s.image.height() as f64,
        );
        let img_x = (x - vx) / scale;
        let img_y = (y - vy) / scale;

        s.start_x = img_x;
        s.start_y = img_y;
        s.current_x = img_x;
        s.current_y = img_y;
        s.is_drawing = true;
        drop(s);
        canvas_click.queue_draw();
    });

    let state_release = state.clone();
    let canvas_release = canvas.clone();
    click.connect_released(move |_, _, x, y| {
        let mut s = state_release.borrow_mut();
        if !s.is_drawing {
            return;
        }
        let (vx, vy, scale) = get_transform(
            canvas_release.width() as f64,
            canvas_release.height() as f64,
            s.image.width() as f64,
            s.image.height() as f64,
        );
        let img_x = (x - vx) / scale;
        let img_y = (y - vy) / scale;

        let x1 = s.start_x.min(img_x).max(0.0) as u32;
        let y1 = s.start_y.min(img_y).max(0.0) as u32;
        let w = (img_x - s.start_x).abs().round() as u32;
        let h = (img_y - s.start_y).abs().round() as u32;

        let start_x = s.start_x;
        let start_y = s.start_y;
        let tool = s.active_tool;

        match tool {
            ToolKind::Blackout => {
                s.redactions.push(RedactionRegion {
                    x: x1,
                    y: y1,
                    width: w.max(4),
                    height: h.max(4),
                    kind: RedactionKind::Blackout,
                });
            }
            ToolKind::Blur => {
                s.redactions.push(RedactionRegion {
                    x: x1,
                    y: y1,
                    width: w.max(4),
                    height: h.max(4),
                    kind: RedactionKind::Blur,
                });
            }
            ToolKind::Pixelate => {
                s.redactions.push(RedactionRegion {
                    x: x1,
                    y: y1,
                    width: w.max(4),
                    height: h.max(4),
                    kind: RedactionKind::Pixelate,
                });
            }
            ToolKind::Arrow => {
                s.annotations.push(AnnotationElement::Arrow {
                    x1: start_x.round() as i32,
                    y1: start_y.round() as i32,
                    x2: img_x.round() as i32,
                    y2: img_y.round() as i32,
                    color: [10, 132, 255, 255],
                    stroke_width: 4,
                });
            }
            ToolKind::Rectangle => {
                s.annotations.push(AnnotationElement::Rectangle {
                    x: x1,
                    y: y1,
                    width: w.max(4),
                    height: h.max(4),
                    color: [10, 132, 255, 255],
                    stroke_width: 3,
                });
            }
            ToolKind::TextCallout => {
                let callout_text = s.callout_text.clone();
                s.annotations.push(AnnotationElement::TextCallout {
                    x: start_x.round() as u32,
                    y: start_y.round() as u32,
                    text: callout_text,
                    color: [10, 132, 255, 255],
                });
            }
        }

        s.is_drawing = false;
        drop(s);
        canvas_release.queue_draw();
    });
    canvas.add_controller(click);

    let motion = gtk4::EventControllerMotion::new();
    let state_motion = state.clone();
    let canvas_motion = canvas.clone();
    motion.connect_motion(move |_, x, y| {
        let mut s = state_motion.borrow_mut();
        if s.is_drawing {
            let (vx, vy, scale) = get_transform(
                canvas_motion.width() as f64,
                canvas_motion.height() as f64,
                s.image.width() as f64,
                s.image.height() as f64,
            );
            s.current_x = (x - vx) / scale;
            s.current_y = (y - vy) / scale;
            drop(s);
            canvas_motion.queue_draw();
        }
    });
    canvas.add_controller(motion);

    // Undo Connection
    let state_undo = state.clone();
    let canvas_undo = canvas.clone();
    undo_btn.connect_clicked(move |_| {
        let mut s = state_undo.borrow_mut();
        if !s.annotations.is_empty() {
            s.annotations.pop();
        } else if !s.redactions.is_empty() {
            s.redactions.pop();
        }
        drop(s);
        canvas_undo.queue_draw();
    });

    root_box.append(&canvas);

    // Bottom Action Bar
    let bottom_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    bottom_box.set_margin_start(16);
    bottom_box.set_margin_end(16);
    bottom_box.set_margin_top(12);
    bottom_box.set_margin_bottom(12);

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

    let save_btn = gtk4::Button::with_label("Save Redacted");
    save_btn.add_css_class("hyprdrop-primary");
    let win_save = window.clone();
    let state_save = state.clone();
    let on_success_rc = Rc::new(on_success);
    save_btn.connect_clicked(move |_| {
        let s = state_save.borrow();
        let rendered = apply_redactions_and_annotations(&s.image, &s.redactions, &s.annotations);
        let out_path = get_suffixed_output_path(&s.source_path, "Redacted", None);
        let ext = out_path.extension().and_then(|e| e.to_str()).unwrap_or("png");
        if let Ok(()) = save_image_with_format(&DynamicImage::ImageRgba8(rendered), &out_path, ext) {
            let _ = ConfigManager::append_history("REDACT", &s.source_path, &out_path, None);
            on_success_rc(out_path);
        }
        win_save.close();
    });
    bottom_box.append(&save_btn);

    root_box.append(&bottom_box);
    window.set_child(Some(&root_box));
    window.present();
}

fn get_transform(cw: f64, ch: f64, iw: f64, ih: f64) -> (f64, f64, f64) {
    let sx = (cw - 32.0) / iw;
    let sy = (ch - 32.0) / ih;
    let scale = sx.min(sy).clamp(0.01, 1.0);
    let vx = (cw - iw * scale) / 2.0;
    let vy = (ch - ih * scale) / 2.0;
    (vx, vy, scale)
}

fn draw_redact_canvas(cr: &Context, cw: f64, ch: f64, state: &StudioState) {
    cr.set_source_rgb(0.06, 0.06, 0.07);
    let _ = cr.paint();

    let iw = state.image.width() as f64;
    let ih = state.image.height() as f64;
    let (vx, vy, scale) = get_transform(cw, ch, iw, ih);

    // Apply current committed redactions onto a rendered image
    let rendered = apply_redactions_and_annotations(&state.image, &state.redactions, &state.annotations);

    let mut bgra = Vec::with_capacity((iw * ih * 4.0) as usize);
    for p in rendered.pixels() {
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
        cairo::Format::ARgb32.stride_for_width(iw as u32).unwrap_or(0),
    ) {
        cr.save().ok();
        cr.translate(vx, vy);
        cr.scale(scale, scale);
        let _ = cr.set_source_surface(&surface, 0.0, 0.0);
        let _ = cr.paint();
        cr.restore().ok();
    }

    // Live preview of active drag
    if state.is_drawing {
        let x1 = vx + state.start_x.min(state.current_x) * scale;
        let y1 = vy + state.start_y.min(state.current_y) * scale;
        let w = (state.current_x - state.start_x).abs() * scale;
        let h = (state.current_y - state.start_y).abs() * scale;

        cr.set_source_rgba(0.04, 0.52, 1.0, 0.7);
        cr.rectangle(x1, y1, w, h);
        let _ = cr.stroke();
    }
}
