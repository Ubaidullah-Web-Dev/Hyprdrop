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
/* --- Hyprdrop Pure Wayland Premium Design System --- */

/* Transparent Layer-Shell Overlay */
window.background.hyprdrop-overlay,
window.hyprdrop-overlay.background,
window.hyprdrop-overlay,
.hyprdrop-overlay,
window.hyprdrop-overlay fixed,
.hyprdrop-overlay-fixed {
    background-color: rgba(0, 0, 0, 0.002);
    background: rgba(0, 0, 0, 0.002);
    background-image: none;
    box-shadow: none;
    border-style: none;
}

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
window.hyprdrop-hotspot contents,
window.hyprdrop-hotspot decoration,
window.hyprdrop-hotspot widget,
window.hyprdrop-hotspot drawingarea,
window.hyprdrop-hotspot > *,
.hyprdrop-hotspot,
.hyprdrop-hotspot-canvas {
    background-color: transparent;
    background: transparent;
    background-image: none;
    box-shadow: none;
    border-style: none;
    border: none;
}

.hyprdrop-hotspot-bar {
    min-width: 6px;
    min-height: 220px;
    background-color: rgba(18, 18, 24, 0.94);
    border-left: 1px solid rgba(255, 255, 255, 0.22);
    border-top: 1px solid rgba(255, 255, 255, 0.12);
    border-bottom: 1px solid rgba(255, 255, 255, 0.12);
    border-right: none;
    border-radius: 6px 0 0 6px;
    box-shadow: -4px 0 16px rgba(0, 0, 0, 0.55);
    opacity: 1.0;
    margin-right: 0px;
    transition: opacity 260ms cubic-bezier(0.16, 1, 0.3, 1),
                margin-right 260ms cubic-bezier(0.16, 1, 0.3, 1),
                min-width 180ms cubic-bezier(0.16, 1, 0.3, 1),
                background-color 180ms ease,
                border-color 180ms ease,
                box-shadow 180ms ease;
}

.hyprdrop-hotspot-bar.hidden-fullscreen {
    opacity: 0.0;
    margin-right: -24px;
    min-width: 0px;
    box-shadow: none;
    border-color: transparent;
}

.hyprdrop-hotspot-indicator {
    min-width: 2px;
    background-color: #00D2FF;
    border-radius: 2px;
    opacity: 0.90;
    transition: all 180ms ease;
}

.hyprdrop-hotspot-bar.hidden-fullscreen .hyprdrop-hotspot-indicator {
    opacity: 0.0;
}

.hyprdrop-hotspot-bar:hover,
.hyprdrop-hotspot-bar.hovered {
    min-width: 10px;
    background-color: rgba(22, 24, 34, 0.98);
    border-left: 1px solid #00D2FF;
    box-shadow: -4px 0 20px rgba(0, 210, 255, 0.6);
}

.hyprdrop-hotspot-bar:hover .hyprdrop-hotspot-indicator,
.hyprdrop-hotspot-bar.hovered .hyprdrop-hotspot-indicator {
    background-color: #38BDF8;
    opacity: 1.0;
    box-shadow: 0 0 8px #00D2FF;
}

.hyprdrop-hotspot-bar.active {
    min-width: 12px;
    background-color: #00D2FF;
    border-left: 1px solid #FFFFFF;
    box-shadow: -6px 0 24px rgba(0, 210, 255, 0.95);
}

.hyprdrop-hotspot-bar.active .hyprdrop-hotspot-indicator {
    background-color: #FFFFFF;
    opacity: 1.0;
}

/* App & Modal Windows - edge-to-edge solid background so Hyprland handles window border & rounding */
window:not(.hyprdrop-overlay):not(.hyprdrop-hotspot):not(.hyprdrop-shelf),
window:not(.hyprdrop-overlay):not(.hyprdrop-hotspot):not(.hyprdrop-shelf) decoration,
window:not(.hyprdrop-overlay):not(.hyprdrop-hotspot):not(.hyprdrop-shelf).csd,
window:not(.hyprdrop-overlay):not(.hyprdrop-hotspot):not(.hyprdrop-shelf).solid-csd,
window:not(.hyprdrop-overlay):not(.hyprdrop-hotspot):not(.hyprdrop-shelf) > box,
.hyprdrop-surface,
.hyprdrop-hub-window,
.mac-window {
    background-color: #0D0E12;
    color: #F1F3F9;
    border: none;
    border-radius: 0;
    box-shadow: none;
}

window:not(.hyprdrop-overlay):not(.hyprdrop-hotspot):not(.hyprdrop-shelf) decoration {
    border-radius: 0;
    border: none;
    box-shadow: none;
    margin: 0;
    padding: 0;
}

/* --- Sidebar Navigation --- */
.hyprdrop-sidebar {
    background-color: #12141C;
    border-right: 1px solid rgba(255, 255, 255, 0.06);
    padding: 20px 14px;
}

.hyprdrop-brand-icon {
    color: #00D2FF;
}

.hyprdrop-brand-title {
    color: #FFFFFF;
    font-size: 15px;
    font-weight: 800;
    letter-spacing: 0.07em;
}

.hyprdrop-brand-badge {
    color: #00D2FF;
    background: rgba(0, 210, 255, 0.12);
    border: 1px solid rgba(0, 210, 255, 0.25);
    border-radius: 4px;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.06em;
    padding: 1px 5px;
    font-family: 'JetBrains Mono', monospace;
}

.hyprdrop-brand-sub {
    color: #636A7D;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
}

.hyprdrop-nav-item {
    background-color: transparent;
    color: #8E96A8;
    border: 1px solid transparent;
    border-left: 3px solid transparent;
    border-radius: 8px;
    padding: 10px 14px;
    font-size: 13px;
    font-weight: 600;
    transition: all 180ms cubic-bezier(0.16, 1, 0.3, 1);
}

.hyprdrop-nav-item:hover {
    background-color: #1A1D27;
    color: #FFFFFF;
    border-color: rgba(255, 255, 255, 0.04);
}

.hyprdrop-nav-item.active {
    background: rgba(0, 210, 255, 0.09);
    border-left: 3px solid #00D2FF;
    color: #00D2FF;
    border-top: 1px solid rgba(0, 210, 255, 0.15);
    border-bottom: 1px solid rgba(0, 210, 255, 0.15);
    border-right: 1px solid rgba(0, 210, 255, 0.15);
}

.hyprdrop-nav-badge {
    background: rgba(255, 255, 255, 0.07);
    color: #A1A7B8;
    border-radius: 10px;
    padding: 2px 7px;
    font-size: 10px;
    font-weight: 700;
    font-family: monospace;
}

.hyprdrop-nav-item.active .hyprdrop-nav-badge {
    background: rgba(0, 210, 255, 0.22);
    color: #00D2FF;
}

/* Sidebar Footer / Daemon status */
.hyprdrop-sidebar-footer {
    background: #161822;
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 10px;
    padding: 12px;
}

.hyprdrop-sidebar-status-title {
    color: #FFFFFF;
    font-weight: 700;
    font-size: 12px;
}

.badge-success-dot {
    color: #10B981;
    font-size: 13px;
}

.badge-warning-dot {
    color: #F59E0B;
    font-size: 13px;
}

/* --- Content Area Header --- */
.hyprdrop-content-header {
    padding: 18px 24px 14px 24px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.hyprdrop-page-title {
    color: #FFFFFF;
    font-size: 18px;
    font-weight: 700;
    letter-spacing: -0.015em;
}

.hyprdrop-page-desc {
    color: #7A8294;
    font-size: 12px;
    font-weight: 400;
}

.hyprdrop-title,
.mac-title {
    color: #FFFFFF;
    font-weight: 700;
    font-size: 14px;
    letter-spacing: -0.01em;
}

.hyprdrop-title-sm {
    color: #F1F3F9;
    font-weight: 600;
    font-size: 13px;
}

.hyprdrop-subtitle,
.mac-subtitle {
    color: #8E96A8;
    font-size: 12px;
    font-weight: 500;
}

.hyprdrop-section-label,
.mac-section-header {
    color: #636A7D;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
}

.hyprdrop-empty-icon {
    color: #3B4254;
}

/* --- Cards & Lists --- */
.hyprdrop-card,
.mac-card {
    background-color: #14161F;
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 12px;
    padding: 16px;
    transition: all 180ms ease;
}

.hyprdrop-card:hover {
    border-color: rgba(255, 255, 255, 0.10);
}

.hyprdrop-interactive-card {
    background-color: #14161F;
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 10px;
    padding: 12px 14px;
    transition: all 160ms cubic-bezier(0.16, 1, 0.3, 1);
}

.hyprdrop-interactive-card:hover {
    background-color: #1A1D28;
    border-color: rgba(255, 255, 255, 0.12);
    transform: translateY(-1px);
}

/* Action Badges in History */
.hyprdrop-action-badge {
    border-radius: 6px;
    padding: 4px 9px;
    font-weight: 700;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    letter-spacing: 0.03em;
}

.action-cyan {
    background-color: rgba(0, 210, 255, 0.12);
    color: #00D2FF;
    border: 1px solid rgba(0, 210, 255, 0.28);
}

.action-green {
    background-color: rgba(16, 185, 129, 0.12);
    color: #10B981;
    border: 1px solid rgba(16, 185, 129, 0.28);
}

.action-purple {
    background-color: rgba(168, 85, 247, 0.12);
    color: #C084FC;
    border: 1px solid rgba(168, 85, 247, 0.28);
}

.action-amber {
    background-color: rgba(245, 158, 11, 0.12);
    color: #FBBF24;
    border: 1px solid rgba(245, 158, 11, 0.28);
}

.action-blue {
    background-color: rgba(59, 130, 246, 0.12);
    color: #60A5FA;
    border: 1px solid rgba(59, 130, 246, 0.28);
}

.hyprdrop-mono,
.mac-mono {
    font-family: 'JetBrains Mono', 'SF Mono', 'Fira Code', monospace;
    font-size: 12px;
    color: #E2E2E8;
}

.hyprdrop-mono-muted {
    font-family: 'JetBrains Mono', 'SF Mono', 'Fira Code', monospace;
    font-size: 11px;
    color: #636A7D;
}

.hyprdrop-mono-badge {
    font-family: 'JetBrains Mono', 'SF Mono', 'Fira Code', monospace;
    font-size: 12px;
    font-weight: 600;
    color: #00D2FF;
    background: rgba(0, 210, 255, 0.10);
    border: 1px solid rgba(0, 210, 255, 0.22);
    border-radius: 6px;
    padding: 3px 8px;
}

.hyprdrop-preview-pill {
    background-color: #10121A;
    border: 1px solid rgba(0, 210, 255, 0.25);
    border-radius: 6px;
    color: #00D2FF;
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    padding: 6px 12px;
}

.hyprdrop-code-preview textview,
.hyprdrop-code-preview text {
    background-color: #0F1118;
    color: #CBD5E1;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    padding: 10px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.06);
}

/* --- Buttons --- */
button.hyprdrop-primary,
button.mac-primary {
    background: linear-gradient(180deg, #0284C7 0%, #0369A1 100%);
    color: #FFFFFF;
    font-weight: 600;
    font-size: 13px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 8px;
    padding: 8px 18px;
    box-shadow: 0 2px 10px rgba(2, 132, 199, 0.35);
    transition: all 180ms cubic-bezier(0.16, 1, 0.3, 1);
}

button.hyprdrop-primary:hover,
button.mac-primary:hover {
    background: linear-gradient(180deg, #0EA5E9 0%, #0284C7 100%);
    box-shadow: 0 4px 16px rgba(14, 165, 233, 0.45);
    transform: translateY(-1px);
}

button.hyprdrop-primary:active,
button.mac-primary:active {
    transform: translateY(0px);
    background: #0369A1;
}

button.hyprdrop-secondary,
button.mac-secondary {
    background-color: #1C1F2B;
    color: #D1D5DB;
    font-weight: 500;
    font-size: 13px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    padding: 7px 16px;
    transition: all 160ms ease;
}

button.hyprdrop-secondary:hover,
button.mac-secondary:hover {
    background-color: #242838;
    border-color: rgba(255, 255, 255, 0.16);
    color: #FFFFFF;
    transform: translateY(-1px);
}

button.hyprdrop-secondary:active,
button.mac-secondary:active {
    background-color: #151722;
    transform: translateY(0px);
}

button.hyprdrop-ghost {
    background: transparent;
    color: #8E96A8;
    border: 1px solid transparent;
    border-radius: 8px;
    padding: 6px 10px;
    transition: all 150ms ease;
}

button.hyprdrop-ghost:hover {
    background: rgba(255, 255, 255, 0.07);
    color: #FFFFFF;
}

button.hyprdrop-ghost-danger:hover {
    background: rgba(239, 68, 68, 0.16);
    color: #EF4444;
}

button.hyprdrop-btn-sm {
    padding: 4px 10px;
    font-size: 12px;
}

button.hyprdrop-danger {
    background-color: rgba(239, 68, 68, 0.15);
    color: #EF4444;
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 8px;
    padding: 8px 16px;
    transition: all 150ms ease;
}

button.hyprdrop-danger:hover {
    background-color: rgba(239, 68, 68, 0.25);
}

button.hyprdrop-pill,
button.mac-pill {
    background-color: #171A24;
    color: #8E96A8;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    padding: 5px 12px;
    font-size: 12px;
    font-weight: 600;
    transition: all 150ms ease;
}

button.hyprdrop-pill:hover,
button.mac-pill:hover {
    color: #FFFFFF;
    background-color: #202432;
    border-color: rgba(255, 255, 255, 0.14);
}

button.hyprdrop-pill.active,
button.mac-pill.active {
    background-color: #0284C7;
    color: #FFFFFF;
    border-color: #38BDF8;
    box-shadow: 0 2px 10px rgba(2, 132, 199, 0.35);
}

/* --- Badges --- */
.badge-success {
    background-color: rgba(16, 185, 129, 0.14);
    color: #10B981;
    border: 1px solid rgba(16, 185, 129, 0.28);
    border-radius: 6px;
    padding: 3px 8px;
    font-weight: 600;
    font-family: monospace;
    font-size: 11px;
}

.badge-warning {
    background-color: rgba(245, 158, 11, 0.14);
    color: #F59E0B;
    border: 1px solid rgba(245, 158, 11, 0.28);
    border-radius: 6px;
    padding: 3px 8px;
    font-weight: 600;
    font-size: 11px;
}

.badge-danger {
    background-color: rgba(239, 68, 68, 0.14);
    color: #EF4444;
    border: 1px solid rgba(239, 68, 68, 0.28);
    border-radius: 6px;
    padding: 3px 8px;
    font-weight: 600;
    font-family: monospace;
    font-size: 11px;
}

.badge-info {
    background-color: rgba(14, 165, 233, 0.14);
    color: #38BDF8;
    border: 1px solid rgba(14, 165, 233, 0.28);
    border-radius: 6px;
    padding: 3px 8px;
    font-weight: 600;
    font-family: monospace;
    font-size: 11px;
}

/* --- Drop Shelf --- */
.hyprdrop-shelf {
    background-color: rgba(16, 18, 26, 0.96);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 14px;
    padding: 14px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.65);
}

/* --- Input Fields & Controls --- */
entry {
    background-color: #10121A;
    color: #FFFFFF;
    border: 1px solid rgba(255, 255, 255, 0.09);
    border-radius: 8px;
    padding: 8px 12px;
    transition: all 180ms ease;
}

entry:focus {
    border-color: #0284C7;
    box-shadow: 0 0 0 2px rgba(2, 132, 199, 0.25);
}

switch {
    background-color: #1D202C;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 12px;
    min-width: 42px;
    min-height: 22px;
    transition: all 180ms cubic-bezier(0.16, 1, 0.3, 1);
}

switch:checked {
    background-color: #0284C7;
    border-color: #38BDF8;
}

switch slider {
    background-color: #D1D5DB;
    border-radius: 50%;
    min-width: 16px;
    min-height: 16px;
    margin: 2px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
    transition: all 180ms ease;
}

switch:checked slider {
    background-color: #FFFFFF;
}

scale trough {
    background-color: #1D202C;
    border-radius: 4px;
    min-height: 6px;
}

scale highlight {
    background: linear-gradient(90deg, #0284C7 0%, #38BDF8 100%);
    border-radius: 4px;
}

scale slider {
    background-color: #FFFFFF;
    border: 1px solid rgba(0, 0, 0, 0.2);
    border-radius: 50%;
    min-width: 16px;
    min-height: 16px;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.35);
    transition: transform 120ms ease;
}

scale slider:hover {
    transform: scale(1.15);
}

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
    box-shadow: 0 0 0 2px #0D0E12, 0 0 0 4px #00D2FF;
    transform: scale(1.05);
}

/* Hotspot Studio & Palette Swatches */
.hyprdrop-monitor-canvas {
    background: transparent;
    border-radius: 12px;
}

button.hyprdrop-swatch-circle,
.hyprdrop-swatch-circle {
    min-width: 22px;
    min-height: 22px;
    border-radius: 9999px;
    border: 1px solid rgba(255, 255, 255, 0.35);
    padding: 0;
    margin: 0 3px;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
    background-image: none;
    transition: transform 140ms ease, box-shadow 140ms ease;
}

button.hyprdrop-swatch-circle:hover,
.hyprdrop-swatch-circle:hover {
    transform: scale(1.28);
    border-color: #FFFFFF;
    box-shadow: 0 0 12px rgba(255, 255, 255, 0.6);
}

.hyprdrop-active-theme-card {
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 12px;
    padding: 16px 20px;
}

.swatch-cyan { background-color: #00D2FF; }
.swatch-emerald { background-color: #10B981; }
.swatch-purple { background-color: #A855F7; }
.swatch-amber { background-color: #F59E0B; }
.swatch-rose { background-color: #EC4899; }
.swatch-white { background-color: #F8FAFC; }
.swatch-dark { background-color: #1E2230; }

.hyprdrop-theme-card {
    background-color: #14161F;
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 12px;
    padding: 16px;
    transition: all 180ms ease;
}

.hyprdrop-theme-card:hover {
    background-color: #181B26;
    border-color: rgba(0, 210, 255, 0.25);
    transform: translateY(-2px);
}

.hyprdrop-theme-pill {
    min-width: 14px;
    min-height: 14px;
    border-radius: 50%;
    border: 1px solid rgba(255, 255, 255, 0.2);
}

/* Minimal Custom Scrollbar */
scrollbar {
    background: transparent;
}

scrollbar trough {
    background: transparent;
}

scrollbar slider {
    background-color: rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    min-width: 6px;
    min-height: 24px;
}

scrollbar slider:hover {
    background-color: rgba(255, 255, 255, 0.24);
}
"#;

use std::cell::RefCell;
use crate::config::AppTheme;

pub fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() >= 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&clean[0..2], 16),
            u8::from_str_radix(&clean[2..4], 16),
            u8::from_str_radix(&clean[4..6], 16),
        ) {
            return (r, g, b);
        }
    }
    (0, 210, 255)
}

pub fn generate_css_theme(theme: &AppTheme) -> String {
    let (p_r, p_g, p_b) = hex_to_rgb(&theme.primary);
    let (s_r, s_g, s_b) = hex_to_rgb(&theme.surface);

    format!(r#"
/* Dynamic Master Theme: {} */
window.background.hyprdrop-hotspot,
window.hyprdrop-hotspot.background,
window.hyprdrop-hotspot,
window.hyprdrop-hotspot contents,
window.hyprdrop-hotspot decoration,
window.hyprdrop-hotspot widget,
window.hyprdrop-hotspot drawingarea,
window.hyprdrop-hotspot > *,
.hyprdrop-hotspot,
.hyprdrop-hotspot-canvas {{
    background-color: transparent;
    background: transparent;
    background-image: none;
    box-shadow: none;
    border-style: none;
    border: none;
}}

.hyprdrop-active-theme-card {{
    border-color: rgba({p_r}, {p_g}, {p_b}, 0.50);
    background: linear-gradient(135deg, rgba({p_r}, {p_g}, {p_b}, 0.12) 0%, {card_bg} 100%);
    box-shadow: 0 4px 24px rgba({p_r}, {p_g}, {p_b}, 0.20);
}}

.hyprdrop-theme-card-active {{
    border: 2px solid {primary};
    background-color: rgba({p_r}, {p_g}, {p_b}, 0.08);
    box-shadow: 0 0 16px rgba({p_r}, {p_g}, {p_b}, 0.28);
}}

window:not(.hyprdrop-overlay):not(.hyprdrop-hotspot):not(.hyprdrop-shelf),
window:not(.hyprdrop-overlay):not(.hyprdrop-hotspot):not(.hyprdrop-shelf) decoration,
window:not(.hyprdrop-overlay):not(.hyprdrop-hotspot):not(.hyprdrop-shelf).csd,
window:not(.hyprdrop-overlay):not(.hyprdrop-hotspot):not(.hyprdrop-shelf).solid-csd,
window:not(.hyprdrop-overlay):not(.hyprdrop-hotspot):not(.hyprdrop-shelf) > box,
.hyprdrop-surface,
.hyprdrop-hub-window,
.mac-window {{
    background-color: {surface};
    color: #F1F3F9;
}}

.hyprdrop-sidebar {{
    background-color: rgba({s_r}, {s_g}, {s_b}, 0.98);
    border-right: 1px solid rgba(255, 255, 255, 0.07);
}}

.hyprdrop-brand-icon {{
    color: {primary};
}}

.hyprdrop-brand-badge {{
    color: {primary};
    background: rgba({p_r}, {p_g}, {p_b}, 0.14);
    border: 1px solid rgba({p_r}, {p_g}, {p_b}, 0.35);
}}

.hyprdrop-nav-item.active {{
    background: rgba({p_r}, {p_g}, {p_b}, 0.12);
    border-left: 3px solid {primary};
    color: {primary};
    border-top: 1px solid rgba({p_r}, {p_g}, {p_b}, 0.22);
    border-bottom: 1px solid rgba({p_r}, {p_g}, {p_b}, 0.22);
    border-right: 1px solid rgba({p_r}, {p_g}, {p_b}, 0.22);
}}

.hyprdrop-nav-item.active .hyprdrop-nav-badge {{
    background: rgba({p_r}, {p_g}, {p_b}, 0.26);
    color: {primary};
}}

button.hyprdrop-primary,
button.mac-primary {{
    background: linear-gradient(180deg, {secondary} 0%, {accent} 100%);
    color: #FFFFFF;
    border: 1px solid rgba(255, 255, 255, 0.2);
    box-shadow: 0 2px 10px rgba({p_r}, {p_g}, {p_b}, 0.38);
}}

button.hyprdrop-primary:hover,
button.mac-primary:hover {{
    background: linear-gradient(180deg, {primary} 0%, {secondary} 100%);
    box-shadow: 0 4px 16px rgba({p_r}, {p_g}, {p_b}, 0.50);
}}

button.hyprdrop-pill.active,
button.mac-pill.active {{
    background-color: {primary};
    color: #FFFFFF;
    border-color: {secondary};
    box-shadow: 0 2px 10px rgba({p_r}, {p_g}, {p_b}, 0.35);
}}

.hyprdrop-card,
.mac-card,
.hyprdrop-theme-card {{
    background-color: {card_bg};
    border: 1px solid rgba(255, 255, 255, 0.07);
}}

.hyprdrop-interactive-card {{
    background-color: {card_bg};
    border: 1px solid rgba(255, 255, 255, 0.06);
}}

.hyprdrop-theme-card:hover,
.hyprdrop-interactive-card:hover {{
    border-color: rgba({p_r}, {p_g}, {p_b}, 0.35);
}}

switch:checked {{
    background-color: {primary};
    border-color: {secondary};
}}

scale highlight {{
    background: linear-gradient(90deg, {accent} 0%, {primary} 100%);
}}

entry:focus {{
    border-color: {primary};
    box-shadow: 0 0 0 2px rgba({p_r}, {p_g}, {p_b}, 0.25);
}}

.badge-info {{
    background-color: rgba({p_r}, {p_g}, {p_b}, 0.14);
    color: {primary};
    border: 1px solid rgba({p_r}, {p_g}, {p_b}, 0.30);
}}

.hyprdrop-preview-pill,
.hyprdrop-mono-badge {{
    color: {primary};
    background: rgba({p_r}, {p_g}, {p_b}, 0.10);
    border: 1px solid rgba({p_r}, {p_g}, {p_b}, 0.28);
}}

.hyprdrop-shelf {{
    background-color: rgba({s_r}, {s_g}, {s_b}, 0.96);
    border: 1px solid rgba({p_r}, {p_g}, {p_b}, 0.32);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.65), 0 0 24px rgba({p_r}, {p_g}, {p_b}, 0.20);
}}
"#,
        theme.name,
        surface = theme.surface,
        card_bg = theme.card_bg,
        primary = theme.primary,
        secondary = theme.secondary,
        accent = theme.accent,
        s_r = s_r,
        s_g = s_g,
        s_b = s_b,
        p_r = p_r,
        p_g = p_g,
        p_b = p_b,
    )
}

thread_local! {
    static ACTIVE_CSS_PROVIDER: RefCell<Option<CssProvider>> = const { RefCell::new(None) };
}

pub fn apply_theme(theme: &AppTheme) {
    let css = format!("{}\n{}", CSS_THEME, generate_css_theme(theme));
    let provider = CssProvider::new();
    provider.load_from_string(&css);

    if let Some(display) = Display::default() {
        ACTIVE_CSS_PROVIDER.with(|cell| {
            if let Some(old) = cell.borrow().as_ref() {
                gtk4::style_context_remove_provider_for_display(&display, old);
            }
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 20,
            );
            *cell.borrow_mut() = Some(provider);
        });
    }
}

pub fn init_css_theme() {
    let cfg = crate::config::ConfigManager::load_config();
    apply_theme(&cfg.theme);
}
