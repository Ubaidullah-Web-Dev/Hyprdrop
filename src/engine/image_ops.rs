use std::path::Path;
use anyhow::{Context, Result, anyhow};
use image::{
    DynamicImage, ImageFormat, ImageReader, Rgba, RgbaImage,
    imageops::{self, FilterType},
};

/// Preset gradient and solid studio backgrounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioBackgroundPreset {
    DeepStudio,
    SunsetOrange,
    MidnightNeon,
    ForestGlow,
    CarbonFiber,
    VelvetViolet,
    CyberpunkAmber,
    ArcticFrost,
    ObsidianSlate,
    RoseGold,
    RoyalBlue,
    EmeraldDusk,
    CleanWhite,
    TitaniumGray,
    InternationalOrange,
    SolarFlare,
    MysticPurple,
    CosmicIndigo,
    NeonCyan,
    VaporwaveDusk,
    AuroraBorealis,
    LavaLamp,
    ElectricLime,
    SunsetHorizon,
    DeepAbyss,
    HyperPink,
    GoldenHour,
    TwilightSky,
    MatrixGreen,
    PeachSorbet,
    MonochromeSilver,
    AuraWave,
    Custom(u8, u8, u8, u8, u8, u8), // (r1, g1, b1, r2, g2, b2)
}

impl StudioBackgroundPreset {
    pub const ALL_PRESETS: &'static [StudioBackgroundPreset] = &[
        StudioBackgroundPreset::DeepStudio,
        StudioBackgroundPreset::SunsetOrange,
        StudioBackgroundPreset::MidnightNeon,
        StudioBackgroundPreset::ForestGlow,
        StudioBackgroundPreset::CarbonFiber,
        StudioBackgroundPreset::VelvetViolet,
        StudioBackgroundPreset::CyberpunkAmber,
        StudioBackgroundPreset::ArcticFrost,
        StudioBackgroundPreset::ObsidianSlate,
        StudioBackgroundPreset::RoseGold,
        StudioBackgroundPreset::RoyalBlue,
        StudioBackgroundPreset::EmeraldDusk,
        StudioBackgroundPreset::CleanWhite,
        StudioBackgroundPreset::TitaniumGray,
        StudioBackgroundPreset::InternationalOrange,
        StudioBackgroundPreset::SolarFlare,
        StudioBackgroundPreset::MysticPurple,
        StudioBackgroundPreset::CosmicIndigo,
        StudioBackgroundPreset::NeonCyan,
        StudioBackgroundPreset::VaporwaveDusk,
        StudioBackgroundPreset::AuroraBorealis,
        StudioBackgroundPreset::LavaLamp,
        StudioBackgroundPreset::ElectricLime,
        StudioBackgroundPreset::SunsetHorizon,
        StudioBackgroundPreset::DeepAbyss,
        StudioBackgroundPreset::HyperPink,
        StudioBackgroundPreset::GoldenHour,
        StudioBackgroundPreset::TwilightSky,
        StudioBackgroundPreset::MatrixGreen,
        StudioBackgroundPreset::PeachSorbet,
        StudioBackgroundPreset::MonochromeSilver,
        StudioBackgroundPreset::AuraWave,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::DeepStudio => "Deep Studio",
            Self::SunsetOrange => "Sunset Orange",
            Self::MidnightNeon => "Midnight Neon",
            Self::ForestGlow => "Forest Glow",
            Self::CarbonFiber => "Carbon Fiber",
            Self::VelvetViolet => "Velvet Violet",
            Self::CyberpunkAmber => "Cyberpunk Amber",
            Self::ArcticFrost => "Arctic Frost",
            Self::ObsidianSlate => "Obsidian Slate",
            Self::RoseGold => "Rose Gold",
            Self::RoyalBlue => "Royal Blue",
            Self::EmeraldDusk => "Emerald Dusk",
            Self::CleanWhite => "Clean White",
            Self::TitaniumGray => "Titanium Gray",
            Self::InternationalOrange => "International Orange",
            Self::SolarFlare => "Solar Flare",
            Self::MysticPurple => "Mystic Purple",
            Self::CosmicIndigo => "Cosmic Indigo",
            Self::NeonCyan => "Neon Cyan",
            Self::VaporwaveDusk => "Vaporwave Dusk",
            Self::AuroraBorealis => "Aurora Borealis",
            Self::LavaLamp => "Lava Lamp",
            Self::ElectricLime => "Electric Lime",
            Self::SunsetHorizon => "Sunset Horizon",
            Self::DeepAbyss => "Deep Abyss",
            Self::HyperPink => "Hyper Pink",
            Self::GoldenHour => "Golden Hour",
            Self::TwilightSky => "Twilight Sky",
            Self::MatrixGreen => "Matrix Green",
            Self::PeachSorbet => "Peach Sorbet",
            Self::MonochromeSilver => "Monochrome Silver",
            Self::AuraWave => "Aura Wave",
            Self::Custom(..) => "Custom",
        }
    }

    pub fn colors(&self) -> ((u8, u8, u8), (u8, u8, u8)) {
        match self {
            Self::DeepStudio => ((22, 22, 26), (36, 36, 44)),
            Self::SunsetOrange => ((255, 87, 34), (255, 142, 60)),
            Self::MidnightNeon => ((15, 23, 42), (56, 189, 248)),
            Self::ForestGlow => ((12, 43, 26), (34, 197, 94)),
            Self::CarbonFiber => ((18, 18, 20), (38, 38, 42)),
            Self::VelvetViolet => ((46, 16, 101), (147, 51, 234)),
            Self::CyberpunkAmber => ((38, 20, 10), (245, 158, 11)),
            Self::ArcticFrost => ((224, 242, 254), (186, 230, 253)),
            Self::ObsidianSlate => ((15, 15, 18), (30, 41, 59)),
            Self::RoseGold => ((244, 63, 94), (251, 146, 60)),
            Self::RoyalBlue => ((29, 78, 216), (96, 165, 250)),
            Self::EmeraldDusk => ((6, 78, 59), (16, 185, 129)),
            Self::CleanWhite => ((248, 250, 252), (241, 245, 249)),
            Self::TitaniumGray => ((51, 65, 85), (100, 116, 139)),
            Self::InternationalOrange => ((255, 69, 0), (255, 107, 43)),
            Self::SolarFlare => ((220, 38, 38), (234, 179, 8)),
            Self::MysticPurple => ((76, 29, 149), (217, 70, 239)),
            Self::CosmicIndigo => ((30, 27, 75), (99, 102, 241)),
            Self::NeonCyan => ((4, 47, 46), (34, 211, 238)),
            Self::VaporwaveDusk => ((131, 24, 67), (147, 51, 234)),
            Self::AuroraBorealis => ((6, 78, 59), (56, 189, 248)),
            Self::LavaLamp => ((127, 29, 29), (249, 115, 22)),
            Self::ElectricLime => ((20, 83, 45), (163, 230, 53)),
            Self::SunsetHorizon => ((124, 45, 18), (251, 146, 60)),
            Self::DeepAbyss => ((3, 7, 18), (30, 41, 59)),
            Self::HyperPink => ((159, 18, 57), (251, 113, 133)),
            Self::GoldenHour => ((113, 63, 18), (250, 204, 21)),
            Self::TwilightSky => ((49, 46, 129), (192, 132, 252)),
            Self::MatrixGreen => ((5, 46, 22), (74, 222, 128)),
            Self::PeachSorbet => ((234, 88, 12), (253, 186, 116)),
            Self::MonochromeSilver => ((24, 24, 27), (113, 113, 122)),
            Self::AuraWave => ((14, 116, 144), (236, 72, 153)),
            Self::Custom(r1, g1, b1, r2, g2, b2) => ((*r1, *g1, *b1), (*r2, *g2, *b2)),
        }
    }
}

/// Target aspect ratios for modal output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetAspectRatio {
    Auto,
    Ratio1x1,
    Ratio16x9,
    Ratio4x3,
    Ratio3x2,
    Ratio9x16,
}

impl TargetAspectRatio {
    pub fn ratio(&self) -> Option<f32> {
        match self {
            Self::Auto => None,
            Self::Ratio1x1 => Some(1.0),
            Self::Ratio16x9 => Some(16.0 / 9.0),
            Self::Ratio4x3 => Some(4.0 / 3.0),
            Self::Ratio3x2 => Some(3.0 / 2.0),
            Self::Ratio9x16 => Some(9.0 / 16.0),
        }
    }
}

/// Loads an image from a path.
pub fn load_image(path: &Path) -> Result<DynamicImage> {
    let reader = ImageReader::open(path)
        .with_context(|| format!("Failed to open file: {}", path.display()))?
        .with_guessed_format()
        .with_context(|| format!("Failed to identify format of: {}", path.display()))?;

    reader
        .decode()
        .with_context(|| format!("Failed to decode image: {}", path.display()))
}

/// Converts and saves an image to target extension.
pub fn convert_image_format(source_path: &Path, output_path: &Path, target_ext: &str) -> Result<()> {
    let img = load_image(source_path)?;
    save_image_with_format(&img, output_path, target_ext)
}

/// Saves dynamic image matching target extension.
pub fn save_image_with_format(img: &DynamicImage, output_path: &Path, target_ext: &str) -> Result<()> {
    let ext_lower = target_ext.trim_start_matches('.').to_lowercase();
    match ext_lower.as_str() {
        "png" => img.save_with_format(output_path, ImageFormat::Png)?,
        "jpg" | "jpeg" => {
            let rgb = img.to_rgb8();
            rgb.save_with_format(output_path, ImageFormat::Jpeg)?;
        }
        "webp" => img.save_with_format(output_path, ImageFormat::WebP)?,
        "avif" => img.save_with_format(output_path, ImageFormat::Avif)?,
        "tiff" | "tif" => img.save_with_format(output_path, ImageFormat::Tiff)?,
        "bmp" => img.save_with_format(output_path, ImageFormat::Bmp)?,
        "ico" => {
            // For ICO, scale to standard 256x256 if larger
            let ico_img = if img.width() > 256 || img.height() > 256 {
                img.resize(256, 256, FilterType::Lanczos3)
            } else {
                img.clone()
            };
            ico_img.save_with_format(output_path, ImageFormat::Ico)?;
        }
        "pdf" => {
            // Save image inside a clean PDF page
            save_image_as_pdf(img, output_path)?;
        }
        other => {
            return Err(anyhow!("Unsupported target image format: {}", other));
        }
    }
    Ok(())
}

/// Saves an image wrapped into a PDF document using Cairo or Lopdf.
pub fn save_image_as_pdf(img: &DynamicImage, output_path: &Path) -> Result<()> {
    let w = img.width() as f64;
    let h = img.height() as f64;

    let surface = cairo::PdfSurface::new(w, h, output_path.to_str().ok_or_else(|| anyhow!("Invalid output path"))?)
        .map_err(|e| anyhow!("Cairo PDF surface creation failed: {:?}", e))?;
    let cr = cairo::Context::new(&surface)
        .map_err(|e| anyhow!("Cairo context creation failed: {:?}", e))?;

    let rgba = img.to_rgba8();
    let mut bgra_data = Vec::with_capacity((w * h * 4.0) as usize);
    for pixel in rgba.pixels() {
        bgra_data.push(pixel[2]); // B
        bgra_data.push(pixel[1]); // G
        bgra_data.push(pixel[0]); // R
        bgra_data.push(pixel[3]); // A
    }

    let stride = cairo::Format::ARgb32.stride_for_width(img.width())
        .map_err(|e| anyhow!("Invalid stride: {:?}", e))?;
    let image_surface = cairo::ImageSurface::create_for_data(
        bgra_data,
        cairo::Format::ARgb32,
        img.width() as i32,
        img.height() as i32,
        stride,
    ).map_err(|e| anyhow!("Cairo image surface error: {:?}", e))?;

    cr.set_source_surface(&image_surface, 0.0, 0.0)
        .map_err(|e| anyhow!("Set surface error: {:?}", e))?;
    cr.paint().map_err(|e| anyhow!("Cairo paint error: {:?}", e))?;

    surface.finish();
    Ok(())
}

/// Crops an image to `(x, y, width, height)`. Clamps bounds safely.
pub fn crop_image(img: &DynamicImage, x: u32, y: u32, width: u32, height: u32) -> DynamicImage {
    let img_w = img.width();
    let img_h = img.height();

    let safe_x = x.min(img_w.saturating_sub(1));
    let safe_y = y.min(img_h.saturating_sub(1));
    let safe_w = width.min(img_w.saturating_sub(safe_x)).max(1);
    let safe_h = height.min(img_h.saturating_sub(safe_y)).max(1);

    img.crop_imm(safe_x, safe_y, safe_w, safe_h)
}

/// Applies smooth rounded corners to an RGBA image.
pub fn apply_rounded_corners(img: &RgbaImage, radius: f32) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    if radius <= 0.5 {
        return img.clone();
    }

    let r = radius.min((w as f32) / 2.0).min((h as f32) / 2.0);
    let mut output = img.clone();

    for y in 0..h {
        for x in 0..w {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;

            // Check four corners
            let mut corner_dist_sq = None;
            if px < r && py < r {
                // Top-left
                corner_dist_sq = Some((px - r).powi(2) + (py - r).powi(2));
            } else if px > (w as f32 - r) && py < r {
                // Top-right
                corner_dist_sq = Some((px - (w as f32 - r)).powi(2) + (py - r).powi(2));
            } else if px < r && py > (h as f32 - r) {
                // Bottom-left
                corner_dist_sq = Some((px - r).powi(2) + (py - (h as f32 - r)).powi(2));
            } else if px > (w as f32 - r) && py > (h as f32 - r) {
                // Bottom-right
                corner_dist_sq = Some((px - (w as f32 - r)).powi(2) + (py - (h as f32 - r)).powi(2));
            }

            if let Some(dist_sq) = corner_dist_sq {
                let dist = dist_sq.sqrt();
                if dist > r {
                    // Fully outside corner circle
                    output.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                } else if dist > (r - 1.0) {
                    // Antialiased transition border
                    let alpha_mult = 1.0 - (dist - (r - 1.0));
                    let p = output.get_pixel_mut(x, y);
                    p[3] = ((p[3] as f32) * alpha_mult).round() as u8;
                }
            }
        }
    }

    output
}

/// Renders an image with studio backdrop, custom padding, rounded corners, and drop shadow.
pub fn render_studio_background(
    img: &DynamicImage,
    preset: StudioBackgroundPreset,
    padding: u32,
    corner_radius: f32,
    shadow_intensity: f32,
    target_ratio: TargetAspectRatio,
) -> RgbaImage {
    let rounded_img = apply_rounded_corners(&img.to_rgba8(), corner_radius);
    let inner_w = rounded_img.width();
    let inner_h = rounded_img.height();

    // Calculate content dimension with padding
    let content_w = inner_w + padding * 2;
    let content_h = inner_h + padding * 2;

    // Determine canvas dimensions based on target aspect ratio
    let (canvas_w, canvas_h) = match target_ratio.ratio() {
        Some(ratio) => {
            let current_ratio = (content_w as f32) / (content_h as f32);
            if current_ratio > ratio {
                // Content is wider than target ratio
                let w = content_w;
                let h = ((content_w as f32) / ratio).round() as u32;
                (w, h.max(content_h))
            } else {
                // Content is taller than target ratio
                let h = content_h;
                let w = ((content_h as f32) * ratio).round() as u32;
                (w.max(content_w), h)
            }
        }
        None => (content_w, content_h),
    };

    let mut canvas = RgbaImage::new(canvas_w, canvas_h);
    let ((r1, g1, b1), (r2, g2, b2)) = preset.colors();

    // Render smooth gradient backdrop
    for y in 0..canvas_h {
        let t = (y as f32) / (canvas_h.max(1) as f32);
        let cr = ((1.0 - t) * (r1 as f32) + t * (r2 as f32)).round() as u8;
        let cg = ((1.0 - t) * (g1 as f32) + t * (g2 as f32)).round() as u8;
        let cb = ((1.0 - t) * (b1 as f32) + t * (b2 as f32)).round() as u8;

        for x in 0..canvas_w {
            canvas.put_pixel(x, y, Rgba([cr, cg, cb, 255]));
        }
    }

    // Offset of inner image to center it
    let inner_x = (canvas_w.saturating_sub(inner_w)) / 2;
    let inner_y = (canvas_h.saturating_sub(inner_h)) / 2;

    // Render soft drop shadow if enabled
    if shadow_intensity > 0.01 {
        let shadow_blur = ((inner_w.max(inner_h) as f32) * 0.04 * shadow_intensity).clamp(6.0, 48.0);
        let shadow_offset_y = (shadow_blur * 0.6).round() as i32;
        let shadow_alpha_max = (0.45 * shadow_intensity).clamp(0.0, 0.7);

        let blur_r = shadow_blur.ceil() as i32;
        let x_start = (inner_x as i32 - blur_r).max(0) as u32;
        let x_end = ((inner_x + inner_w) as i32 + blur_r).min(canvas_w as i32) as u32;
        let y_start = (inner_y as i32 - blur_r + shadow_offset_y).max(0) as u32;
        let y_end = ((inner_y + inner_h) as i32 + blur_r + shadow_offset_y).min(canvas_h as i32) as u32;

        for py in y_start..y_end {
            for px in x_start..x_end {
                // Compute distance from box
                let box_x = (px as f32).clamp(inner_x as f32, (inner_x + inner_w) as f32);
                let box_y = ((py as i32 - shadow_offset_y) as f32).clamp(inner_y as f32, (inner_y + inner_h) as f32);
                let d = ((px as f32 - box_x).powi(2) + ((py as i32 - shadow_offset_y) as f32 - box_y).powi(2)).sqrt();

                if d < shadow_blur {
                    let falloff = 1.0 - (d / shadow_blur);
                    let shadow_a = falloff * falloff * shadow_alpha_max;
                    let bg = canvas.get_pixel(px, py);
                    let r = ((bg[0] as f32) * (1.0 - shadow_a)).round() as u8;
                    let g = ((bg[1] as f32) * (1.0 - shadow_a)).round() as u8;
                    let b = ((bg[2] as f32) * (1.0 - shadow_a)).round() as u8;
                    canvas.put_pixel(px, py, Rgba([r, g, b, 255]));
                }
            }
        }
    }

    // Blend inner image onto canvas
    imageops::overlay(&mut canvas, &rounded_img, inner_x as i64, inner_y as i64);

    canvas
}

/// Auto-trims transparent borders around an image.
pub fn auto_trim_transparent(img: &RgbaImage) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    let mut min_x = w;
    let mut max_x = 0;
    let mut min_y = h;
    let mut max_y = 0;

    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y);
            if p[3] > 10 {
                if x < min_x { min_x = x; }
                if x > max_x { max_x = x; }
                if y < min_y { min_y = y; }
                if y > max_y { max_y = y; }
            }
        }
    }

    if min_x > max_x || min_y > max_y {
        return img.clone();
    }

    let crop_w = (max_x - min_x + 1).max(1);
    let crop_h = (max_y - min_y + 1).max(1);
    imageops::crop_imm(img, min_x, min_y, crop_w, crop_h).to_image()
}

/// Adjusts brightness (-100 to +100) and contrast (-100 to +100).
pub fn adjust_brightness_contrast(img: &DynamicImage, brightness: i32, contrast: f32) -> DynamicImage {
    let mut adjusted = if brightness != 0 {
        img.brighten(brightness)
    } else {
        img.clone()
    };

    if (contrast - 0.0).abs() > 0.01 {
        adjusted = adjusted.adjust_contrast(contrast);
    }
    adjusted
}
