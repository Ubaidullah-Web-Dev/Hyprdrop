use clap::Parser;
use hyprdrop::app::HyprdropApp;
use hyprdrop::cli::Cli;

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    let exit_code = HyprdropApp::run(cli);
    if exit_code == gtk4::glib::ExitCode::SUCCESS {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}
