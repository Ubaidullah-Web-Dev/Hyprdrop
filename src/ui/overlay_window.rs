use std::cell::RefCell;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;
use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use crate::config::ConfigManager;
use crate::engine::compress::compress_file;
use crate::engine::ocr_clipboard::{copy_image_to_clipboard, ocr_image_and_copy};
use crate::engine::{
    FileCategory, execute_conversion_batch, get_conversion_slices, get_quick_action_slices,
};
use crate::hyprland::layer_shell::configure_fullscreen_layer_shell_overlay;
use crate::ui::drop_shelf::DropShelf;
use crate::ui::modals::{
    open_add_bg_modal, open_archive_view_modal, open_crop_modal, open_metadata_modal, open_quick_edit_modal,
    open_redact_annotate_modal,
};
use crate::ui::radial_menu::{
    HitZone, INNER_RADIUS, OUTER_RADIUS, RADIAL_SIZE, RadialMenu, RadialMode, get_slice_support_status,
    hit_test,
};

thread_local! {
    static ACTIVE_OVERLAY: RefCell<Option<gtk4::ApplicationWindow>> = const { RefCell::new(None) };
}

pub struct OverlayWindow;

impl OverlayWindow {
    /// Checks whether an overlay window is currently open.
    pub fn is_open(_app: &gtk4::Application) -> bool {
        ACTIVE_OVERLAY.with(|cell| cell.borrow().is_some())
    }

    /// Closes all active overlay windows.
    pub fn close_all(app: &gtk4::Application) {
        let win_opt = ACTIVE_OVERLAY.with(|cell| cell.borrow_mut().take());
        if let Some(win) = win_opt {
            win.close();
        }
        for w in app.windows() {
            if w.has_css_class("hyprdrop-overlay") || w.title().as_deref() == Some("Hyprdrop Overlay") {
                w.close();
            }
        }
    }

    /// Toggles the overlay window: if open, closes it; if closed, spawns it.
    pub fn toggle(app: &gtk4::Application, preloaded_file: Option<PathBuf>) {
        if Self::is_open(app) {
            Self::close_all(app);
        } else {
            Self::spawn(app, preloaded_file);
        }
    }

    /// Spawns the overlay window with an optional single preloaded file.
    pub fn spawn(app: &gtk4::Application, preloaded_file: Option<PathBuf>) {
        let files = preloaded_file
            .map(|f| vec![f])
            .unwrap_or_else(detect_selected_files);
        Self::spawn_with_files(app, files);
    }

    /// Spawns the overlay window with a list of preloaded files (multi-file drop support).
    pub fn spawn_with_files(app: &gtk4::Application, preloaded_files: Vec<PathBuf>) {
        Self::close_all(app);

        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("Hyprdrop Overlay")
            .resizable(false)
            .decorated(false)
            .build();

        window.add_css_class("hyprdrop-overlay");
        let (pos_x, pos_y) = configure_fullscreen_layer_shell_overlay(&window, RADIAL_SIZE, RADIAL_SIZE, None);

        let radial = Rc::new(RadialMenu::new_with_files(preloaded_files));

        let fixed = gtk4::Fixed::new();
        fixed.set_hexpand(true);
        fixed.set_vexpand(true);
        fixed.add_css_class("hyprdrop-overlay-fixed");
        fixed.put(&radial.widget, pos_x, pos_y);

        // Click Gesture on the fullscreen overlay window - clicking anywhere outside wheel dismisses immediately
        let click = gtk4::GestureClick::new();
        click.set_button(0); // All buttons (left, right, middle)

        let radial_click = radial.clone();
        let win_click = window.clone();
        let app_click = app.clone();

        click.connect_pressed(move |gesture, _, x, y| {
            let cx = pos_x + (RADIAL_SIZE as f64) / 2.0;
            let cy = pos_y + (RADIAL_SIZE as f64) / 2.0;

            let s = radial_click.state.borrow();
            let slices = match s.mode {
                RadialMode::FormatConversion => get_conversion_slices(s.file_category),
                RadialMode::QuickActions => get_quick_action_slices(s.file_category),
            };

            let hit = hit_test(x, y, cx, cy, INNER_RADIUS, OUTER_RADIUS, slices.len());

            let button = gesture.current_button();
            if button == gdk::BUTTON_SECONDARY {
                match hit {
                    HitZone::CenterHub | HitZone::Slice(_) => {
                        drop(s);
                        radial_click.toggle_mode();
                    }
                    HitZone::Outside => {
                        drop(s);
                        win_click.close();
                    }
                }
                return;
            }

            match hit {
                HitZone::CenterHub => {
                    let files = s.current_files.clone();
                    drop(s);
                    if files.is_empty() {
                        prompt_file_for_hub(&win_click, &radial_click);
                    } else {
                        radial_click.toggle_mode();
                    }
                }
                HitZone::Slice(idx) => {
                    if idx < slices.len() {
                        let slice_name = slices[idx];
                        let ext = s.current_files.first()
                            .and_then(|p| p.extension())
                            .and_then(|e| e.to_str());
                        let (is_enabled, _) = get_slice_support_status(s.file_category, s.mode, slice_name, ext);
                        if !is_enabled {
                            drop(s);
                            return;
                        }

                        let files = s.current_files.clone();
                        let current_mode = s.mode;
                        drop(s);

                        if !files.is_empty() {
                            handle_slice_activation(
                                &win_click,
                                &app_click,
                                &files,
                                slice_name,
                                current_mode,
                            );
                        } else {
                            // File not preloaded: prompt user to pick file for this slice!
                            prompt_file_for_slice(
                                &win_click,
                                &app_click,
                                &radial_click,
                                slice_name,
                                current_mode,
                            );
                        }
                    }
                }
                HitZone::Outside => {
                    // Click outside dismisses immediately
                    drop(s);
                    win_click.close();
                }
            }
        });
        window.add_controller(click);

        // Drop Target for files dragged into the radial overlay
        let drop_target = gtk4::DropTarget::new(
            glib::types::Type::INVALID,
            gdk::DragAction::COPY | gdk::DragAction::MOVE,
        );
        drop_target.set_preload(true);
        drop_target.set_types(&[
            gdk::FileList::static_type(),
            gio::File::static_type(),
            glib::types::Type::STRING,
        ]);

        drop_target.connect_enter(move |_, _x, _y| {
            gdk::DragAction::COPY
        });

        let radial_motion2 = radial.clone();
        drop_target.connect_motion(move |_, x, y| {
            let mut s = radial_motion2.state.borrow_mut();
            let slices = match s.mode {
                RadialMode::FormatConversion => get_conversion_slices(s.file_category),
                RadialMode::QuickActions => get_quick_action_slices(s.file_category),
            };
            let cx = (RADIAL_SIZE as f64) / 2.0;
            let cy = (RADIAL_SIZE as f64) / 2.0;
            let zone = hit_test(x, y, cx, cy, INNER_RADIUS, OUTER_RADIUS, slices.len());
            if s.hovered_zone != zone {
                s.hovered_zone = zone;
                drop(s);
                radial_motion2.widget.queue_draw();
            }
            gdk::DragAction::COPY
        });

        let radial_drop = radial.clone();
        let win_drop = window.clone();
        let app_drop = app.clone();

        drop_target.connect_drop(move |_, val, _, _| {
            let paths = extract_paths_from_drop_value(val);
            if paths.is_empty() {
                return false;
            }

            let s = radial_drop.state.borrow();
            let slices = match s.mode {
                RadialMode::FormatConversion => get_conversion_slices(s.file_category),
                RadialMode::QuickActions => get_quick_action_slices(s.file_category),
            };

            match s.hovered_zone {
                HitZone::Slice(idx) if idx < slices.len() => {
                    let slice_name = slices[idx];
                    let mode = s.mode;
                    let cat = FileCategory::from_path(&paths[0]);
                    let ext = paths[0].extension().and_then(|e| e.to_str());
                    let (is_enabled, _) = get_slice_support_status(cat, mode, slice_name, ext);
                    drop(s);
                    if is_enabled {
                        handle_slice_activation(&win_drop, &app_drop, &paths, slice_name, mode);
                    } else {
                        radial_drop.set_files(paths);
                    }
                    true
                }
                _ => {
                    drop(s);
                    // Load all dropped files into wheel
                    radial_drop.set_files(paths);
                    true
                }
            }
        });
        radial.widget.add_controller(drop_target);

        // Keyboard Controls
        let key_controller = gtk4::EventControllerKey::new();
        let win_key = window.clone();
        let radial_key = radial.clone();
        let app_key = app.clone();

        key_controller.connect_key_pressed(move |_, keyval, _, _| {
            match keyval {
                gdk::Key::Escape => {
                    win_key.close();
                    glib::Propagation::Stop
                }
                gdk::Key::Tab | gdk::Key::space => {
                    radial_key.toggle_mode();
                    glib::Propagation::Stop
                }
                gdk::Key::_1 | gdk::Key::_2 | gdk::Key::_3 | gdk::Key::_4 |
                gdk::Key::_5 | gdk::Key::_6 | gdk::Key::_7 | gdk::Key::_8 => {
                    let num = match keyval {
                        gdk::Key::_1 => 0,
                        gdk::Key::_2 => 1,
                        gdk::Key::_3 => 2,
                        gdk::Key::_4 => 3,
                        gdk::Key::_5 => 4,
                        gdk::Key::_6 => 5,
                        gdk::Key::_7 => 6,
                        _ => 7,
                    };

                    let s = radial_key.state.borrow();
                    let slices = match s.mode {
                        RadialMode::FormatConversion => get_conversion_slices(s.file_category),
                        RadialMode::QuickActions => get_quick_action_slices(s.file_category),
                    };

                    if num < slices.len() {
                        let slice_name = slices[num];
                        let ext = s.current_files.first()
                            .and_then(|p| p.extension())
                            .and_then(|e| e.to_str());
                        let (is_enabled, _) = get_slice_support_status(s.file_category, s.mode, slice_name, ext);
                        if !is_enabled {
                            drop(s);
                            return glib::Propagation::Stop;
                        }

                        let files = s.current_files.clone();
                        let mode = s.mode;
                        drop(s);

                        if !files.is_empty() {
                            handle_slice_activation(&win_key, &app_key, &files, slice_name, mode);
                        }
                    }
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
        window.add_controller(key_controller);

        window.set_child(Some(&fixed));

        // Background Drop Target on fullscreen overlay to catch files dropped anywhere
        let bg_drop_target = gtk4::DropTarget::new(
            glib::types::Type::INVALID,
            gdk::DragAction::COPY | gdk::DragAction::MOVE,
        );
        bg_drop_target.set_preload(true);
        bg_drop_target.set_types(&[
            gdk::FileList::static_type(),
            gio::File::static_type(),
            glib::types::Type::STRING,
        ]);
        let radial_bg_drop = radial.clone();
        bg_drop_target.connect_drop(move |_, val, _, _| {
            let paths = extract_paths_from_drop_value(val);
            if !paths.is_empty() {
                radial_bg_drop.set_files(paths);
                true
            } else {
                false
            }
        });
        window.add_controller(bg_drop_target);

        let win_weak = window.downgrade();
        window.connect_destroy(move |_| {
            ACTIVE_OVERLAY.with(|cell| {
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
            crate::engine::trim_memory();
        });

        ACTIVE_OVERLAY.with(|cell| {
            *cell.borrow_mut() = Some(window.clone());
        });

        window.present();
    }
}

/// Auto-detects selected files from active file manager or clipboard without sending synthetic keyboard events.
pub fn detect_selected_files() -> Vec<PathBuf> {
    let mut files = Vec::new();

    // 1. Try querying the active file manager (e.g. Thunar, Nautilus, Nemo) selection via accessibility bus
    let helper_cmd = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("hyprdrop-detect-selection")))
        .filter(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string())
        .or_else(|| {
            dirs::home_dir()
                .map(|h| h.join(".local/bin/hyprdrop-detect-selection"))
                .filter(|p| p.exists())
                .map(|p| p.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "hyprdrop-detect-selection".to_string());

    if let Ok(output) = Command::new("timeout")
        .args(["0.5s", &helper_cmd])
        .output()
        && output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let p = PathBuf::from(trimmed);
                if p.exists() {
                    files.push(p);
                }
            }
        }
        if !files.is_empty() {
            return files;
        }
    }

    // Helper to resolve a trimmed URI or path into an existing PathBuf
    let resolve_file = |trimmed: &str| -> Option<PathBuf> {
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        if let Ok((p, _)) = glib::filename_from_uri(trimmed) {
            if p.exists() {
                return Some(p);
            }
        } else if let Some(stripped) = trimmed.strip_prefix("file://") {
            let p = PathBuf::from(stripped);
            if p.exists() {
                return Some(p);
            }
        } else {
            let p = PathBuf::from(trimmed);
            if p.is_absolute() && p.exists() {
                return Some(p);
            }
        }
        None
    };

    // 2. Try to read text/uri-list from standard clipboard (standard file manager copy format) with strict timeout
    if let Ok(output) = Command::new("timeout")
        .args(["0.15s", "wl-paste", "-t", "text/uri-list"])
        .output()
        && output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if let Some(p) = resolve_file(line.trim()) {
                files.push(p);
            }
        }
        if !files.is_empty() {
            return files;
        }
    }

    // 3. Try to read x-special/gnome-copied-files (GNOME/Nautilus/Thunar format) with strict timeout
    if let Ok(output) = Command::new("timeout")
        .args(["0.15s", "wl-paste", "-t", "x-special/gnome-copied-files"])
        .output()
        && output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed == "copy" || trimmed == "cut" {
                continue;
            }
            if let Some(p) = resolve_file(trimmed) {
                files.push(p);
            }
        }
        if !files.is_empty() {
            return files;
        }
    }

    Vec::new()
}

/// Auto-detects a single selected file.
pub fn detect_selected_file() -> Option<PathBuf> {
    detect_selected_files().into_iter().next()
}

/// Robustly extracts local filesystem paths from a GTK DropTarget value.
pub fn extract_paths_from_drop_value(val: &glib::Value) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // 1. Check gdk::FileList
    if let Ok(file_list) = val.get::<gdk::FileList>() {
        for f in file_list.files() {
            if let Some(p) = f.path()
                && p.exists() {
                    paths.push(p);
                }
        }
        if !paths.is_empty() {
            return paths;
        }
    }

    // 2. Check gio::File
    if let Ok(file) = val.get::<gio::File>()
        && let Some(p) = file.path()
            && p.exists() {
                paths.push(p);
                return paths;
            }

    // 3. Check String / GString (text/uri-list)
    let text = if let Ok(s) = val.get::<glib::GString>() {
        Some(s.to_string())
    } else {
        val.get::<String>().ok()
    };

    if let Some(text) = text {
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Ok((p, _)) = glib::filename_from_uri(trimmed) {
                if p.exists() {
                    paths.push(p);
                }
            } else if let Some(stripped) = trimmed.strip_prefix("file://") {
                let p = PathBuf::from(stripped);
                if p.exists() {
                    paths.push(p);
                }
            } else {
                let p = PathBuf::from(trimmed);
                if p.exists() {
                    paths.push(p);
                }
            }
        }
    }

    paths
}

fn handle_slice_activation(
    window: &gtk4::ApplicationWindow,
    app: &gtk4::Application,
    files: &[PathBuf],
    slice_name: &str,
    mode: RadialMode,
) {
    if files.is_empty() {
        return;
    }

    let first_file = &files[0];
    let cat = FileCategory::from_path(first_file);
    let ext = first_file.extension().and_then(|e| e.to_str());
    let (is_enabled, _) = get_slice_support_status(cat, mode, slice_name, ext);
    if !is_enabled {
        return;
    }

    match mode {
        RadialMode::FormatConversion => {
            // Dismiss overlay window immediately
            window.close();

            let target_format = slice_name.to_lowercase();
            let files_to_convert = files.to_vec();

            // Run conversions in parallel background Rayon threads
            std::thread::spawn(move || {
                let results = execute_conversion_batch(&files_to_convert, &target_format);

                glib::idle_add_once(move || {
                    let mut created: Vec<PathBuf> = Vec::new();
                    for (src, res) in files_to_convert.iter().zip(results) {
                        match res {
                            Ok(out_path) => {
                                let _ = ConfigManager::append_history(
                                    &format!("CONVERT TO {}", target_format.to_uppercase()),
                                    src,
                                    &out_path,
                                    None,
                                );
                                created.push(out_path);
                            }
                            Err(e) => {
                                eprintln!("Conversion error for {:?}: {:?}", src, e);
                            }
                        }
                    }

                    // If drop shelf enabled, spawn drop shelf with first converted output
                    let cfg = ConfigManager::load_config();
                    if cfg.show_drop_shelf && !created.is_empty() {
                        let first = created[0].clone();
                        DropShelf::spawn(first, None, move |chained_file| {
                            if let Some(active_app) = gio::Application::default()
                                .and_then(|a| a.downcast::<gtk4::Application>().ok()) {
                                OverlayWindow::spawn(&active_app, Some(chained_file));
                            }
                        });
                    }
                });
            });
        }
        RadialMode::QuickActions => {
            match slice_name {
                "CROP" => {
                    let app_shelf = app.clone();
                    let file = first_file.clone();
                    window.close();
                    open_crop_modal(Some(app), &file, move |out_path| {
                        let app_reopen = app_shelf.clone();
                        DropShelf::spawn(out_path, None, move |chained_file| {
                            OverlayWindow::spawn(&app_reopen, Some(chained_file));
                        });
                    });
                }
                "ADD BG" => {
                    let app_shelf = app.clone();
                    let file = first_file.clone();
                    window.close();
                    open_add_bg_modal(Some(app), &file, move |out_path| {
                        let app_reopen = app_shelf.clone();
                        DropShelf::spawn(out_path, None, move |chained_file| {
                            OverlayWindow::spawn(&app_reopen, Some(chained_file));
                        });
                    });
                }
                "COMPRESS" => {
                    window.close();
                    let src = first_file.clone();
                    std::thread::spawn(move || {
                        let res = compress_file(&src);
                        glib::idle_add_once(move || {
                            if let Ok(info) = res {
                                let badge = format!("Saved {:.0}%", info.percent_saved);
                                let _ = ConfigManager::append_history(
                                    "COMPRESS",
                                    &src,
                                    &info.output_path,
                                    Some(info.percent_saved),
                                );
                                DropShelf::spawn(
                                    info.output_path,
                                    Some(badge),
                                    move |chained_file| {
                                        if let Some(active_app) = gio::Application::default()
                                            .and_then(|a| a.downcast::<gtk4::Application>().ok()) {
                                            OverlayWindow::spawn(&active_app, Some(chained_file));
                                        }
                                    },
                                );
                            }
                        });
                    });
                }
                "REDACT" => {
                    let app_shelf = app.clone();
                    let file = first_file.clone();
                    window.close();
                    open_redact_annotate_modal(Some(app), &file, move |out_path| {
                        let app_reopen = app_shelf.clone();
                        DropShelf::spawn(out_path, None, move |chained_file| {
                            OverlayWindow::spawn(&app_reopen, Some(chained_file));
                        });
                    });
                }
                "AUTO TRIM" => {
                    let src = first_file.clone();
                    window.close();
                    std::thread::spawn(move || {
                        if let Ok(img) = crate::engine::image_ops::load_image(&src) {
                            let rgba = img.to_rgba8();
                            let trimmed = crate::engine::image_ops::auto_trim_transparent(&rgba);
                            let out_path = crate::engine::naming::get_suffixed_output_path(&src, "Trimmed", Some("png"));
                            if trimmed.save(&out_path).is_ok() {
                                let _ = ConfigManager::append_history("AUTO TRIM", &src, &out_path, None);
                                glib::idle_add_once(move || {
                                    DropShelf::spawn(out_path, Some("Auto-Trimmed".to_string()), move |chained_file| {
                                        if let Some(active_app) = gio::Application::default()
                                            .and_then(|a| a.downcast::<gtk4::Application>().ok()) {
                                            OverlayWindow::spawn(&active_app, Some(chained_file));
                                        }
                                    });
                                });
                            }
                        }
                    });
                }
                "METADATA" => {
                    let app_shelf = app.clone();
                    let file = first_file.clone();
                    window.close();
                    open_metadata_modal(Some(app), &file, move |out_path| {
                        let app_reopen = app_shelf.clone();
                        DropShelf::spawn(out_path, None, move |chained_file| {
                            OverlayWindow::spawn(&app_reopen, Some(chained_file));
                        });
                    });
                }
                "EDIT" => {
                    let app_shelf = app.clone();
                    let file = first_file.clone();
                    window.close();
                    open_quick_edit_modal(Some(app), &file, move |out_path| {
                        let app_reopen = app_shelf.clone();
                        DropShelf::spawn(out_path, None, move |chained_file| {
                            OverlayWindow::spawn(&app_reopen, Some(chained_file));
                        });
                    });
                }
                "EXTRACT HERE" => {
                    let src = first_file.clone();
                    window.close();
                    std::thread::spawn(move || {
                        let parent = src.parent().unwrap_or_else(|| std::path::Path::new("."));
                        let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("extracted");
                        let dest = parent.join(stem);
                        let _ = std::fs::create_dir_all(&dest);
                        if let Ok(extracted) = crate::engine::archive_ops::extract_archive(&src, &dest) {
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
                }
                "EXTRACT TO..." => {
                    let src = first_file.clone();
                    window.close();
                    let dialog = gtk4::FileDialog::builder()
                        .title(format!("Extract {} to...", src.file_name().and_then(|n| n.to_str()).unwrap_or("Archive")))
                        .modal(true)
                        .build();
                    dialog.select_folder(gtk4::Window::NONE, gio::Cancellable::NONE, move |res| {
                        if let Ok(folder) = res {
                            if let Some(dest_dir) = folder.path() {
                                let src_bg = src.clone();
                                std::thread::spawn(move || {
                                    if let Ok(extracted) = crate::engine::archive_ops::extract_archive(&src_bg, &dest_dir) {
                                        let _ = ConfigManager::append_history("EXTRACT ARCHIVE", &src_bg, &extracted, None);
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
                            }
                        }
                    });
                }
                "LIST FILES" => {
                    let file = first_file.clone();
                    window.close();
                    open_archive_view_modal(Some(app), &file);
                }
                "TEST ARCHIVE" => {
                    let src = first_file.clone();
                    window.close();
                    std::thread::spawn(move || {
                        match crate::engine::archive_ops::test_archive_integrity(&src) {
                            Ok((count, bytes)) => {
                                let _ = std::process::Command::new("notify-send")
                                    .args([
                                        "-a", "Hyprdrop",
                                        "-i", "emblem-ok-symbolic",
                                        "Archive Test Passed",
                                        &format!("Verified {} entries ({}) without errors", count, crate::engine::format_file_size(bytes)),
                                    ])
                                    .spawn();
                            }
                            Err(e) => {
                                let _ = std::process::Command::new("notify-send")
                                    .args([
                                        "-a", "Hyprdrop",
                                        "-i", "dialog-error-symbolic",
                                        "Archive Test Failed",
                                        &format!("Errors detected: {}", e),
                                    ])
                                    .spawn();
                            }
                        }
                    });
                }
                "OCR / COPY" => {
                    window.close();
                    let src = first_file.clone();
                    std::thread::spawn(move || {
                        match ocr_image_and_copy(&src) {
                            Ok(text) => {
                                let preview = if text.chars().count() > 80 {
                                    format!("{}…", text.chars().take(80).collect::<String>())
                                } else {
                                    text.clone()
                                };
                                let _ = std::process::Command::new("notify-send")
                                    .args(["-a", "Hyprdrop", "-i", "edit-copy-symbolic", "OCR Text Copied to Clipboard", &preview])
                                    .spawn();
                            }
                            Err(_) => {
                                if copy_image_to_clipboard(&src).is_ok() {
                                    let _ = std::process::Command::new("notify-send")
                                        .args(["-a", "Hyprdrop", "-i", "image-x-generic-symbolic", "Image Copied to Clipboard", "Image copied with MIME image/png"])
                                        .spawn();
                                }
                            }
                        }
                    });
                }
                _ => {}
            }
        }
    }
}

fn prompt_file_for_slice(
    window: &gtk4::ApplicationWindow,
    app: &gtk4::Application,
    radial: &Rc<RadialMenu>,
    slice_name: &'static str,
    mode: RadialMode,
) {
    let dialog = gtk4::FileDialog::builder()
        .title(format!("Select File to {}", slice_name))
        .modal(true)
        .build();

    let win_clone = window.clone();
    let app_clone = app.clone();
    let radial_clone = radial.clone();

    dialog.open(Some(window), gio::Cancellable::NONE, move |res| {
        if let Ok(file) = res {
            if let Some(path) = file.path() {
                radial_clone.set_files(vec![path.clone()]);
                handle_slice_activation(&win_clone, &app_clone, &[path], slice_name, mode);
            }
        }
    });
}

fn prompt_file_for_hub(
    window: &gtk4::ApplicationWindow,
    radial: &Rc<RadialMenu>,
) {
    let dialog = gtk4::FileDialog::builder()
        .title("Select File for Hyprdrop")
        .modal(true)
        .build();

    let radial_clone = radial.clone();

    dialog.open(Some(window), gio::Cancellable::NONE, move |res| {
        if let Ok(file) = res {
            if let Some(path) = file.path() {
                radial_clone.set_files(vec![path]);
            }
        }
    });
}
