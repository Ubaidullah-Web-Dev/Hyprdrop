use image::{DynamicImage, Rgba, RgbaImage, imageops};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RedactionKind {
    Blackout,
    Blur,
    Pixelate,
}

#[derive(Debug, Clone)]
pub struct RedactionRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub kind: RedactionKind,
}

#[derive(Debug, Clone)]
pub enum AnnotationElement {
    Rectangle {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        color: [u8; 4],
        stroke_width: u32,
    },
    Arrow {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        color: [u8; 4],
        stroke_width: u32,
    },
    TextCallout {
        x: u32,
        y: u32,
        text: String,
        color: [u8; 4],
    },
}

/// Applies redaction regions and annotations onto an image.
pub fn apply_redactions_and_annotations(
    img: &DynamicImage,
    redactions: &[RedactionRegion],
    annotations: &[AnnotationElement],
) -> RgbaImage {
    let mut canvas = img.to_rgba8();
    let (img_w, img_h) = (canvas.width(), canvas.height());

    // Apply redactions
    for r in redactions {
        let rx = r.x.min(img_w.saturating_sub(1));
        let ry = r.y.min(img_h.saturating_sub(1));
        let rw = r.width.min(img_w.saturating_sub(rx)).max(1);
        let rh = r.height.min(img_h.saturating_sub(ry)).max(1);

        match r.kind {
            RedactionKind::Blackout => {
                for y in ry..(ry + rh) {
                    for x in rx..(rx + rw) {
                        canvas.put_pixel(x, y, Rgba([0, 0, 0, 255]));
                    }
                }
            }
            RedactionKind::Pixelate => {
                let block_size = 12u32.min(rw).min(rh).max(4);
                for by in (ry..(ry + rh)).step_by(block_size as usize) {
                    for bx in (rx..(rx + rw)).step_by(block_size as usize) {
                        let bw = block_size.min(rx + rw - bx);
                        let bh = block_size.min(ry + rh - by);

                        // Average color in block
                        let mut sum_r = 0u64;
                        let mut sum_g = 0u64;
                        let mut sum_b = 0u64;
                        let mut count = 0u64;

                        for py in by..(by + bh) {
                            for px in bx..(bx + bw) {
                                let p = canvas.get_pixel(px, py);
                                sum_r += p[0] as u64;
                                sum_g += p[1] as u64;
                                sum_b += p[2] as u64;
                                count += 1;
                            }
                        }

                        if let (Some(r), Some(g), Some(b)) = (
                            sum_r.checked_div(count),
                            sum_g.checked_div(count),
                            sum_b.checked_div(count),
                        ) {
                            let avg_p = Rgba([r as u8, g as u8, b as u8, 255]);
                            for py in by..(by + bh) {
                                for px in bx..(bx + bw) {
                                    canvas.put_pixel(px, py, avg_p);
                                }
                            }
                        }
                    }
                }
            }
            RedactionKind::Blur => {
                let sub_img = DynamicImage::ImageRgba8(imageops::crop_imm(&canvas, rx, ry, rw, rh).to_image());
                let blurred = sub_img.blur(8.0).to_rgba8();
                imageops::overlay(&mut canvas, &blurred, rx as i64, ry as i64);
            }
        }
    }

    // Apply geometric annotations
    for a in annotations {
        match a {
            AnnotationElement::Rectangle { x, y, width, height, color, stroke_width } => {
                let col = Rgba(*color);
                let sw = *stroke_width;
                let x2 = (*x + *width).min(img_w);
                let y2 = (*y + *height).min(img_h);

                // Top & bottom horizontal bars
                for px in *x..x2 {
                    for w in 0..sw {
                        if *y + w < img_h { canvas.put_pixel(px, *y + w, col); }
                        if y2.saturating_sub(1 + w) >= *y { canvas.put_pixel(px, y2.saturating_sub(1 + w), col); }
                    }
                }
                // Left & right vertical bars
                for py in *y..y2 {
                    for w in 0..sw {
                        if *x + w < img_w { canvas.put_pixel(*x + w, py, col); }
                        if x2.saturating_sub(1 + w) >= *x { canvas.put_pixel(x2.saturating_sub(1 + w), py, col); }
                    }
                }
            }
            AnnotationElement::Arrow { x1, y1, x2, y2, color, stroke_width } => {
                draw_arrow_line(&mut canvas, *x1, *y1, *x2, *y2, *stroke_width, Rgba(*color));
            }
            AnnotationElement::TextCallout { .. } => {}
        }
    }

    // Apply text callouts with vector typography
    render_text_callouts_cairo(&mut canvas, annotations);

    canvas
}

fn draw_arrow_line(
    canvas: &mut RgbaImage,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    thickness: u32,
    color: Rgba<u8>,
) {
    let (w, h) = (canvas.width() as i32, canvas.height() as i32);
    let dx = (x2 - x1) as f32;
    let dy = (y2 - y1) as f32;
    let length = (dx * dx + dy * dy).sqrt();
    if length < 1.0 {
        return;
    }

    let steps = length.ceil() as i32;
    let r = (thickness as f32 / 2.0).max(1.0);

    // Draw main line
    for step in 0..=steps {
        let t = (step as f32) / (steps as f32);
        let cx = x1 as f32 + t * dx;
        let cy = y1 as f32 + t * dy;

        let r_int = r.ceil() as i32;
        for oy in -r_int..=r_int {
            for ox in -r_int..=r_int {
                if (ox as f32).powi(2) + (oy as f32).powi(2) <= r * r {
                    let px = cx.round() as i32 + ox;
                    let py = cy.round() as i32 + oy;
                    if px >= 0 && px < w && py >= 0 && py < h {
                        canvas.put_pixel(px as u32, py as u32, color);
                    }
                }
            }
        }
    }

    // Draw arrowhead at (x2, y2)
    let angle = dy.atan2(dx);
    let head_length = (thickness as f32 * 4.0).max(14.0);
    let head_angle = 0.5f32; // ~30 degrees

    for sign in &[-1.0f32, 1.0f32] {
        let wing_angle = angle + std::f32::consts::PI + sign * head_angle;
        let wx = x2 as f32 + head_length * wing_angle.cos();
        let wy = y2 as f32 + head_length * wing_angle.sin();

        let wing_steps = head_length.ceil() as i32;
        for step in 0..=wing_steps {
            let t = (step as f32) / (wing_steps as f32);
            let px = (x2 as f32 + t * (wx - x2 as f32)).round() as i32;
            let py = (y2 as f32 + t * (wy - y2 as f32)).round() as i32;
            if px >= 0 && px < w && py >= 0 && py < h {
                canvas.put_pixel(px as u32, py as u32, color);
            }
        }
    }
}

fn render_text_callouts_cairo(canvas: &mut RgbaImage, annotations: &[AnnotationElement]) {
    let callouts: Vec<&AnnotationElement> = annotations
        .iter()
        .filter(|a| matches!(a, AnnotationElement::TextCallout { .. }))
        .collect();

    if callouts.is_empty() {
        return;
    }

    let w = canvas.width() as i32;
    let h = canvas.height() as i32;
    let stride = cairo::Format::ARgb32.stride_for_width(w as u32).unwrap_or(w * 4);

    let mut bgra = Vec::with_capacity((w * h * 4) as usize);
    for p in canvas.pixels() {
        bgra.push(p[2]);
        bgra.push(p[1]);
        bgra.push(p[0]);
        bgra.push(p[3]);
    }

    if let Ok(mut surface) = cairo::ImageSurface::create_for_data(
        bgra,
        cairo::Format::ARgb32,
        w,
        h,
        stride,
    ) {
        if let Ok(cr) = cairo::Context::new(&surface) {
            for a in callouts {
                if let AnnotationElement::TextCallout { x, y, text, color } = a {
                    let cx = *x as f64;
                    let cy = *y as f64;

                    cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                    cr.set_font_size(13.0);
                    let (tw, th, x_bear, y_bear) = match cr.text_extents(text) {
                        Ok(ext) => (ext.width(), ext.height(), ext.x_bearing(), ext.y_bearing()),
                        Err(_) => (50.0, 14.0, 0.0, -10.0),
                    };

                    let pad_h = 8.0;
                    let pad_v = 5.0;
                    let pw = tw + pad_h * 2.0;
                    let ph = th + pad_v * 2.0;
                    let px = (cx - pw / 2.0).clamp(4.0, (w as f64) - pw - 4.0);
                    let py = (cy - ph - 8.0).clamp(4.0, (h as f64) - ph - 4.0);

                    // Callout pointer pin triangle pointing to (cx, cy)
                    cr.move_to(cx, cy);
                    cr.line_to(cx - 5.0, py + ph);
                    cr.line_to(cx + 5.0, py + ph);
                    cr.close_path();
                    cr.set_source_rgba(
                        color[0] as f64 / 255.0,
                        color[1] as f64 / 255.0,
                        color[2] as f64 / 255.0,
                        0.95,
                    );
                    let _ = cr.fill();

                    // Rounded pill background
                    let radius = 6.0;
                    cr.new_sub_path();
                    cr.arc(px + pw - radius, py + radius, radius, -std::f64::consts::PI / 2.0, 0.0);
                    cr.arc(px + pw - radius, py + ph - radius, radius, 0.0, std::f64::consts::PI / 2.0);
                    cr.arc(px + radius, py + ph - radius, radius, std::f64::consts::PI / 2.0, std::f64::consts::PI);
                    cr.arc(px + radius, py + radius, radius, std::f64::consts::PI, 3.0 * std::f64::consts::PI / 2.0);
                    cr.close_path();

                    cr.set_source_rgba(0.08, 0.08, 0.10, 0.94);
                    let _ = cr.fill_preserve();
                    cr.set_source_rgba(
                        color[0] as f64 / 255.0,
                        color[1] as f64 / 255.0,
                        color[2] as f64 / 255.0,
                        0.95,
                    );
                    cr.set_line_width(1.5);
                    let _ = cr.stroke();

                    // Text
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.move_to(px + pad_h - x_bear, py + pad_v - y_bear);
                    let _ = cr.show_text(text);
                }
            }
        }
        surface.flush();
        if let Ok(data) = surface.data() {
            for y in 0..h as u32 {
                for x in 0..w as u32 {
                    let idx = (y as usize * stride as usize) + (x as usize * 4);
                    if idx + 3 < data.len() {
                        let b = data[idx];
                        let g = data[idx + 1];
                        let r = data[idx + 2];
                        let a = data[idx + 3];
                        canvas.put_pixel(x, y, Rgba([r, g, b, a]));
                    }
                }
            }
        }
    }
}
