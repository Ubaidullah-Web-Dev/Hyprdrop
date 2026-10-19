pub mod drop_shelf;
pub mod hotspot;
pub mod modals;
pub mod overlay_window;
pub mod radial_menu;
pub mod settings_app;

use gtk4::gdk::Display;
use gtk4::CssProvider;

pub const CSS_THEME: &str = r#"
/* --- Hyprdrop Industrial Dark Studio Theme --- */

window.hyprdrop-overlay {
    background-color: transparent;
}

window.hyprdrop-hotspot {
    background-color: transparent;
}

.hyprdrop-hotspot-bar {
    background-color: rgba(33, 33, 36, 0.45);
    border-left: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 4px 0 0 4px;
    transition: all 150ms ease-out;
}

.hyprdrop-hotspot-bar:hover,
.hyprdrop-hotspot-bar.active {
    background-color: #FF5722;
    box-shadow: 0 0 14px rgba(255, 87, 34, 0.6);
}

.hyprdrop-surface {
    background-color: rgba(22, 22, 24, 0.94);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    box-shadow: 0 16px 36px rgba(0, 0, 0, 0.45);
}

.hyprdrop-card {
    background-color: #212124;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    padding: 10px;
}

.hyprdrop-title {
    color: #F4F4F6;
    font-weight: 700;
    font-size: 15px;
    letter-spacing: -0.02em;
}

.hyprdrop-subtitle {
    color: #9E9EA8;
    font-size: 12px;
}

.hyprdrop-mono {
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 12px;
}

/* Primary Button (International Warm Orange) */
button.hyprdrop-primary {
    background-color: #FF5722;
    color: #FFFFFF;
    font-weight: 600;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    padding: 8px 16px;
    transition: background-color 150ms ease-out, transform 100ms ease-out;
}

button.hyprdrop-primary:hover {
    background-color: #FF784E;
}

button.hyprdrop-primary:active {
    transform: translateY(1px);
    background-color: #E64A19;
}

/* Secondary Button */
button.hyprdrop-secondary {
    background-color: #212124;
    color: #F4F4F6;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    padding: 8px 14px;
    transition: background-color 150ms ease-out;
}

button.hyprdrop-secondary:hover {
    background-color: #2E2E33;
    border-color: rgba(255, 255, 255, 0.16);
}

button.hyprdrop-secondary:active {
    background-color: #1A1A1C;
    transform: translateY(1px);
}

/* Destructive / Reset Button */
button.hyprdrop-danger {
    background-color: rgba(239, 68, 68, 0.15);
    color: #EF4444;
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 6px;
    padding: 8px 14px;
}

button.hyprdrop-danger:hover {
    background-color: rgba(239, 68, 68, 0.25);
}

/* Pills & Presets */
button.hyprdrop-pill {
    background-color: #212124;
    color: #9E9EA8;
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 16px;
    padding: 4px 12px;
    font-size: 11px;
    font-weight: 600;
}

button.hyprdrop-pill:hover {
    color: #F4F4F6;
    background-color: #2C2C30;
}

button.hyprdrop-pill.active {
    background-color: #FF5722;
    color: #FFFFFF;
    border-color: #FF784E;
}

/* Badges */
.badge-success {
    background-color: rgba(16, 185, 129, 0.15);
    color: #10B981;
    border-radius: 4px;
    padding: 2px 8px;
    font-weight: 600;
    font-family: monospace;
    font-size: 11px;
}

.badge-warning {
    background-color: rgba(245, 158, 11, 0.15);
    color: #F59E0B;
    border-radius: 4px;
    padding: 2px 8px;
    font-weight: 600;
    font-size: 11px;
}

/* Drop Shelf Widget */
.hyprdrop-shelf {
    background-color: rgba(22, 22, 24, 0.96);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 12px;
    padding: 12px;
    box-shadow: 0 12px 28px rgba(0, 0, 0, 0.5);
}

/* Sliders */
scale trough {
    background-color: #212124;
    border-radius: 4px;
    min-height: 6px;
}

scale highlight {
    background-color: #FF5722;
    border-radius: 4px;
}

scale slider {
    background-color: #F4F4F6;
    border-radius: 50%;
    min-width: 14px;
    min-height: 14px;
}

/* Studio Background 16 Presets */
.bg-preset-0 { background: linear-gradient(135deg, rgb(22, 22, 26), rgb(36, 36, 44)); border-radius: 6px; border: 1px solid rgba(255,255,255,0.15); }
.bg-preset-1 { background: linear-gradient(135deg, rgb(255, 87, 34), rgb(255, 142, 60)); border-radius: 6px; border: 1px solid rgba(255,255,255,0.15); }
.bg-preset-2 { background: linear-gradient(135deg, rgb(15, 23, 42), rgb(56, 189, 248)); border-radius: 6px; border: 1px solid rgba(255,255,255,0.15); }
.bg-preset-3 { background: linear-gradient(135deg, rgb(12, 43, 26), rgb(34, 197, 94)); border-radius: 6px; border: 1px solid rgba(255,255,255,0.15); }
.bg-preset-4 { background: linear-gradient(135deg, rgb(18, 18, 20), rgb(38, 38, 42)); border-radius: 6px; border: 1px solid rgba(255,255,255,0.15); }
.bg-preset-5 { background: linear-gradient(135deg, rgb(46, 16, 101), rgb(147, 51, 234)); border-radius: 6px; border: 1px solid rgba(255,255,255,0.15); }
.bg-preset-6 { background: linear-gradient(135deg, rgb(38, 20, 10), rgb(245, 158, 11)); border-radius: 6px; border: 1px solid rgba(255,255,255,0.15); }
.bg-preset-7 { background: linear-gradient(135deg, rgb(224, 242, 254), rgb(186, 230, 253)); border-radius: 6px; border: 1px solid rgba(255,255,255,0.15); }
.bg-preset-8 { background: linear-gradient(135deg, rgb(15, 15, 18), rgb(30, 41, 59)); border-radius: 6px; border: 1px solid rgba(255,255,255,0.15); }
.bg-preset-9 { background: linear-gradient(135deg, rgb(244, 63, 94), rgb(251, 146, 60)); border-radius: 6px; border: 1px solid rgba(255,255,255,0.15); }
.bg-preset-10 { background: linear-gradient(135deg, rgb(29, 78, 216), rgb(96, 165, 250)); border-radius: 6px; border: 1px solid rgba(255,255,255,0.15); }
.bg-preset-11 { background: linear-gradient(135deg, rgb(6, 78, 59), rgb(16, 185, 129)); border-radius: 6px; border: 1px solid rgba(255,255,255,0.15); }
.bg-preset-12 { background: linear-gradient(135deg, rgb(248, 250, 252), rgb(241, 245, 249)); border-radius: 6px; border: 1px solid rgba(255,255,255,0.15); }
.bg-preset-13 { background: linear-gradient(135deg, rgb(51, 65, 85), rgb(100, 116, 139)); border-radius: 6px; border: 1px solid rgba(255,255,255,0.15); }
.bg-preset-14 { background: linear-gradient(135deg, rgb(255, 69, 0), rgb(255, 107, 43)); border-radius: 6px; border: 1px solid rgba(255,255,255,0.15); }
.bg-preset-15 { background: linear-gradient(135deg, rgb(220, 38, 38), rgb(234, 179, 8)); border-radius: 6px; border: 1px solid rgba(255,255,255,0.15); }
"#;

pub fn init_css_theme() {
    let provider = CssProvider::new();
    provider.load_from_string(CSS_THEME);

    if let Some(display) = Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
