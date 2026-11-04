# OmniRing UI Redesign & Feature Development Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform OmniRing with a modern Logitech Options+ Bubble UI overlay (`bubbles.webp`), a dedicated Action Library & Profile Configurator desktop window (`menu.png`), and an advanced multi-mode input trigger system (Hold, Keyboard+Mouse Combo, Corner Hover, Esc dismiss, and Outside-Click auto-close).

**Architecture:**
- **Overlay Window (`bubbles.webp`)**: Lightweight non-activating transparent canvas overlay rendering 8 circular bubble nodes with white icons, outer floating pill labels with subtle shadows, and a central dismiss hub (`×`).
- **Configurator Window (`menu.png`)**: Separate desktop window featuring a top profile switcher strip, live interactive radial preview, and a searchable accordion action library sidebar for configuring slots, shortcuts, and trigger gestures.
- **Native Input Engine (Win32 & Rust)**: Extended low-level mouse and keyboard hooks supporting configurable trigger modes (Hold, Combo via `GetAsyncKeyState`, Corner Hover dwell), outside-click auto-close, and instant `VK_ESCAPE` dismissal.

**Tech Stack:** Tauri v2, Rust 2021, Win32 API (`windows` crate 0.62), HTML5 Canvas 2D, Vanilla TypeScript, CSS3, Vite 6.

**Spec:** Decisions aligned during Grill Me session:
1. Two separate window modes: live overlay (`bubbles.webp`) and configurator (`menu.png`).
2. Declarative trigger config supporting `Hold`, `Combo`, and `CornerHover`.
3. Native dismiss handling via `WH_KEYBOARD_LL` (Esc) and `WH_MOUSE_LL` (outside-click auto-close).
4. Pure Canvas 2D rendering for bubble nodes, floating pill badges, and Fitts's Law angular hit-testing.

## Global Constraints
- **Zero Local Rust Invocation**: Do NOT execute `cargo`, `rustc`, `cargo check`, or `cargo build` on the Windows host.
- **Local Verification**: Static analysis, JSON schema validation, unit test inspection, and `npm run build` (`tsc && vite build`).
- **Communication Style**: Pragmatic, direct, evidence-based, no em dashes.

## Review Focus
1. **Focus Stealing Prevention**: Overlay window must retain `WS_EX_NOACTIVATE | WS_EX_TOPMOST` so foreground apps never lose text cursor focus.
2. **Esc Key Suppression**: When Esc cancels an active radial menu, the key event must be cleanly suppressed (`LRESULT(1)`) so it does not cancel work in the underlying application.
3. **Outside Click Boundary Clamping**: Clicking outside the outer radius of the radial menu when active must dismiss the overlay immediately without blocking the click from reaching background apps.
4. **Combo Trigger Accuracy**: In Combo mode (e.g., Ctrl + Right Click), ordinary Right Click without Ctrl must pass through unaffected to system context menus.
5. **High-DPI Canvas Alignment**: Bubble nodes and text labels must render razor-sharp on 100%, 125%, 150%, and 200% Windows display scaling without pixelation or coordinate drift.

---

### Task 1: Redesign Canvas Polar Renderer to Logitech Options+ Bubble Nodes UI

**Files:**
- Modify: `src/scripts/canvas_renderer.ts`
- Modify: `src/styles/overlay.css`
- Test: `test-renderer.mjs` (scratch verification script)

**Interfaces:**
- Consumes: `RingConfig`, `SliceConfig`, `RadialPlacement`, relative cursor coordinates `(x, y)`.
- Produces: `CanvasRenderer` rendering circular bubble nodes, outward floating pill labels, central dismiss button (`×`), hover states, and Fitts's Law angular hit testing.

- [x] **Step 1: Write verification script for bubble node geometry and hit testing**

Create `test-renderer.mjs` validating:
- Center hub cancellation radius (< 26px returns `-1` or special hub indicator).
- 8-node angular sector distribution with 45-degree intervals.
- Bubble node hit detection and outer boundary tolerance.

- [x] **Step 2: Run verification script to confirm initial failure**

Run: `node test-renderer.mjs`
Expected: FAIL (or verify differences between old annular pie and new bubble nodes).

- [x] **Step 3: Implement Bubble Nodes UI in `src/scripts/canvas_renderer.ts`**

Implement:
1. Node layout: center hub dismiss circle (`r = 22px`) with `×` glyph.
2. Bubble action circles: black/dark nodes (`r = 24px`) placed on an inner orbital radius (`r = 95px`), with clean white vector glyphs.
3. Floating pill badges: white rounded rectangles (`border-radius: 6px`, `box-shadow: 0 4px 12px rgba(0,0,0,0.12)`) on outer orbital radius (`r = 175px`) aligned along slice radial vectors.
4. Active hover effects:
   - Bubble node scales to 1.12x with subtle white/accent stroke and drop glow.
   - Pill badge gains high contrast or accent border.
5. Angular + radial hit testing adhering to Fitts's Law.

- [x] **Step 4: Update CSS in `src/styles/overlay.css`**

Ensure canvas container has zero background, hardware-accelerated transforms, and clean pointer events.

- [x] **Step 5: Run tests and frontend build to verify**

Run: `node test-renderer.mjs && npm run build`
Expected: PASS (0 errors).

---

### Task 2: Advanced Native Trigger System & Auto-Close Engine (Rust Backend)

**Files:**
- Modify: `src-tauri/src/hook/worker.rs`
- Modify: `src-tauri/src/gesture/detector.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: Win32 `WH_MOUSE_LL`, `WH_KEYBOARD_LL`, `GetAsyncKeyState`.
- Produces: `TriggerMode` (`Hold`, `Combo`, `CornerHover`), outside-click auto-close, Esc cancellation hook, and Tauri command `set_trigger_mode`.

- [x] **Step 1: Define `TriggerMode` and `ComboConfig` models**

In `src-tauri/src/hook/worker.rs` & `src-tauri/src/gesture/detector.rs`:
- `TriggerMode::Hold { button: MouseButton, hold_ms: u32 }`
- `TriggerMode::Combo { modifiers: Vec<String>, button: MouseButton }`
- `TriggerMode::CornerHover { corner: String, dwell_ms: u32 }`

- [x] **Step 2: Implement Combo detection via `GetAsyncKeyState`**

In `hook::worker`:
- When a mouse event occurs matching the trigger button:
  - If mode is `Combo`, check modifier state (`VK_CONTROL`, `VK_SHIFT`, `VK_MENU`).
  - If modifiers are held, intercept and trigger radial open.
  - If modifiers are not held, pass through immediately to system.

- [x] **Step 3: Implement Outside-Click Auto-Close and Esc Hook**

In `hook::worker`:
- Install lightweight `WH_KEYBOARD_LL`: when state is `RadialActive` and `VK_ESCAPE` is pressed, cancel radial menu, emit `RadialCancel`, hide overlay, and return `LRESULT(1)`.
- In `WH_MOUSE_LL`: when state is `RadialActive` and mouse down occurs outside `outer_radius + tolerance`, emit `RadialCancel` and hide overlay.

- [x] **Step 4: Expose and register Tauri commands in `src-tauri/src/lib.rs`**

Add `get_trigger_mode` and `set_trigger_mode` commands and register them with `tauri::generate_handler!`.

- [x] **Step 5: Verify Rust syntax and module consistency via static inspection script**

Run: Python static validation script checking all structs, enums, command registrations, and unit tests.

---

### Task 3: Configurator Window UI & Action Library (`menu.png`)

**Files:**
- Create: `config.html`
- Create: `src/scripts/config.ts`
- Create: `src/styles/config.css`
- Modify: `vite.config.ts`
- Modify: `src-tauri/tauri.conf.json`

**Interfaces:**
- Consumes: Tauri profile commands (`list_available_profiles`, `get_active_profile`, `reload_profiles`, `open_config_folder`, `set_trigger_mode`).
- Produces: Interactive desktop configurator with top profile bar, 8-slot wheel visualizer, searchable Action Library drawer, and gesture trigger settings.

- [x] **Step 1: Configure Multi-Page Build in `vite.config.ts` and `tauri.conf.json`**

- In `vite.config.ts`: configure `rollupOptions.input` with `main: index.html` and `config: config.html`.
- In `tauri.conf.json`: register `configurator` window (`url: "config.html"`, `width: 980`, `height: 680`, `visible: false`).

- [x] **Step 2: Create `config.html` structure based on `menu.png`**

- Top app bar: profile tabs (`System / Default`, `VS Code`, `Notepad`, `+ Add Profile`).
- Main layout:
  - Left canvas (70%): Live interactive 8-slot preview wheel with bubble nodes, floating labels, slot selection highlight, and mode switchers.
  - Right sidebar (30%): Action Library header (`ALL ACTIONS`), live search bar, and collapsible category accordions:
    - `System Actions` (Lock, Volume, Media, Screenshot)
    - `Device Navigation & Folders` (Explorer, Back, Switch App)
    - `Keyboard Shortcuts` (Ctrl+C, Ctrl+V, Undo, Redo, Custom)
    - `App Launchers & URLs` (Browser, Terminal, Custom App)
  - Trigger & Settings Tab:
    - Mode selector: Hold (X1/Middle), Combo (Ctrl+RightClick), Corner Hover.

- [x] **Step 3: Implement `src/styles/config.css`**

- Modern desktop styling matching `menu.png`: clean sans-serif typography, light theme (`#F8F9FA` background, `#FFFFFF` panels, `#E4E4E7` borders, `#5E48E8` accent purple).
- Smooth accordion animations, responsive sidebar layout, and badge shadows.

- [x] **Step 4: Implement `src/scripts/config.ts`**

- Connect to Tauri IPC: load profiles, switch active profile, update slot actions, save changes to disk, and sync with overlay.
- Search filter filtering action cards in real-time.
- Interactive slot binding: click slot on preview wheel -> click action card to assign.

- [x] **Step 5: Verify build with `npm run build`**

Run: `npm run build`
Expected: Successful compilation of `index.html` and `config.html` bundles into `dist/`.

---

### Task 4: System Tray Integration & End-to-End Interaction Coordination

**Files:**
- Modify: `src-tauri/src/tray/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/scripts/main.ts`

**Interfaces:**
- Consumes: System Tray menu clicks, overlay IPC events (`radial-open`, `radial-release`, `cancelled`).
- Produces: Seamless user flow: open configurator via tray, auto-close overlay on Esc, outside click, or selection.

- [x] **Step 1: Add "Configure OmniRing" to Tray Menu**

In `src-tauri/src/tray/mod.rs`:
- Add menu item `Configure OmniRing` (`open_configurator`) which shows and focuses the `configurator` window.
- Implement command `open_configurator_window`.

- [x] **Step 2: Update `src/scripts/main.ts` for New Bubble Interaction**

- Handle center dismiss hub click -> cancel and hide overlay.
- Handle outside click and Escape key within the webview fallback.
- Auto-close window before executing terminal slice action.

- [x] **Step 3: Run comprehensive verification**

- Run `npm run build`.
- Execute static analysis and unit tests on Rust backend.
- Validate JSON schemas and profile structures.

---

### Task 5: Memory Bank Update & Documentation Review

**Files:**
- Modify: `memory-bank/architecture.md`
- Modify: `memory-bank/design-document.md`
- Modify: `memory-bank/progress.md`
- Modify: `README.md`

- [x] **Step 1: Update architecture and design documents**

Document new Bubble Nodes UI, Configurator window, and multi-mode trigger system.

- [x] **Step 2: Update progress log**

Record all completed tasks, verification outcomes, and release readiness.
