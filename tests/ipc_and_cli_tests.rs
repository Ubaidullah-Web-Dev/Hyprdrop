use std::path::PathBuf;
use clap::Parser;
use hyprdrop::cli::{Cli, Commands};
use hyprdrop::ipc::IpcMessage;

#[test]
fn test_cli_daemon_flag() {
    let cli = Cli::try_parse_from(["hyprdrop", "--daemon"]).expect("Failed to parse --daemon");
    assert!(cli.daemon);
    assert!(cli.file.is_none());
    assert!(cli.command.is_none());
}

#[test]
fn test_cli_trigger_subcommand_with_file() {
    let cli = Cli::try_parse_from(["hyprdrop", "trigger", "/tmp/sample.png"])
        .expect("Failed to parse trigger");
    assert!(!cli.daemon);
    match cli.command {
        Some(Commands::Trigger { file }) => {
            assert_eq!(file, Some(PathBuf::from("/tmp/sample.png")));
        }
        _ => panic!("Expected Commands::Trigger"),
    }
}

#[test]
fn test_cli_toggle_subcommand() {
    let cli = Cli::try_parse_from(["hyprdrop", "toggle"]).expect("Failed to parse toggle");
    assert!(matches!(cli.command, Some(Commands::Toggle)));
}

#[test]
fn test_cli_settings_subcommand() {
    let cli = Cli::try_parse_from(["hyprdrop", "settings"]).expect("Failed to parse settings");
    assert!(matches!(cli.command, Some(Commands::Settings)));
}

#[test]
fn test_cli_direct_file_argument() {
    let cli = Cli::try_parse_from(["hyprdrop", "/home/user/document.pdf"])
        .expect("Failed to parse direct file");
    assert_eq!(cli.file, Some(PathBuf::from("/home/user/document.pdf")));
}

#[test]
fn test_ipc_message_serialization() {
    let msg1 = IpcMessage::Trigger {
        file: Some(PathBuf::from("/tmp/test.png")),
    };
    let json1 = serde_json::to_string(&msg1).expect("Failed to serialize");
    let deser1: IpcMessage = serde_json::from_str(&json1).expect("Failed to deserialize");
    match deser1 {
        IpcMessage::Trigger { file } => {
            assert_eq!(file, Some(PathBuf::from("/tmp/test.png")));
        }
        _ => panic!("Wrong variant"),
    }

    let msg2 = IpcMessage::Toggle;
    let json2 = serde_json::to_string(&msg2).expect("Failed to serialize");
    let deser2: IpcMessage = serde_json::from_str(&json2).expect("Failed to deserialize");
    assert!(matches!(deser2, IpcMessage::Toggle));

    let msg3 = IpcMessage::Settings;
    let json3 = serde_json::to_string(&msg3).expect("Failed to serialize");
    let deser3: IpcMessage = serde_json::from_str(&json3).expect("Failed to deserialize");
    assert!(matches!(deser3, IpcMessage::Settings));
}
