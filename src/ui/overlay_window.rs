use std::path::PathBuf;
use std::rc::Rc;
use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use crate::config::ConfigManager;
use crate::engine::compress::compress_file;
use crate::engine::ocr_clipboard::{copy_image_to_clipboard, ocr_image_and_copy};
use crate::engine::{
    execute_conversion_batch, get_conversion_slices, get_quick_action_slices,
};
use crate::hyprland::layer_shell::configure_layer_shell_window;
use crate::ui::drop_shelf::DropShelf;
use crate::ui::modals::{
    open_add_bg_modal, open_crop_modal, open_metadata_modal, open_quick_edit_modal,
    open_redact_annotate_modal,
};
use crate::ui::radial_menu::{HitZone, RADIAL_SIZE, RadialMenu, RadialMode};

pub struct OverlayWindow;

impl OverlayWindow {
    /// Checks whether an overlay window is currently open.
    pub fn is_open(app: &gtk4::Application) -> bool {
        app.windows().iter().any(|w| {
            w.has_css_class("hyprdrop-overlay") || w.title().as_deref() == Some("Hyprdrop Overlay")
        })
    }

    /// Closes all active overlay windows.
    pub fn close_all(app: &gtk4::Application) {
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
        let files = preloaded_file.map(|f| vec![f]).unwrap_or_default();
        Self::spawn_with_files(app, files);
    }

    /// Spawns the overlay window with a list of preloaded files (multi-file drop support).
    pub fn spawn_with_files(app: &gtk4::Application, preloaded_files: Vec<PathBuf>) {
        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("Hyprdrop Overlay")
            .default_width(RADIAL_SIZE)
            .default_height(RADIAL_SIZE)
            .resizable(false)
            .decorated(false)
            .build();

        window.add_css_class("hyprdrop-overlay");
        configure_layer_shell_window(&window, RADIAL_SIZE, RADIAL_SIZE, None);

        let radial = Rc::new(RadialMenu::new_with_files(preloaded_files));

        // Click Gesture on Radial Menu
        let click = gtk4::GestureClick::new();
        click.set_button(0); // All buttons (left, right, middle)

        let radial_click = radial.clone();
        let win_click = window.clone();
        let app_click = app.clone();

        click.connect_pressed(move |gesture, _, _, _| {
            let button = gesture.current_button();
            if button == gdk::BUTTON_SECONDARY {
                // Right-Click flips mode
                radial_click.toggle_mode();
                return;
            }

            let s = radial_click.state.borrow();
            match s.hovered_zone {
                HitZone::CenterHub => {
                    drop(s);
                    radial_click.toggle_mode();
                }
                HitZone::Slice(idx) => {
                    let slices = match s.mode {
                        RadialMode::FormatConversion => get_conversion_slices(s.file_category),
                        RadialMode::QuickActions => get_quick_action_slices(s.file_category),
                    };

                    if idx < slices.len() {
                        let slice_name = slices[idx];
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
                        }
                    }
                }
                HitZone::Outside => {
                    // Click outside dismisses
                    drop(s);
                    win_click.close();
                }
            }
        });
        radial.widget.add_controller(click);

        // Drop Target for files dragged into the radial overlay
        // Accepts gdk::FileList, gio::File, and text/uri-list strings
        let drop_target = gtk4::DropTarget::new(
            glib::types::Type::INVALID,
            gdk::DragAction::COPY | gdk::DragAction::MOVE,
        );
        drop_target.set_types(&[
            gdk::FileList::static_type(),
            gio::File::static_type(),
            glib::types::Type::STRING,
        ]);

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
                    drop(s);
                    handle_slice_activation(&win_drop, &app_drop, &paths, slice_name, mode);
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
        window.add_controller(drop_target);

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

        window.set_child(Some(&radial.widget));
        window.present();
    }
}

/// Robustly extracts local filesystem paths from a GTK DropTarget value.
/// Supports gdk::FileList, gio::File, and text/uri-list strings.
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
    _app: &gtk4::Application,
    source_files: &[PathBuf],
    slice_name: &str,
    mode: RadialMode,
) {
    if source_files.is_empty() {
        return;
    }

    match mode {
        RadialMode::FormatConversion => {
            window.close();
            let src_files = source_files.to_vec();
            let slice = slice_name.to_string();

            // Background parallel conversion using Rayon
            std::thread::spawn(move || {
                let results = execute_conversion_batch(&src_files, &slice);
                let mut successful_outputs = Vec::new();

                for (src, res) in src_files.iter().zip(results) {
                    match res {
                        Ok(out_path) => {
                            let _ = ConfigManager::append_history(
                                &format!("CONVERT: {}", slice),
                                src,
                                &out_path,
                                None,
                            );
                            successful_outputs.push(out_path);
                        }
                        Err(e) => {
                            eprintln!("Conversion error for {}: {:?}", src.display(), e);
                        }
                    }
                }

                if let Some(first_out) = successful_outputs.first().cloned() {
                    let badge = if successful_outputs.len() > 1 {
                        Some(format!("{} files converted", successful_outputs.len()))
                    } else {
                        None
                    };

                    glib::idle_add_once(move || {
                        DropShelf::spawn(first_out, badge, |chained_file| {
                            if let Some(app) = gio::Application::default()
                                .and_then(|a| a.downcast::<gtk4::Application>().ok())
                            {
                                OverlayWindow::spawn(&app, Some(chained_file));
                            }
                        });
                    });
                }
            });
        }
        RadialMode::QuickActions => {
            let primary_file = source_files[0].clone();

            let on_chain = |chained_file: PathBuf| {
                if let Some(app) = gio::Application::default()
                    .and_then(|a| a.downcast::<gtk4::Application>().ok())
                {
                    OverlayWindow::spawn(&app, Some(chained_file));
                }
            };

            match slice_name {
                "CROP" => {
                    window.close();
                    open_crop_modal(window, &primary_file, move |out_path| {
                        DropShelf::spawn(out_path, None, on_chain);
                    });
                }
                "ADD BG" => {
                    window.close();
                    open_add_bg_modal(window, &primary_file, move |out_path| {
                        DropShelf::spawn(out_path, None, on_chain);
                    });
                }
                "COMPRESS" => {
                    window.close();
                    let files_to_compress = source_files.to_vec();
                    std::thread::spawn(move || {
                        let mut last_output = None;
                        let mut total_saved = 0.0;
                        let count = files_to_compress.len();

                        for src in &files_to_compress {
                            if let Ok(res) = compress_file(src) {
                                total_saved += res.percent_saved;
                                let _ = ConfigManager::append_history(
                                    "COMPRESS",
                                    src,
                                    &res.output_path,
                                    Some(res.percent_saved),
                                );
                                last_output = Some(res.output_path);
                            }
                        }

                        if let Some(out_path) = last_output {
                            let avg_saved = if count > 0 { total_saved / count as f64 } else { 0.0 };
                            let badge = format!("Saved {:.0}%", avg_saved);
                            glib::idle_add_once(move || {
                                DropShelf::spawn(out_path, Some(badge), on_chain);
                            });
                        }
                    });
                }
                "REDACT" | "ANNOTATE" => {
                    window.close();
                    open_redact_annotate_modal(window, &primary_file, move |out_path| {
                        DropShelf::spawn(out_path, None, on_chain);
                    });
                }
                "METADATA" => {
                    window.close();
                    open_metadata_modal(window, &primary_file, move |out_path| {
                        DropShelf::spawn(out_path, None, on_chain);
                    });
                }
                "EDIT" => {
                    window.close();
                    open_quick_edit_modal(window, &primary_file, move |out_path| {
                        DropShelf::spawn(out_path, None, on_chain);
                    });
                }
                "OCR / COPY" => {
                    window.close();
                    let src = primary_file;
                    std::thread::spawn(move || {
                        // Try OCR first if tesseract is available
                        if let Ok(extracted) = ocr_image_and_copy(&src) {
                            println!("OCR extracted {} chars and copied to clipboard", extracted.len());
                        } else {
                            // Direct file/image copy
                            let _ = copy_image_to_clipboard(&src);
                        }
                    });
                }
                _ => {
                    window.close();
                }
            }
        }
    }
}
