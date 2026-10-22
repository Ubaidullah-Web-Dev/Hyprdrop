use gtk4::gdk;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use crate::hyprland::{CursorPos, get_cursor_position, get_monitors};

/// Initializes a GTK4 Window as an Overlay Layer Shell window centered on cursor.
pub fn configure_layer_shell_window(
    window: &gtk4::ApplicationWindow,
    width: i32,
    height: i32,
    cursor_override: Option<CursorPos>,
) {
    if !gtk4_layer_shell::is_supported() {
        // Fallback for non-layer-shell environments (X11 / test environments)
        window.set_default_size(width, height);
        return;
    }

    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_namespace(Some("hyprdrop"));
    window.set_keyboard_mode(KeyboardMode::OnDemand);

    let cursor = cursor_override.unwrap_or_else(get_cursor_position);
    let monitors = get_monitors();

    // Find monitor containing cursor using logical dimensions (scaled by m.scale)
    let current_mon = monitors.iter().find(|m| {
        let logical_w = (m.width as f64 / m.scale.max(0.1)) as i32;
        let logical_h = (m.height as f64 / m.scale.max(0.1)) as i32;
        cursor.x >= m.x
            && cursor.x < (m.x + logical_w)
            && cursor.y >= m.y
            && cursor.y < (m.y + logical_h)
    });

    let (mon_x, mon_y, mon_w, mon_h) = match current_mon {
        Some(m) => (
            m.x,
            m.y,
            (m.width as f64 / m.scale.max(0.1)) as i32,
            (m.height as f64 / m.scale.max(0.1)) as i32,
        ),
        None => (0, 0, 1920, 1080),
    };

    // Calculate margins relative to monitor top-left
    let rel_x = cursor.x - mon_x;
    let rel_y = cursor.y - mon_y;

    let half_w = width / 2;
    let half_h = height / 2;

    let margin_left = (rel_x - half_w).clamp(16, (mon_w - width - 16).max(16));
    let margin_top = (rel_y - half_h).clamp(16, (mon_h - height - 16).max(16));

    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Right, false);
    window.set_anchor(Edge::Bottom, false);

    window.set_margin(Edge::Left, margin_left);
    window.set_margin(Edge::Top, margin_top);
}

/// Initializes a GTK4 Window as a full-monitor transparent Overlay Layer Shell window,
/// and computes the (x, y) coordinates within the monitor to center the radial wheel at cursor.
pub fn configure_fullscreen_layer_shell_overlay(
    window: &gtk4::ApplicationWindow,
    content_width: i32,
    content_height: i32,
    cursor_override: Option<CursorPos>,
) -> (f64, f64) {
    let cursor = cursor_override.unwrap_or_else(get_cursor_position);
    let monitors = get_monitors();

    // Find monitor containing cursor using logical dimensions (scaled by m.scale)
    let current_mon = monitors.iter().find(|m| {
        let logical_w = (m.width as f64 / m.scale.max(0.1)) as i32;
        let logical_h = (m.height as f64 / m.scale.max(0.1)) as i32;
        cursor.x >= m.x
            && cursor.x < (m.x + logical_w)
            && cursor.y >= m.y
            && cursor.y < (m.y + logical_h)
    });

    let (mon_x, mon_y, mon_w, mon_h) = match current_mon {
        Some(m) => (
            m.x,
            m.y,
            (m.width as f64 / m.scale.max(0.1)) as i32,
            (m.height as f64 / m.scale.max(0.1)) as i32,
        ),
        None => (0, 0, 1920, 1080),
    };

    let rel_x = cursor.x - mon_x;
    let rel_y = cursor.y - mon_y;

    let half_w = content_width / 2;
    let half_h = content_height / 2;

    let pos_x = (rel_x - half_w).clamp(16, (mon_w - content_width - 16).max(16));
    let pos_y = (rel_y - half_h).clamp(16, (mon_h - content_height - 16).max(16));

    if !gtk4_layer_shell::is_supported() {
        window.set_default_size(content_width, content_height);
        return (0.0, 0.0);
    }

    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_namespace(Some("hyprdrop"));
    window.set_keyboard_mode(KeyboardMode::OnDemand);

    // Anchor to all 4 edges so the window covers the entire monitor as an input dismiss barrier
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Bottom, true);

    window.set_margin(Edge::Left, 0);
    window.set_margin(Edge::Right, 0);
    window.set_margin(Edge::Top, 0);
    window.set_margin(Edge::Bottom, 0);

    // Assign to the monitor where the cursor currently resides
    if let Some(display) = gdk::Display::default() {
        let gdk_monitors = display.monitors();
        for i in 0..gdk_monitors.n_items() {
            if let Some(m) = gdk_monitors.item(i).and_then(|obj| obj.downcast::<gdk::Monitor>().ok()) {
                let geom = m.geometry();
                if cursor.x >= geom.x()
                    && cursor.x < geom.x() + geom.width()
                    && cursor.y >= geom.y()
                    && cursor.y < geom.y() + geom.height()
                {
                    window.set_monitor(Some(&m));
                    break;
                }
            }
        }
    }

    (pos_x as f64, pos_y as f64)
}
