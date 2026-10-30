# Hyprdrop — Matugen (Material You) Dynamic Theming

Hyprdrop natively supports dynamic Material You color extraction from your active desktop wallpaper using [Matugen](https://github.com/InioX/matugen).

## How It Works

1. **Template**: `hyprdrop.json` extracts Material You colors:
   - `primary`: Dynamic accent color for wheel slices, buttons, and switches.
   - `secondary`: Hover and secondary accent color.
   - `accent`: Highlights, sliders, and active indicator tracks.
   - `surface`: Background for radial wheel, modals, and hub windows.
   - `card_bg`: Elevated container and card surface.

2. **Matugen Configuration (`~/.config/matugen/config.toml`)**:
   ```toml
   [templates.hyprdrop]
   input_path  = '~/.config/matugen/templates/hyprdrop.json'
   output_path = '~/.config/hyprdrop/matugen.json'
   post_hook   = 'hyprdrop reload-theme 2>/dev/null || :'
   ```

3. **Live Hot Reload**:
   When wallpaper changes and Matugen executes, it regenerates `~/.config/hyprdrop/matugen.json` and runs `hyprdrop reload-theme`. If Hyprdrop daemon is running, it instantly hot-reloads the GTK CSS provider and edge hotspot without needing to restart.
