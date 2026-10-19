use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use crate::hyprland::get_cursor_position;
use crate::ui::overlay_window::{OverlayWindow, extract_paths_from_drop_value};

pub struct HotspotWindow;

impl HotspotWindow {
    /// Spawns a slim, persistent edge drop target hotspot anchored to the screen edge.
    /// This acts as a reliable fallback when file managers intercept keyboard grabs.
    pub fn spawn(app: &gtk4::Application) {
        if !gtk4_layer_shell::is_supported() {
            return;
        }

        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("Hyprdrop Edge Hotspot")
            .default_width(8)
            .default_height(180)
            .resizable(false)
            .decorated(false)
            .build();

        window.add_css_class("hyprdrop-hotspot");

        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_namespace(Some("hyprdrop-hotspot"));

        // Anchor to the Right edge, centered vertically
        window.set_anchor(Edge::Right, true);
        window.set_anchor(Edge::Top, false);
        window.set_anchor(Edge::Bottom, false);
        window.set_anchor(Edge::Left, false);
        window.set_margin(Edge::Right, 0);

        let bar = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        bar.set_size_request(8, 180);
        bar.add_css_class("hyprdrop-hotspot-bar");
        bar.set_tooltip_text(Some("Hyprdrop Hotspot — Drag files here to trigger radial menu"));

        // Drop target on hotspot
        let drop_target = gtk4::DropTarget::new(
            glib::types::Type::INVALID,
            gdk::DragAction::COPY | gdk::DragAction::MOVE,
        );
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

        window.set_child(Some(&bar));
        window.present();
    }
}
