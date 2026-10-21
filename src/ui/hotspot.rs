use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use crate::hyprland::get_cursor_position;
use crate::ui::overlay_window::{OverlayWindow, extract_paths_from_drop_value};

#[derive(Clone)]
pub struct HotspotController {
    pub window: gtk4::ApplicationWindow,
    pub revealer: gtk4::Revealer,
    pub bar: gtk4::Box,
    pub is_fullscreen_hidden: Rc<Cell<bool>>,
}

impl HotspotController {
    pub fn set_fullscreen_hidden(&self, hidden: bool) {
        if hidden == self.is_fullscreen_hidden.get() {
            return;
        }
        self.is_fullscreen_hidden.set(hidden);

        if hidden {
            // Animate sliding out to the right + fading out
            self.bar.add_css_class("hidden-fullscreen");
            self.revealer.set_reveal_child(false);

            // Once the 260ms transition finishes, unmap window so it does not intercept pointer events
            let win_weak = self.window.downgrade();
            let is_hidden_cell = self.is_fullscreen_hidden.clone();
            glib::timeout_add_local_once(Duration::from_millis(280), move || {
                if is_hidden_cell.get() {
                    if let Some(win) = win_weak.upgrade() {
                        win.set_visible(false);
                    }
                }
            });
        } else {
            // Restore window visibility first, then animate sliding in from right + fading in
            self.window.set_visible(true);
            self.bar.remove_css_class("hidden-fullscreen");
            self.revealer.set_reveal_child(true);
        }
    }
}

thread_local! {
    static ACTIVE_HOTSPOT: RefCell<Option<HotspotController>> = const { RefCell::new(None) };
}

static LISTENER_INITIALIZED: AtomicBool = AtomicBool::new(false);

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
            .default_width(16)
            .default_height(220)
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

        let revealer = gtk4::Revealer::new();
        revealer.set_transition_type(gtk4::RevealerTransitionType::SlideLeft);
        revealer.set_transition_duration(260);
        revealer.set_halign(gtk4::Align::End);
        revealer.set_valign(gtk4::Align::Center);

        let bar = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        bar.set_size_request(6, 220);
        bar.set_halign(gtk4::Align::End);
        bar.set_valign(gtk4::Align::Center);
        bar.add_css_class("hyprdrop-hotspot-bar");
        bar.set_tooltip_text(Some("Hyprdrop — Drag files here to convert or edit"));

        let indicator = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        indicator.set_size_request(2, -1);
        indicator.set_vexpand(true);
        indicator.set_margin_top(8);
        indicator.set_margin_bottom(8);
        indicator.set_halign(gtk4::Align::Center);
        indicator.add_css_class("hyprdrop-hotspot-indicator");
        bar.append(&indicator);

        // Drop target on hotspot bar
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
            bar_leave.remove_css_class("hovered");
        });

        let bar_drop_target = bar.clone();
        drop_target.connect_drop(move |_, val, _, _| {
            bar_drop_target.remove_css_class("active");
            bar_drop_target.remove_css_class("hovered");
            let paths = extract_paths_from_drop_value(val);
            if paths.is_empty() {
                return false;
            }

            // Spawn radial overlay at current cursor position with dropped files
            let _cursor = get_cursor_position();
            OverlayWindow::spawn_with_files(&app_drop, paths);
            true
        });

        bar.add_controller(drop_target);

        // Click on hotspot opens radial overlay at cursor immediately (zero delay)
        let click = gtk4::GestureClick::new();
        let app_click = app.clone();
        let bar_click = bar.clone();
        click.connect_pressed(move |_, _, _, _| {
            bar_click.remove_css_class("hovered");
            bar_click.remove_css_class("active");
            let files = crate::ui::overlay_window::detect_selected_files();
            OverlayWindow::spawn_with_files(&app_click, files);
        });
        bar.add_controller(click);

        // Motion controller to guarantee visual hover feedback with automatic cleanup
        let motion = gtk4::EventControllerMotion::new();
        let bar_motion = bar.clone();
        motion.connect_enter(move |_, _, _| {
            bar_motion.add_css_class("hovered");
        });
        let bar_motion_leave = bar.clone();
        motion.connect_leave(move |_| {
            bar_motion_leave.remove_css_class("hovered");
            bar_motion_leave.remove_css_class("active");
        });
        bar.add_controller(motion);

        revealer.set_child(Some(&bar));

        let is_fs_initial = crate::hyprland::check_is_fullscreen();
        if is_fs_initial {
            revealer.set_reveal_child(false);
            bar.add_css_class("hidden-fullscreen");
            window.set_visible(false);
        } else {
            revealer.set_reveal_child(true);
        }

        window.set_child(Some(&revealer));

        let ctrl = HotspotController {
            window: window.clone(),
            revealer,
            bar,
            is_fullscreen_hidden: Rc::new(Cell::new(is_fs_initial)),
        };

        let win_weak = window.downgrade();
        window.connect_destroy(move |_| {
            ACTIVE_HOTSPOT.with(|cell| {
                if let Ok(mut opt) = cell.try_borrow_mut()
                    && let Some(ref c) = *opt
                {
                    if let Some(curr) = win_weak.upgrade() {
                        if c.window == curr {
                            *opt = None;
                        }
                    } else {
                        *opt = None;
                    }
                }
            });
        });

        ACTIVE_HOTSPOT.with(|cell| {
            *cell.borrow_mut() = Some(ctrl);
        });

        if !is_fs_initial {
            window.present();
        }

        // Initialize background Hyprland socket2 fullscreen event listener
        if !LISTENER_INITIALIZED.swap(true, Ordering::SeqCst) {
            crate::hyprland::listen_hyprland_fullscreen_events(|is_fs| {
                glib::idle_add_once(move || {
                    Self::set_fullscreen_hidden(is_fs);
                });
            });
        }
    }

    /// Sets whether the hotspot should be hidden because a fullscreen window is active.
    pub fn set_fullscreen_hidden(hidden: bool) {
        ACTIVE_HOTSPOT.with(|cell| {
            if let Some(ctrl) = cell.borrow().as_ref() {
                ctrl.set_fullscreen_hidden(hidden);
            }
        });
    }

    /// Closes the active edge hotspot if running.
    pub fn close() {
        let ctrl_opt = ACTIVE_HOTSPOT.with(|cell| cell.borrow_mut().take());
        if let Some(ctrl) = ctrl_opt {
            ctrl.window.close();
        }
    }

    /// Toggles the edge drop target hotspot on or off. Returns whether it is now active.
    pub fn toggle(app: &gtk4::Application) -> bool {
        let existing = ACTIVE_HOTSPOT.with(|cell| cell.borrow_mut().take());
        if let Some(ctrl) = existing {
            ctrl.window.close();
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
