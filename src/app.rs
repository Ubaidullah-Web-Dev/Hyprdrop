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
        // If settings command or app launcher runs and daemon isn't running, ensure daemon starts
        let is_settings_or_default = matches!(cli.command, Some(Commands::Settings))
            || (cli.command.is_none() && cli.file.is_none());
        if !cli.daemon && is_settings_or_default && !crate::hyprland::is_daemon_running() {
            crate::hyprland::ensure_daemon_running();
        }

        // Handle safe uninstallation CLI invocation
        let is_uninstall = cli.uninstall
            || matches!(cli.command, Some(Commands::Uninstall { .. }));

        if is_uninstall {
            let auto_confirm = matches!(cli.command, Some(Commands::Uninstall { yes: true }));
            if !auto_confirm {
                use std::io::{self, Write};
                println!("==========================================================");
                println!("           Hyprdrop — Safe System Uninstaller             ");
                println!("==========================================================");
                println!("This will safely revert all system changes made by Hyprdrop:");
                println!("  • Revert Hyprland window rules and mouse keybinds (with backup)");
                println!("  • Unregister Matugen theme templates from config.toml (with backup)");
                println!("  • Remove Matugen hyprdrop template and generated theme");
                println!("  • Disable and remove hyprdrop.service systemd user unit");
                println!("  • Remove desktop entry and icons");
                println!("  • Remove Hyprdrop executables from ~/.local/bin\n");
                print!("Are you sure you want to proceed with uninstallation? [y/N]: ");
                let _ = io::stdout().flush();

                let mut input = String::new();
                if io::stdin().read_line(&mut input).is_err() || !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                    println!("==> Uninstallation cancelled. No changes were made.");
                    return glib::ExitCode::SUCCESS;
                }
            }

            println!("\n==> Reverting Hyprdrop integrations...");
            match crate::uninstaller::perform_uninstall(true) {
                Ok(report) => {
                    for msg in report.messages {
                        println!("    [✓] {}", msg);
                    }
                    println!("\n==========================================================");
                    println!("   Hyprdrop has been uninstalled successfully!            ");
                    println!("   All backups safely preserved in ~/.config/hypr & matugen");
                    println!("==========================================================");
                    return glib::ExitCode::SUCCESS;
                }
                Err(e) => {
                    eprintln!("==> Error during uninstallation: {}", e);
                    return glib::ExitCode::FAILURE;
                }
            }
        }

        // First check if this is a client command targeting an existing daemon
        if !cli.daemon {
            let msg = if let Some(ref cmd) = cli.command {
                match cmd {
                    Commands::Setup => {
                        println!("==> Configuring Hyprland for Hyprdrop...");
                        match crate::hyprland::installer::configure_hyprland() {
                            Ok(result) => {
                                if result.already_configured {
                                    println!("    [✓] Hyprland is already configured for Hyprdrop.");
                                } else {
                                    println!("    [✓] Injected window rules, layer rules, and SHIFT+Click bind into Hyprland configuration!");
                                    if let Some(bak) = result.backup_path {
                                        println!("    [✓] Created timestamped backup: {}", bak.display());
                                    }
                                    println!("    [✓] Reloaded Hyprland session via hyprctl reload.");
                                }
                                return glib::ExitCode::SUCCESS;
                            }
                            Err(e) => {
                                eprintln!("    [!] Failed to auto-configure Hyprland: {}", e);
                                eprintln!("    Please add the following bind to your hyprland.conf:");
                                eprintln!("      bind = SHIFT, mouse:272, exec, hyprdrop trigger");
                                return glib::ExitCode::FAILURE;
                            }
                        }
                    }
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
                    Commands::Update => {
                        println!("==> Checking for and installing latest Hyprdrop release...");
                        let script = "curl -fsSL https://raw.githubusercontent.com/Ubaidullah-Web-Dev/Hyprdrop/main/install.sh | bash -s -- --bin";
                        let status = std::process::Command::new("bash")
                            .arg("-c")
                            .arg(script)
                            .status();
                        match status {
                            Ok(s) if s.success() => {
                                println!("\n==> Hyprdrop updated successfully!");
                                return glib::ExitCode::SUCCESS;
                            }
                            Ok(s) => {
                                eprintln!("\n==> Update failed with status code: {}", s);
                                return glib::ExitCode::FAILURE;
                            }
                            Err(e) => {
                                eprintln!("\n==> Failed to execute updater: {}", e);
                                return glib::ExitCode::FAILURE;
                            }
                        }
                    }
                    Commands::ReloadTheme => {
                        println!("==> Reloading Hyprdrop theme...");
                        let mut cfg = crate::config::ConfigManager::load_config();
                        if let Some(matugen_theme) = crate::config::ConfigManager::load_matugen_theme() {
                            cfg.theme = matugen_theme;
                            let _ = crate::config::ConfigManager::save_config(&cfg);
                            println!("    [✓] Loaded latest Matugen Material You colors (primary: {})", cfg.theme.primary);
                        }
                        if let Ok(true) = IpcClient::send(&IpcMessage::ReloadTheme) {
                            println!("    [✓] Notified running Hyprdrop daemon to apply new theme.");
                        } else {
                            println!("    [✓] Theme updated in configuration.");
                        }
                        return glib::ExitCode::SUCCESS;
                    }
                    Commands::Uninstall { .. } => unreachable!(),
                }
            } else if let Some(ref f) = cli.file {
                Some(IpcMessage::Trigger {
                    file: Some(f.clone()),
                    files: Some(vec![f.clone()]),
                })
            } else {
                // Opened from application launcher without arguments: show Settings / Main Menu
                Some(IpcMessage::Settings)
            };

            if let Some(ref m) = msg
                && let Ok(true) = IpcClient::send(m) {
                    // Message successfully handled by running daemon
                    return glib::ExitCode::SUCCESS;
                }

            // Daemon is not running. If this was a trigger or toggle,
            // check user preference for whether to allow triggering when closed.
            let is_trigger_or_toggle = matches!(
                cli.command,
                Some(Commands::Trigger { .. }) | Some(Commands::Toggle)
            );
            if is_trigger_or_toggle {
                let cfg = crate::config::ConfigManager::load_config();
                if !cfg.allow_trigger_when_closed {
                    return glib::ExitCode::SUCCESS;
                }
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
            crate::engine::trim_memory();
            crate::hyprland::ensure_hyprland_floating_rules();
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
                    Commands::Setup | Commands::Update | Commands::ReloadTheme | Commands::Uninstall { .. } => {
                        return;
                    }
                    Commands::Trigger { file } => {
                        let target = file.clone().or_else(|| cli_activate.file.clone());
                        OverlayWindow::spawn(app, target);
                    }
                    Commands::Toggle => {
                        OverlayWindow::toggle(app, cli_activate.file.clone());
                    }
                    Commands::Settings => {
                        if !crate::hyprland::is_daemon_running() {
                            Self::start_daemon_loop(app);
                        }
                        SettingsApp::spawn(app);
                    }
                }
            } else if let Some(ref file) = cli_activate.file {
                OverlayWindow::spawn(app, Some(file.clone()));
            } else {
                // Default launcher fallback: ensure daemon loop and spawn Settings window
                if !crate::hyprland::is_daemon_running() {
                    Self::start_daemon_loop(app);
                }
                SettingsApp::spawn(app);
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
        let mut ticks = 0u64;
        let mut last_matugen_mtime = None;

        // 30ms polling interval for ultra-low latency response
        glib::timeout_add_local(Duration::from_millis(30), move || {
            ticks = ticks.wrapping_add(1);
            if ticks.is_multiple_of(15) {
                // Check for Matugen palette updates (~every 450ms)
                let m_path = crate::config::ConfigManager::matugen_theme_path();
                if let Ok(meta) = std::fs::metadata(&m_path) {
                    if let Ok(mtime) = meta.modified() {
                        if let Some(prev) = last_matugen_mtime {
                            if mtime != prev {
                                last_matugen_mtime = Some(mtime);
                                let cfg = crate::config::ConfigManager::load_config();
                                if cfg.theme.name.to_lowercase().contains("matugen") {
                                    crate::ui::init_css_theme();
                                    let updated = crate::config::ConfigManager::load_config();
                                    crate::ui::hotspot::HotspotWindow::update_theme_live(
                                        &updated.theme.primary,
                                        &updated.theme.secondary,
                                        &updated.theme.accent,
                                    );
                                    println!("    [✓] Matugen wallpaper theme hot-reloaded automatically");
                                }
                            }
                        } else {
                            last_matugen_mtime = Some(mtime);
                        }
                    }
                }
            }
            if ticks.is_multiple_of(30) {
                // Periodic fallback sync (~every 900ms) for fullscreen state
                let is_fs = crate::hyprland::check_is_fullscreen();
                crate::ui::hotspot::HotspotWindow::set_fullscreen_hidden(is_fs);
            }
            if ticks.is_multiple_of(300) {
                // Periodically trim heap memory back to OS kernel (~every 9s)
                crate::engine::trim_memory();
            }
            let l = listener_timer.borrow();
            while let Some(msg) = IpcServer::try_recv(&l) {
                match msg {
                    IpcMessage::Trigger { file, files } => {
                        let targets = if let Some(fs) = files {
                            fs
                        } else if let Some(f) = file {
                            vec![f]
                        } else {
                            crate::ui::overlay_window::detect_selected_files()
                        };
                        OverlayWindow::spawn_with_files(&app_timer, targets);
                    }
                    IpcMessage::Toggle => {
                        OverlayWindow::toggle(&app_timer, None);
                    }
                    IpcMessage::Settings => {
                        crate::hyprland::ensure_hyprland_floating_rules();
                        SettingsApp::spawn(&app_timer);
                    }
                    IpcMessage::ReloadHotspot => {
                        crate::ui::hotspot::HotspotWindow::reload(&app_timer);
                    }
                    IpcMessage::ReloadTheme => {
                        crate::ui::init_css_theme();
                        let cfg = crate::config::ConfigManager::load_config();
                        crate::ui::hotspot::HotspotWindow::update_theme_live(
                            &cfg.theme.primary,
                            &cfg.theme.secondary,
                            &cfg.theme.accent,
                        );
                        println!("    [✓] Live reloaded Hyprdrop theme in background daemon");
                    }
                    IpcMessage::Ping => {}
                }
            }
            glib::ControlFlow::Continue
        });

        println!("Hyprdrop daemon listening on IPC socket...");
    }
}
