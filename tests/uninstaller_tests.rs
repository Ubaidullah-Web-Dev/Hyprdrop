use hyprdrop::hyprland::installer::strip_hyprdrop_rules;
use hyprdrop::uninstaller::strip_matugen_hyprdrop_section;

#[test]
fn test_strip_hyprdrop_rules_lua() {
    let original_lua = r#"
hl.window_rule({
    name = "kitty_rule",
    match = { class = "kitty" },
    float = true,
})

-- --- Hyprdrop Radial Utility Configuration ---
hl.window_rule({
    name = "hyprdrop_float_class",
    match = { class = ".*hyprdrop.*" },
    float = true,
    center = true,
    size = "660 660",
})

hl.bind(
    "SHIFT + mouse:272",
    hl.dsp.exec_cmd("hyprdrop trigger"),
    { description = "Hyprdrop Radial Trigger", mouse = true }
)

-- User Section
hl.bind("SUPER + Q", hl.dsp.exec_cmd("kitty"))
"#;

    let stripped = strip_hyprdrop_rules(original_lua);
    assert!(!stripped.contains("hyprdrop"));
    assert!(stripped.contains("kitty_rule"));
    assert!(stripped.contains("kitty"));
    assert!(stripped.contains("SUPER + Q"));
    assert!(stripped.contains("-- User Section"));
}

#[test]
fn test_strip_hyprdrop_rules_conf() {
    let original_conf = r#"
# User configs
windowrulev2 = float, class:^(kitty)$
bind = SUPER, Q, exec, kitty

# --- Hyprdrop Radial Utility Configuration ---
windowrulev2 = float, class:^(.*hyprdrop.*)$
windowrulev2 = center, class:^(.*hyprdrop.*)$
windowrulev2 = size 660 660, class:^(.*hyprdrop.*)$
layerrule = blur, hyprdrop
bind = SHIFT, mouse:272, exec, hyprdrop trigger

# Final configs
bind = SUPER, E, exec, nautilus
"#;

    let stripped = strip_hyprdrop_rules(original_conf);
    assert!(!stripped.contains("hyprdrop"));
    assert!(stripped.contains("kitty"));
    assert!(stripped.contains("nautilus"));
    assert!(stripped.contains("# Final configs"));
}

#[test]
fn test_strip_matugen_hyprdrop_section() {
    let matugen_toml = r#"[config]
reload_apps = true

[templates.gtk3]
input_path  = '~/.config/matugen/templates/gtk3.css'
output_path = '~/.config/matugen/generated/gtk3.css'

# =============================================================================
# Hyprdrop Radial Utility (Dynamic Material You Palette)
# =============================================================================
[templates.hyprdrop]
input_path  = '~/.config/matugen/templates/hyprdrop.json'
output_path = '~/.config/hyprdrop/matugen.json'
post_hook   = 'hyprdrop reload-theme 2>/dev/null || :'

[templates.kitty]
input_path  = '~/.config/matugen/templates/kitty.conf'
output_path = '~/.config/kitty/colors.conf'
"#;

    let stripped = strip_matugen_hyprdrop_section(matugen_toml);
    assert!(!stripped.contains("[templates.hyprdrop]"));
    assert!(!stripped.contains("Hyprdrop Radial Utility"));
    assert!(!stripped.contains("hyprdrop.json"));
    // Other sections must remain 100% intact
    assert!(stripped.contains("[templates.gtk3]"));
    assert!(stripped.contains("[templates.kitty]"));
    assert!(stripped.contains("colors.conf"));
}
