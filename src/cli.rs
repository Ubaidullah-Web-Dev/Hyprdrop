use std::path::PathBuf;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "hyprdrop")]
#[command(author = "Hyprdrop Contributors")]
#[command(version = "0.1.0")]
#[command(about = "Radial drag-and-drop file converter, quick editor, and drop-shelf for Hyprland")]
pub struct Cli {
    #[arg(short, long, help = "Run as background daemon")]
    pub daemon: bool,

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
}
