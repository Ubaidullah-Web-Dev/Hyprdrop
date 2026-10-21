use std::cell::RefCell;
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use crate::hyprland::get_cursor_position;
use crate::ui::overlay_window::{OverlayWindow, extract_paths_from_drop_value};

thread_local! {
    static ACTIVE_HOTSPOT: RefCell<Option<gtk4::ApplicationWindow>> = const { RefCell::new(None) };
}

pub struct HotspotWindow;

impl HotspotWindow {
    /// Spawns a slim, persistent edge drop target hotspot anchored to the screen edge.
    /// This acts as a reliable fallback when file managers intercept keyboard grabs.
    pub fn spawn(app: &gtk4::Application) {
        if !gtk4_layer_shell::is_supported() {
            return;
        }

        Self::close();

        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("Hyprdrop Edge Hotspot")
            .default_width(72)
            .default_height(360)
            .resizable(false)
            .decorated(false)
            .build();

        window.add_css_class("hyprdrop-hotspot");

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_namespace(Some("hyprdrop-hotspot"));

        // Anchor to the Right edge, centered vertically
        window.set_anchor(Edge::Right, true);
        window.set_anchor(Edge::Top, false);
        window.set_anchor(Edge::Bottom, false);
        window.set_anchor(Edge::Left, false);
        window.set_margin(Edge::Right, 0);

        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        container.set_halign(gtk4::Align::End);
        container.set_valign(gtk4::Align::Center);
        container.set_margin_top(36);
        container.set_margin_bottom(36);
        container.set_margin_start(36);

        let bar = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        bar.set_size_request(6, 220);
        bar.add_css_class("hyprdrop-hotspot-bar");
        bar.set_tooltip_text(Some("Hyprdrop — Drag files here to convert or edit"));
        bar.set_valign(gtk4::Align::Center);

        let indicator = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        indicator.set_size_request(2, -1);
        indicator.set_vexpand(true);
        indicator.set_margin_top(8);
        indicator.set_margin_bottom(8);
        indicator.set_halign(gtk4::Align::Center);
        indicator.add_css_class("hyprdrop-hotspot-indicator");
        bar.append(&indicator);

        container.append(&bar);

        // Drop target on hotspot
        let drop_target = gtk4::DropTarget::new(
            glib::types::Type::INVALID,
            gdk::DragAction::COPY | gdk::DragAction::MOVE,
        );
        drop_target.set_preload(true);
        drop_target.set_types(&[
            gdk::FileList::static_type(),
            gtk4::gio::File::static_type(),
            glib::types::Type::STRING,
        ]);

        let app_drop = app.clone();
        let bar_drop = bar.clone();

        drop_target.connect_enter(move |_, _, _| {
            bar_drop.add_css_class("active");
            gdk::DragAction::COPY
        });

        let bar_leave = bar.clone();
        drop_target.connect_leave(move |_| {
            bar_leave.remove_css_class("active");
        });

        let bar_drop_target = bar.clone();
        drop_target.connect_drop(move |_, val, _, _| {
            bar_drop_target.remove_css_class("active");
            let paths = extract_paths_from_drop_value(val);
            if paths.is_empty() {
                return false;
            }

            // Spawn radial overlay at current cursor position with dropped files
            let _cursor = get_cursor_position();
            OverlayWindow::spawn_with_files(&app_drop, paths);
            true
        });

        window.add_controller(drop_target);

        // Click on hotspot opens radial overlay at cursor
        let click = gtk4::GestureClick::new();
        let app_click = app.clone();
        click.connect_pressed(move |_, _, _, _| {
            OverlayWindow::spawn(&app_click, None);
        });
        window.add_controller(click);

        // Motion controller to guarantee visual hover feedback
        let motion = gtk4::EventControllerMotion::new();
        let bar_motion = bar.clone();
        motion.connect_enter(move |_, _, _| {
            bar_motion.add_css_class("hovered");
        });
        let bar_motion_leave = bar.clone();
        motion.connect_leave(move |_| {
            bar_motion_leave.remove_css_class("hovered");
        });
        window.add_controller(motion);

        window.set_child(Some(&container));

        let win_weak = window.downgrade();
        window.connect_destroy(move |_| {
            ACTIVE_HOTSPOT.with(|cell| {
                if let Ok(mut opt) = cell.try_borrow_mut()
                    && let Some(ref w) = *opt
                {
                    if let Some(curr) = win_weak.upgrade() {
                        if w == &curr {
                            *opt = None;
                        }
                    } else {
                        *opt = None;
                    }
                }
            });
        });

        ACTIVE_HOTSPOT.with(|cell| {
            *cell.borrow_mut() = Some(window.clone());
        });

        window.present();
    }

    /// Closes the active edge hotspot if running.
    pub fn close() {
        let win_opt = ACTIVE_HOTSPOT.with(|cell| cell.borrow_mut().take());
        if let Some(win) = win_opt {
            win.close();
        }
    }

    /// Toggles the edge drop target hotspot on or off. Returns whether it is now active.
    pub fn toggle(app: &gtk4::Application) -> bool {
        let existing = ACTIVE_HOTSPOT.with(|cell| cell.borrow_mut().take());
        if let Some(win) = existing {
            win.close();
            false
        } else {
            Self::spawn(app);
            true
        }
    }

    /// Checks if the edge drop target hotspot is currently active.
    pub fn is_active() -> bool {
        ACTIVE_HOTSPOT.with(|cell| cell.borrow().is_some())
    }

}

