use std::path::{Path, PathBuf};
use std::rc::Rc;
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use crate::config::ConfigManager;
use crate::engine::compress::compress_file;
use crate::engine::ocr_clipboard::{copy_image_to_clipboard, ocr_image_and_copy};
use crate::engine::{execute_conversion, get_conversion_slices, get_quick_action_slices};
use crate::hyprland::layer_shell::configure_layer_shell_window;
use crate::ui::drop_shelf::DropShelf;
use crate::ui::modals::{
    open_add_bg_modal, open_crop_modal, open_metadata_modal, open_quick_edit_modal,
    open_redact_annotate_modal,
};
use crate::ui::radial_menu::{HitZone, RADIAL_SIZE, RadialMenu, RadialMode};

pub struct OverlayWindow;

impl OverlayWindow {
    pub fn spawn(app: &gtk4::Application, preloaded_file: Option<PathBuf>) {
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

        let radial = Rc::new(RadialMenu::new(preloaded_file));

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
                        let maybe_file = s.current_file.clone();
                        let current_mode = s.mode;
                        drop(s);

                        if let Some(file_path) = maybe_file {
                            handle_slice_activation(
                                &win_click,
                                &app_click,
                                &file_path,
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
        let drop_target = gtk4::DropTarget::new(
            gdk::FileList::static_type(),
            gdk::DragAction::COPY | gdk::DragAction::MOVE,
        );

        let radial_drop = radial.clone();
        let win_drop = window.clone();
        let app_drop = app.clone();

        drop_target.connect_drop(move |_, val, _, _| {
            if let Ok(file_list) = val.get::<gdk::FileList>() {
                let files = file_list.files();
                if let Some(first_file) = files.first() {
                    if let Some(path) = first_file.path() {
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
                                handle_slice_activation(&win_drop, &app_drop, &path, slice_name, mode);
                                return true;
                            }
                            _ => {
                                drop(s);
                                // Just load file into wheel
                                radial_drop.set_file(path);
                                return true;
                            }
                        }
                    }
                }
            }
            false
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
                        let maybe_file = s.current_file.clone();
                        let mode = s.mode;
                        drop(s);

                        if let Some(file_path) = maybe_file {
                            handle_slice_activation(&win_key, &app_key, &file_path, slice_name, mode);
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

fn handle_slice_activation(
    window: &gtk4::ApplicationWindow,
    _app: &gtk4::Application,
    source_file: &Path,
    slice_name: &str,
    mode: RadialMode,
) {
    let source_path = source_file.to_path_buf();

    match mode {
        RadialMode::FormatConversion => {
            window.close();
            let src = source_path.clone();
            let slice = slice_name.to_string();

            // Background thread conversion
            std::thread::spawn(move || {
                match execute_conversion(&src, &slice) {
                    Ok(out_path) => {
                        let _ = ConfigManager::append_history(
                            &format!("CONVERT: {}", slice),
                            &src,
                            &out_path,
                            None,
                        );
                        glib::idle_add_once(move || {
                            DropShelf::spawn(out_path, None, |chained_file| {
                                let app = gtk4::Application::default();
                                OverlayWindow::spawn(&app, Some(chained_file));
                            });
                        });
                    }
                    Err(e) => {
                        eprintln!("Conversion error for {}: {:?}", src.display(), e);
                    }
                }
            });
        }
        RadialMode::QuickActions => {
            let on_chain = |chained_file: PathBuf| {
                let app = gtk4::Application::default();
                OverlayWindow::spawn(&app, Some(chained_file));
            };

            match slice_name {
                "CROP" => {
                    window.close();
                    open_crop_modal(window, &source_path, move |out_path| {
                        DropShelf::spawn(out_path, None, on_chain);
                    });
                }
                "ADD BG" => {
                    window.close();
                    open_add_bg_modal(window, &source_path, move |out_path| {
                        DropShelf::spawn(out_path, None, on_chain);
                    });
                }
                "COMPRESS" => {
                    window.close();
                    let src = source_path.clone();
                    std::thread::spawn(move || {
                        if let Ok(res) = compress_file(&src) {
                            let badge = format!("Saved {:.0}%", res.percent_saved);
                            let _ = ConfigManager::append_history(
                                "COMPRESS",
                                &src,
                                &res.output_path,
                                Some(res.percent_saved),
                            );
                            glib::idle_add_once(move || {
                                DropShelf::spawn(res.output_path, Some(badge), |chained_file| {
                                    let app = gtk4::Application::default();
                                    OverlayWindow::spawn(&app, Some(chained_file));
                                });
                            });
                        }
                    });
                }
                "REDACT" | "ANNOTATE" => {
                    window.close();
                    open_redact_annotate_modal(window, &source_path, move |out_path| {
                        DropShelf::spawn(out_path, None, on_chain);
                    });
                }
                "METADATA" => {
                    window.close();
                    open_metadata_modal(window, &source_path, move |out_path| {
                        DropShelf::spawn(out_path, None, on_chain);
                    });
                }
                "EDIT" => {
                    window.close();
                    open_quick_edit_modal(window, &source_path, move |out_path| {
                        DropShelf::spawn(out_path, None, on_chain);
                    });
                }
                "OCR / COPY" => {
                    window.close();
                    let src = source_path.clone();
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
