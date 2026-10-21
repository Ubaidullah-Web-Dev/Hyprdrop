
const SNI_XML: &str = r#"
<node>
  <interface name="org.kde.StatusNotifierItem">
    <property name="Category" type="s" access="read"/>
    <property name="Id" type="s" access="read"/>
    <property name="Title" type="s" access="read"/>
    <property name="Status" type="s" access="read"/>
    <property name="WindowId" type="u" access="read"/>
    <property name="IconThemePath" type="s" access="read"/>
    <property name="ItemIsMenu" type="b" access="read"/>
    <property name="Menu" type="o" access="read"/>
    <property name="IconName" type="s" access="read"/>
    <property name="IconPixmap" type="a(iiay)" access="read"/>
    <property name="OverlayIconName" type="s" access="read"/>
    <property name="OverlayIconPixmap" type="a(iiay)" access="read"/>
    <property name="AttentionIconName" type="s" access="read"/>
    <property name="AttentionIconPixmap" type="a(iiay)" access="read"/>
    <property name="AttentionMovieName" type="s" access="read"/>
    <property name="ToolTip" type="(sa(iiay)ss)" access="read"/>

    <method name="ContextMenu">
      <arg type="i" name="x" direction="in"/>
      <arg type="i" name="y" direction="in"/>
    </method>
    <method name="Activate">
      <arg type="i" name="x" direction="in"/>
      <arg type="i" name="y" direction="in"/>
    </method>
    <method name="SecondaryActivate">
      <arg type="i" name="x" direction="in"/>
      <arg type="i" name="y" direction="in"/>
    </method>
    <method name="Scroll">
      <arg type="i" name="delta" direction="in"/>
      <arg type="s" name="orientation" direction="in"/>
    </method>

    <signal name="NewTitle"/>
    <signal name="NewIcon"/>
    <signal name="NewAttentionIcon"/>
    <signal name="NewOverlayIcon"/>
    <signal name="NewToolTip"/>
    <signal name="NewStatus">
      <arg type="s" name="status"/>
    </signal>
  </interface>
</node>
"#;

const DBUSMENU_XML: &str = r#"
<node>
  <interface name="com.canonical.dbusmenu">
    <property name="Version" type="u" access="read"/>
    <property name="TextDirection" type="s" access="read"/>
    <property name="Status" type="s" access="read"/>
    <property name="IconThemePath" type="as" access="read"/>

    <method name="GetLayout">
      <arg type="i" name="parentId" direction="in"/>
      <arg type="i" name="recursionDepth" direction="in"/>
      <arg type="as" name="propertyNames" direction="in"/>
      <arg type="u" name="revision" direction="out"/>
      <arg type="(ia{sv}av)" name="layout" direction="out"/>
    </method>

    <method name="GetGroupProperties">
      <arg type="ai" name="ids" direction="in"/>
      <arg type="as" name="propertyNames" direction="in"/>
      <arg type="a(ia{sv})" name="properties" direction="out"/>
    </method>

    <method name="GetProperty">
      <arg type="i" name="id" direction="in"/>
      <arg type="s" name="name" direction="in"/>
      <arg type="v" name="value" direction="out"/>
    </method>

    <method name="Event">
      <arg type="i" name="id" direction="in"/>
      <arg type="s" name="eventId" direction="in"/>
      <arg type="v" name="data" direction="in"/>
      <arg type="u" name="timestamp" direction="in"/>
    </method>

    <method name="EventGroup">
      <arg type="a(isvu)" name="events" direction="in"/>
      <arg type="ai" name="idErrors" direction="out"/>
    </method>

    <method name="AboutToShow">
      <arg type="i" name="id" direction="in"/>
      <arg type="b" name="needUpdate" direction="out"/>
    </method>

    <method name="AboutToShowGroup">
      <arg type="ai" name="ids" direction="in"/>
      <arg type="ai" name="updatesNeeded" direction="out"/>
      <arg type="ai" name="idErrors" direction="out"/>
    </method>

    <signal name="ItemsPropertiesUpdated">
      <arg type="a(ia{sv})" name="updatedProps"/>
      <arg type="a(ias)" name="removedProps"/>
    </signal>
    <signal name="LayoutUpdated">
      <arg type="u" name="revision"/>
      <arg type="i" name="parent"/>
    </signal>
    <signal name="ItemActivationRequested">
      <arg type="i" name="id"/>
      <arg type="u" name="timestamp"/>
    </signal>
  </interface>
</node>
"#;

use std::cell::RefCell;
use std::collections::HashMap;
use anyhow::{Context, Result, anyhow};
use gtk4::gio;
use gtk4::gio::prelude::ApplicationExt;
use gtk4::glib::{self, prelude::ToVariant};
use crate::ui::hotspot::HotspotWindow;
use crate::ui::overlay_window::OverlayWindow;
use crate::ui::settings_app::SettingsApp;


pub struct TrayManager {
    _conn: gio::DBusConnection,
    _sni_reg: gio::RegistrationId,
    _menu_reg: gio::RegistrationId,
    _owner_id: gio::OwnerId,
}


thread_local! {
    static TRAY_INSTANCE: RefCell<Option<TrayManager>> = const { RefCell::new(None) };
}

impl TrayManager {
    /// Spawns the StatusNotifierItem (SNI) tray icon and registers it with the session D-Bus.
    pub fn spawn(app: &gtk4::Application) -> Result<()> {
        let conn = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE)
            .context("Failed to connect to D-Bus session bus")?;

        let sni_node = gio::DBusNodeInfo::for_xml(SNI_XML)
            .context("Failed to parse SNI introspection XML")?;
        let sni_iface = sni_node
            .lookup_interface("org.kde.StatusNotifierItem")
            .ok_or_else(|| anyhow!("SNI interface definition not found"))?;

        let menu_node = gio::DBusNodeInfo::for_xml(DBUSMENU_XML)
            .context("Failed to parse DBusMenu introspection XML")?;
        let menu_iface = menu_node
            .lookup_interface("com.canonical.dbusmenu")
            .ok_or_else(|| anyhow!("DBusMenu interface definition not found"))?;

        let icon_theme_path = get_icon_theme_path();

        // Register StatusNotifierItem object at /StatusNotifierItem
        let app_sni = app.clone();
        let theme_path_sni = icon_theme_path.clone();
        let sni_reg = conn
            .register_object("/StatusNotifierItem", &sni_iface)
            .method_call(move |_, _, _, _, method_name, _, invocation| {
                match method_name {
                    "Activate" => {
                        OverlayWindow::spawn(&app_sni, None);
                        invocation.return_value(None);
                    }
                    "SecondaryActivate" => {
                        SettingsApp::spawn(&app_sni);
                        invocation.return_value(None);
                    }
                    "ContextMenu" | "Scroll" => {
                        invocation.return_value(None);
                    }
                    _ => {
                        invocation.return_value(None);
                    }
                }
            })
            .property(move |_, _, _, _, prop_name| {
                match prop_name {
                    "Category" => "ApplicationStatus".to_variant(),
                    "Id" => "hyprdrop".to_variant(),
                    "Title" => "Hyprdrop".to_variant(),
                    "Status" => "Active".to_variant(),
                    "WindowId" => 0u32.to_variant(),
                    "IconThemePath" => theme_path_sni.as_str().to_variant(),
                    "ItemIsMenu" => false.to_variant(),
                    "Menu" => glib::variant::ObjectPath::try_from("/MenuBar")
                        .expect("valid object path")
                        .to_variant(),
                    "IconName" => "hyprdrop".to_variant(),
                    "IconPixmap" => vec![generate_icon_pixmap(32)].to_variant(),
                    "OverlayIconName" => "".to_variant(),
                    "OverlayIconPixmap" => Vec::<(i32, i32, Vec<u8>)>::new().to_variant(),
                    "AttentionIconName" => "".to_variant(),
                    "AttentionIconPixmap" => Vec::<(i32, i32, Vec<u8>)>::new().to_variant(),
                    "AttentionMovieName" => "".to_variant(),
                    "ToolTip" => (
                        "hyprdrop",
                        Vec::<(i32, i32, Vec<u8>)>::new(),
                        "Hyprdrop",
                        "Radial file converter & drop shelf",
                    )
                        .to_variant(),
                    _ => "".to_variant(),
                }
            })
            .build()
            .context("Failed to register SNI D-Bus object")?;

        // Register DBusMenu object at /MenuBar
        let app_menu = app.clone();
        let conn_menu = conn.clone();
        let theme_path_menu = icon_theme_path;
        let menu_reg = conn
            .register_object("/MenuBar", &menu_iface)
            .method_call(move |_, _, _, _, method_name, params, invocation| {
                match method_name {
                    "GetLayout" => {
                        let root_layout = build_menu_layout();
                        let response = (1u32, root_layout).to_variant();
                        invocation.return_value(Some(&response));
                    }
                    "GetGroupProperties" => {
                        let ids: Vec<i32> = params
                            .child_value(0)
                            .get::<Vec<i32>>()
                            .unwrap_or_default();
                        let props = get_menu_group_properties(&ids);
                        let response = (props,).to_variant();
                        invocation.return_value(Some(&response));
                    }
                    "GetProperty" => {
                        let id = params.child_value(0).get::<i32>().unwrap_or(0);
                        let name = params.child_value(1).get::<String>().unwrap_or_default();
                        let prop = get_single_property(id, &name);
                        let wrapped = glib::Variant::from_variant(&prop);
                        let response = (wrapped,).to_variant();
                        invocation.return_value(Some(&response));
                    }
                    "Event" => {
                        let id = params.child_value(0).get::<i32>().unwrap_or(0);
                        let event_id = params.child_value(1).get::<String>().unwrap_or_default();
                        if event_id == "clicked" {
                            dispatch_menu_action(&app_menu, &conn_menu, id);
                        }
                        invocation.return_value(None);
                    }
                    "EventGroup" => {
                        if let Some(events) = params.child_value(0).get::<Vec<(i32, String, glib::Variant, u32)>>() {
                            for (id, event_id, _, _) in events {
                                if event_id == "clicked" {
                                    dispatch_menu_action(&app_menu, &conn_menu, id);
                                }
                            }
                        }
                        let empty_errors: Vec<i32> = Vec::new();
                        let response = (empty_errors,).to_variant();
                        invocation.return_value(Some(&response));
                    }
                    "AboutToShow" => {
                        let response = (false,).to_variant();
                        invocation.return_value(Some(&response));
                    }
                    "AboutToShowGroup" => {
                        let empty_a: Vec<i32> = Vec::new();
                        let empty_b: Vec<i32> = Vec::new();
                        let response = (empty_a, empty_b).to_variant();
                        invocation.return_value(Some(&response));
                    }
                    _ => {
                        invocation.return_value(None);
                    }
                }
            })
            .property(move |_, _, _, _, prop_name| {
                match prop_name {
                    "Version" => 3u32.to_variant(),
                    "TextDirection" => "ltr".to_variant(),
                    "Status" => "normal".to_variant(),
                    "IconThemePath" => vec![theme_path_menu.as_str()].to_variant(),
                    _ => "".to_variant(),
                }
            })
            .build()
            .context("Failed to register DBusMenu D-Bus object")?;

        // Request a unique well-known StatusNotifierItem bus name
        let pid = std::process::id();
        let bus_name = format!("org.kde.StatusNotifierItem-{}-1", pid);

        // Register item with StatusNotifierWatcher on bus name acquired
        let owner_id = gio::bus_own_name_on_connection(
            &conn,
            &bus_name,
            gio::BusNameOwnerFlags::REPLACE | gio::BusNameOwnerFlags::ALLOW_REPLACEMENT,
            move |conn, name| {
                println!("Hyprdrop acquired D-Bus name {}, registering with StatusNotifierWatcher", name);
                conn.call(
                    Some("org.kde.StatusNotifierWatcher"),
                    "/StatusNotifierWatcher",
                    "org.kde.StatusNotifierWatcher",
                    "RegisterStatusNotifierItem",
                    Some(&(name,).to_variant()),
                    None,
                    gio::DBusCallFlags::NONE,
                    -1,
                    gio::Cancellable::NONE,
                    |_| {},
                );
            },
            |_, name| {
                eprintln!("Hyprdrop lost D-Bus name {}", name);
            },
        );

        // Also watch for StatusNotifierWatcher to handle panel/Waybar restarts gracefully
        let bus_name_watch = bus_name.clone();
        let conn_watch = conn.clone();
        gio::bus_watch_name_on_connection(
            &conn,
            "org.kde.StatusNotifierWatcher",
            gio::BusNameWatcherFlags::NONE,
            move |_, _, _| {
                conn_watch.call(
                    Some("org.kde.StatusNotifierWatcher"),
                    "/StatusNotifierWatcher",
                    "org.kde.StatusNotifierWatcher",
                    "RegisterStatusNotifierItem",
                    Some(&(bus_name_watch.as_str(),).to_variant()),
                    None,
                    gio::DBusCallFlags::NONE,
                    -1,
                    gio::Cancellable::NONE,
                    |_| {},
                );
            },
            |_, _| {},
        );

        TRAY_INSTANCE.with(|cell| {
            *cell.borrow_mut() = Some(TrayManager {
                _conn: conn,
                _sni_reg: sni_reg,
                _menu_reg: menu_reg,
                _owner_id: owner_id,
            });
        });

        println!("Hyprdrop system tray icon registered (SNI bus: {})", bus_name);
        Ok(())
    }
}

/// Dispatches menu clicks to Hyprdrop actions on the GTK main thread.
fn dispatch_menu_action(app: &gtk4::Application, conn: &gio::DBusConnection, id: i32) {
    match id {
        1 => {
            // Trigger Overlay
            OverlayWindow::spawn(app, None);
        }
        2 => {
            // Settings & Activity History
            SettingsApp::spawn(app);
        }
        3 => {
            // Toggle Edge Hotspot
            let _ = HotspotWindow::toggle(app);
            // Notify tray hosts (e.g. Waybar) that menu layout updated to refresh state/label
            let _ = conn.emit_signal(
                None,
                "/MenuBar",
                "com.canonical.dbusmenu",
                "LayoutUpdated",
                Some(&(2u32, 0i32).to_variant()),
            );
        }
        5 => {
            // Quit Hyprdrop daemon
            let _ = std::process::Command::new("systemctl")
                .args(["--user", "stop", "hyprdrop.service"])
                .spawn();
            app.quit();
        }
        _ => {}
    }
}

/// Builds the DBusMenu layout structure matching the canonical dbusmenu specification.
fn build_menu_layout() -> (i32, HashMap<&'static str, glib::Variant>, Vec<glib::Variant>) {
    let mut root_props: HashMap<&'static str, glib::Variant> = HashMap::new();
    root_props.insert("children-display", "submenu".to_variant());

    let mut children: Vec<glib::Variant> = Vec::new();
    let empty_sub: Vec<glib::Variant> = Vec::new();

    // 1. Trigger Overlay
    let mut p1: HashMap<&'static str, glib::Variant> = HashMap::new();
    p1.insert("label", "Trigger Overlay".to_variant());
    p1.insert("icon-name", "hyprdrop".to_variant());
    p1.insert("enabled", true.to_variant());
    p1.insert("visible", true.to_variant());
    children.push((1i32, p1, empty_sub.clone()).to_variant());

    // 2. Settings & Activity History
    let mut p2: HashMap<&'static str, glib::Variant> = HashMap::new();
    p2.insert("label", "Settings & Activity History".to_variant());
    p2.insert("icon-name", "preferences-system".to_variant());
    p2.insert("enabled", true.to_variant());
    p2.insert("visible", true.to_variant());
    children.push((2i32, p2, empty_sub.clone()).to_variant());

    // 3. Toggle Edge Hotspot
    let mut p3: HashMap<&'static str, glib::Variant> = HashMap::new();
    let hotspot_label = if HotspotWindow::is_active() {
        "Toggle Edge Hotspot (Active)"
    } else {
        "Toggle Edge Hotspot (Hidden)"
    };
    p3.insert("label", hotspot_label.to_variant());
    p3.insert("icon-name", "view-paged".to_variant());
    p3.insert("enabled", true.to_variant());
    p3.insert("visible", true.to_variant());
    children.push((3i32, p3, empty_sub.clone()).to_variant());

    // 4. Separator
    let mut p4: HashMap<&'static str, glib::Variant> = HashMap::new();
    p4.insert("type", "separator".to_variant());
    p4.insert("visible", true.to_variant());
    children.push((4i32, p4, empty_sub.clone()).to_variant());

    // 5. Quit
    let mut p5: HashMap<&'static str, glib::Variant> = HashMap::new();
    p5.insert("label", "Quit Hyprdrop".to_variant());
    p5.insert("icon-name", "application-exit".to_variant());
    p5.insert("enabled", true.to_variant());
    p5.insert("visible", true.to_variant());
    children.push((5i32, p5, empty_sub).to_variant());

    (0i32, root_props, children)
}

/// Returns properties for a list of item IDs.
fn get_menu_group_properties(ids: &[i32]) -> Vec<(i32, HashMap<String, glib::Variant>)> {
    let mut list = Vec::new();
    for &id in ids {
        let mut map: HashMap<String, glib::Variant> = HashMap::new();
        match id {
            1 => {
                map.insert("label".into(), "Trigger Overlay".to_variant());
                map.insert("icon-name".into(), "hyprdrop".to_variant());
                map.insert("enabled".into(), true.to_variant());
                map.insert("visible".into(), true.to_variant());
            }
            2 => {
                map.insert("label".into(), "Settings & Activity History".to_variant());
                map.insert("icon-name".into(), "preferences-system".to_variant());
                map.insert("enabled".into(), true.to_variant());
                map.insert("visible".into(), true.to_variant());
            }
            3 => {
                let hotspot_label = if HotspotWindow::is_active() {
                    "Toggle Edge Hotspot (Active)"
                } else {
                    "Toggle Edge Hotspot (Hidden)"
                };
                map.insert("label".into(), hotspot_label.to_variant());
                map.insert("icon-name".into(), "view-paged".to_variant());
                map.insert("enabled".into(), true.to_variant());
                map.insert("visible".into(), true.to_variant());
            }
            4 => {
                map.insert("type".into(), "separator".to_variant());
                map.insert("visible".into(), true.to_variant());
            }
            5 => {
                map.insert("label".into(), "Quit Hyprdrop".to_variant());
                map.insert("icon-name".into(), "application-exit".to_variant());
                map.insert("enabled".into(), true.to_variant());
                map.insert("visible".into(), true.to_variant());
            }
            _ => continue,
        }
        list.push((id, map));
    }
    list
}

/// Returns a single property variant for an item.
fn get_single_property(id: i32, name: &str) -> glib::Variant {
    let group = get_menu_group_properties(&[id]);
    if let Some((_, map)) = group.into_iter().next()
        && let Some(val) = map.get(name)
    {
        return val.clone();
    }

    "".to_variant()
}

/// Generates a sharp, standalone 32x32 ARGB icon pixmap adhering to StatusNotifierItem specs.
/// Uses Studio Graphite (#161618) and International Warm Orange (#FF5722) matching Hyprdrop's design.
pub fn generate_icon_pixmap(size: i32) -> (i32, i32, Vec<u8>) {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    let center = (size as f32 - 1.0) / 2.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let r = (dx * dx + dy * dy).sqrt();

            if r <= 5.5 {
                // Studio Graphite #161618
                data.push(255); // A
                data.push(22);  // R
                data.push(22);  // G
                data.push(24);  // B
            } else if (7.5..=13.5).contains(&r) {
                // International Warm Orange #FF5722
                data.push(255); // A
                data.push(255); // R
                data.push(87);  // G
                data.push(34);  // B
            } else if r > 5.5 && r < 7.5 {
                // Transparent inner ring gap
                data.push(0);
                data.push(0);
                data.push(0);
                data.push(0);
            } else if r > 13.5 && r <= 14.5 {
                // Smooth anti-aliased outer edge
                let alpha = ((14.5 - r) * 255.0).clamp(0.0, 255.0) as u8;
                data.push(alpha);
                data.push(255);
                data.push(87);
                data.push(34);
            } else {
                // Transparent background
                data.push(0);
                data.push(0);
                data.push(0);
                data.push(0);
            }
        }
    }

    (size, size, data)
}

/// Discovers icon asset directories.
fn get_icon_theme_path() -> String {
    if let Ok(curr) = std::env::current_dir() {
        let p = curr.join("assets");
        if p.exists() {
            return p.display().to_string();
        }
    }
    if let Some(data_dir) = dirs::data_dir() {
        let p = data_dir.join("icons/hicolor/scalable/apps");
        if p.exists() {
            return p.display().to_string();
        }
    }
    "/usr/share/icons/hicolor/scalable/apps".to_string()
}
