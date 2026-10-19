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

#[test]
fn test_object_path_and_tooltip_variant() {
    use gtk4::glib::{self, prelude::*};

    let obj = glib::variant::ObjectPath::try_from("/MenuBar").unwrap();
    let obj_var = obj.to_variant();
    assert_eq!(obj_var.type_().as_str(), "o");

    let empty_pixmap: Vec<(i32, i32, Vec<u8>)> = Vec::new();
    let tooltip = ("hyprdrop", empty_pixmap, "Hyprdrop", "Radial file converter & drop shelf").to_variant();
    assert_eq!(tooltip.type_().as_str(), "(sa(iiay)ss)");
}

#[test]
fn test_dbusmenu_method_return_signatures() {
    use std::collections::HashMap;
    use gtk4::glib::{self, prelude::*};

    // 1. AboutToShow returns (b)
    let about_to_show_ret = (false,).to_variant();
    assert_eq!(about_to_show_ret.type_().as_str(), "(b)");
    assert!(about_to_show_ret.is_container());

    // 2. GetGroupProperties returns (a(ia{sv}))
    let mut map = HashMap::<String, glib::Variant>::new();
    map.insert("label".into(), "Trigger Overlay".to_variant());
    let props: Vec<(i32, HashMap<String, glib::Variant>)> = vec![(1, map)];
    let get_group_props_ret = (props,).to_variant();
    assert_eq!(get_group_props_ret.type_().as_str(), "(a(ia{sv}))");
    assert!(get_group_props_ret.is_container());

    // 3. GetProperty returns (v)
    let label_var = "Trigger Overlay".to_variant();
    let wrapped = glib::Variant::from_variant(&label_var);
    let get_prop_ret = (wrapped,).to_variant();
    assert_eq!(get_prop_ret.type_().as_str(), "(v)");
    assert!(get_prop_ret.is_container());

    // 4. EventGroup returns (ai)
    let empty_errs: Vec<i32> = Vec::new();
    let event_group_ret = (empty_errs,).to_variant();
    assert_eq!(event_group_ret.type_().as_str(), "(ai)");
    assert!(event_group_ret.is_container());

    // 5. AboutToShowGroup returns (aiai)
    let empty_a: Vec<i32> = Vec::new();
    let empty_b: Vec<i32> = Vec::new();
    let about_to_show_group_ret = (empty_a, empty_b).to_variant();
    assert_eq!(about_to_show_group_ret.type_().as_str(), "(aiai)");
    assert!(about_to_show_group_ret.is_container());
}




