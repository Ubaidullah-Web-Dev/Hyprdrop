use std::path::PathBuf;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "hyprdrop")]
#[command(author = "Hyprdrop Contributors")]
#[command(version)]
#[command(about = "Radial drag-and-drop file converter, quick editor, and drop-shelf for Hyprland")]
pub struct Cli {
    #[arg(short, long, help = "Run as background daemon")]
    pub daemon: bool,

    #[arg(long, help = "Safely uninstall Hyprdrop and revert all integrations")]
    pub uninstall: bool,

    #[arg(value_name = "FILE", help = "Optional file to preload into radial overlay")]
    pub file: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Triggers the radial overlay under the cursor
    Trigger {
        #[arg(value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Toggles radial overlay visibility
    Toggle,
    /// Opens desktop settings and activity history
    Settings,
    /// Automatically configures Hyprland rules and mouse keybinds
    #[command(alias = "configure")]
    Setup,
    /// Checks for and installs the latest Hyprdrop release
    Update,
    /// Reloads theme colors (e.g. following a Matugen wallpaper update)
    #[command(alias = "theme-reload")]
    ReloadTheme,
    /// Completely and safely uninstalls Hyprdrop and reverts all integrations
    Uninstall {
        /// Automatically confirm uninstallation without prompting
        #[arg(short = 'y', long)]
        yes: bool,
    },
}
