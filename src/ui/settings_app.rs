use std::cell::RefCell;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;
use gtk4::prelude::*;

use crate::config::{ConfigManager, HistoryRecord};
use crate::engine::format_file_size;
use crate::hyprland::installer::{
    configure_hyprland, get_hyprland_config_info, CONF_CONFIG_SNIPPET, LUA_CONFIG_SNIPPET,
};

thread_local! {
    static ACTIVE_SETTINGS_WINDOW: RefCell<Option<gtk4::ApplicationWindow>> = const { RefCell::new(None) };
}

pub struct SettingsApp;

impl SettingsApp {
    pub fn spawn(app: &gtk4::Application) {
        // If already open, present the existing window
        let already_open = ACTIVE_SETTINGS_WINDOW.with(|cell| {
            if let Some(win) = cell.borrow().as_ref() {
                win.present();
                true
            } else {
                false
            }
        });

        if already_open {
            return;
        }

        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("Hyprdrop Hub")
            .default_width(960)
            .default_height(640)
            .build();

        window.add_css_class("hyprdrop-hub-window");
        window.add_css_class("hyprdrop-surface");

        let win_ref = window.clone();
        ACTIVE_SETTINGS_WINDOW.with(|cell| {
            *cell.borrow_mut() = Some(win_ref);
        });

        window.connect_destroy(|_| {
            ACTIVE_SETTINGS_WINDOW.with(|cell| {
                *cell.borrow_mut() = None;
            });
            crate::engine::trim_memory();
        });

        // Root horizontal container: Left Sidebar + Right Main Content
        let root_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);

        // --- LEFT SIDEBAR (240px wide) ---
        let sidebar = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        sidebar.add_css_class("hyprdrop-sidebar");
        sidebar.set_size_request(240, -1);

        // Brand Header
        let brand_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        brand_box.set_margin_bottom(20);

        let brand_icon = gtk4::Image::from_icon_name("view-grid-symbolic");
        brand_icon.set_pixel_size(24);
        brand_icon.add_css_class("hyprdrop-brand-icon");
        brand_box.append(&brand_icon);

        let brand_text_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        brand_text_box.set_hexpand(true);

        let brand_title_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        let brand_title = gtk4::Label::new(Some("HYPRDROP"));
        brand_title.add_css_class("hyprdrop-brand-title");
        brand_title.set_halign(gtk4::Align::Start);
        brand_title_row.append(&brand_title);

        let brand_badge = gtk4::Label::new(Some("v0.1.0"));
        brand_badge.add_css_class("hyprdrop-brand-badge");
        brand_title_row.append(&brand_badge);
        brand_text_box.append(&brand_title_row);

        let brand_sub = gtk4::Label::new(Some("WAYLAND WORKSPACE"));
        brand_sub.add_css_class("hyprdrop-brand-sub");
        brand_sub.set_halign(gtk4::Align::Start);
        brand_text_box.append(&brand_sub);

        brand_box.append(&brand_text_box);
        sidebar.append(&brand_box);

        // Sidebar Nav Items
        let nav_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

        let history_records = ConfigManager::load_history();
        let history_count = history_records.len();

        let nav_activity = create_nav_button(
            "document-open-recent-symbolic",
            "Activity Log",
            Some(&history_count.to_string()),
        );
        let nav_health = create_nav_button(
            "utilities-system-monitor-symbolic",
            "System & Health",
            None,
        );
        let nav_prefs = create_nav_button(
            "emblem-system-symbolic",
            "Preferences",
            None,
        );
        let nav_hyprland = create_nav_button(
            "preferences-desktop-display-symbolic",
            "Hyprland Setup",
            None,
        );

        nav_activity.add_css_class("active");

        nav_box.append(&nav_activity);
        nav_box.append(&nav_health);
        nav_box.append(&nav_prefs);
        nav_box.append(&nav_hyprland);
        sidebar.append(&nav_box);

        // Sidebar Spacer
        let side_spacer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        side_spacer.set_vexpand(true);
        sidebar.append(&side_spacer);

        // Sidebar Footer: Daemon Status & Control Card
        let daemon_card = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        daemon_card.add_css_class("hyprdrop-sidebar-footer");

        let daemon_status_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let is_running = crate::hyprland::is_daemon_running();

        let status_dot = gtk4::Label::new(Some(if is_running { "●" } else { "○" }));
        status_dot.add_css_class(if is_running { "badge-success-dot" } else { "badge-warning-dot" });
        daemon_status_row.append(&status_dot);

        let daemon_label_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        daemon_label_box.set_hexpand(true);

        let daemon_main_lbl = gtk4::Label::new(Some(if is_running { "Daemon Active" } else { "Daemon Inactive" }));
        daemon_main_lbl.add_css_class("hyprdrop-sidebar-status-title");
        daemon_main_lbl.set_halign(gtk4::Align::Start);
        daemon_label_box.append(&daemon_main_lbl);

        let daemon_sub_lbl = gtk4::Label::new(Some("systemd user service"));
        daemon_sub_lbl.add_css_class("hyprdrop-brand-sub");
        daemon_sub_lbl.set_halign(gtk4::Align::Start);
        daemon_label_box.append(&daemon_sub_lbl);

        daemon_status_row.append(&daemon_label_box);
        daemon_card.append(&daemon_status_row);

        let restart_btn = gtk4::Button::from_icon_name("view-refresh-symbolic");
        restart_btn.set_label("Restart Engine");
        restart_btn.add_css_class("hyprdrop-secondary");
        restart_btn.add_css_class("hyprdrop-btn-sm");

        let dot_clone = status_dot.clone();
        let main_lbl_clone = daemon_main_lbl.clone();
        restart_btn.connect_clicked(move |_| {
            let _ = crate::hyprland::ensure_daemon_running();
            let running = crate::hyprland::is_daemon_running();
            if running {
                dot_clone.set_text("●");
                dot_clone.remove_css_class("badge-warning-dot");
                dot_clone.add_css_class("badge-success-dot");
                main_lbl_clone.set_text("Daemon Active");
            } else {
                dot_clone.set_text("○");
                dot_clone.remove_css_class("badge-success-dot");
                dot_clone.add_css_class("badge-warning-dot");
                main_lbl_clone.set_text("Daemon Inactive");
            }
        });

        daemon_card.append(&restart_btn);
        sidebar.append(&daemon_card);
        root_box.append(&sidebar);

        // --- RIGHT MAIN CONTENT COLUMN ---
        let main_col = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        main_col.set_hexpand(true);
        main_col.set_vexpand(true);

        // Content Area Header Bar
        let header_bar = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        header_bar.add_css_class("hyprdrop-content-header");

        let title_col = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        title_col.set_hexpand(true);

        let page_title = gtk4::Label::new(Some("Activity Log"));
        page_title.add_css_class("hyprdrop-page-title");
        page_title.set_halign(gtk4::Align::Start);
        title_col.append(&page_title);

        let page_desc = gtk4::Label::new(Some("Recent file transformations, optimizations and studio exports"));
        page_desc.add_css_class("hyprdrop-page-desc");
        page_desc.set_halign(gtk4::Align::Start);
        title_col.append(&page_desc);

        header_bar.append(&title_col);

        // Window Close Button
        let close_btn = gtk4::Button::from_icon_name("window-close-symbolic");
        close_btn.add_css_class("hyprdrop-ghost");
        close_btn.add_css_class("hyprdrop-ghost-danger");
        close_btn.set_tooltip_text(Some("Close Window"));
        let win_close = window.clone();
        close_btn.connect_clicked(move |_| win_close.close());
        header_bar.append(&close_btn);

        main_col.append(&header_bar);

        // Stack Container for Pages
        let stack = gtk4::Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
        stack.set_transition_duration(180);
        stack.set_vexpand(true);
        stack.set_hexpand(true);

        // Build Pages
        let history_tab = build_history_tab();
        let health_tab = build_health_tab();
        let prefs_tab = build_preferences_tab();
        let hyprland_tab = build_hyprland_tab();

        stack.add_named(&history_tab, Some("activity"));
        stack.add_named(&health_tab, Some("health"));
        stack.add_named(&prefs_tab, Some("preferences"));
        stack.add_named(&hyprland_tab, Some("hyprland"));

        main_col.append(&stack);
        root_box.append(&main_col);

        // Connect Navigation Buttons
        let nav_buttons = [
            (nav_activity.clone(), "activity", "Activity Log", "Recent file transformations, optimizations and studio exports"),
            (nav_health.clone(), "health", "System & Health", "Diagnostic health checker for system dependencies and multimedia engines"),
            (nav_prefs.clone(), "preferences", "Preferences", "Global naming rules, compression thresholds and workflow options"),
            (nav_hyprland.clone(), "hyprland", "Hyprland Setup", "Window rules, blur layers and desktop hotkey integration"),
        ];

        let all_btns: Vec<gtk4::Button> = nav_buttons.iter().map(|(b, _, _, _)| b.clone()).collect();

        for (btn, page_name, p_title, p_desc) in nav_buttons {
            let stack_clone = stack.clone();
            let all_b_clone = all_btns.clone();
            let b_clone = btn.clone();
            let p_title_lbl = page_title.clone();
            let p_desc_lbl = page_desc.clone();
            let p_name = page_name.to_string();
            let p_title_str = p_title.to_string();
            let p_desc_str = p_desc.to_string();

            btn.connect_clicked(move |_| {
                for b in &all_b_clone {
                    b.remove_css_class("active");
                }
                b_clone.add_css_class("active");
                stack_clone.set_visible_child_name(&p_name);
                p_title_lbl.set_text(&p_title_str);
                p_desc_lbl.set_text(&p_desc_str);
            });
        }

        window.set_child(Some(&root_box));
        crate::hyprland::ensure_hyprland_floating_rules();
        window.present();
    }
}

fn create_nav_button(icon_name: &str, label: &str, badge_text: Option<&str>) -> gtk4::Button {
    let btn = gtk4::Button::new();
    btn.add_css_class("hyprdrop-nav-item");

    let h_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);

    let icon = gtk4::Image::from_icon_name(icon_name);
    icon.set_pixel_size(16);
    h_box.append(&icon);

    let lbl = gtk4::Label::new(Some(label));
    lbl.set_halign(gtk4::Align::Start);
    lbl.set_hexpand(true);
    h_box.append(&lbl);

    if let Some(text) = badge_text {
        let badge = gtk4::Label::new(Some(text));
        badge.add_css_class("hyprdrop-nav-badge");
        h_box.append(&badge);
    }

    btn.set_child(Some(&h_box));
    btn
}

// ============================================================================
// 1. ACTIVITY HISTORY TAB
// ============================================================================
fn build_history_tab() -> gtk4::Box {
    let box_view = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    box_view.set_margin_start(24);
    box_view.set_margin_end(24);
    box_view.set_margin_top(16);
    box_view.set_margin_bottom(16);

    let scrolled = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .build();

    let list_container = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    let list_container_rc = Rc::new(list_container.clone());

    // Toolbar (Count + Clear)
    let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    toolbar.set_margin_bottom(4);

    let records = ConfigManager::load_history();
    let count_lbl = gtk4::Label::new(Some(&format!("{} actions stored", records.len())));
    count_lbl.add_css_class("hyprdrop-page-desc");
    count_lbl.set_halign(gtk4::Align::Start);
    count_lbl.set_hexpand(true);
    toolbar.append(&count_lbl);

    let clear_btn = gtk4::Button::from_icon_name("edit-clear-symbolic");
    clear_btn.set_label("Clear History");
    clear_btn.add_css_class("hyprdrop-ghost");
    clear_btn.add_css_class("hyprdrop-btn-sm");

    let count_lbl_clone = count_lbl.clone();
    let list_container_clone = list_container_rc.clone();
    clear_btn.connect_clicked(move |_| {
        let _ = ConfigManager::clear_history();
        count_lbl_clone.set_text("0 actions stored");
        render_history_items(&list_container_clone, &[]);
    });
    toolbar.append(&clear_btn);
    box_view.append(&toolbar);

    render_history_items(&list_container, &records);

    scrolled.set_child(Some(&*list_container_rc));
    box_view.append(&scrolled);
    box_view
}

fn render_history_items(container: &gtk4::Box, records: &[HistoryRecord]) {
    // Remove existing children
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    if records.is_empty() {
        let empty_box = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        empty_box.set_vexpand(true);
        empty_box.set_valign(gtk4::Align::Center);
        empty_box.set_halign(gtk4::Align::Center);
        empty_box.set_margin_top(60);
        empty_box.set_margin_bottom(60);

        let icon = gtk4::Image::from_icon_name("document-open-recent-symbolic");
        icon.set_pixel_size(48);
        icon.add_css_class("hyprdrop-empty-icon");
        empty_box.append(&icon);

        let title = gtk4::Label::new(Some("No File Activity Yet"));
        title.add_css_class("hyprdrop-title");
        empty_box.append(&title);

        let desc = gtk4::Label::new(Some(
            "Drop files onto the right-edge hotspot or invoke Shift+Mouse to convert, crop, compress, and edit.",
        ));
        desc.add_css_class("hyprdrop-page-desc");
        desc.set_justify(gtk4::Justification::Center);
        empty_box.append(&desc);

        container.append(&empty_box);
        return;
    }

    for r in records {
        let row_card = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        row_card.add_css_class("hyprdrop-interactive-card");

        // Action badge
        let act_lbl = gtk4::Label::new(Some(&r.action));
        act_lbl.add_css_class("hyprdrop-action-badge");
        let act_lower = r.action.to_lowercase();
        if act_lower.contains("crop") || act_lower.contains("adjust") {
            act_lbl.add_css_class("action-cyan");
        } else if act_lower.contains("compress") {
            act_lbl.add_css_class("action-green");
        } else if act_lower.contains("pdf") || act_lower.contains("ocr") {
            act_lbl.add_css_class("action-purple");
        } else if act_lower.contains("bg") || act_lower.contains("redact") {
            act_lbl.add_css_class("action-amber");
        } else {
            act_lbl.add_css_class("action-blue");
        }
        row_card.append(&act_lbl);

        // Filename and Sizes
        let text_col = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        text_col.set_hexpand(true);

        let fname = r.output_path.file_name().and_then(|n| n.to_str()).unwrap_or("output");
        let name_lbl = gtk4::Label::new(Some(fname));
        name_lbl.add_css_class("hyprdrop-title");
        name_lbl.set_halign(gtk4::Align::Start);
        name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
        text_col.append(&name_lbl);

        let stats_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let size_str = format!(
            "{}  →  {}",
            format_file_size(r.original_size_bytes),
            format_file_size(r.output_size_bytes)
        );
        let size_lbl = gtk4::Label::new(Some(&size_str));
        size_lbl.add_css_class("hyprdrop-mono");
        size_lbl.set_halign(gtk4::Align::Start);
        stats_row.append(&size_lbl);

        if let Some(pct) = r.savings_pct {
            let pct_badge = gtk4::Label::new(Some(&format!("Saved {:.0}%", pct)));
            pct_badge.add_css_class("badge-success");
            stats_row.append(&pct_badge);
        }

        text_col.append(&stats_row);
        row_card.append(&text_col);

        // Actions: Open File & Reveal Folder
        let actions_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        actions_box.set_valign(gtk4::Align::Center);

        let open_btn = gtk4::Button::from_icon_name("document-open-symbolic");
        open_btn.add_css_class("hyprdrop-ghost");
        open_btn.set_tooltip_text(Some("Open processed file"));
        let out_p = r.output_path.clone();
        open_btn.connect_clicked(move |_| {
            let _ = open::that(&out_p);
        });
        actions_box.append(&open_btn);

        let folder_btn = gtk4::Button::from_icon_name("folder-symbolic");
        folder_btn.add_css_class("hyprdrop-ghost");
        folder_btn.set_tooltip_text(Some("Reveal in file manager"));
        let folder_p = r.output_path.parent().unwrap_or(Path::new("/")).to_path_buf();
        folder_btn.connect_clicked(move |_| {
            let _ = open::that(&folder_p);
        });
        actions_box.append(&folder_btn);

        row_card.append(&actions_box);
        container.append(&row_card);
    }
}

// ============================================================================
// 2. SYSTEM & HEALTH TAB
// ============================================================================
fn build_health_tab() -> gtk4::Box {
    let box_view = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    box_view.set_margin_start(24);
    box_view.set_margin_end(24);
    box_view.set_margin_top(16);
    box_view.set_margin_bottom(16);

    let scrolled = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .build();

    let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);

    let tools = [
        ("hyprctl", "Hyprland IPC client & display coordinate manager", true),
        ("wl-clipboard", "Wayland native clipboard bridge (wl-copy / wl-paste)", true),
        ("ffmpeg", "Hardware-accelerated video/audio transcoder & palettegen GIF engine", false),
        ("pdftoppm", "High-DPI PDF rasterizer for document preview & conversion", false),
        ("tesseract", "Neural OCR engine for extracting text from captured regions", false),
        ("pandoc", "Multi-format markdown and document compilation engine", false),
        ("libreoffice", "Headless office suite for DOCX, ODT and document conversions", false),
    ];

    let mut installed_count = 0;
    for (cmd, _, _) in tools {
        if Command::new("which").arg(cmd).output().map(|o| o.status.success()).unwrap_or(false) {
            installed_count += 1;
        }
    }

    // Health Overview Banner Card
    let overview_card = gtk4::Box::new(gtk4::Orientation::Horizontal, 14);
    overview_card.add_css_class("hyprdrop-card");

    let status_icon = gtk4::Image::from_icon_name(if installed_count >= 5 {
        "security-high-symbolic"
    } else {
        "security-medium-symbolic"
    });
    status_icon.set_pixel_size(28);
    status_icon.add_css_class("badge-success");
    overview_card.append(&status_icon);

    let overview_text = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    overview_text.set_hexpand(true);

    let summary_title = gtk4::Label::new(Some(&format!(
        "Engine Diagnostics: {} / {} Capabilities Available",
        installed_count,
        tools.len()
    )));
    summary_title.add_css_class("hyprdrop-title");
    summary_title.set_halign(gtk4::Align::Start);
    overview_text.append(&summary_title);

    let summary_desc = gtk4::Label::new(Some(
        "Hyprdrop automatically leverages available CLI binaries for zero-overhead local transformations.",
    ));
    summary_desc.add_css_class("hyprdrop-page-desc");
    summary_desc.set_halign(gtk4::Align::Start);
    overview_text.append(&summary_desc);
    overview_card.append(&overview_text);

    content_box.append(&overview_card);

    // Section 1: Core Wayland Engines
    let core_title = gtk4::Label::new(Some("WAYLAND & HYPRLAND CORE"));
    core_title.add_css_class("hyprdrop-section-label");
    core_title.set_halign(gtk4::Align::Start);
    content_box.append(&core_title);

    let core_list = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    for (cmd, desc, is_core) in tools {
        if !is_core {
            continue;
        }
        append_tool_card(&core_list, cmd, desc);
    }
    content_box.append(&core_list);

    // Section 2: Media & Document Acceleration Engines
    let media_title = gtk4::Label::new(Some("MEDIA & TRANSFORMATION ENGINES"));
    media_title.add_css_class("hyprdrop-section-label");
    media_title.set_halign(gtk4::Align::Start);
    media_title.set_margin_top(8);
    content_box.append(&media_title);

    let media_list = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    for (cmd, desc, is_core) in tools {
        if is_core {
            continue;
        }
        append_tool_card(&media_list, cmd, desc);
    }
    content_box.append(&media_list);

    scrolled.set_child(Some(&content_box));
    box_view.append(&scrolled);
    box_view
}

fn append_tool_card(container: &gtk4::Box, cmd: &str, desc: &str) {
    let resolved_path = Command::new("which")
        .arg(cmd)
        .output()
        .ok()
        .and_then(|o| if o.status.success() { String::from_utf8(o.stdout).ok() } else { None })
        .map(|s| s.trim().to_string());

    let is_installed = resolved_path.is_some();

    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    row.add_css_class("hyprdrop-interactive-card");

    let text_col = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    text_col.set_hexpand(true);

    let name_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let name_lbl = gtk4::Label::new(Some(cmd));
    name_lbl.add_css_class("hyprdrop-title");
    name_row.append(&name_lbl);

    if let Some(path) = &resolved_path {
        let path_lbl = gtk4::Label::new(Some(path));
        path_lbl.add_css_class("hyprdrop-mono-muted");
        name_row.append(&path_lbl);
    }
    text_col.append(&name_row);

    let desc_lbl = gtk4::Label::new(Some(desc));
    desc_lbl.add_css_class("hyprdrop-page-desc");
    desc_lbl.set_halign(gtk4::Align::Start);
    text_col.append(&desc_lbl);

    row.append(&text_col);

    let badge = if is_installed {
        let b = gtk4::Label::new(Some("● INSTALLED"));
        b.add_css_class("badge-success");
        b
    } else {
        let b = gtk4::Label::new(Some("○ OPTIONAL"));
        b.add_css_class("badge-warning");
        b
    };
    badge.set_valign(gtk4::Align::Center);
    row.append(&badge);

    container.append(&row);
}

// ============================================================================
// 3. PREFERENCES TAB
// ============================================================================
fn build_preferences_tab() -> gtk4::Box {
    let box_view = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    box_view.set_margin_start(24);
    box_view.set_margin_end(24);
    box_view.set_margin_top(16);
    box_view.set_margin_bottom(16);

    let scrolled = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .build();

    let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    let cfg = ConfigManager::load_config();

    // 1. Output Naming Architecture Card
    let naming_card = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    naming_card.add_css_class("hyprdrop-card");

    let n_title = gtk4::Label::new(Some("Output File Naming Pattern"));
    n_title.add_css_class("hyprdrop-title");
    n_title.set_halign(gtk4::Align::Start);
    naming_card.append(&n_title);

    let n_desc = gtk4::Label::new(Some(
        "Define the template used when writing converted or exported media to disk.",
    ));
    n_desc.add_css_class("hyprdrop-page-desc");
    n_desc.set_halign(gtk4::Align::Start);
    naming_card.append(&n_desc);

    let name_entry = gtk4::Entry::new();
    name_entry.set_text(&cfg.naming_pattern);
    name_entry.set_placeholder_text(Some("{stem} ({counter}).{ext}"));
    naming_card.append(&name_entry);

    // Live preview label
    let preview_lbl = gtk4::Label::new(None);
    preview_lbl.add_css_class("hyprdrop-preview-pill");
    preview_lbl.set_halign(gtk4::Align::Start);

    let update_preview = |entry: &gtk4::Entry, label: &gtk4::Label| {
        let pattern = entry.text().to_string();
        let sample = pattern
            .replace("{stem}", "wallpaper")
            .replace("{counter}", "1")
            .replace("{ext}", "png");
        label.set_text(&format!("Live Preview: {}", sample));
    };

    update_preview(&name_entry, &preview_lbl);

    let preview_lbl_clone = preview_lbl.clone();
    name_entry.connect_changed(move |e| {
        update_preview(e, &preview_lbl_clone);
    });

    // Helper Variable Chips
    let chip_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let chip_info = gtk4::Label::new(Some("Insert variable:"));
    chip_info.add_css_class("hyprdrop-mono-muted");
    chip_row.append(&chip_info);

    let vars = ["{stem}", "{counter}", "{ext}"];
    for var in vars {
        let chip = gtk4::Button::with_label(var);
        chip.add_css_class("hyprdrop-pill");
        chip.add_css_class("hyprdrop-btn-sm");
        let entry_clone = name_entry.clone();
        let var_str = var.to_string();
        chip.connect_clicked(move |_| {
            let cur = entry_clone.text().to_string();
            entry_clone.set_text(&format!("{}{}", cur, var_str));
        });
        chip_row.append(&chip);
    }
    naming_card.append(&chip_row);
    naming_card.append(&preview_lbl);
    content_box.append(&naming_card);

    // 2. Media Compression & Quality
    let media_card = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    media_card.add_css_class("hyprdrop-card");

    let m_title = gtk4::Label::new(Some("Media Compression & Quality"));
    m_title.add_css_class("hyprdrop-title");
    m_title.set_halign(gtk4::Align::Start);
    media_card.append(&m_title);

    let jq_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    let jq_label_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    jq_label_box.set_hexpand(true);

    let jq_title = gtk4::Label::new(Some("Default JPEG Quality"));
    jq_title.add_css_class("hyprdrop-title-sm");
    jq_title.set_halign(gtk4::Align::Start);
    jq_label_box.append(&jq_title);

    let jq_desc = gtk4::Label::new(Some("Target quantization level for JPEG encoding and resizing"));
    jq_desc.add_css_class("hyprdrop-page-desc");
    jq_desc.set_halign(gtk4::Align::Start);
    jq_label_box.append(&jq_desc);
    jq_row.append(&jq_label_box);

    let jq_value_lbl = gtk4::Label::new(Some(&format!("{}%", cfg.jpeg_quality)));
    jq_value_lbl.add_css_class("hyprdrop-mono-badge");

    let jq_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 10.0, 100.0, 5.0);
    jq_scale.set_value(cfg.jpeg_quality as f64);
    jq_scale.set_size_request(160, -1);

    let jq_val_clone = jq_value_lbl.clone();
    jq_scale.connect_value_changed(move |s| {
        jq_val_clone.set_text(&format!("{:.0}%", s.value()));
    });

    jq_row.append(&jq_scale);
    jq_row.append(&jq_value_lbl);
    media_card.append(&jq_row);
    content_box.append(&media_card);

    // 3. Drop Shelf & Floating Behavior
    let shelf_card = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    shelf_card.add_css_class("hyprdrop-card");

    let s_title = gtk4::Label::new(Some("Drop Shelf Overlay"));
    s_title.add_css_class("hyprdrop-title");
    s_title.set_halign(gtk4::Align::Start);
    shelf_card.append(&s_title);

    // Toggle row
    let toggle_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    let toggle_text = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    toggle_text.set_hexpand(true);

    let t_title = gtk4::Label::new(Some("Display Shelf on Operation Finish"));
    t_title.add_css_class("hyprdrop-title-sm");
    t_title.set_halign(gtk4::Align::Start);
    toggle_text.append(&t_title);

    let t_desc = gtk4::Label::new(Some("Keeps exported files hovering on-screen ready to drag directly into apps"));
    t_desc.add_css_class("hyprdrop-page-desc");
    t_desc.set_halign(gtk4::Align::Start);
    toggle_text.append(&t_desc);
    toggle_row.append(&toggle_text);

    let shelf_switch = gtk4::Switch::new();
    shelf_switch.set_active(cfg.show_drop_shelf);
    shelf_switch.set_valign(gtk4::Align::Center);
    toggle_row.append(&shelf_switch);
    shelf_card.append(&toggle_row);

    // Timeout slider row
    let timeout_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    timeout_row.set_margin_top(8);

    let to_text = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    to_text.set_hexpand(true);

    let to_title = gtk4::Label::new(Some("Shelf Auto-Dismiss Timeout"));
    to_title.add_css_class("hyprdrop-title-sm");
    to_title.set_halign(gtk4::Align::Start);
    to_text.append(&to_title);

    let to_desc = gtk4::Label::new(Some("Seconds before hovering shelf smoothly fades out"));
    to_desc.add_css_class("hyprdrop-page-desc");
    to_desc.set_halign(gtk4::Align::Start);
    to_text.append(&to_desc);
    timeout_row.append(&to_text);

    let timeout_val_lbl = gtk4::Label::new(Some(&format!("{}s", cfg.drop_shelf_timeout_secs)));
    timeout_val_lbl.add_css_class("hyprdrop-mono-badge");

    let timeout_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 5.0, 120.0, 5.0);
    timeout_scale.set_value(cfg.drop_shelf_timeout_secs as f64);
    timeout_scale.set_size_request(160, -1);

    let to_val_clone = timeout_val_lbl.clone();
    timeout_scale.connect_value_changed(move |s| {
        to_val_clone.set_text(&format!("{:.0}s", s.value()));
    });

    timeout_row.append(&timeout_scale);
    timeout_row.append(&timeout_val_lbl);
    shelf_card.append(&timeout_row);
    content_box.append(&shelf_card);

    // 4. Autostart On Login
    let auto_card = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    auto_card.add_css_class("hyprdrop-card");

    let auto_text = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    auto_text.set_hexpand(true);

    let a_title = gtk4::Label::new(Some("Launch on System Boot"));
    a_title.add_css_class("hyprdrop-title");
    a_title.set_halign(gtk4::Align::Start);
    auto_text.append(&a_title);

    let a_desc = gtk4::Label::new(Some(
        "Starts background service and edge hotspot via systemd user unit upon graphical login.",
    ));
    a_desc.add_css_class("hyprdrop-page-desc");
    a_desc.set_halign(gtk4::Align::Start);
    auto_text.append(&a_desc);
    auto_card.append(&auto_text);

    let autostart_switch = gtk4::Switch::new();
    let is_auto = crate::hyprland::is_autostart_enabled();
    autostart_switch.set_active(is_auto);
    autostart_switch.set_valign(gtk4::Align::Center);
    autostart_switch.connect_state_set(move |_, state| {
        let _ = crate::hyprland::set_autostart(state);
        let mut c = ConfigManager::load_config();
        c.autostart = state;
        let _ = ConfigManager::save_config(&c);
        gtk4::glib::Propagation::Proceed
    });
    auto_card.append(&autostart_switch);
    content_box.append(&auto_card);

    // Bottom Save Row
    let save_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    save_row.set_margin_top(8);

    let save_btn = gtk4::Button::with_label("Save Preferences");
    save_btn.add_css_class("hyprdrop-primary");

    let save_feedback = gtk4::Label::new(None);
    save_feedback.add_css_class("badge-success");
    save_feedback.set_visible(false);

    let name_entry_clone = name_entry.clone();
    let jq_scale_clone = jq_scale.clone();
    let to_scale_clone = timeout_scale.clone();
    let shelf_sw_clone = shelf_switch.clone();
    let fb_clone = save_feedback.clone();

    save_btn.connect_clicked(move |_| {
        let mut c = ConfigManager::load_config();
        let txt = name_entry_clone.text().to_string();
        if !txt.trim().is_empty() {
            c.naming_pattern = txt;
        }
        c.jpeg_quality = jq_scale_clone.value().round() as u8;
        c.drop_shelf_timeout_secs = to_scale_clone.value().round() as u32;
        c.show_drop_shelf = shelf_sw_clone.is_active();

        if ConfigManager::save_config(&c).is_ok() {
            fb_clone.set_text("✓ Preferences saved");
            fb_clone.set_visible(true);
        }
    });

    save_row.append(&save_btn);
    save_row.append(&save_feedback);
    content_box.append(&save_row);

    scrolled.set_child(Some(&content_box));
    box_view.append(&scrolled);
    box_view
}

// ============================================================================
// 4. HYPRLAND INTEGRATION TAB
// ============================================================================
fn build_hyprland_tab() -> gtk4::Box {
    let box_view = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    box_view.set_margin_start(24);
    box_view.set_margin_end(24);
    box_view.set_margin_top(16);
    box_view.set_margin_bottom(16);

    let scrolled = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .build();

    let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);

    let (is_conf, config_path, format) = get_hyprland_config_info();
    let file_name = config_path.file_name().and_then(|n| n.to_str()).unwrap_or("hyprland.lua");

    // Detection Card
    let det_card = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    det_card.add_css_class("hyprdrop-card");

    let d_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    let d_title = gtk4::Label::new(Some("Hyprland Configuration Target"));
    d_title.add_css_class("hyprdrop-title");
    d_title.set_hexpand(true);
    d_title.set_halign(gtk4::Align::Start);
    d_header.append(&d_title);

    let format_badge = gtk4::Label::new(Some(format.display_name()));
    format_badge.add_css_class("badge-info");
    d_header.append(&format_badge);
    det_card.append(&d_header);

    let path_lbl = gtk4::Label::new(Some(&config_path.display().to_string()));
    path_lbl.add_css_class("hyprdrop-mono-muted");
    path_lbl.set_halign(gtk4::Align::Start);
    det_card.append(&path_lbl);

    let status_str = if is_conf {
        format!("✓ Active configuration detected in {} (Window rules, blur layers & keybinds active)", file_name)
    } else {
        format!("○ Configuration rules not yet written to {}", file_name)
    };

    let status_lbl = gtk4::Label::new(Some(&status_str));
    status_lbl.add_css_class(if is_conf { "badge-success" } else { "badge-warning" });
    status_lbl.set_halign(gtk4::Align::Start);
    det_card.append(&status_lbl);

    let setup_btn = gtk4::Button::with_label(&format!(
        "Auto-Configure Hyprland Rules ({})",
        format.display_name()
    ));
    setup_btn.add_css_class("hyprdrop-primary");
    setup_btn.set_halign(gtk4::Align::Start);

    let status_lbl_clone = status_lbl.clone();
    let btn_clone = setup_btn.clone();
    setup_btn.connect_clicked(move |_| {
        match configure_hyprland() {
            Ok(res) => {
                if res.already_configured {
                    status_lbl_clone.set_text("✓ Hyprland is already fully configured for Hyprdrop.");
                } else {
                    let backup_str = res
                        .backup_path
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "No prior file".to_string());
                    let file_display = res.config_path.file_name().and_then(|n| n.to_str()).unwrap_or("config");
                    status_lbl_clone.set_text(&format!(
                        "✓ Successfully injected rules into {}! Backup created: {}",
                        file_display, backup_str
                    ));
                    btn_clone.set_sensitive(false);
                }
            }
            Err(e) => {
                status_lbl_clone.set_text(&format!("Configuration error: {:?}", e));
            }
        }
    });
    det_card.append(&setup_btn);
    content_box.append(&det_card);

    // Rule Snippet Preview Card
    let snippet_card = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    snippet_card.add_css_class("hyprdrop-card");

    let snip_title = gtk4::Label::new(Some("Rule Definitions Preview"));
    snip_title.add_css_class("hyprdrop-title");
    snip_title.set_halign(gtk4::Align::Start);
    snippet_card.append(&snip_title);

    let snip_desc = gtk4::Label::new(Some(
        "Hyprdrop injects clean non-invasive rules for blur, ignorezero, layer animations, and Shift+Click binds.",
    ));
    snip_desc.add_css_class("hyprdrop-page-desc");
    snip_desc.set_halign(gtk4::Align::Start);
    snippet_card.append(&snip_desc);

    let snippet_text = match format {
        crate::hyprland::installer::ConfigFormat::Lua => LUA_CONFIG_SNIPPET,
        crate::hyprland::installer::ConfigFormat::Conf => CONF_CONFIG_SNIPPET,
    };

    let snippet_view = gtk4::TextView::new();
    snippet_view.set_editable(false);
    snippet_view.set_cursor_visible(false);
    snippet_view.add_css_class("hyprdrop-code-preview");
    snippet_view.buffer().set_text(snippet_text);
    snippet_view.set_size_request(-1, 180);

    let snip_scrolled = gtk4::ScrolledWindow::builder()
        .child(&snippet_view)
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .build();
    snip_scrolled.set_size_request(-1, 180);

    snippet_card.append(&snip_scrolled);
    content_box.append(&snippet_card);

    scrolled.set_child(Some(&content_box));
    box_view.append(&scrolled);
    box_view
}
