use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use gtk4::gio::ApplicationFlags;
use gtk4::glib;
use gtk4::prelude::*;
use crate::cli::{Cli, Commands};
use crate::ipc::{IpcClient, IpcMessage, IpcServer};
use crate::ui::init_css_theme;
use crate::ui::overlay_window::OverlayWindow;
use crate::ui::settings_app::SettingsApp;

pub struct HyprdropApp;

impl HyprdropApp {
    pub fn run(cli: Cli) -> glib::ExitCode {
        // First check if this is a client command targeting an existing daemon
        if !cli.daemon {
            let msg = if let Some(ref cmd) = cli.command {
                match cmd {
                    Commands::Trigger { file } => {
                        let explicit = file.clone().or_else(|| cli.file.clone());
                        let files = if let Some(ref f) = explicit {
                            vec![f.clone()]
                        } else {
                            crate::ui::overlay_window::detect_selected_files()
                        };
                        Some(IpcMessage::Trigger {
                            file: files.first().cloned(),
                            files: Some(files),
                        })
                    }
                    Commands::Toggle => Some(IpcMessage::Toggle),
                    Commands::Settings => Some(IpcMessage::Settings),
                }
            } else {
                let explicit = cli.file.clone();
                let files = if let Some(ref f) = explicit {
                    vec![f.clone()]
                } else {
                    crate::ui::overlay_window::detect_selected_files()
                };
                Some(IpcMessage::Trigger {
                    file: files.first().cloned(),
                    files: Some(files),
                })
            };

            if let Some(ref m) = msg
                && let Ok(true) = IpcClient::send(m) {
                    // Message successfully handled by running daemon
                    return glib::ExitCode::SUCCESS;
                }
        }

        // Otherwise, initialize GTK Application
        let app_flags = ApplicationFlags::NON_UNIQUE;

        let app = gtk4::Application::builder()
            .application_id("org.hyprland.hyprdrop")
            .flags(app_flags)
            .build();

        let cli_rc = Rc::new(cli);

        app.connect_startup(|_| {
            init_css_theme();
        });

        app.connect_shutdown(|_| {
            let sock_path = crate::config::ConfigManager::socket_path();
            if sock_path.exists() {
                let _ = std::fs::remove_file(sock_path);
            }
        });

        let cli_activate = cli_rc.clone();
        app.connect_activate(move |app| {
            if cli_activate.daemon {
                Self::start_daemon_loop(app);
            } else if let Some(ref cmd) = cli_activate.command {
                match cmd {
                    Commands::Trigger { file } => {
                        let target = file.clone().or_else(|| cli_activate.file.clone());
                        OverlayWindow::spawn(app, target);
                    }
                    Commands::Toggle => {
                        OverlayWindow::toggle(app, cli_activate.file.clone());
                    }
                    Commands::Settings => {
                        SettingsApp::spawn(app);
                    }
                }
            } else if let Some(ref file) = cli_activate.file {
                OverlayWindow::spawn(app, Some(file.clone()));
            } else {
                // Default: open overlay window ready for drops
                OverlayWindow::spawn(app, None);
            }
        });

        app.run_with_args::<&str>(&[])
    }

    fn start_daemon_loop(app: &gtk4::Application) {
        // Retain hold guard so GTK application daemon stays alive indefinitely
        Box::leak(Box::new(app.hold()));

        // Spawn edge hotspot fallback widget anchored to screen edge
        crate::ui::hotspot::HotspotWindow::spawn(app);

        // Spawn StatusNotifierItem / DBusMenu system tray icon
        if let Err(e) = crate::ui::tray::TrayManager::spawn(app) {
            eprintln!("Failed to initialize system tray icon: {:?}", e);
        }


        let listener = match IpcServer::bind() {
            Ok(l) => Rc::new(RefCell::new(l)),
            Err(e) => {
                eprintln!("Failed to bind IPC socket: {:?}", e);
                return;
            }
        };

        let app_timer = app.clone();
        let listener_timer = listener.clone();

        // 30ms polling interval for ultra-low latency response
        glib::timeout_add_local(Duration::from_millis(30), move || {
            let l = listener_timer.borrow();
            while let Some(msg) = IpcServer::try_recv(&l) {
                match msg {
                    IpcMessage::Trigger { file, files } => {
                        let mut targets = files.unwrap_or_default();
                        if targets.is_empty() {
                            if let Some(f) = file {
                                targets.push(f);
                            } else {
                                targets = crate::ui::overlay_window::detect_selected_files();
                            }
                        }
                        OverlayWindow::spawn_with_files(&app_timer, targets);
                    }
                    IpcMessage::Toggle => {
                        OverlayWindow::toggle(&app_timer, None);
                    }
                    IpcMessage::Settings => {
                        SettingsApp::spawn(&app_timer);
                    }
                    IpcMessage::Ping => {}
                }
            }
            glib::ControlFlow::Continue
        });

        println!("Hyprdrop daemon listening on IPC socket...");
    }
}
