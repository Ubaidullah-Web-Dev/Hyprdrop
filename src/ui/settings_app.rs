use std::process::Command;
use gtk4::prelude::*;
use crate::config::ConfigManager;
use crate::engine::format_file_size;
use crate::hyprland::installer::{configure_hyprland, get_hyprland_config_info};


pub struct SettingsApp;

impl SettingsApp {
    pub fn spawn(app: &gtk4::Application) {
        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("Hyprdrop Settings & Activity")
            .default_width(740)
            .default_height(580)
            .build();

        window.add_css_class("mac-window");

        let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        root_box.set_margin_start(24);
        root_box.set_margin_end(24);
        root_box.set_margin_top(16);
        root_box.set_margin_bottom(20);

        // macOS Header with traffic lights
        let header_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);

        let traffic_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        traffic_box.set_valign(gtk4::Align::Center);

        let dot_close = gtk4::Button::new();
        dot_close.add_css_class("mac-traffic-dot");
        dot_close.add_css_class("mac-dot-close");
        let win_close = window.clone();
        dot_close.connect_clicked(move |_| win_close.close());

        let dot_min = gtk4::Button::new();
        dot_min.add_css_class("mac-traffic-dot");
        dot_min.add_css_class("mac-dot-min");

        let dot_max = gtk4::Button::new();
        dot_max.add_css_class("mac-traffic-dot");
        dot_max.add_css_class("mac-dot-max");

        traffic_box.append(&dot_close);
        traffic_box.append(&dot_min);
        traffic_box.append(&dot_max);
        header_box.append(&traffic_box);

        let title_lbl = gtk4::Label::new(Some("Hyprdrop"));
        title_lbl.add_css_class("mac-title");
        title_lbl.set_markup("<span size='large' weight='bold'>Hyprdrop</span>");
        header_box.append(&title_lbl);

        let ver_lbl = gtk4::Label::new(Some("v0.1.0 • macOS Studio Edition"));
        ver_lbl.add_css_class("mac-subtitle");
        header_box.append(&ver_lbl);

        let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header_box.append(&spacer);

        root_box.append(&header_box);

        // Notebook / Tabs
        let notebook = gtk4::Notebook::new();
        notebook.set_vexpand(true);

        // Tab 1: Conversion History
        let history_tab = build_history_tab();
        notebook.append_page(&history_tab, Some(&gtk4::Label::new(Some("Activity History"))));

        // Tab 2: System & Dependency Health
        let health_tab = build_health_tab();
        notebook.append_page(&health_tab, Some(&gtk4::Label::new(Some("System & Health"))));

        // Tab 3: Hyprland & Preferences
        let prefs_tab = build_preferences_tab();
        notebook.append_page(&prefs_tab, Some(&gtk4::Label::new(Some("Hyprland & Config"))));

        root_box.append(&notebook);

        window.set_child(Some(&root_box));
        window.present();
    }
}

fn build_history_tab() -> gtk4::Box {
    let box_view = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    box_view.set_margin_top(12);

    let scrolled = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .build();

    let list_box = gtk4::ListBox::new();
    list_box.add_css_class("hyprdrop-card");
    list_box.set_selection_mode(gtk4::SelectionMode::None);

    let records = ConfigManager::load_history();
    if records.is_empty() {
        let empty_row = gtk4::ListBoxRow::new();
        let empty_lbl = gtk4::Label::new(Some("No recent conversion or edit activity yet."));
        empty_lbl.add_css_class("hyprdrop-subtitle");
        empty_lbl.set_margin_top(24);
        empty_lbl.set_margin_bottom(24);
        empty_row.set_child(Some(&empty_lbl));
        list_box.append(&empty_row);
    } else {
        for r in records {
            let row = gtk4::ListBoxRow::new();
            let r_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
            r_box.set_margin_start(10);
            r_box.set_margin_end(10);
            r_box.set_margin_top(8);
            r_box.set_margin_bottom(8);

            let act_badge = gtk4::Label::new(Some(&r.action));
            act_badge.add_css_class("hyprdrop-pill");
            r_box.append(&act_badge);

            let file_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
            file_box.set_hexpand(true);

            let fname = r.output_path.file_name().and_then(|n| n.to_str()).unwrap_or("output");
            let name_lbl = gtk4::Label::new(Some(fname));
            name_lbl.add_css_class("hyprdrop-title");
            name_lbl.set_halign(gtk4::Align::Start);
            name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
            file_box.append(&name_lbl);

            let size_text = format!(
                "{} -> {}",
                format_file_size(r.original_size_bytes),
                format_file_size(r.output_size_bytes)
            );
            let size_lbl = gtk4::Label::new(Some(&size_text));
            size_lbl.add_css_class("hyprdrop-mono");
            size_lbl.set_halign(gtk4::Align::Start);
            file_box.append(&size_lbl);

            r_box.append(&file_box);

            if let Some(pct) = r.savings_pct {
                let badge = gtk4::Label::new(Some(&format!("Saved {:.0}%", pct)));
                badge.add_css_class("badge-success");
                r_box.append(&badge);
            }

            let open_btn = gtk4::Button::from_icon_name("document-open-symbolic");
            open_btn.add_css_class("hyprdrop-secondary");
            let out_p = r.output_path.clone();
            open_btn.connect_clicked(move |_| {
                let _ = open::that(&out_p);
            });
            r_box.append(&open_btn);

            row.set_child(Some(&r_box));
            list_box.append(&row);
        }
    }

    scrolled.set_child(Some(&list_box));
    box_view.append(&scrolled);
    box_view
}

fn build_health_tab() -> gtk4::Box {
    let box_view = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    box_view.set_margin_top(12);

    let info_lbl = gtk4::Label::new(Some("System Tool & Dependency Health Checker"));
    info_lbl.add_css_class("hyprdrop-title");
    info_lbl.set_halign(gtk4::Align::Start);
    box_view.append(&info_lbl);

    let list_box = gtk4::ListBox::new();
    list_box.add_css_class("hyprdrop-card");
    list_box.set_selection_mode(gtk4::SelectionMode::None);

    let tools = [
        ("ffmpeg", "Video/audio conversions and palettegen GIF engine"),
        ("pdftoppm", "High-DPI PDF page rasterization"),
        ("libreoffice", "Document format conversion (DOCX/ODT/PDF)"),
        ("pandoc", "Markup & document text compilation"),
        ("wl-clipboard", "Wayland clipboard integration (wl-copy)"),
        ("tesseract", "OCR text extraction from images"),
        ("hyprctl", "Hyprland IPC & cursor position query"),
    ];

    for (cmd, desc) in tools {
        let is_installed = Command::new("which").arg(cmd).output().map(|o| o.status.success()).unwrap_or(false);

        let row = gtk4::ListBoxRow::new();
        let r_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        r_box.set_margin_start(12);
        r_box.set_margin_end(12);
        r_box.set_margin_top(10);
        r_box.set_margin_bottom(10);

        let text_col = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        text_col.set_hexpand(true);

        let name_lbl = gtk4::Label::new(Some(cmd));
        name_lbl.add_css_class("hyprdrop-title");
        name_lbl.set_halign(gtk4::Align::Start);
        text_col.append(&name_lbl);

        let desc_lbl = gtk4::Label::new(Some(desc));
        desc_lbl.add_css_class("hyprdrop-subtitle");
        desc_lbl.set_halign(gtk4::Align::Start);
        text_col.append(&desc_lbl);

        r_box.append(&text_col);

        let badge = if is_installed {
            let b = gtk4::Label::new(Some("INSTALLED"));
            b.add_css_class("badge-success");
            b
        } else {
            let b = gtk4::Label::new(Some("NOT FOUND (OPTIONAL)"));
            b.add_css_class("badge-warning");
            b
        };
        r_box.append(&badge);

        row.set_child(Some(&r_box));
        list_box.append(&row);
    }

    box_view.append(&list_box);
    box_view
}

fn build_preferences_tab() -> gtk4::Box {
    let box_view = gtk4::Box::new(gtk4::Orientation::Vertical, 14);
    box_view.set_margin_top(12);

    let hypr_section = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    hypr_section.add_css_class("hyprdrop-card");

    let h_title = gtk4::Label::new(Some("Hyprland Desktop Integration"));
    h_title.add_css_class("hyprdrop-title");
    h_title.set_halign(gtk4::Align::Start);
    hypr_section.append(&h_title);

    let (is_conf, config_path, format) = get_hyprland_config_info();
    let config_file_name = config_path.file_name().and_then(|n| n.to_str()).unwrap_or("hyprland.lua");
    let status_text = if is_conf {
        format!(
            "Status: Hyprland configuration active in {} ({} syntax: blur, ignorezero, Shift+Mouse bind configured).",
            config_file_name,
            format.display_name()
        )
    } else {
        format!(
            "Status: Hyprland rules not yet detected in {}.",
            config_path.display()
        )
    };
    let h_status = gtk4::Label::new(Some(&status_text));
    h_status.add_css_class("hyprdrop-subtitle");
    h_status.set_halign(gtk4::Align::Start);
    hypr_section.append(&h_status);

    let config_btn = gtk4::Button::with_label(&format!(
        "Configure Hyprland Automatically ({}: Backup + Rules)",
        format.display_name()
    ));
    config_btn.add_css_class("hyprdrop-primary");
    config_btn.set_halign(gtk4::Align::Start);

    let status_lbl_clone = h_status.clone();
    let btn_clone = config_btn.clone();
    config_btn.connect_clicked(move |_| {
        match configure_hyprland() {
            Ok(res) => {
                if res.already_configured {
                    status_lbl_clone.set_text("Hyprland is already configured for Hyprdrop.");
                } else {
                    let backup_str = res.backup_path.map(|p| p.display().to_string()).unwrap_or_else(|| "No prior file".to_string());
                    let file_display = res.config_path.file_name().and_then(|n| n.to_str()).unwrap_or("config");
                    status_lbl_clone.set_text(&format!(
                        "Successfully configured {} ({})! Backup: {}",
                        file_display,
                        res.format.display_name(),
                        backup_str
                    ));
                    btn_clone.set_sensitive(false);
                }
            }
            Err(e) => {
                status_lbl_clone.set_text(&format!("Configuration error: {:?}", e));
            }
        }
    });
    hypr_section.append(&config_btn);
    box_view.append(&hypr_section);


    // General Preferences Card
    let pref_section = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    pref_section.add_css_class("hyprdrop-card");

    let p_title = gtk4::Label::new(Some("General Preferences"));
    p_title.add_css_class("hyprdrop-title");
    p_title.set_halign(gtk4::Align::Start);
    pref_section.append(&p_title);

    let cfg = ConfigManager::load_config();

    // Output Naming Pattern
    let name_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    let name_lbl = gtk4::Label::new(Some("Output Naming Pattern:"));
    name_lbl.add_css_class("hyprdrop-subtitle");
    name_lbl.set_halign(gtk4::Align::Start);
    name_box.append(&name_lbl);

    let name_entry = gtk4::Entry::new();
    name_entry.set_text(&cfg.naming_pattern);
    name_entry.set_placeholder_text(Some("{stem} ({counter}).{ext}"));
    name_box.append(&name_entry);

    let name_hint = gtk4::Label::new(Some("Supported variables: {stem}, {counter}, {ext}"));
    name_hint.add_css_class("hyprdrop-subtitle");
    name_hint.set_halign(gtk4::Align::Start);
    name_box.append(&name_hint);
    pref_section.append(&name_box);

    // JPEG Quality Slider
    let jq_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    let jq_lbl = gtk4::Label::new(Some("Default JPEG Quality:"));
    jq_lbl.add_css_class("hyprdrop-subtitle");
    jq_box.append(&jq_lbl);

    let jq_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 10.0, 100.0, 5.0);
    jq_scale.set_value(cfg.jpeg_quality as f64);
    jq_scale.set_hexpand(true);
    jq_box.append(&jq_scale);
    pref_section.append(&jq_box);

    // Drop Shelf Timeout
    let ds_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    let ds_lbl = gtk4::Label::new(Some("Drop Shelf Timeout (seconds):"));
    ds_lbl.add_css_class("hyprdrop-subtitle");
    ds_box.append(&ds_lbl);

    let ds_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 5.0, 120.0, 5.0);
    ds_scale.set_value(cfg.drop_shelf_timeout_secs as f64);
    ds_scale.set_hexpand(true);
    ds_box.append(&ds_scale);
    pref_section.append(&ds_box);

    // Show Drop Shelf Switch
    let sw_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    let sw_lbl = gtk4::Label::new(Some("Show Drop Shelf on Completion:"));
    sw_lbl.add_css_class("hyprdrop-subtitle");
    sw_box.append(&sw_lbl);

    let sw_widget = gtk4::Switch::new();
    sw_widget.set_active(cfg.show_drop_shelf);
    sw_widget.set_halign(gtk4::Align::End);
    sw_widget.set_hexpand(true);
    sw_box.append(&sw_widget);
    pref_section.append(&sw_box);

    // Save status feedback label
    let save_status = gtk4::Label::new(None);
    save_status.add_css_class("hyprdrop-subtitle");
    save_status.set_halign(gtk4::Align::Start);

    // Save button
    let save_pref_btn = gtk4::Button::with_label("Save Preferences");
    save_pref_btn.add_css_class("hyprdrop-secondary");
    save_pref_btn.set_halign(gtk4::Align::Start);

    let jq_scale_clone = jq_scale.clone();
    let ds_scale_clone = ds_scale.clone();
    let sw_clone = sw_widget.clone();
    let name_entry_clone = name_entry.clone();
    let save_status_clone = save_status.clone();

    save_pref_btn.connect_clicked(move |_| {
        let mut c = ConfigManager::load_config();
        let pattern_text = name_entry_clone.text().to_string();
        if !pattern_text.trim().is_empty() {
            c.naming_pattern = pattern_text;
        }
        c.jpeg_quality = jq_scale_clone.value().round() as u8;
        c.drop_shelf_timeout_secs = ds_scale_clone.value().round() as u32;
        c.show_drop_shelf = sw_clone.is_active();

        match ConfigManager::save_config(&c) {
            Ok(_) => save_status_clone.set_text("Preferences saved successfully!"),
            Err(e) => save_status_clone.set_text(&format!("Error saving preferences: {:?}", e)),
        }
    });
    pref_section.append(&save_pref_btn);
    pref_section.append(&save_status);

    box_view.append(&pref_section);
    box_view
}
