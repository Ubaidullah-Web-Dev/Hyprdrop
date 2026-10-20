pub mod drop_shelf;
pub mod hotspot;
pub mod modals;
pub mod overlay_window;
pub mod radial_menu;
pub mod settings_app;
pub mod tray;

use gtk4::gdk::Display;
use gtk4::CssProvider;

pub const CSS_THEME: &str = r#"
/* --- Hyprdrop macOS Studio Electric Blue Design System --- */

/* Transparent Layer-Shell Overlay */
window.background.hyprdrop-overlay,
window.hyprdrop-overlay.background,
window.hyprdrop-overlay,
.hyprdrop-overlay,
window.hyprdrop-overlay > box,
window.hyprdrop-overlay > contents,
window.hyprdrop-overlay decoration,
window.hyprdrop-overlay widget,
window.hyprdrop-overlay drawingarea,
.hyprdrop-radial-canvas {
    background-color: transparent;
    background: transparent;
    background-image: none;
    box-shadow: none;
    border-style: none;
}

window.background.hyprdrop-hotspot,
window.hyprdrop-hotspot.background,
window.hyprdrop-hotspot,
.hyprdrop-hotspot,
window.hyprdrop-hotspot > * {
    background-color: transparent;
    background: transparent;
    background-image: none;
    box-shadow: none;
    border-style: none;
}

.hyprdrop-hotspot-bar {
    background-color: rgba(30, 30, 36, 0.45);
    border-left: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 4px 0 0 4px;
    transition: all 180ms cubic-bezier(0.16, 1, 0.3, 1);
}

.hyprdrop-hotspot-bar:hover,
.hyprdrop-hotspot-bar.active {
    background-color: #0A84FF;
    box-shadow: 0 0 16px rgba(10, 132, 255, 0.7);
}

/* macOS Studio Surface */
.hyprdrop-surface,
.mac-window {
    background-color: rgba(18, 18, 22, 0.96);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 20px;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.65);
}

.hyprdrop-card,
.mac-card {
    background-color: #1A1A20;
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 12px;
    padding: 12px;
}

.hyprdrop-title,
.mac-title {
    color: #F8F8FC;
    font-weight: 700;
    font-size: 15px;
    letter-spacing: -0.015em;
}

.hyprdrop-subtitle,
.mac-subtitle {
    color: #8E8E96;
    font-size: 12px;
    font-weight: 500;
}

.mac-section-header {
    color: #8E8E96;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
}

.hyprdrop-mono,
.mac-mono {
    font-family: 'JetBrains Mono', 'SF Mono', 'Fira Code', monospace;
    font-size: 12px;
    color: #E2E2E8;
}

/* Traffic Light Buttons */
.mac-traffic-dot {
    border-radius: 50%;
    min-width: 12px;
    min-height: 12px;
    padding: 0;
    border: none;
}

.mac-dot-close {
    background-color: #FF5F56;
    border: 1px solid #E0443E;
}

.mac-dot-close:hover {
    background-color: #FF7B74;
}

.mac-dot-min {
    background-color: #FFBD2E;
    border: 1px solid #DEA123;
}

.mac-dot-min:hover {
    background-color: #FFCF66;
}

.mac-dot-max {
    background-color: #27C93F;
    border: 1px solid #1AAB29;
}

.mac-dot-max:hover {
    background-color: #42D858;
}

/* Primary Button (macOS Electric Blue) */
button.hyprdrop-primary,
button.mac-primary {
    background-color: #0A84FF;
    color: #FFFFFF;
    font-weight: 600;
    font-size: 13px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 20px;
    padding: 8px 20px;
    box-shadow: 0 4px 16px rgba(10, 132, 255, 0.4);
    transition: all 180ms cubic-bezier(0.16, 1, 0.3, 1);
}

button.hyprdrop-primary:hover,
button.mac-primary:hover {
    background-color: #2E95FF;
    box-shadow: 0 6px 20px rgba(10, 132, 255, 0.55);
}

button.hyprdrop-primary:active,
button.mac-primary:active {
    transform: translateY(1px);
    background-color: #006FD6;
}

/* Secondary Button (macOS Sleek Dark Pill) */
button.hyprdrop-secondary,
button.mac-secondary {
    background-color: #24242C;
    color: #EAEAEE;
    font-weight: 500;
    font-size: 13px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 20px;
    padding: 8px 18px;
    transition: all 150ms ease-out;
}

button.hyprdrop-secondary:hover,
button.mac-secondary:hover {
    background-color: #2E2E38;
    border-color: rgba(255, 255, 255, 0.16);
    color: #FFFFFF;
}

button.hyprdrop-secondary:active,
button.mac-secondary:active {
    background-color: #1A1A20;
    transform: translateY(1px);
}

/* Destructive / Reset Button */
button.hyprdrop-danger {
    background-color: rgba(255, 69, 58, 0.15);
    color: #FF453A;
    border: 1px solid rgba(255, 69, 58, 0.3);
    border-radius: 20px;
    padding: 8px 16px;
}

button.hyprdrop-danger:hover {
    background-color: rgba(255, 69, 58, 0.25);
}

/* Pills & Presets (macOS Segmented Style) */
button.hyprdrop-pill,
button.mac-pill {
    background-color: #222228;
    color: #9E9EA6;
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 16px;
    padding: 5px 14px;
    font-size: 12px;
    font-weight: 600;
    transition: all 150ms ease-out;
}

button.hyprdrop-pill:hover,
button.mac-pill:hover {
    color: #FFFFFF;
    background-color: #2C2C34;
}

button.hyprdrop-pill.active,
button.mac-pill.active {
    background-color: #0A84FF;
    color: #FFFFFF;
    border-color: #0A84FF;
    box-shadow: 0 2px 10px rgba(10, 132, 255, 0.45);
}

/* Badges */
.badge-success {
    background-color: rgba(48, 209, 88, 0.15);
    color: #30D158;
    border-radius: 6px;
    padding: 2px 8px;
    font-weight: 600;
    font-family: monospace;
    font-size: 11px;
}

.badge-warning {
    background-color: rgba(255, 159, 10, 0.15);
    color: #FF9F0A;
    border-radius: 6px;
    padding: 2px 8px;
    font-weight: 600;
    font-size: 11px;
}

/* Drop Shelf Widget */
.hyprdrop-shelf {
    background-color: rgba(18, 18, 22, 0.96);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 16px;
    padding: 14px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.6);
}

/* Sleek macOS Sliders */
scale trough {
    background-color: #24242A;
    border-radius: 4px;
    min-height: 5px;
}

scale highlight {
    background-color: #0A84FF;
    border-radius: 4px;
}

scale slider {
    background-color: #FFFFFF;
    border: 1px solid rgba(0, 0, 0, 0.2);
    border-radius: 50%;
    min-width: 16px;
    min-height: 16px;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.35);
}

scale slider:hover {
    transform: scale(1.1);
}

/* Palette Swatch Buttons */
.mac-swatch-btn {
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    transition: all 150ms ease-out;
    padding: 0;
}

.mac-swatch-btn:hover {
    transform: scale(1.08);
}

.mac-swatch-btn.active {
    box-shadow: 0 0 0 2px #121216, 0 0 0 4px #0A84FF;
    transform: scale(1.05);
}
"#;

pub fn init_css_theme() {
    let provider = CssProvider::new();
    provider.load_from_string(CSS_THEME);

    if let Some(display) = Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );
    }
}
