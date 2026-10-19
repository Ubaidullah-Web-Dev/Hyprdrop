# Design System: Hyprdrop (`hyprdrop`)

## 1. Visual Theme & Atmosphere
An industrial-grade, tactical Wayland studio utility built for Hyprland.
The atmosphere is restrained, surgical, and responsive — inspired by precision measurement tools and high-end hardware audio interfaces.
The radial wheel and modal surfaces use a darkened studio graphite backdrop (`#161618`) with 92% alpha to leverage Hyprland's native dual-cavity background blur.
Active interactions and targeted format slices illuminate with International Warm Orange (`#FF5722`), providing instantaneous spatial feedback without distraction.

## 2. Color Palette & Roles
- **Studio Surface** (`#161618`, 92% opacity / `rgba(22, 22, 24, 0.92)`) — Primary background surface for radial wheel, modals, and shelf.
- **Elevated Slate Card** (`#212124`) — Cards, slices, input surfaces, and preset pills.
- **Tactile Border** (`rgba(255, 255, 255, 0.08)`) — 1px subtle structural border separating slices, dialogs, and controls.
- **Warm Orange Accent** (`#FF5722`) — Active slice highlight, primary action buttons, crop boundary handles, and focus indicators.
- **Signal Hover** (`#FF784E`) — Hover highlight for interactive buttons and pills.
- **Deep Void** (`#0D0D0E`) — Modal canvas background behind image previews.
- **Text Crisp Primary** (`#F4F4F6`) — Primary typography, slice titles, modal headings.
- **Text Muted Steel** (`#9E9EA8`) — Secondary file sizes, dimension labels, hint captions.
- **Success Mint** (`#10B981`) — Compression savings indicator and success toasts.
- **Danger Crimson** (`#EF4444`) — Redaction tool active stroke and reset confirmation.

## 3. Typography Rules
- **Sans Display / Labels**: Clean sans-serif (`Cantarell`, `Geist`, `Noto Sans`, system sans) with tight letter-spacing (-0.02em). Weight-driven hierarchy (600 for slice labels, 700 for headings).
- **Monospace Tabular**: Tabular numbers (`JetBrains Mono`, `Fira Code`, monospace) for all dimensions (`1920 × 1080`), file sizes (`410 KB`, `1.4 MB`), aspect ratios, and compression percentage metrics (`Saved 64%`).
- **Banned**: Inter, generic serif fonts, neon glows, emoji spam.

## 4. Component Stylings
- **Radial Wheel Widget**: 380x380 px Cairo canvas. Donut geometry (Inner radius: 52px, Outer radius: 174px). Center hub displaying file size / hovered action. Slices rendered with precision arc math and subtle 1px border dividers.
- **Buttons**: Tactile flat surfaces (`#212124` secondary, `#FF5722` primary) with 6px border-radius and -1px translateY active press state.
- **Inputs & Sliders**: Flat elevated inputs with crisp monospace readouts. Sliders feature warm orange fill tracks.
- **Modals**: Window geometry centered on monitor or cursor, bordered with `rgba(255, 255, 255, 0.08)`, frosted background.
- **Drop Shelf**: Minimal floating card with Draggable thumbnail, file size badge, and close button.
- **System Tray Icon (SNI)**: 32x32 px vector/ARGB icon. Donut geometry echoing radial widget geometry in International Warm Orange (`#FF5722`) with Studio Graphite (`#161618`) center hub. D-Bus context menu providing quick actions: Trigger Overlay, Settings & Activity History, Toggle Edge Hotspot, and Quit.


## 5. Layout & Spatial Principles
- Center-anchored coordinate mapping: Cursor-driven spawning directly under pointer via `hyprctl cursorpos -j`.
- Generous breathing room with zero overlapping text.
- Clean separation between primary canvas and floating toolbars.

## 6. Motion & Interaction
- Seamless mode flip (Mode 1 Format Conversion <-> Mode 2 Quick Actions) with smooth slice transitions.
- Interactive crop handles with live rule-of-thirds gridlines during drag.
- Instantaneous hover feedback (under 10ms render latency).

## 7. Anti-Patterns (Banned)
- No neon purple/cyan AI gradients.
- No unstyled generic GTK default widgets.
- No emojis anywhere in production UI.
- No fabricated telemetry or fake metrics.
- No floating non-anchored windows or laggy popups.
