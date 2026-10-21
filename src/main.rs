use clap::Parser;
use hyprdrop::app::HyprdropApp;
use hyprdrop::cli::Cli;

fn main() -> std::process::ExitCode {
    // Hyprdrop is designed for ultra-low memory footprint (~30-50MB RAM).
    // Desktop sessions (like Hyprland/systemd) often export GSK_RENDERER=gl or ngl globally,
    // which unconditionally loads libLLVM (76MB) and libgallium (33MB).
    // Enforce GSK_RENDERER=cairo unless specifically overridden via HYPRDROP_RENDERER.
    if let Ok(renderer) = std::env::var("HYPRDROP_RENDERER") {
        unsafe {
            std::env::set_var("GSK_RENDERER", renderer);
        }
    } else {
        unsafe {
            std::env::set_var("GSK_RENDERER", "cairo");
        }
    }

    let cli = Cli::parse();
    let exit_code = HyprdropApp::run(cli);
    if exit_code == gtk4::glib::ExitCode::SUCCESS {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}
