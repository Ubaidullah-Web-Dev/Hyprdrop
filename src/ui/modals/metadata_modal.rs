use std::path::{Path, PathBuf};
use std::rc::Rc;
use gtk4::prelude::*;
use crate::config::ConfigManager;
use crate::engine::format_file_size;
use crate::engine::metadata::{inspect_file_metadata, strip_file_metadata};

pub fn open_metadata_modal(
    app: Option<&gtk4::Application>,
    file_path: &Path,
    on_success: impl Fn(PathBuf) + 'static,
) {
    let meta_info = match inspect_file_metadata(file_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error inspecting metadata: {:?}", e);
            return;
        }
    };

    let window = gtk4::Window::builder()
        .title("Metadata Inspector — Hyprdrop")
        .default_width(620)
        .default_height(500)
        .build();

    if let Some(a) = app {
        a.add_window(&window);
    }

    window.add_css_class("hyprdrop-surface");

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    root_box.set_margin_start(18);
    root_box.set_margin_end(18);
    root_box.set_margin_top(14);
    root_box.set_margin_bottom(16);
    root_box.add_css_class("hyprdrop-surface");

    // Header
    let header_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);

    let title_icon = gtk4::Image::from_icon_name("dialog-information-symbolic");
    title_icon.set_pixel_size(18);
    title_icon.add_css_class("hyprdrop-brand-icon");
    header_box.append(&title_icon);

    let title_lbl = gtk4::Label::new(Some("Metadata Inspector"));
    title_lbl.add_css_class("hyprdrop-title");
    title_lbl.set_hexpand(true);
    title_lbl.set_halign(gtk4::Align::Start);
    header_box.append(&title_lbl);

    let size_badge = gtk4::Label::new(Some(&format_file_size(meta_info.file_size_bytes)));
    size_badge.add_css_class("badge-warning");
    header_box.append(&size_badge);

    let close_btn = gtk4::Button::from_icon_name("window-close-symbolic");
    close_btn.add_css_class("hyprdrop-ghost");
    close_btn.add_css_class("hyprdrop-ghost-danger");
    close_btn.set_tooltip_text(Some("Close"));
    let win_close = window.clone();
    close_btn.connect_clicked(move |_| win_close.close());
    header_box.append(&close_btn);

    root_box.append(&header_box);

    let file_lbl = gtk4::Label::new(Some(
        file_path.file_name().and_then(|n| n.to_str()).unwrap_or("File"),
    ));
    file_lbl.add_css_class("hyprdrop-subtitle");
    file_lbl.set_halign(gtk4::Align::Start);
    root_box.append(&file_lbl);

    // Scrollable List of Metadata Keys and Values
    let scrolled = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .build();

    let list_box = gtk4::ListBox::new();
    list_box.add_css_class("hyprdrop-card");
    list_box.set_selection_mode(gtk4::SelectionMode::None);

    if meta_info.fields.is_empty() {
        let empty_row = gtk4::ListBoxRow::new();
        let empty_lbl = gtk4::Label::new(Some("No EXIF or extended metadata found in file."));
        empty_lbl.add_css_class("hyprdrop-subtitle");
        empty_lbl.set_margin_top(16);
        empty_lbl.set_margin_bottom(16);
        empty_row.set_child(Some(&empty_lbl));
        list_box.append(&empty_row);
    } else {
        for (key, val) in &meta_info.fields {
            let row = gtk4::ListBoxRow::new();
            let row_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
            row_box.set_margin_start(10);
            row_box.set_margin_end(10);
            row_box.set_margin_top(8);
            row_box.set_margin_bottom(8);

            let k_lbl = gtk4::Label::new(Some(key));
            k_lbl.add_css_class("hyprdrop-subtitle");
            k_lbl.set_width_chars(20);
            k_lbl.set_halign(gtk4::Align::Start);

            let v_lbl = gtk4::Label::new(Some(val));
            v_lbl.add_css_class("hyprdrop-mono");
            v_lbl.set_hexpand(true);
            v_lbl.set_halign(gtk4::Align::Start);
            v_lbl.set_wrap(true);

            row_box.append(&k_lbl);
            row_box.append(&v_lbl);
            row.set_child(Some(&row_box));
            list_box.append(&row);
        }
    }

    scrolled.set_child(Some(&list_box));
    root_box.append(&scrolled);

    // Bottom Action Bar
    let bottom_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    let close_btn = gtk4::Button::with_label("Close");
    close_btn.add_css_class("hyprdrop-secondary");
    let win_close = window.clone();
    close_btn.connect_clicked(move |_| {
        win_close.close();
    });
    bottom_box.append(&close_btn);

    let b_spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    b_spacer.set_hexpand(true);
    bottom_box.append(&b_spacer);

    let strip_btn = gtk4::Button::with_label("Strip All Metadata");
    strip_btn.add_css_class("hyprdrop-primary");
    let win_strip = window.clone();
    let src_path = file_path.to_path_buf();
    let on_success_rc = Rc::new(on_success);
    strip_btn.connect_clicked(move |_| {
        if let Ok(out_path) = strip_file_metadata(&src_path) {
            let _ = ConfigManager::append_history("METADATA STRIP", &src_path, &out_path, None);
            on_success_rc(out_path);
        }
        win_strip.close();
    });
    bottom_box.append(&strip_btn);

    root_box.append(&bottom_box);

    window.connect_destroy(move |_| {
        crate::engine::trim_memory();
    });

    window.set_child(Some(&root_box));
    crate::hyprland::ensure_hyprland_floating_rules();
    window.present();
}
