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
        let health_scrolled = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .child(&health_tab)
            .build();
        notebook.append_page(&health_scrolled, Some(&gtk4::Label::new(Some("System & Health"))));

        // Tab 3: Hyprland & Preferences & Autostart
        let prefs_tab = build_preferences_tab();
        let prefs_scrolled = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .child(&prefs_tab)
            .build();
        notebook.append_page(&prefs_scrolled, Some(&gtk4::Label::new(Some("Preferences & Autostart"))));

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
    box_view.set_margin_bottom(12);
    box_view.set_margin_start(4);
    box_view.set_margin_end(4);

    // 1. Autostart on Boot & Background Daemon Card
    let autostart_section = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    autostart_section.add_css_class("hyprdrop-card");

    let auto_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    let auto_title_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    auto_title_box.set_hexpand(true);

    let auto_title = gtk4::Label::new(Some("Autostart on Boot (Daemon & App)"));
    auto_title.add_css_class("hyprdrop-title");
    auto_title.set_halign(gtk4::Align::Start);
    auto_title_box.append(&auto_title);

    let auto_desc = gtk4::Label::new(Some(
        "Automatically start the Hyprdrop daemon and background services on system boot / login",
    ));
    auto_desc.add_css_class("hyprdrop-subtitle");
    auto_desc.set_halign(gtk4::Align::Start);
    auto_title_box.append(&auto_desc);
    auto_header.append(&auto_title_box);

    let autostart_switch = gtk4::Switch::new();
    let current_autostart = crate::hyprland::is_autostart_enabled();
    autostart_switch.set_active(current_autostart);
    autostart_switch.set_valign(gtk4::Align::Center);
    auto_header.append(&autostart_switch);
    autostart_section.append(&auto_header);

    // Live status label
    let autostart_status = gtk4::Label::new(None);
    autostart_status.set_halign(gtk4::Align::Start);
    if current_autostart {
        autostart_status.set_markup(
            "<span foreground='#30D158' weight='bold'>● Autostart is ENABLED</span> <span foreground='#8E8E96'>(systemd user service &amp; XDG autostart active)</span>",
        );
    } else {
        autostart_status.set_markup(
            "<span foreground='#FF453A' weight='bold'>○ Autostart is DISABLED</span> <span foreground='#8E8E96'>(daemon and app will NOT start on boot)</span>",
        );
    }
    autostart_section.append(&autostart_status);

    // Daemon Runtime Status row
    let daemon_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    daemon_row.set_margin_top(4);

    let daemon_lbl = gtk4::Label::new(Some("Background Daemon Runtime:"));
    daemon_lbl.add_css_class("hyprdrop-subtitle");
    daemon_lbl.set_halign(gtk4::Align::Start);
    daemon_row.append(&daemon_lbl);

    let is_running = crate::hyprland::is_daemon_running();
    let daemon_badge = gtk4::Label::new(Some(if is_running { "ACTIVE / RUNNING" } else { "INACTIVE / STOPPED" }));
    daemon_badge.add_css_class(if is_running { "badge-success" } else { "badge-warning" });
    daemon_row.append(&daemon_badge);

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    daemon_row.append(&spacer);

    let daemon_btn = gtk4::Button::with_label(if is_running { "Restart Daemon" } else { "Start Daemon" });
    daemon_btn.add_css_class("hyprdrop-secondary");
    daemon_btn.set_valign(gtk4::Align::Center);

    let daemon_badge_clone = daemon_badge.clone();
    let daemon_btn_clone = daemon_btn.clone();
    daemon_btn.connect_clicked(move |_| {
        let _ = crate::hyprland::ensure_daemon_running();
        let running_now = crate::hyprland::is_daemon_running();
        if running_now {
            daemon_badge_clone.set_text("ACTIVE / RUNNING");
            daemon_badge_clone.remove_css_class("badge-warning");
            daemon_badge_clone.remove_css_class("badge-danger");
            daemon_badge_clone.add_css_class("badge-success");
            daemon_btn_clone.set_label("Restart Daemon");
        } else {
            daemon_badge_clone.set_text("INACTIVE / STOPPED");
            daemon_badge_clone.remove_css_class("badge-success");
            daemon_badge_clone.add_css_class("badge-warning");
            daemon_btn_clone.set_label("Start Daemon");
        }
    });
    daemon_row.append(&daemon_btn);
    autostart_section.append(&daemon_row);

    // Switch handler
    let status_lbl_clone = autostart_status.clone();
    let daemon_badge_clone_sw = daemon_badge.clone();
    let daemon_btn_clone_sw = daemon_btn.clone();
    autostart_switch.connect_state_set(move |_, state| {
        match crate::hyprland::set_autostart(state) {
            Ok(_) => {
                let mut cfg = ConfigManager::load_config();
                cfg.autostart = state;
                let _ = ConfigManager::save_config(&cfg);
                if state {
                    status_lbl_clone.set_markup(
                        "<span foreground='#30D158' weight='bold'>● Autostart is ENABLED</span> <span foreground='#8E8E96'>(systemd user service &amp; XDG autostart active)</span>",
                    );
                } else {
                    status_lbl_clone.set_markup(
                        "<span foreground='#FF453A' weight='bold'>○ Autostart is DISABLED</span> <span foreground='#8E8E96'>(daemon and app will NOT start on boot)</span>",
                    );
                }
            }
            Err(e) => {
                status_lbl_clone.set_markup(&format!(
                    "<span foreground='#FF453A'>Failed to update autostart: {:?}</span>",
                    e
                ));
            }
        }
        let running_now = crate::hyprland::is_daemon_running();
        if running_now {
            daemon_badge_clone_sw.set_text("ACTIVE / RUNNING");
            daemon_badge_clone_sw.remove_css_class("badge-warning");
            daemon_badge_clone_sw.remove_css_class("badge-danger");
            daemon_badge_clone_sw.add_css_class("badge-success");
            daemon_btn_clone_sw.set_label("Restart Daemon");
        } else {
            daemon_badge_clone_sw.set_text("INACTIVE / STOPPED");
            daemon_badge_clone_sw.remove_css_class("badge-success");
            daemon_badge_clone_sw.add_css_class("badge-warning");
            daemon_btn_clone_sw.set_label("Start Daemon");
        }
        gtk4::glib::Propagation::Proceed
    });

    box_view.append(&autostart_section);

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
