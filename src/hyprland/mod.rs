pub mod autostart;
pub mod installer;
pub mod layer_shell;

pub use autostart::{ensure_daemon_running, is_autostart_enabled, is_daemon_running, set_autostart};

use std::process::Command;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct CursorPos {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MonitorInfo {
    pub id: i64,
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    pub focused: bool,
}

/// Queries `hyprctl cursorpos -j` for current mouse pointer coordinates.
/// Falls back to monitor center or (960, 540) if outside Hyprland.
pub fn get_cursor_position() -> CursorPos {
    if let Ok(output) = Command::new("hyprctl").args(["cursorpos", "-j"]).output()
        && output.status.success()
            && let Ok(pos) = serde_json::from_slice::<CursorPos>(&output.stdout) {
                return pos;
            }

    // Fallback: check focused monitor
    if let Some(mon) = get_focused_monitor() {
        let logical_w = (mon.width as f64 / mon.scale.max(0.1)) as i32;
        let logical_h = (mon.height as f64 / mon.scale.max(0.1)) as i32;
        return CursorPos {
            x: mon.x + logical_w / 2,
            y: mon.y + logical_h / 2,
        };
    }

    CursorPos { x: 960, y: 540 }
}

/// Queries `hyprctl monitors -j` for all connected monitors.
pub fn get_monitors() -> Vec<MonitorInfo> {
    let Ok(output) = Command::new("hyprctl").args(["monitors", "-j"]).output() else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    serde_json::from_slice::<Vec<MonitorInfo>>(&output.stdout).unwrap_or_default()
}

/// Gets the currently focused monitor or first available monitor.
pub fn get_focused_monitor() -> Option<MonitorInfo> {
    let monitors = get_monitors();
    monitors
        .iter()
        .find(|m| m.focused)
        .cloned()
        .or_else(|| monitors.into_iter().next())
}

/// Enforces floating and centering rules for all Hyprdrop windows and studio modals at runtime via `hyprctl`.
pub fn ensure_hyprland_floating_rules() {
    // Modern Lua-based dynamic window rules
    let lua_code = r#"
        if hl and hl.window_rule then
            hl.window_rule({ name = "hyprdrop_float_class", match = { class = ".*hyprdrop.*" }, float = true, center = true })
            hl.window_rule({ name = "hyprdrop_float_title", match = { title = ".*Hyprdrop.*" }, float = true, center = true })
        end
    "#;
    let _ = Command::new("hyprctl").args(["eval", lua_code]).output();

    // Legacy keyword windowrulev2 for classic .conf setups
    let _ = Command::new("hyprctl")
        .args(["keyword", "windowrulev2", "float, class:^(.*hyprdrop.*)$"])
        .output();
    let _ = Command::new("hyprctl")
        .args(["keyword", "windowrulev2", "center, class:^(.*hyprdrop.*)$"])
        .output();
    let _ = Command::new("hyprctl")
        .args(["keyword", "windowrulev2", "float, title:^(.*Hyprdrop.*)$"])
        .output();
    let _ = Command::new("hyprctl")
        .args(["keyword", "windowrulev2", "center, title:^(.*Hyprdrop.*)$"])
        .output();
}
