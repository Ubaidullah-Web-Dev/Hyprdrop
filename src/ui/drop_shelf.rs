use std::path::PathBuf;
use std::rc::Rc;
use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use crate::engine::format_file_size;
use crate::engine::ocr_clipboard::copy_image_to_clipboard;

pub struct DropShelf;

impl DropShelf {
    pub fn spawn(
        file_path: PathBuf,
        savings_badge: Option<String>,
        on_chain: impl Fn(PathBuf) + 'static,
    ) {
        let active_app = gio::Application::default().and_then(|a| a.downcast::<gtk4::Application>().ok());
        let mut builder = gtk4::ApplicationWindow::builder()
            .title("Hyprdrop — Drop Shelf")
            .default_width(320)
            .default_height(96)
            .resizable(false)
            .decorated(false);

        if let Some(ref app) = active_app {
            builder = builder.application(app);
        }
        let window = builder.build();

        window.add_css_class("hyprdrop-shelf");

        // If layer shell supported, anchor to bottom right
        if gtk4_layer_shell::is_supported() {
            window.init_layer_shell();
            window.set_layer(Layer::Overlay);
            window.set_namespace(Some("hyprdrop-shelf"));
            window.set_anchor(Edge::Right, true);
            window.set_anchor(Edge::Bottom, true);
            window.set_margin(Edge::Right, 24);
            window.set_margin(Edge::Bottom, 24);
        }

        let main_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
        main_box.set_margin_start(10);
        main_box.set_margin_end(10);
        main_box.set_margin_top(10);
        main_box.set_margin_bottom(10);

        // Draggable Thumbnail Card
        let thumb_card = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        thumb_card.set_size_request(64, 64);
        thumb_card.add_css_class("hyprdrop-card");
        thumb_card.set_tooltip_text(Some("Drag me to another app or into radial wheel to chain!"));

        let icon = gtk4::Image::from_icon_name("document-send-symbolic");
        icon.set_pixel_size(32);
        icon.set_valign(gtk4::Align::Center);
        icon.set_halign(gtk4::Align::Center);
        thumb_card.append(&icon);

        // Attach GtkDragSource to thumb_card
        let drag_source = gtk4::DragSource::new();
        drag_source.set_actions(gdk::DragAction::COPY);

        let drag_path = file_path.clone();
        drag_source.connect_prepare(move |_, _, _| {
            let gio_file = gio::File::for_path(&drag_path);
            let file_list = gdk::FileList::from_array(&[gio_file]);
            Some(gdk::ContentProvider::for_value(&file_list.to_value()))
        });
        thumb_card.add_controller(drag_source);

        main_box.append(&thumb_card);

        // File Details Column
        let details_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        details_box.set_hexpand(true);
        details_box.set_valign(gtk4::Align::Center);

        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Output");
        let name_lbl = gtk4::Label::new(Some(file_name));
        name_lbl.add_css_class("hyprdrop-title");
        name_lbl.set_halign(gtk4::Align::Start);
        name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
        name_lbl.set_max_width_chars(18);
        details_box.append(&name_lbl);

        let sub_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        let size_str = match std::fs::metadata(&file_path) {
            Ok(m) => format_file_size(m.len()),
            Err(_) => "0 B".to_string(),
        };
        let size_lbl = gtk4::Label::new(Some(&size_str));
        size_lbl.add_css_class("hyprdrop-mono");
        sub_box.append(&size_lbl);

        if let Some(badge) = savings_badge {
            let badge_lbl = gtk4::Label::new(Some(&badge));
            badge_lbl.add_css_class("badge-success");
            sub_box.append(&badge_lbl);
        }
        details_box.append(&sub_box);
        main_box.append(&details_box);

        // Action Buttons Column
        let act_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        act_box.set_valign(gtk4::Align::Center);

        // Chain Button (Re-opens in radial menu!)
        let chain_btn = gtk4::Button::from_icon_name("media-playlist-repeat-symbolic");
        chain_btn.add_css_class("hyprdrop-pill");
        chain_btn.set_tooltip_text(Some("Chain another action on this file"));
        let win_chain = window.clone();
        let chain_path = file_path.clone();
        let on_chain_rc = Rc::new(on_chain);
        chain_btn.connect_clicked(move |_| {
            on_chain_rc(chain_path.clone());
            win_chain.close();
        });
        act_box.append(&chain_btn);

        // Open Button
        let open_btn = gtk4::Button::from_icon_name("document-open-symbolic");
        open_btn.add_css_class("hyprdrop-pill");
        open_btn.set_tooltip_text(Some("Open file with default viewer"));
        let open_path = file_path.clone();
        open_btn.connect_clicked(move |_| {
            let _ = open::that(&open_path);
        });
        act_box.append(&open_btn);

        // Copy button
        let copy_btn = gtk4::Button::from_icon_name("edit-copy-symbolic");
        copy_btn.add_css_class("hyprdrop-pill");
        copy_btn.set_tooltip_text(Some("Copy file to clipboard"));
        let copy_path = file_path.clone();
        copy_btn.connect_clicked(move |_| {
            let _ = copy_image_to_clipboard(&copy_path);
        });
        act_box.append(&copy_btn);

        // Close button
        let close_btn = gtk4::Button::from_icon_name("window-close-symbolic");
        close_btn.add_css_class("hyprdrop-pill");
        let win_close = window.clone();
        close_btn.connect_clicked(move |_| {
            win_close.close();
        });
        act_box.append(&close_btn);

        main_box.append(&act_box);
        window.set_child(Some(&main_box));
        window.present();

        // Optional auto-dismiss after 15 seconds
        let win_timer = window.clone();
        glib::timeout_add_seconds_local(15, move || {
            win_timer.close();
            glib::ControlFlow::Break
        });
    }
}
