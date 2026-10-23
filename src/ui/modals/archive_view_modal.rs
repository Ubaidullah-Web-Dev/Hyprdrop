use std::path::Path;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use crate::config::ConfigManager;
use crate::engine::archive_ops::{extract_archive, list_archive_entries, test_archive_integrity};
use crate::engine::format_file_size;
use crate::ui::drop_shelf::DropShelf;
use crate::ui::overlay_window::OverlayWindow;

pub fn open_archive_view_modal(
    app: Option<&gtk4::Application>,
    file_path: &Path,
) {
    let entries = match list_archive_entries(file_path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error reading archive: {:?}", e);
            let _ = std::process::Command::new("notify-send")
                .args(["-a", "Hyprdrop", "-i", "dialog-error-symbolic", "Archive Error", &format!("Cannot read archive: {}", e)])
                .spawn();
            return;
        }
    };

    let total_uncompressed: u64 = entries.iter().map(|e| e.uncompressed_size).sum();
    let total_count = entries.len();
    let archive_path = file_path.to_path_buf();

    let window = gtk4::Window::builder()
        .title("Archive Viewer — Hyprdrop")
        .default_width(680)
        .default_height(540)
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

    let title_icon = gtk4::Image::from_icon_name("package-x-generic-symbolic");
    title_icon.set_pixel_size(20);
    title_icon.add_css_class("hyprdrop-brand-icon");
    header_box.append(&title_icon);

    let title_lbl = gtk4::Label::new(Some("Archive Viewer"));
    title_lbl.add_css_class("hyprdrop-title");
    title_lbl.set_hexpand(true);
    title_lbl.set_halign(gtk4::Align::Start);
    header_box.append(&title_lbl);

    let count_badge = gtk4::Label::new(Some(&format!("{} files", total_count)));
    count_badge.add_css_class("badge-warning");
    header_box.append(&count_badge);

    let size_badge = gtk4::Label::new(Some(&format!("Total: {}", format_file_size(total_uncompressed))));
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
        file_path.file_name().and_then(|n| n.to_str()).unwrap_or("Archive"),
    ));
    file_lbl.add_css_class("hyprdrop-subtitle");
    file_lbl.set_halign(gtk4::Align::Start);
    root_box.append(&file_lbl);

    // Search filter entry
    let search_entry = gtk4::SearchEntry::new();
    search_entry.set_placeholder_text(Some("Filter archive files..."));
    search_entry.add_css_class("hyprdrop-search");
    root_box.append(&search_entry);

    // Scrolled list of entries
    let scrolled = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .build();

    let list_box = gtk4::ListBox::new();
    list_box.set_selection_mode(gtk4::SelectionMode::None);
    list_box.add_css_class("hyprdrop-listbox");

    // Populate list items
    for item in &entries {
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
        row.set_margin_start(12);
        row.set_margin_end(12);
        row.set_margin_top(6);
        row.set_margin_bottom(6);

        let icon_name = if item.is_dir {
            "folder-symbolic"
        } else {
            "text-x-generic-symbolic"
        };
        let icon = gtk4::Image::from_icon_name(icon_name);
        icon.set_pixel_size(16);
        icon.set_opacity(0.8);
        row.append(&icon);

        let path_lbl = gtk4::Label::new(Some(&item.path));
        path_lbl.set_hexpand(true);
        path_lbl.set_halign(gtk4::Align::Start);
        path_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
        row.append(&path_lbl);

        if !item.is_dir && item.uncompressed_size > 0 {
            let sz_lbl = gtk4::Label::new(Some(&format_file_size(item.uncompressed_size)));
            sz_lbl.add_css_class("dim-label");
            sz_lbl.set_halign(gtk4::Align::End);
            row.append(&sz_lbl);
        }

        // Store path in widget name for easy search filtering
        row.set_widget_name(&item.path.to_lowercase());
        list_box.append(&row);
    }

    // Connect search filter
    let list_box_ref = list_box.clone();
    search_entry.connect_search_changed(move |entry| {
        let query = entry.text().to_lowercase();
        let mut child = list_box_ref.first_child();
        while let Some(widget) = child {
            if query.is_empty() {
                widget.set_visible(true);
            } else {
                let name = widget.widget_name().to_string();
                widget.set_visible(name.contains(&query));
            }
            child = widget.next_sibling();
        }
    });

    scrolled.set_child(Some(&list_box));
    root_box.append(&scrolled);

    // Bottom Action Bar
    let bottom_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    bottom_box.set_margin_top(8);

    // Test Integrity Button
    let test_btn = gtk4::Button::with_label("Test Integrity");
    test_btn.add_css_class("hyprdrop-secondary-btn");
    let test_path = archive_path.clone();
    test_btn.connect_clicked(move |_| {
        let p = test_path.clone();
        std::thread::spawn(move || {
            let res = test_archive_integrity(&p).map_err(|e| e.to_string());
            glib::idle_add_once(move || {
                match res {
                    Ok((count, bytes)) => {
                        let _ = std::process::Command::new("notify-send")
                            .args([
                                "-a", "Hyprdrop",
                                "-i", "emblem-ok-symbolic",
                                "Archive Integrity Verified",
                                &format!("Successfully verified {} entries ({}) without errors.", count, format_file_size(bytes)),
                            ])
                            .spawn();
                    }
                    Err(e) => {
                        let _ = std::process::Command::new("notify-send")
                            .args([
                                "-a", "Hyprdrop",
                                "-i", "dialog-error-symbolic",
                                "Archive Test Failed",
                                &format!("Integrity error: {}", e),
                            ])
                            .spawn();
                    }
                }
            });
        });
    });
    bottom_box.append(&test_btn);

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    bottom_box.append(&spacer);

    // Extract To... Button
    let extract_to_btn = gtk4::Button::with_label("Extract To...");
    extract_to_btn.add_css_class("hyprdrop-secondary-btn");
    let win_extract_to = window.clone();
    let src_extract_to = archive_path.clone();

    extract_to_btn.connect_clicked(move |_| {
        let dialog = gtk4::FileDialog::builder()
            .title("Select Destination Directory")
            .modal(true)
            .build();

        let parent_for_dialog = win_extract_to.clone();
        let parent_for_close = win_extract_to.clone();
        let src = src_extract_to.clone();

        dialog.select_folder(Some(&parent_for_dialog), gio::Cancellable::NONE, move |res| {
            if let Ok(folder) = res {
                if let Some(dest_dir) = folder.path() {
                    parent_for_close.close();
                    let src_bg = src.clone();
                    std::thread::spawn(move || {
                        if let Ok(extracted) = extract_archive(&src_bg, &dest_dir) {
                            let _ = ConfigManager::append_history("EXTRACT ARCHIVE", &src_bg, &extracted, None);
                            let _ = std::process::Command::new("notify-send")
                                .args([
                                    "-a", "Hyprdrop",
                                    "-i", "package-x-generic-symbolic",
                                    "Archive Extracted",
                                    &format!("Files extracted to {}", extracted.display()),
                                ])
                                .spawn();
                            glib::idle_add_once(move || {
                                DropShelf::spawn(extracted, Some("Extracted".to_string()), move |chained_file| {
                                    if let Some(active_app) = gio::Application::default()
                                        .and_then(|a| a.downcast::<gtk4::Application>().ok()) {
                                        OverlayWindow::spawn(&active_app, Some(chained_file));
                                    }
                                });
                            });
                        }
                    });
                }
            }
        });
    });
    bottom_box.append(&extract_to_btn);

    // Extract Here Button (Primary Action)
    let extract_here_btn = gtk4::Button::with_label("Extract Here");
    extract_here_btn.add_css_class("hyprdrop-primary-btn");
    let win_extract_here = window.clone();
    let src_extract_here = archive_path.clone();

    extract_here_btn.connect_clicked(move |_| {
        win_extract_here.close();
        let src = src_extract_here.clone();
        std::thread::spawn(move || {
            let parent = src.parent().unwrap_or_else(|| Path::new("."));
            let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("extracted");
            let dest = parent.join(stem);
            let _ = std::fs::create_dir_all(&dest);
            if let Ok(extracted) = extract_archive(&src, &dest) {
                let _ = ConfigManager::append_history("EXTRACT ARCHIVE", &src, &extracted, None);
                let _ = std::process::Command::new("notify-send")
                    .args([
                        "-a", "Hyprdrop",
                        "-i", "package-x-generic-symbolic",
                        "Archive Extracted",
                        &format!("Extracted to {}", extracted.display()),
                    ])
                    .spawn();
                glib::idle_add_once(move || {
                    DropShelf::spawn(extracted, Some("Extracted".to_string()), move |chained_file| {
                        if let Some(active_app) = gio::Application::default()
                            .and_then(|a| a.downcast::<gtk4::Application>().ok()) {
                            OverlayWindow::spawn(&active_app, Some(chained_file));
                        }
                    });
                });
            }
        });
    });
    bottom_box.append(&extract_here_btn);

    root_box.append(&bottom_box);
    window.set_child(Some(&root_box));
    window.present();
}
