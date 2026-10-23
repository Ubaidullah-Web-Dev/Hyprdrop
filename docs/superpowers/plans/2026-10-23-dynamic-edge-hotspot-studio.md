# Dynamic Edge Hotspot Studio & Main Menu Overhaul (v0.2.0) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform the static edge hotspot into a fully dynamic, draggable, smoothly morphing perimeter widget that bends cleanly around screen corners, discriminates between clicks and drags, offers lock/unlock protection, and is backed by a dedicated Edge Hotspot Studio page within a cleanly categorized Settings application.

**Architecture:** 
1. Parameterize screen perimeter as a continuous 1D cyclic space $s \in [0, 2(W+H))$ with sub-pixel Cairo path morphing around corners.
2. Implement click vs. drag gesture discrimination with delta thresholds in `HotspotWindow`.
3. Expand `AppConfig` with dynamic geometry, colors, auto-hide, and lock states, backed by live IPC/signal updates.
4. Overhaul `SettingsApp` with 5 modular studio pages (Activity, Hotspot Studio, Themes, Prefs, Hyprland/Health) featuring an interactive live-preview canvas.

**Tech Stack:** Rust 2024, GTK4 (`gtk4`, `cairo-rs`, `gtk4-layer-shell`), `rayon`, `glib`, Hyprland IPC.

---

## Global Constraints
- Daemon idle memory footprint must remain strictly $< 25 \text{ MB}$.
- CPU utilization when idle must be $0.0\%$; redraws must only occur on hover, drag events, or config updates.
- In-app versioning must dynamically reference `env!("CARGO_PKG_VERSION")`.
- Wayland layer-shell must handle repositioning and resizing without flickering or crashing compositor surfaces.

---

## Review Focus
1. **Click vs. Drag Precision:** Clicking without moving pointer ($< 6\text{px}$) must trigger Hyprdrop radial overlay; dragging ($\ge 6\text{px}$) when unlocked must move hotspot without spawning overlay.
2. **Corner Wrap Morphing:** A hotspot positioned across a corner (e.g. 70% top, 30% right) must render as a continuous, seamless bent pill with anti-aliased corner curvature.
3. **Lock Enforcement:** When `hotspot_locked = true`, dragging must be completely disabled, locking the hotspot rigidly at the user's desktop position.
4. **Auto-Hide Smoothness:** Proximity hover and drag-motion must smoothly fade the hotspot between idle alpha and full opacity without abrupt pops.
5. **Real-time Live Sync:** Sliders or color changes in the Settings App must update the active desktop hotspot immediately without requiring daemon restarts.

---

### Task 1: Perimeter Geometry & Math Engine

**Files:**
- Create: `src/engine/perimeter_math.rs`
- Modify: `src/engine/mod.rs`
- Test: `tests/perimeter_math_tests.rs`

- [ ] **Step 1: Write unit tests for perimeter mapping, corner morphing, and edge detection**
  - Test mapping normalized perimeter position $t \in [0.0, 1.0)$ to edge coordinates $(x, y)$.
  - Test detecting whether an interval $[s - L/2, s + L/2]$ is on an edge or spans a corner.
  - Test proportional split calculations (e.g. 70% on top, 30% on right).
- [ ] **Step 2: Run tests to verify failure** (`cargo test --test perimeter_math_tests`)
- [ ] **Step 3: Implement `PerimeterMath` and `EdgeAnchor` calculations**
  - Define `enum ScreenEdge { Top, Right, Bottom, Left, CornerTopRight, CornerBottomRight, CornerBottomLeft, CornerTopLeft }`.
  - Implement continuous coordinate wrapping and Cairo path generation for linear and bent pill segments.
- [ ] **Step 4: Run tests and verify all pass**
- [ ] **Step 5: Commit changes** (`feat(engine): implement perimeter math and continuous corner morphing`)

---

### Task 2: Config Expansion & Live IPC Synchronization

**Files:**
- Modify: `src/config.rs:10-50`
- Modify: `src/ipc.rs`
- Modify: `src/app.rs`
- Test: `tests/config_hotspot_tests.rs`

- [ ] **Step 1: Write unit tests for new hotspot config serialization and default values**
- [ ] **Step 2: Run tests to verify failure**
- [ ] **Step 3: Add hotspot configuration fields to `AppConfig`**
  - `hotspot_enabled: bool`
  - `hotspot_locked: bool`
  - `hotspot_perimeter_pos: f64` (0.0 to 1.0)
  - `hotspot_length: u32` (80–450)
  - `hotspot_thickness: u32` (4–24)
  - `hotspot_margin: i32` (0–40)
  - `hotspot_corner_radius: u32` (2–16)
  - `hotspot_auto_hide: bool`
  - `hotspot_idle_opacity: f64` (0.05–1.0)
  - `hotspot_color: String`
  - `hotspot_hover_color: String`
  - `hotspot_accent_color: String`
- [ ] **Step 4: Add IPC command `IpcMessage::UpdateHotspot` for live configuration pushes**
- [ ] **Step 5: Run tests and verify all pass**
- [ ] **Step 6: Commit changes** (`feat(config): add dynamic hotspot parameters and live IPC sync`)

---

### Task 3: Interactive Draggable & Morphing Desktop Hotspot

**Files:**
- Modify: `src/ui/hotspot.rs`
- Test: `tests/hotspot_gesture_tests.rs`

- [ ] **Step 1: Implement custom Cairo drawing func in `HotspotWindow` using `PerimeterMath`**
  - Support straight island pills (horizontal or vertical).
  - Support soft curved L-shape corner pills with configurable corner radius.
- [ ] **Step 2: Implement Click vs. Drag Gesture Controller**
  - Track pointer drag distance.
  - If $\Delta < 6\text{px}$, treat as Click $\rightarrow$ activate Hyprdrop radial overlay.
  - If $\Delta \ge 6\text{px}$ and unlocked, update perimeter coordinate, recalculate layer-shell anchors/margins, and re-render.
- [ ] **Step 3: Implement Lock State & Visual Handle Indicator**
  - When unlocked, render subtle tactile grip dots on the bar.
  - When locked, render clean minimalist bar.
- [ ] **Step 4: Implement Proximity Auto-Hide Animation**
  - Enter/leave motion controllers smoothly interpolate alpha via `Revealer` or Cairo alpha.
- [ ] **Step 5: Run tests and verify compile & behavioral checks**
- [ ] **Step 6: Commit changes** (`feat(ui): interactive draggable desktop hotspot with corner morphing`)

---

### Task 4: Main Settings App Overhaul & "Edge Hotspot Studio"

**Files:**
- Modify: `src/ui/settings_app.rs`
- Modify: `assets/style.css`

- [ ] **Step 1: Re-categorize Settings Sidebar into 5 Distinct Studios**
  1. 📋 Activity Log
  2. ⚡ Edge Hotspot Studio
  3. 🎨 Appearance & Themes
  4. ⚙️ Preferences & Behavior
  5. 🖥️ Hyprland & Health
- [ ] **Step 2: Build the Edge Hotspot Studio Page**
  - Interactive Monitor Canvas displaying screen frame and live draggable hotspot preview.
  - Lock / Unlock Position Switch with dynamic status badges.
  - Quick Snap edge buttons (`Top`, `Right`, `Bottom`, `Left`, `Corners`).
  - Sliders for Length, Thickness, Margin, Corner Radius.
  - Color Pickers / Preset Palettes for Idle Color, Hover Glow, and Dropping Active.
  - Auto-Hide Proximity Switch.
- [ ] **Step 3: Connect Live Signal Dispatch**
  - Slider adjustments instantly update the active desktop hotspot via `HotspotWindow::apply_config`.
- [ ] **Step 4: Run manual and automated integration checks**
- [ ] **Step 5: Commit changes** (`feat(settings): add edge hotspot studio and re-categorize main menu`)

---

### Task 5: End-to-End Verification, Performance Optimization & v0.2.0 Release

**Files:**
- Modify: `Cargo.toml`
- Modify: `scripts/release.sh`
- Modify: `README.md`

- [ ] **Step 1: Run full test suite (`cargo test`) across naming, media, math, and UI modules**
- [ ] **Step 2: Profile memory and CPU footprint**
  - Confirm idle memory $< 20 \text{ MB}$.
  - Confirm CPU at $0.0\%$ during idle.
- [ ] **Step 3: Execute `./scripts/release.sh 0.2.0`**
  - Build optimized release binary.
  - Package distribution tarballs.
  - Push git commit, tag `v0.2.0`, and upload release assets to GitHub.
- [ ] **Step 4: Final verification of `hyprdrop --version` and daemon status**
