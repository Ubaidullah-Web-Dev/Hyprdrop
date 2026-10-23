use std::cell::RefCell;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;
use gtk4::prelude::*;
use gtk4::cairo::Context;

use crate::config::{AppConfig, ConfigManager, HistoryRecord};
use crate::engine::format_file_size;
use crate::engine::perimeter_math::{PerimeterMath, ScreenEdge};
use crate::hyprland::installer::{
    configure_hyprland, get_hyprland_config_info, CONF_CONFIG_SNIPPET, LUA_CONFIG_SNIPPET,
};
use crate::ipc::{IpcClient, IpcMessage};
use crate::ui::hotspot::parse_hex_color;

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
            .default_width(980)
            .default_height(660)
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

        let brand_badge = gtk4::Label::new(Some(&format!("v{}", env!("CARGO_PKG_VERSION"))));
        brand_badge.add_css_class("hyprdrop-brand-badge");
        brand_title_row.append(&brand_badge);
        brand_text_box.append(&brand_title_row);

        let brand_sub = gtk4::Label::new(Some("WAYLAND WORKSPACE"));
        brand_sub.add_css_class("hyprdrop-brand-sub");
        brand_sub.set_halign(gtk4::Align::Start);
        brand_text_box.append(&brand_sub);

        brand_box.append(&brand_text_box);
        sidebar.append(&brand_box);

        // Sidebar Nav Items (5 Distinct Studios)
        let nav_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

        let history_records = ConfigManager::load_history();
        let history_count = history_records.len();

        let nav_activity = create_nav_button(
            "document-open-recent-symbolic",
            "Activity Log",
            Some(&history_count.to_string()),
        );
        let nav_hotspot = create_nav_button(
            "input-touchscreen-symbolic",
            "Edge Hotspot",
            None,
        );
        let nav_themes = create_nav_button(
            "applications-graphics-symbolic",
            "Themes & Colors",
            None,
        );
        let nav_prefs = create_nav_button(
            "emblem-system-symbolic",
            "Preferences",
            None,
        );
        let nav_system = create_nav_button(
            "utilities-system-monitor-symbolic",
            "Hyprland & Health",
            None,
        );

        nav_activity.add_css_class("active");

        nav_box.append(&nav_activity);
        nav_box.append(&nav_hotspot);
        nav_box.append(&nav_themes);
        nav_box.append(&nav_prefs);
        nav_box.append(&nav_system);
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
        let hotspot_tab = build_hotspot_studio_tab();
        let themes_tab = build_themes_tab();
        let prefs_tab = build_preferences_tab();
        let system_tab = build_hyprland_and_health_tab();

        stack.add_named(&history_tab, Some("activity"));
        stack.add_named(&hotspot_tab, Some("hotspot"));
        stack.add_named(&themes_tab, Some("themes"));
        stack.add_named(&prefs_tab, Some("preferences"));
        stack.add_named(&system_tab, Some("system"));
        stack.set_visible_child_name("activity");

        main_col.append(&stack);
        root_box.append(&main_col);

        // Connect Navigation Buttons
        let nav_buttons = [
            (nav_activity.clone(), "activity", "Activity Log", "Recent file transformations, optimizations and studio exports"),
            (nav_hotspot.clone(), "hotspot", "Edge Hotspot Studio", "Interactive perimeter placement, corner morphing and dynamic sizing"),
            (nav_themes.clone(), "themes", "Themes & Colors", "Curated color themes and aesthetic palettes for edge hotspot and wheel"),
            (nav_prefs.clone(), "preferences", "Preferences", "Global naming rules, compression thresholds and workflow options"),
            (nav_system.clone(), "system", "Hyprland & Health", "Window rules, blur layers and system engine diagnostics"),
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
            "Drop files onto the edge hotspot or invoke Shift+Mouse to convert, crop, compress, and edit.",
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
// 2. EDGE HOTSPOT STUDIO TAB
// ============================================================================
fn build_hotspot_studio_tab() -> gtk4::Box {
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
    let cfg = Rc::new(RefCell::new(ConfigManager::load_config()));

    // --- CARD 1: Interactive Screen Simulator & Quick Snap ---
    let sim_card = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    sim_card.add_css_class("hyprdrop-card");

    let sim_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    let sim_title_col = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    sim_title_col.set_hexpand(true);

    let sim_title = gtk4::Label::new(Some("Interactive Desktop Screen Simulator"));
    sim_title.add_css_class("hyprdrop-title");
    sim_title.set_halign(gtk4::Align::Start);
    sim_title_col.append(&sim_title);

    let sim_desc = gtk4::Label::new(Some(
        "Click or drag anywhere around the screen edges to relocate the hotspot live. Continuous corner bending morphs automatically.",
    ));
    sim_desc.add_css_class("hyprdrop-page-desc");
    sim_desc.set_halign(gtk4::Align::Start);
    sim_title_col.append(&sim_desc);
    sim_header.append(&sim_title_col);

    // Initial Span Edge Calculation for Status Badge
    let initial_pos = cfg.borrow().hotspot_perimeter_pos;
    let initial_math = PerimeterMath::new(1920.0, 1080.0);
    let initial_span = initial_math.calculate_span(initial_math.t_to_s(initial_pos), cfg.borrow().hotspot_length as f64);
    let edge_badge = gtk4::Label::new(Some(&format!("● {} ({:.1}%)", PerimeterMath::edge_name(&initial_span.edge), initial_pos * 100.0)));
    edge_badge.add_css_class("badge-info");
    sim_header.append(&edge_badge);
    sim_card.append(&sim_header);

    // Drawing Canvas for Monitor Simulation (440x220)
    let drawing_area = gtk4::DrawingArea::new();
    drawing_area.set_content_width(440);
    drawing_area.set_content_height(220);
    drawing_area.add_css_class("hyprdrop-monitor-canvas");

    let cfg_draw = cfg.clone();
    drawing_area.set_draw_func(move |_area, cr, width, height| {
        draw_mini_monitor(cr, width as f64, height as f64, &cfg_draw.borrow());
    });

    // Outer center alignment for monitor
    let canvas_center_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    canvas_center_box.set_halign(gtk4::Align::Center);
    canvas_center_box.append(&drawing_area);
    sim_card.append(&canvas_center_box);

    // Forward declaration of position scale so monitor drag can update it
    let pos_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 100.0, 0.5);
    pos_scale.set_value(initial_pos * 100.0);
    pos_scale.set_size_request(200, -1);

    // Mouse Interaction on Monitor Canvas (Click / Drag to position)
    let drag_gesture = gtk4::GestureDrag::new();
    let cfg_drag = cfg.clone();
    let da_drag = drawing_area.clone();
    let pos_scale_drag = pos_scale.clone();
    let edge_badge_drag = edge_badge.clone();

    drag_gesture.connect_drag_update(move |_g, offset_x, offset_y| {
        let start_opt = _g.start_point();
        if let Some((start_x, start_y)) = start_opt {
            let cur_x = start_x + offset_x;
            let cur_y = start_y + offset_y;

            // Monitor screen bounds
            let sx = 28.0;
            let sy = 20.0;
            let sw = 440.0 - 56.0;
            let sh = 220.0 - 54.0;

            let px = (cur_x - sx).clamp(0.0, sw);
            let py = (cur_y - sy).clamp(0.0, sh);

            let mini_math = PerimeterMath::new(sw, sh);
            let s = mini_math.cursor_to_perimeter_s(px, py);
            let t = mini_math.s_to_t(s).clamp(0.0, 0.9999);

            cfg_drag.borrow_mut().hotspot_perimeter_pos = t;
            let _ = ConfigManager::save_config(&cfg_drag.borrow());
            let _ = IpcClient::send(&IpcMessage::ReloadHotspot);

            pos_scale_drag.set_value(t * 100.0);
            let math = PerimeterMath::new(1920.0, 1080.0);
            let span = math.calculate_span(math.t_to_s(t), cfg_drag.borrow().hotspot_length as f64);
            edge_badge_drag.set_text(&format!("● {} ({:.1}%)", PerimeterMath::edge_name(&span.edge), t * 100.0));
            da_drag.queue_draw();
        }
    });

    drawing_area.add_controller(drag_gesture);

    // Quick Snap Buttons Rows
    let snap_container = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    snap_container.set_margin_top(4);

    let row1 = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let row1_lbl = gtk4::Label::new(Some("Edge Snaps:"));
    row1_lbl.add_css_class("hyprdrop-mono-muted");
    row1_lbl.set_size_request(90, -1);
    row1_lbl.set_halign(gtk4::Align::Start);
    row1.append(&row1_lbl);

    let edge_snaps = [
        ("Top Edge", 0.125),
        ("Right Edge", 0.375),
        ("Bottom Edge", 0.625),
        ("Left Edge", 0.875),
    ];

    for (name, target_t) in edge_snaps {
        let btn = gtk4::Button::with_label(name);
        btn.add_css_class("hyprdrop-pill");
        btn.add_css_class("hyprdrop-btn-sm");

        let cfg_s = cfg.clone();
        let da_s = drawing_area.clone();
        let ps_s = pos_scale.clone();
        let badge_s = edge_badge.clone();
        btn.connect_clicked(move |_| {
            cfg_s.borrow_mut().hotspot_perimeter_pos = target_t;
            let _ = ConfigManager::save_config(&cfg_s.borrow());
            let _ = IpcClient::send(&IpcMessage::ReloadHotspot);
            ps_s.set_value(target_t * 100.0);
            let math = PerimeterMath::new(1920.0, 1080.0);
            let span = math.calculate_span(math.t_to_s(target_t), cfg_s.borrow().hotspot_length as f64);
            badge_s.set_text(&format!("● {} ({:.1}%)", PerimeterMath::edge_name(&span.edge), target_t * 100.0));
            da_s.queue_draw();
        });
        row1.append(&btn);
    }
    snap_container.append(&row1);

    let row2 = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let row2_lbl = gtk4::Label::new(Some("Corner Morphs:"));
    row2_lbl.add_css_class("hyprdrop-mono-muted");
    row2_lbl.set_size_request(90, -1);
    row2_lbl.set_halign(gtk4::Align::Start);
    row2.append(&row2_lbl);

    let corner_snaps = [
        ("Top-Right ↱", 0.250),
        ("Bottom-Right ↳", 0.500),
        ("Bottom-Left ↲", 0.750),
        ("Top-Left ↰", 0.000),
    ];

    for (name, target_t) in corner_snaps {
        let btn = gtk4::Button::with_label(name);
        btn.add_css_class("hyprdrop-pill");
        btn.add_css_class("hyprdrop-btn-sm");

        let cfg_s = cfg.clone();
        let da_s = drawing_area.clone();
        let ps_s = pos_scale.clone();
        let badge_s = edge_badge.clone();
        btn.connect_clicked(move |_| {
            cfg_s.borrow_mut().hotspot_perimeter_pos = target_t;
            let _ = ConfigManager::save_config(&cfg_s.borrow());
            let _ = IpcClient::send(&IpcMessage::ReloadHotspot);
            ps_s.set_value(target_t * 100.0);
            let math = PerimeterMath::new(1920.0, 1080.0);
            let span = math.calculate_span(math.t_to_s(target_t), cfg_s.borrow().hotspot_length as f64);
            badge_s.set_text(&format!("● {} ({:.1}%)", PerimeterMath::edge_name(&span.edge), target_t * 100.0));
            da_s.queue_draw();
        });
        row2.append(&btn);
    }
    snap_container.append(&row2);
    sim_card.append(&snap_container);
    content_box.append(&sim_card);

    // --- CARD 2: Position Lock & Desktop Drag Switch ---
    let lock_card = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    lock_card.add_css_class("hyprdrop-card");

    let lock_text = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    lock_text.set_hexpand(true);

    let l_title = gtk4::Label::new(Some("Lock Hotspot Position"));
    l_title.add_css_class("hyprdrop-title");
    l_title.set_halign(gtk4::Align::Start);
    lock_text.append(&l_title);

    let l_desc = gtk4::Label::new(Some(
        "When locked, dragging directly on your desktop is prevented so the hotspot cannot be accidentally relocated.",
    ));
    l_desc.add_css_class("hyprdrop-page-desc");
    l_desc.set_halign(gtk4::Align::Start);
    lock_text.append(&l_desc);
    lock_card.append(&lock_text);

    let is_locked = cfg.borrow().hotspot_locked;
    let lock_badge = gtk4::Label::new(Some(if is_locked {
        "🔒 LOCKED (Safe Mode)"
    } else {
        "🔓 UNLOCKED (Drag freely)"
    }));
    lock_badge.add_css_class(if is_locked { "badge-warning" } else { "badge-success" });
    lock_badge.set_valign(gtk4::Align::Center);
    lock_card.append(&lock_badge);

    let lock_switch = gtk4::Switch::new();
    lock_switch.set_active(is_locked);
    lock_switch.set_valign(gtk4::Align::Center);

    let cfg_lock = cfg.clone();
    let da_lock = drawing_area.clone();
    let lock_badge_clone = lock_badge.clone();
    lock_switch.connect_state_set(move |_, state| {
        cfg_lock.borrow_mut().hotspot_locked = state;
        let _ = ConfigManager::save_config(&cfg_lock.borrow());
        let _ = IpcClient::send(&IpcMessage::ReloadHotspot);
        if state {
            lock_badge_clone.set_text("🔒 LOCKED (Safe Mode)");
            lock_badge_clone.remove_css_class("badge-success");
            lock_badge_clone.add_css_class("badge-warning");
        } else {
            lock_badge_clone.set_text("🔓 UNLOCKED (Drag freely)");
            lock_badge_clone.remove_css_class("badge-warning");
            lock_badge_clone.add_css_class("badge-success");
        }
        da_lock.queue_draw();
        gtk4::glib::Propagation::Proceed
    });
    lock_card.append(&lock_switch);
    content_box.append(&lock_card);

    // --- CARD 3: Precision Geometry & Dynamic Sizing Studio ---
    let size_card = gtk4::Box::new(gtk4::Orientation::Vertical, 14);
    size_card.add_css_class("hyprdrop-card");

    let s_header = gtk4::Label::new(Some("Dynamic Hotspot Sizing & Perimeter Coordinates"));
    s_header.add_css_class("hyprdrop-title");
    s_header.set_halign(gtk4::Align::Start);
    size_card.append(&s_header);

    // 1. Perimeter Coordinate Slider
    let pos_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    let pos_text = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    pos_text.set_hexpand(true);

    let pos_title = gtk4::Label::new(Some("Perimeter Coordinate (0% – 100%)"));
    pos_title.add_css_class("hyprdrop-title-sm");
    pos_title.set_halign(gtk4::Align::Start);
    pos_text.append(&pos_title);

    let pos_desc = gtk4::Label::new(Some("Continuous loop along screen perimeter (Top → Right → Bottom → Left)"));
    pos_desc.add_css_class("hyprdrop-page-desc");
    pos_desc.set_halign(gtk4::Align::Start);
    pos_text.append(&pos_desc);
    pos_row.append(&pos_text);

    let pos_val_lbl = gtk4::Label::new(Some(&format!("{:.1}%", initial_pos * 100.0)));
    pos_val_lbl.add_css_class("hyprdrop-mono-badge");

    let cfg_pos = cfg.clone();
    let da_pos = drawing_area.clone();
    let badge_pos = edge_badge.clone();
    let pos_val_clone = pos_val_lbl.clone();
    pos_scale.connect_value_changed(move |s| {
        let t = s.value() / 100.0;
        cfg_pos.borrow_mut().hotspot_perimeter_pos = t;
        let _ = ConfigManager::save_config(&cfg_pos.borrow());
        let _ = IpcClient::send(&IpcMessage::ReloadHotspot);
        pos_val_clone.set_text(&format!("{:.1}%", s.value()));
        let math = PerimeterMath::new(1920.0, 1080.0);
        let span = math.calculate_span(math.t_to_s(t), cfg_pos.borrow().hotspot_length as f64);
        badge_pos.set_text(&format!("● {} ({:.1}%)", PerimeterMath::edge_name(&span.edge), s.value()));
        da_pos.queue_draw();
    });

    pos_row.append(&pos_scale);
    pos_row.append(&pos_val_lbl);
    size_card.append(&pos_row);

    // 2. Length Slider
    let (len_row, len_scale, _) = create_slider_row(
        "Hotspot Length",
        "Total length along screen edge or corner bend span",
        80.0,
        450.0,
        10.0,
        cfg.borrow().hotspot_length as f64,
        "px",
    );
    let cfg_len = cfg.clone();
    let da_len = drawing_area.clone();
    len_scale.connect_value_changed(move |s| {
        cfg_len.borrow_mut().hotspot_length = s.value().round() as u32;
        let _ = ConfigManager::save_config(&cfg_len.borrow());
        let _ = IpcClient::send(&IpcMessage::ReloadHotspot);
        da_len.queue_draw();
    });
    size_card.append(&len_row);

    // 3. Thickness Slider
    let (thick_row, thick_scale, _) = create_slider_row(
        "Hotspot Thickness",
        "Width / depth of the tactile bar when idle",
        4.0,
        24.0,
        1.0,
        cfg.borrow().hotspot_thickness as f64,
        "px",
    );
    let cfg_thick = cfg.clone();
    let da_thick = drawing_area.clone();
    thick_scale.connect_value_changed(move |s| {
        cfg_thick.borrow_mut().hotspot_thickness = s.value().round() as u32;
        let _ = ConfigManager::save_config(&cfg_thick.borrow());
        let _ = IpcClient::send(&IpcMessage::ReloadHotspot);
        da_thick.queue_draw();
    });
    size_card.append(&thick_row);

    // 4. Margin from Screen Edge Slider
    let (margin_row, margin_scale, _) = create_slider_row(
        "Distance From Edge",
        "Inset spacing between monitor bezel and hotspot bar",
        0.0,
        40.0,
        2.0,
        cfg.borrow().hotspot_margin as f64,
        "px",
    );
    let cfg_margin = cfg.clone();
    let da_margin = drawing_area.clone();
    margin_scale.connect_value_changed(move |s| {
        cfg_margin.borrow_mut().hotspot_margin = s.value().round() as i32;
        let _ = ConfigManager::save_config(&cfg_margin.borrow());
        let _ = IpcClient::send(&IpcMessage::ReloadHotspot);
        da_margin.queue_draw();
    });
    size_card.append(&margin_row);

    // 5. Corner Rounding Radius Slider
    let (rad_row, rad_scale, _) = create_slider_row(
        "Corner Rounding Radius",
        "Smoothness of pill caps and L-bend corner curvature",
        2.0,
        16.0,
        1.0,
        cfg.borrow().hotspot_corner_radius as f64,
        "px",
    );
    let cfg_rad = cfg.clone();
    let da_rad = drawing_area.clone();
    rad_scale.connect_value_changed(move |s| {
        cfg_rad.borrow_mut().hotspot_corner_radius = s.value().round() as u32;
        let _ = ConfigManager::save_config(&cfg_rad.borrow());
        let _ = IpcClient::send(&IpcMessage::ReloadHotspot);
        da_rad.queue_draw();
    });
    size_card.append(&rad_row);

    content_box.append(&size_card);

    // --- CARD 4: Behavior & Auto-Hide Switches ---
    let beh_card = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    beh_card.add_css_class("hyprdrop-card");

    let b_title = gtk4::Label::new(Some("Visibility & Proximity Behavior"));
    b_title.add_css_class("hyprdrop-title");
    b_title.set_halign(gtk4::Align::Start);
    beh_card.append(&b_title);

    // Enable hotspot toggle
    let en_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    let en_text = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    en_text.set_hexpand(true);

    let en_title = gtk4::Label::new(Some("Enable Edge Hotspot"));
    en_title.add_css_class("hyprdrop-title-sm");
    en_title.set_halign(gtk4::Align::Start);
    en_text.append(&en_title);

    let en_desc = gtk4::Label::new(Some("Shows layer-shell drop target bar along configured screen perimeter"));
    en_desc.add_css_class("hyprdrop-page-desc");
    en_desc.set_halign(gtk4::Align::Start);
    en_text.append(&en_desc);
    en_row.append(&en_text);

    let en_switch = gtk4::Switch::new();
    en_switch.set_active(cfg.borrow().hotspot_enabled);
    en_switch.set_valign(gtk4::Align::Center);

    let cfg_en = cfg.clone();
    let da_en = drawing_area.clone();
    en_switch.connect_state_set(move |_, state| {
        cfg_en.borrow_mut().hotspot_enabled = state;
        let _ = ConfigManager::save_config(&cfg_en.borrow());
        let _ = IpcClient::send(&IpcMessage::ReloadHotspot);
        da_en.queue_draw();
        gtk4::glib::Propagation::Proceed
    });
    en_row.append(&en_switch);
    beh_card.append(&en_row);

    // Proximity auto-hide toggle
    let ah_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    let ah_text = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    ah_text.set_hexpand(true);

    let ah_title = gtk4::Label::new(Some("Proximity Auto-Hide"));
    ah_title.add_css_class("hyprdrop-title-sm");
    ah_title.set_halign(gtk4::Align::Start);
    ah_text.append(&ah_title);

    let ah_desc = gtk4::Label::new(Some("Subtly dims the bar until mouse approaches or files are dragged toward it"));
    ah_desc.add_css_class("hyprdrop-page-desc");
    ah_desc.set_halign(gtk4::Align::Start);
    ah_text.append(&ah_desc);
    ah_row.append(&ah_text);

    let ah_switch = gtk4::Switch::new();
    ah_switch.set_active(cfg.borrow().hotspot_auto_hide);
    ah_switch.set_valign(gtk4::Align::Center);

    let cfg_ah = cfg.clone();
    let da_ah = drawing_area.clone();
    ah_switch.connect_state_set(move |_, state| {
        cfg_ah.borrow_mut().hotspot_auto_hide = state;
        let _ = ConfigManager::save_config(&cfg_ah.borrow());
        let _ = IpcClient::send(&IpcMessage::ReloadHotspot);
        da_ah.queue_draw();
        gtk4::glib::Propagation::Proceed
    });
    ah_row.append(&ah_switch);
    beh_card.append(&ah_row);
    content_box.append(&beh_card);

    // --- CARD 5: Custom Color & Glow Styling ---
    let col_card = gtk4::Box::new(gtk4::Orientation::Vertical, 14);
    col_card.add_css_class("hyprdrop-card");

    let c_title = gtk4::Label::new(Some("Color & Luminescence Customization"));
    c_title.add_css_class("hyprdrop-title");
    c_title.set_halign(gtk4::Align::Start);
    col_card.append(&c_title);

    let swatches = [
        ("#00D2FF", "swatch-cyan"),
        ("#10B981", "swatch-emerald"),
        ("#A855F7", "swatch-purple"),
        ("#F59E0B", "swatch-amber"),
        ("#EC4899", "swatch-rose"),
        ("#F8FAFC", "swatch-white"),
        ("#1E2230", "swatch-dark"),
    ];

    // Base Color Row
    let (base_row, base_entry) = create_swatch_color_row(
        "Base Hotspot Color",
        "Primary idle tone of the tactile perimeter bar",
        &cfg.borrow().hotspot_color,
        &swatches,
    );
    let cfg_base = cfg.clone();
    let da_base = drawing_area.clone();
    base_entry.connect_changed(move |e| {
        let hex = e.text().to_string();
        if hex.starts_with('#') && hex.len() >= 7 {
            cfg_base.borrow_mut().hotspot_color = hex;
            let _ = ConfigManager::save_config(&cfg_base.borrow());
            let _ = IpcClient::send(&IpcMessage::ReloadHotspot);
            da_base.queue_draw();
        }
    });
    col_card.append(&base_row);

    // Hover Color Row
    let (hov_row, hov_entry) = create_swatch_color_row(
        "Hover Luminescence",
        "Glow color when cursor hovers near or over the hotspot",
        &cfg.borrow().hotspot_hover_color,
        &swatches,
    );
    let cfg_hov = cfg.clone();
    let da_hov = drawing_area.clone();
    hov_entry.connect_changed(move |e| {
        let hex = e.text().to_string();
        if hex.starts_with('#') && hex.len() >= 7 {
            cfg_hov.borrow_mut().hotspot_hover_color = hex;
            let _ = ConfigManager::save_config(&cfg_hov.borrow());
            let _ = IpcClient::send(&IpcMessage::ReloadHotspot);
            da_hov.queue_draw();
        }
    });
    col_card.append(&hov_row);

    // Accent Color Row
    let (act_row, act_entry) = create_swatch_color_row(
        "Active Drop Accent",
        "Highlight aura when dragged files enter drop target boundary",
        &cfg.borrow().hotspot_accent_color,
        &swatches,
    );
    let cfg_act = cfg.clone();
    let da_act = drawing_area.clone();
    act_entry.connect_changed(move |e| {
        let hex = e.text().to_string();
        if hex.starts_with('#') && hex.len() >= 7 {
            cfg_act.borrow_mut().hotspot_accent_color = hex;
            let _ = ConfigManager::save_config(&cfg_act.borrow());
            let _ = IpcClient::send(&IpcMessage::ReloadHotspot);
            da_act.queue_draw();
        }
    });
    col_card.append(&act_row);

    content_box.append(&col_card);

    // Save & Feedback Row
    let save_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    save_row.set_margin_top(8);

    let save_btn = gtk4::Button::with_label("Save Hotspot Configuration");
    save_btn.add_css_class("hyprdrop-primary");

    let save_feedback = gtk4::Label::new(Some("✓ All changes apply live to your active desktop hotspot"));
    save_feedback.add_css_class("badge-success");

    let cfg_save = cfg.clone();
    save_btn.connect_clicked(move |_| {
        let _ = ConfigManager::save_config(&cfg_save.borrow());
        let _ = IpcClient::send(&IpcMessage::ReloadHotspot);
    });

    save_row.append(&save_btn);
    save_row.append(&save_feedback);
    content_box.append(&save_row);

    scrolled.set_child(Some(&content_box));
    box_view.append(&scrolled);
    box_view
}

// ============================================================================
// 3. APPEARANCE & THEMES TAB
// ============================================================================
struct ThemePreset {
    name: &'static str,
    desc: &'static str,
    base: &'static str,
    hover: &'static str,
    accent: &'static str,
    css_class: &'static str,
}

const THEME_PRESETS: &[ThemePreset] = &[
    ThemePreset {
        name: "Electric Cyan",
        desc: "Signature Wayland electric cyan with razor luminescence",
        base: "#00D2FF",
        hover: "#38BDF8",
        accent: "#0284C7",
        css_class: "swatch-cyan",
    },
    ThemePreset {
        name: "Tokyo Night",
        desc: "Iconic Japanese neon nightscape palette with violet and deep indigo",
        base: "#BB9AF7",
        hover: "#C0CAF5",
        accent: "#7AA2F7",
        css_class: "swatch-purple",
    },
    ThemePreset {
        name: "Catppuccin Mocha",
        desc: "Comforting pastel lavender, mauve and flamingo for cozy dark setups",
        base: "#89B4FA",
        hover: "#CBA6F7",
        accent: "#F5C2E7",
        css_class: "swatch-cyan",
    },
    ThemePreset {
        name: "Emerald Matrix",
        desc: "Crisp cybernetic emerald green optimized for laser focus",
        base: "#10B981",
        hover: "#34D399",
        accent: "#059669",
        css_class: "swatch-emerald",
    },
    ThemePreset {
        name: "Solar Ember",
        desc: "Warm amber and intense solar flares for warm workspace desk setups",
        base: "#F59E0B",
        hover: "#FBBF24",
        accent: "#EA580C",
        css_class: "swatch-amber",
    },
    ThemePreset {
        name: "Monochrome Stealth",
        desc: "Pure minimalist industrial monochrome for distraction-free workflows",
        base: "#E2E8F0",
        hover: "#FFFFFF",
        accent: "#64748B",
        css_class: "swatch-white",
    },
];

fn build_themes_tab() -> gtk4::Box {
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

    // Header Card
    let header_card = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    header_card.add_css_class("hyprdrop-card");

    let t_title = gtk4::Label::new(Some("Curated Wayland Color Themes"));
    t_title.add_css_class("hyprdrop-title");
    t_title.set_halign(gtk4::Align::Start);
    header_card.append(&t_title);

    let t_desc = gtk4::Label::new(Some(
        "Instantly coordinate color themes across your edge drop hotspot and radial wheel overlays.",
    ));
    t_desc.add_css_class("hyprdrop-page-desc");
    t_desc.set_halign(gtk4::Align::Start);
    header_card.append(&t_desc);
    content_box.append(&header_card);

    let feedback_lbl = gtk4::Label::new(None);
    feedback_lbl.add_css_class("badge-success");
    feedback_lbl.set_visible(false);

    // Theme Preset Cards
    for preset in THEME_PRESETS {
        let card = gtk4::Box::new(gtk4::Orientation::Horizontal, 14);
        card.add_css_class("hyprdrop-theme-card");

        // Swatch Preview Pills
        let pills_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        pills_box.set_valign(gtk4::Align::Center);

        for &c_class in &[preset.css_class, "swatch-white", "swatch-cyan"] {
            let pill = gtk4::Label::new(None);
            pill.add_css_class("hyprdrop-theme-pill");
            pill.add_css_class(c_class);
            pills_box.append(&pill);
        }
        card.append(&pills_box);

        let info_col = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        info_col.set_hexpand(true);

        let name_lbl = gtk4::Label::new(Some(preset.name));
        name_lbl.add_css_class("hyprdrop-title");
        name_lbl.set_halign(gtk4::Align::Start);
        info_col.append(&name_lbl);

        let desc_lbl = gtk4::Label::new(Some(preset.desc));
        desc_lbl.add_css_class("hyprdrop-page-desc");
        desc_lbl.set_halign(gtk4::Align::Start);
        info_col.append(&desc_lbl);
        card.append(&info_col);

        let apply_btn = gtk4::Button::with_label("Apply Theme");
        apply_btn.add_css_class("hyprdrop-secondary");
        apply_btn.add_css_class("hyprdrop-btn-sm");
        apply_btn.set_valign(gtk4::Align::Center);

        let b = preset.base.to_string();
        let h = preset.hover.to_string();
        let a = preset.accent.to_string();
        let name = preset.name.to_string();
        let fb = feedback_lbl.clone();

        apply_btn.connect_clicked(move |_| {
            let mut cfg = ConfigManager::load_config();
            cfg.hotspot_color = b.clone();
            cfg.hotspot_hover_color = h.clone();
            cfg.hotspot_accent_color = a.clone();
            let _ = ConfigManager::save_config(&cfg);
            let _ = IpcClient::send(&IpcMessage::ReloadHotspot);

            fb.set_text(&format!("✓ Applied '{}' theme live", name));
            fb.set_visible(true);
        });

        card.append(&apply_btn);
        content_box.append(&card);
    }

    content_box.append(&feedback_lbl);

    scrolled.set_child(Some(&content_box));
    box_view.append(&scrolled);
    box_view
}

// ============================================================================
// 4. PREFERENCES TAB
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
// 5. HYPRLAND & HEALTH CONSOLIDATED TAB
// ============================================================================
fn build_hyprland_and_health_tab() -> gtk4::Box {
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

    // SECTION 1: HYPRLAND CONFIGURATION TARGET
    let hl_section_label = gtk4::Label::new(Some("HYPRLAND WORKSPACE INTEGRATION"));
    hl_section_label.add_css_class("hyprdrop-section-label");
    hl_section_label.set_halign(gtk4::Align::Start);
    content_box.append(&hl_section_label);

    let (is_conf, config_path, format) = get_hyprland_config_info();
    let file_name = config_path.file_name().and_then(|n| n.to_str()).unwrap_or("hyprland.lua");

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

    let snippet_text = match format {
        crate::hyprland::installer::ConfigFormat::Lua => LUA_CONFIG_SNIPPET,
        crate::hyprland::installer::ConfigFormat::Conf => CONF_CONFIG_SNIPPET,
    };

    let snippet_view = gtk4::TextView::new();
    snippet_view.set_editable(false);
    snippet_view.set_cursor_visible(false);
    snippet_view.add_css_class("hyprdrop-code-preview");
    snippet_view.buffer().set_text(snippet_text);
    snippet_view.set_size_request(-1, 140);

    let snip_scrolled = gtk4::ScrolledWindow::builder()
        .child(&snippet_view)
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .build();
    snip_scrolled.set_size_request(-1, 140);

    snippet_card.append(&snip_scrolled);
    content_box.append(&snippet_card);

    // SECTION 2: SYSTEM HEALTH DIAGNOSTICS
    let sys_section_label = gtk4::Label::new(Some("COMPOSITOR & MEDIA ENGINE HEALTH"));
    sys_section_label.add_css_class("hyprdrop-section-label");
    sys_section_label.set_halign(gtk4::Align::Start);
    sys_section_label.set_margin_top(8);
    content_box.append(&sys_section_label);

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

    let core_list = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    for (cmd, desc, _) in tools {
        append_tool_card(&core_list, cmd, desc);
    }
    content_box.append(&core_list);

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
// HELPER FUNCTIONS & DRAWING ROUTINES
// ============================================================================
fn create_slider_row(
    title: &str,
    desc: &str,
    min: f64,
    max: f64,
    step: f64,
    initial: f64,
    suffix: &str,
) -> (gtk4::Box, gtk4::Scale, gtk4::Label) {
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    let text_col = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    text_col.set_hexpand(true);

    let t_lbl = gtk4::Label::new(Some(title));
    t_lbl.add_css_class("hyprdrop-title-sm");
    t_lbl.set_halign(gtk4::Align::Start);
    text_col.append(&t_lbl);

    let d_lbl = gtk4::Label::new(Some(desc));
    d_lbl.add_css_class("hyprdrop-page-desc");
    d_lbl.set_halign(gtk4::Align::Start);
    text_col.append(&d_lbl);
    row.append(&text_col);

    let val_str = if step.fract() == 0.0 {
        format!("{:.0}{}", initial, suffix)
    } else {
        format!("{:.1}{}", initial, suffix)
    };
    let val_lbl = gtk4::Label::new(Some(&val_str));
    val_lbl.add_css_class("hyprdrop-mono-badge");

    let scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, min, max, step);
    scale.set_value(initial);
    scale.set_size_request(160, -1);

    let val_clone = val_lbl.clone();
    let sfx = suffix.to_string();
    scale.connect_value_changed(move |s| {
        let v = s.value();
        if step.fract() == 0.0 {
            val_clone.set_text(&format!("{:.0}{}", v, sfx));
        } else {
            val_clone.set_text(&format!("{:.1}{}", v, sfx));
        }
    });

    row.append(&scale);
    row.append(&val_lbl);
    (row, scale, val_lbl)
}

fn create_swatch_color_row(
    title: &str,
    desc: &str,
    initial_hex: &str,
    swatches: &[(&'static str, &'static str)],
) -> (gtk4::Box, gtk4::Entry) {
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    let text_col = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    text_col.set_hexpand(true);

    let t_lbl = gtk4::Label::new(Some(title));
    t_lbl.add_css_class("hyprdrop-title-sm");
    t_lbl.set_halign(gtk4::Align::Start);
    text_col.append(&t_lbl);

    let d_lbl = gtk4::Label::new(Some(desc));
    d_lbl.add_css_class("hyprdrop-page-desc");
    d_lbl.set_halign(gtk4::Align::Start);
    text_col.append(&d_lbl);
    row.append(&text_col);

    let entry = gtk4::Entry::new();
    entry.set_text(initial_hex);
    entry.set_size_request(100, -1);
    row.append(&entry);

    let swatch_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    swatch_box.set_valign(gtk4::Align::Center);
    for (hex, css_cls) in swatches {
        let btn = gtk4::Button::new();
        btn.add_css_class("hyprdrop-swatch-circle");
        btn.add_css_class(css_cls);
        btn.set_tooltip_text(Some(hex));

        let entry_clone = entry.clone();
        let h_str = hex.to_string();
        btn.connect_clicked(move |_| {
            entry_clone.set_text(&h_str);
        });
        swatch_box.append(&btn);
    }
    row.append(&swatch_box);

    (row, entry)
}

fn draw_mini_monitor(
    cr: &Context,
    width: f64,
    height: f64,
    cfg: &AppConfig,
) {
    use std::f64::consts::PI;
    let _ = cr.save();

    // 1. Draw Monitor Bezel
    let bezel_pad_x = 18.0;
    let bezel_pad_y = 12.0;
    let bezel_w = width - bezel_pad_x * 2.0;
    let bezel_h = height - bezel_pad_y * 2.0 - 10.0;

    // Monitor stand
    let stand_cx = width / 2.0;
    let stand_y0 = bezel_pad_y + bezel_h;
    cr.set_source_rgba(0.12, 0.14, 0.20, 0.95);
    cr.rectangle(stand_cx - 14.0, stand_y0, 28.0, 8.0);
    let _ = cr.fill();
    // Stand foot
    cr.rectangle(stand_cx - 36.0, stand_y0 + 6.0, 72.0, 4.0);
    let _ = cr.fill();

    // Outer bezel
    draw_rounded_rectangle(cr, bezel_pad_x, bezel_pad_y, bezel_w, bezel_h, 10.0);
    cr.set_source_rgba(0.09, 0.10, 0.14, 0.98);
    let _ = cr.fill_preserve();
    cr.set_source_rgba(0.20, 0.23, 0.32, 0.85);
    cr.set_line_width(1.5);
    let _ = cr.stroke();

    // Chin indicator LED
    cr.arc(stand_cx, bezel_pad_y + bezel_h - 4.0, 1.8, 0.0, 2.0 * PI);
    cr.set_source_rgba(0.0, 0.82, 1.0, 0.85);
    let _ = cr.fill();

    // 2. Screen Area (Desktop interior)
    let scr_inset = 6.0;
    let sx = bezel_pad_x + scr_inset;
    let sy = bezel_pad_y + scr_inset;
    let sw = bezel_w - scr_inset * 2.0;
    let sh = bezel_h - scr_inset * 2.0 - 6.0;

    draw_rounded_rectangle(cr, sx, sy, sw, sh, 4.0);
    cr.set_source_rgba(0.04, 0.05, 0.08, 1.0);
    let _ = cr.fill_preserve();
    cr.set_source_rgba(0.15, 0.18, 0.26, 0.7);
    cr.set_line_width(1.0);
    let _ = cr.stroke();

    // Perimeter Guide (Dashed line)
    let _ = cr.save();
    let dashes = [4.0, 4.0];
    cr.set_dash(&dashes, 0.0);
    cr.set_source_rgba(0.25, 0.30, 0.42, 0.35);
    cr.set_line_width(1.0);
    cr.rectangle(sx + 2.0, sy + 2.0, sw - 4.0, sh - 4.0);
    let _ = cr.stroke();
    let _ = cr.restore();

    // Center watermark
    cr.set_source_rgba(0.40, 0.46, 0.60, 0.25);
    cr.select_font_face("Sans", gtk4::cairo::FontSlant::Normal, gtk4::cairo::FontWeight::Bold);
    cr.set_font_size(10.0);
    let text = "DESKTOP SCREEN PREVIEW";
    if let Ok(extents) = cr.text_extents(text) {
        cr.move_to(sx + (sw - extents.width()) / 2.0, sy + (sh + extents.height()) / 2.0);
        let _ = cr.show_text(text);
    }

    // 3. Hotspot Span & Drawing
    if cfg.hotspot_enabled {
        let math = PerimeterMath::new(sw, sh);
        let s = math.t_to_s(cfg.hotspot_perimeter_pos);
        let scaled_len = (cfg.hotspot_length as f64 / 1920.0 * (sw + sh)).clamp(28.0, 140.0);
        let scaled_thick = (cfg.hotspot_thickness as f64 * 0.75).clamp(4.0, 14.0);
        let scaled_margin = (cfg.hotspot_margin as f64 * 0.25).clamp(0.0, 8.0);
        let scaled_radius = (cfg.hotspot_corner_radius as f64 * 0.6).clamp(2.0, 6.0);

        let span = math.calculate_span(s, scaled_len);
        let (r, g, b, _) = parse_hex_color(&cfg.hotspot_color, 0.95);
        let center_coord = math.edge_center_coord(&span);

        match span.edge {
            ScreenEdge::Top => {
                let rx = sx + center_coord - span.length / 2.0;
                let ry = sy + scaled_margin;
                draw_rounded_rectangle(cr, rx, ry, span.length, scaled_thick, scaled_radius);
                fill_and_glow(cr, r, g, b, cfg.hotspot_locked);
            }
            ScreenEdge::Bottom => {
                let rx = sx + center_coord - span.length / 2.0;
                let ry = sy + sh - scaled_margin - scaled_thick;
                draw_rounded_rectangle(cr, rx, ry, span.length, scaled_thick, scaled_radius);
                fill_and_glow(cr, r, g, b, cfg.hotspot_locked);
            }
            ScreenEdge::Left => {
                let rx = sx + scaled_margin;
                let ry = sy + center_coord - span.length / 2.0;
                draw_rounded_rectangle(cr, rx, ry, scaled_thick, span.length, scaled_radius);
                fill_and_glow(cr, r, g, b, cfg.hotspot_locked);
            }
            ScreenEdge::Right => {
                let rx = sx + sw - scaled_margin - scaled_thick;
                let ry = sy + center_coord - span.length / 2.0;
                draw_rounded_rectangle(cr, rx, ry, scaled_thick, span.length, scaled_radius);
                fill_and_glow(cr, r, g, b, cfg.hotspot_locked);
            }
            ScreenEdge::CornerTopRight { len_top, len_right } => {
                draw_mini_corner_l(cr, sx + sw - scaled_margin, sy + scaled_margin, len_top, len_right, scaled_thick, scaled_radius, true, false);
                fill_and_glow(cr, r, g, b, cfg.hotspot_locked);
            }
            ScreenEdge::CornerBottomRight { len_right, len_bottom } => {
                draw_mini_corner_l(cr, sx + sw - scaled_margin, sy + sh - scaled_margin, len_bottom, len_right, scaled_thick, scaled_radius, true, true);
                fill_and_glow(cr, r, g, b, cfg.hotspot_locked);
            }
            ScreenEdge::CornerBottomLeft { len_bottom, len_left } => {
                draw_mini_corner_l(cr, sx + scaled_margin, sy + sh - scaled_margin, len_bottom, len_left, scaled_thick, scaled_radius, false, true);
                fill_and_glow(cr, r, g, b, cfg.hotspot_locked);
            }
            ScreenEdge::CornerTopLeft { len_left, len_top } => {
                draw_mini_corner_l(cr, sx + scaled_margin, sy + scaled_margin, len_top, len_left, scaled_thick, scaled_radius, false, false);
                fill_and_glow(cr, r, g, b, cfg.hotspot_locked);
            }
        }
    }

    let _ = cr.restore();
}

fn draw_rounded_rectangle(cr: &Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    use std::f64::consts::PI;
    let r = r.min(w / 2.0).min(h / 2.0);
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -PI / 2.0, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
    cr.arc(x + r, y + h - r, r, PI / 2.0, PI);
    cr.arc(x + r, y + r, r, PI, 3.0 * PI / 2.0);
    cr.close_path();
}

fn fill_and_glow(cr: &Context, r: f64, g: f64, b: f64, _locked: bool) {
    cr.set_source_rgba(r, g, b, 0.95);
    let _ = cr.fill_preserve();
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.40);
    cr.set_line_width(1.0);
    let _ = cr.stroke();
}

fn draw_mini_corner_l(
    cr: &Context,
    cx: f64,
    cy: f64,
    len_h: f64,
    len_v: f64,
    thick: f64,
    radius: f64,
    is_right: bool,
    is_bottom: bool,
) {
    use std::f64::consts::PI;
    let r = radius.min(thick / 2.0);
    cr.new_sub_path();

    if is_right && !is_bottom {
        // Top-Right: cx is right edge, cy is top edge
        cr.move_to(cx - len_h, cy + thick);
        cr.line_to(cx - len_h, cy);
        cr.arc(cx - r, cy + r, r, -PI / 2.0, 0.0);
        cr.line_to(cx, cy + len_v);
        cr.line_to(cx - thick, cy + len_v);
        cr.line_to(cx - thick, cy + thick);
        cr.close_path();
    } else if is_right && is_bottom {
        // Bottom-Right: cx is right edge, cy is bottom edge
        cr.move_to(cx, cy - len_v);
        cr.arc(cx - r, cy - r, r, 0.0, PI / 2.0);
        cr.line_to(cx - len_h, cy);
        cr.line_to(cx - len_h, cy - thick);
        cr.line_to(cx - thick, cy - thick);
        cr.line_to(cx - thick, cy - len_v);
        cr.close_path();
    } else if !is_right && is_bottom {
        // Bottom-Left: cx is left edge, cy is bottom edge
        cr.move_to(cx, cy - len_v);
        cr.line_to(cx + thick, cy - len_v);
        cr.line_to(cx + thick, cy - thick);
        cr.line_to(cx + len_h, cy - thick);
        cr.line_to(cx + len_h, cy);
        cr.arc(cx + r, cy - r, r, PI / 2.0, PI);
        cr.close_path();
    } else {
        // Top-Left: cx is left edge, cy is top edge
        cr.move_to(cx, cy + len_v);
        cr.arc(cx + r, cy + r, r, PI, 3.0 * PI / 2.0);
        cr.line_to(cx + len_h, cy);
        cr.line_to(cx + len_h, cy + thick);
        cr.line_to(cx + thick, cy + thick);
        cr.line_to(cx + thick, cy + len_v);
        cr.close_path();
    }
}
