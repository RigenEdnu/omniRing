import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { CanvasRenderer, RingConfig, SliceConfig, ActionConfig } from "./canvas_renderer";

export interface ProfileConfig {
  profile_name: string;
  process_targets: string[];
  inner_radius: number;
  outer_radius: number;
  center_action?: ActionConfig;
  slices: SliceConfig[];
  submenus?: Record<string, { title?: string; slices: SliceConfig[] }>;
}

export type TriggerMode =
  | { mode: "Hold"; button: string; hold_ms: number }
  | { mode: "Combo"; modifiers: string[]; button: string }
  | { mode: "CornerHover"; corner: string; dwell_ms: number };

export interface LibraryActionItem {
  id: string;
  label: string;
  description: string;
  icon: string;
  category: "system" | "navigation" | "shortcuts" | "launchers";
  badge?: string;
  action: ActionConfig;
}

// Built-in Action Library items based on menu.png reference
export const ACTION_LIBRARY: LibraryActionItem[] = [
  // 1. System Actions
  {
    id: "act_play_pause",
    label: "Play / Pause",
    description: "Toggle audio/video playback",
    icon: "play-pause",
    category: "system",
    badge: "MediaPlayPause",
    action: { type: "Shortcut", keys: ["MediaPlayPause"] },
  },
  {
    id: "act_sys_volume",
    label: "System Volume Mute",
    description: "Mute or unmute system audio",
    icon: "options",
    category: "system",
    badge: "AudioVolumeMute",
    action: { type: "Shortcut", keys: ["AudioVolumeMute"] },
  },
  {
    id: "act_next_track",
    label: "Next Track",
    description: "Skip to next music track",
    icon: "play-pause",
    category: "system",
    badge: "MediaNextTrack",
    action: { type: "Shortcut", keys: ["MediaNextTrack"] },
  },
  {
    id: "act_prev_track",
    label: "Previous Track",
    description: "Return to previous track",
    icon: "play-pause",
    category: "system",
    badge: "MediaPrevTrack",
    action: { type: "Shortcut", keys: ["MediaPrevTrack"] },
  },
  {
    id: "act_brightness",
    label: "Screen Brightness",
    description: "Open display settings",
    icon: "options",
    category: "system",
    badge: "Display",
    action: { type: "OpenApp", command: "ms-settings:display" },
  },
  {
    id: "act_lock",
    label: "Lock Workstation",
    description: "Lock Windows and secure screen",
    icon: "lock",
    category: "system",
    badge: "Win + L",
    action: { type: "Shortcut", keys: ["Meta", "l"] },
  },
  {
    id: "act_screenshot",
    label: "Take Screenshot",
    description: "Launch Windows Snipping Tool",
    icon: "screenshot",
    category: "system",
    badge: "Win + Shift + S",
    action: { type: "Shortcut", keys: ["Meta", "Shift", "s"] },
  },
  {
    id: "act_emoji",
    label: "Emoji & Symbols",
    description: "Open Windows emoji picker",
    icon: "emoji",
    category: "system",
    badge: "Win + .",
    action: { type: "Shortcut", keys: ["Meta", "."] },
  },

  // 2. Device Navigation & Folders
  {
    id: "act_explorer",
    label: "File Explorer",
    description: "Open Windows File Explorer",
    icon: "folder",
    category: "navigation",
    badge: "Win + E",
    action: { type: "Shortcut", keys: ["Meta", "e"] },
  },
  {
    id: "act_switch_app",
    label: "Switch App / Task View",
    description: "View open windows and desktops",
    icon: "layout",
    category: "navigation",
    badge: "Win + Tab",
    action: { type: "Shortcut", keys: ["Meta", "Tab"] },
  },
  {
    id: "act_browser_back",
    label: "Browser Back",
    description: "Navigate to previous page",
    icon: "folder",
    category: "navigation",
    badge: "Alt + Left",
    action: { type: "Shortcut", keys: ["Alt", "ArrowLeft"] },
  },
  {
    id: "act_browser_forward",
    label: "Browser Forward",
    description: "Navigate forward to next page",
    icon: "folder",
    category: "navigation",
    badge: "Alt + Right",
    action: { type: "Shortcut", keys: ["Alt", "ArrowRight"] },
  },
  {
    id: "act_desktop",
    label: "Show Desktop",
    description: "Minimize all windows",
    icon: "layout",
    category: "navigation",
    badge: "Win + D",
    action: { type: "Shortcut", keys: ["Meta", "d"] },
  },

  // 3. Keyboard Shortcuts
  {
    id: "act_copy",
    label: "Copy",
    description: "Copy selection to clipboard",
    icon: "copy",
    category: "shortcuts",
    badge: "Ctrl + C",
    action: { type: "Shortcut", keys: ["Control", "c"] },
  },
  {
    id: "act_paste",
    label: "Paste",
    description: "Paste clipboard contents",
    icon: "paste",
    category: "shortcuts",
    badge: "Ctrl + V",
    action: { type: "Shortcut", keys: ["Control", "v"] },
  },
  {
    id: "act_cut",
    label: "Cut",
    description: "Cut selection to clipboard",
    icon: "copy",
    category: "shortcuts",
    badge: "Ctrl + X",
    action: { type: "Shortcut", keys: ["Control", "x"] },
  },
  {
    id: "act_undo",
    label: "Undo",
    description: "Revert the last change",
    icon: "undo",
    category: "shortcuts",
    badge: "Ctrl + Z",
    action: { type: "Shortcut", keys: ["Control", "z"] },
  },
  {
    id: "act_redo",
    label: "Redo",
    description: "Reapply reverted change",
    icon: "redo",
    category: "shortcuts",
    badge: "Ctrl + Y",
    action: { type: "Shortcut", keys: ["Control", "y"] },
  },
  {
    id: "act_save",
    label: "Save Document",
    description: "Save active file",
    icon: "code",
    category: "shortcuts",
    badge: "Ctrl + S",
    action: { type: "Shortcut", keys: ["Control", "s"] },
  },
  {
    id: "act_select_all",
    label: "Select All",
    description: "Select all items or text",
    icon: "code",
    category: "shortcuts",
    badge: "Ctrl + A",
    action: { type: "Shortcut", keys: ["Control", "a"] },
  },
  {
    id: "act_new_note",
    label: "New Note / Document",
    description: "Create new file or note",
    icon: "new-note",
    category: "shortcuts",
    badge: "Ctrl + N",
    action: { type: "Shortcut", keys: ["Control", "n"] },
  },

  // 4. App Launchers & Web URLs
  {
    id: "act_chatgpt",
    label: "ChatGPT AI",
    description: "Open ChatGPT in default browser",
    icon: "sparkles",
    category: "launchers",
    badge: "https://chatgpt.com",
    action: { type: "OpenUrl", url: "https://chatgpt.com" },
  },
  {
    id: "act_claude",
    label: "Claude AI",
    description: "Open Anthropic Claude AI",
    icon: "sparkles",
    category: "launchers",
    badge: "https://claude.ai",
    action: { type: "OpenUrl", url: "https://claude.ai" },
  },
  {
    id: "act_google",
    label: "Google Search",
    description: "Open Google in default browser",
    icon: "search",
    category: "launchers",
    badge: "https://google.com",
    action: { type: "OpenUrl", url: "https://www.google.com" },
  },
  {
    id: "act_terminal",
    label: "Windows Terminal",
    description: "Launch Windows Terminal",
    icon: "terminal",
    category: "launchers",
    badge: "wt.exe",
    action: { type: "OpenApp", command: "wt.exe" },
  },
  {
    id: "act_powershell",
    label: "PowerShell",
    description: "Launch PowerShell CLI",
    icon: "terminal",
    category: "launchers",
    badge: "powershell.exe",
    action: { type: "OpenApp", command: "powershell.exe" },
  },
  {
    id: "act_vscode",
    label: "VS Code",
    description: "Launch Visual Studio Code",
    icon: "code",
    category: "launchers",
    badge: "code",
    action: { type: "OpenApp", command: "code" },
  },
  {
    id: "act_settings",
    label: "Windows Settings",
    description: "Open Windows System Settings",
    icon: "options",
    category: "launchers",
    badge: "ms-settings:",
    action: { type: "OpenApp", command: "ms-settings:" },
  },
];

// Fallback profiles when Tauri IPC is not available (browser dev/preview)
const MOCK_PROFILES: ProfileConfig[] = [
  {
    profile_name: "Default Global",
    process_targets: ["*"],
    inner_radius: 26,
    outer_radius: 200,
    slices: [
      { id: "s1", label: "Play/Pause", icon: "play-pause", action: { type: "Shortcut", keys: ["MediaPlayPause"] } },
      { id: "s2", label: "New note", icon: "new-note", action: { type: "Shortcut", keys: ["Control", "n"] } },
      { id: "s3", label: "Explore AI", icon: "sparkles", action: { type: "OpenUrl", url: "https://chatgpt.com" } },
      { id: "s4", label: "Lock workstation", icon: "lock", action: { type: "Shortcut", keys: ["Meta", "l"] } },
      { id: "s5", label: "Settings", icon: "options", action: { type: "OpenApp", command: "ms-settings:" } },
      { id: "s6", label: "Screenshot", icon: "screenshot", action: { type: "Shortcut", keys: ["Meta", "Shift", "s"] } },
      { id: "s7", label: "Emoji", icon: "emoji", action: { type: "Shortcut", keys: ["Meta", "."] } },
      { id: "s8", label: "Files", icon: "folder", action: { type: "Shortcut", keys: ["Meta", "e"] } },
    ],
  },
  {
    profile_name: "VS Code",
    process_targets: ["code.exe", "code"],
    inner_radius: 26,
    outer_radius: 200,
    slices: [
      { id: "vc_format", label: "Format Code", icon: "code", action: { type: "Shortcut", keys: ["Shift", "Alt", "f"] } },
      { id: "vc_terminal", label: "Terminal", icon: "terminal", action: { type: "Shortcut", keys: ["Control", "`"] } },
      { id: "vc_search", label: "Search All", icon: "search", action: { type: "Shortcut", keys: ["Control", "Shift", "f"] } },
      { id: "vc_sidebar", label: "Toggle Sidebar", icon: "layout", action: { type: "Shortcut", keys: ["Control", "b"] } },
      { id: "vc_command", label: "Command Palette", icon: "options", action: { type: "Shortcut", keys: ["Control", "Shift", "p"] } },
      { id: "vc_git", label: "Source Control", icon: "sparkles", action: { type: "Shortcut", keys: ["Control", "Shift", "g"] } },
      { id: "vc_comment", label: "Toggle Comment", icon: "copy", action: { type: "Shortcut", keys: ["Control", "/"] } },
      { id: "vc_save", label: "Save All", icon: "code", action: { type: "Shortcut", keys: ["Control", "k", "s"] } },
    ],
  },
  {
    profile_name: "Notepad",
    process_targets: ["notepad.exe", "notepad"],
    inner_radius: 26,
    outer_radius: 200,
    slices: [
      { id: "np_copy", label: "Copy", icon: "copy", action: { type: "Shortcut", keys: ["Control", "c"] } },
      { id: "np_paste", label: "Paste", icon: "paste", action: { type: "Shortcut", keys: ["Control", "v"] } },
      { id: "np_cut", label: "Cut", icon: "copy", action: { type: "Shortcut", keys: ["Control", "x"] } },
      { id: "np_undo", label: "Undo", icon: "undo", action: { type: "Shortcut", keys: ["Control", "z"] } },
      { id: "np_save", label: "Save", icon: "code", action: { type: "Shortcut", keys: ["Control", "s"] } },
      { id: "np_find", label: "Find", icon: "search", action: { type: "Shortcut", keys: ["Control", "f"] } },
      { id: "np_all", label: "Select All", icon: "code", action: { type: "Shortcut", keys: ["Control", "a"] } },
      { id: "np_new", label: "New Window", icon: "new-note", action: { type: "Shortcut", keys: ["Control", "Shift", "n"] } },
    ],
  },
];

class ConfiguratorApp {
  private profiles: ProfileConfig[] = [];
  private activeProfile: ProfileConfig | null = null;
  private selectedSlotIndex: number | null = null;
  private renderer: CanvasRenderer | null = null;
  private canvas: HTMLCanvasElement;
  private undoStack: SliceConfig[][] = [];
  private redoStack: SliceConfig[][] = [];

  constructor() {
    this.canvas = document.getElementById("config-wheel-canvas") as HTMLCanvasElement;
    this.init();
  }

  private async init(): Promise<void> {
    this.initCanvasRenderer();
    this.setupViewTabs();
    this.setupAccordion();
    this.setupActionLibrary();
    this.setupSearch();
    this.setupTriggerSettings();
    this.setupModals();
    this.setupDockControls();
    await this.loadProfiles();
  }

  private initCanvasRenderer(): void {
    if (!this.canvas) return;

    this.renderer = new CanvasRenderer(this.canvas, {
      profile_name: "Default Global",
      inner_radius: 26,
      outer_radius: 200,
      slices: [],
    });

    this.canvas.addEventListener("mousemove", (e) => {
      const rect = this.canvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;

      if (!this.renderer) return;
      const isHub = this.renderer.isCenterHub(x, y);
      if (isHub) {
        this.renderer.setActiveSlice(null);
      } else {
        const hit = this.renderer.hitTest(x, y);
        this.renderer.setActiveSlice(hit === -1 ? null : hit);
      }
      this.drawWheel();
    });

    this.canvas.addEventListener("mouseleave", () => {
      if (!this.renderer) return;
      this.renderer.setActiveSlice(null);
      this.drawWheel();
    });

    this.canvas.addEventListener("click", (e) => {
      const rect = this.canvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;

      if (!this.renderer) return;
      if (this.renderer.isCenterHub(x, y)) {
        this.selectSlot(null);
        return;
      }

      const hit = this.renderer.hitTest(x, y);
      if (hit !== -1) {
        this.selectSlot(hit === this.selectedSlotIndex ? null : hit);
      }
    });
  }

  private drawWheel(): void {
    if (!this.renderer) return;
    this.renderer.render();

    // Draw active slot highlight ring
    if (this.selectedSlotIndex !== null && this.activeProfile) {
      const slices = this.activeProfile.slices;
      if (this.selectedSlotIndex >= 0 && this.selectedSlotIndex < slices.length) {
        const { mid } = this.renderer.getSliceAngles(this.selectedSlotIndex);
        const center = this.renderer.getCenter();
        const nodeX = center.x + this.renderer.nodeOrbitalRadius * Math.cos(mid);
        const nodeY = center.y + this.renderer.nodeOrbitalRadius * Math.sin(mid);

        const ctx = this.canvas.getContext("2d");
        if (ctx) {
          ctx.save();
          ctx.beginPath();
          ctx.arc(nodeX, nodeY, this.renderer.nodeRadius * 1.35, 0, Math.PI * 2);
          ctx.strokeStyle = "#5E48E8";
          ctx.lineWidth = 3.5;
          ctx.shadowColor = "rgba(94, 72, 232, 0.75)";
          ctx.shadowBlur = 16;
          ctx.stroke();

          // Outer dashed guide ring
          ctx.beginPath();
          ctx.arc(nodeX, nodeY, this.renderer.nodeRadius * 1.6, 0, Math.PI * 2);
          ctx.strokeStyle = "rgba(94, 72, 232, 0.4)";
          ctx.lineWidth = 1.5;
          ctx.setLineDash([4, 4]);
          ctx.stroke();
          ctx.restore();
        }
      }
    }
  }

  private selectSlot(index: number | null): void {
    this.selectedSlotIndex = index;
    const banner = document.getElementById("slot-selection-banner");
    const clearBtn = document.getElementById("btn-clear-slot") as HTMLButtonElement;

    if (banner) {
      if (index !== null && this.activeProfile && this.activeProfile.slices[index]) {
        const slice = this.activeProfile.slices[index];
        banner.className = "slot-selection-info active-slot";
        banner.innerHTML = `<span class="selection-status">Selected: <strong>Slot ${index + 1} (${slice.label})</strong> &bull; Click an action from library to assign</span>`;
        if (clearBtn) clearBtn.disabled = false;
      } else {
        banner.className = "slot-selection-info";
        banner.innerHTML = `<span class="selection-status">Click a slot on the ring to configure its action</span>`;
        if (clearBtn) clearBtn.disabled = true;
      }
    }

    this.drawWheel();
  }

  private async loadProfiles(): Promise<void> {
    try {
      const list = await invoke<ProfileConfig[]>("list_available_profiles");
      if (list && list.length > 0) {
        this.profiles = list;
      } else {
        this.profiles = [...MOCK_PROFILES];
      }
    } catch {
      this.profiles = [...MOCK_PROFILES];
    }

    try {
      const active = await invoke<ProfileConfig>("get_active_profile");
      if (active) {
        const found = this.profiles.find((p) => p.profile_name === active.profile_name);
        this.activeProfile = found || active;
      } else {
        this.activeProfile = this.profiles[0];
      }
    } catch {
      this.activeProfile = this.profiles[0];
    }

    try {
      await listen<ProfileConfig>("omniring://profile-changed", (event) => {
        if (event.payload && event.payload.profile_name) {
          const incoming = event.payload;
          const idx = this.profiles.findIndex((p) => p.profile_name === incoming.profile_name);
          if (idx >= 0) {
            this.profiles[idx] = incoming;
          } else {
            this.profiles.push(incoming);
            this.renderProfileTabs();
          }
          if (this.activeProfile?.profile_name === incoming.profile_name) {
            this.activeProfile = incoming;
            this.applyActiveProfile();
          }
        }
      });
    } catch {
      // Standalone browser mode fallback
    }

    this.renderProfileTabs();
    this.applyActiveProfile();
  }

  private async syncActiveProfile(): Promise<void> {
    if (!this.activeProfile) return;
    try {
      await invoke("save_profile", { profile: this.activeProfile });
    } catch {
      // Standalone browser mode fallback
    }
  }

  private renderProfileTabs(): void {
    const nav = document.getElementById("profile-tabs");
    if (!nav) return;
    nav.innerHTML = "";

    this.profiles.forEach((profile) => {
      const btn = document.createElement("button");
      btn.className = `profile-tab ${profile.profile_name === this.activeProfile?.profile_name ? "active" : ""}`;
      btn.innerHTML = `
        <svg class="tab-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="3" y="3" width="7" height="7"></rect>
          <rect x="14" y="3" width="7" height="7"></rect>
          <rect x="14" y="14" width="7" height="7"></rect>
          <rect x="3" y="14" width="7" height="7"></rect>
        </svg>
        <span>${profile.profile_name}</span>
      `;
      btn.addEventListener("click", () => this.switchProfile(profile.profile_name));
      nav.appendChild(btn);
    });
  }

  private async switchProfile(name: string): Promise<void> {
    const target = this.profiles.find((p) => p.profile_name === name);
    if (!target) return;

    try {
      await invoke("set_active_profile_by_name", { name });
    } catch {
      // Standalone browser mode fallback
    }

    this.activeProfile = target;
    this.selectedSlotIndex = null;
    this.undoStack = [];
    this.redoStack = [];
    this.renderProfileTabs();
    this.applyActiveProfile();
    this.showToast(`Switched to profile: ${name}`);
  }

  private applyActiveProfile(): void {
    if (!this.activeProfile || !this.renderer) return;

    const nameEl = document.getElementById("active-profile-name");
    const procEl = document.getElementById("active-profile-process");
    if (nameEl) nameEl.textContent = this.activeProfile.profile_name;
    if (procEl) {
      const targets = this.activeProfile.process_targets.join(", ");
      procEl.textContent = `Target: ${targets === "*" ? "All Applications (*)" : targets}`;
    }

    const ringConfig: RingConfig = {
      profile_name: this.activeProfile.profile_name,
      inner_radius: this.activeProfile.inner_radius || 26,
      outer_radius: this.activeProfile.outer_radius || 200,
      slices: this.activeProfile.slices,
      submenus: this.activeProfile.submenus,
    };

    this.renderer.setConfig(ringConfig);
    this.selectSlot(null);
    this.updateUndoRedoButtons();
  }

  private setupViewTabs(): void {
    const tabRing = document.getElementById("view-tab-ring");
    const tabTrigger = document.getElementById("view-tab-trigger");
    const secRing = document.getElementById("section-ring-editor");
    const secTrigger = document.getElementById("section-trigger-settings");

    if (tabRing && tabTrigger && secRing && secTrigger) {
      tabRing.addEventListener("click", () => {
        tabRing.classList.add("active");
        tabRing.setAttribute("aria-selected", "true");
        tabTrigger.classList.remove("active");
        tabTrigger.setAttribute("aria-selected", "false");
        secRing.classList.add("active");
        secTrigger.classList.remove("active");
        this.drawWheel();
      });

      tabTrigger.addEventListener("click", () => {
        tabTrigger.classList.add("active");
        tabTrigger.setAttribute("aria-selected", "true");
        tabRing.classList.remove("active");
        tabRing.setAttribute("aria-selected", "false");
        secTrigger.classList.add("active");
        secRing.classList.remove("active");
      });
    }

    const btnFolder = document.getElementById("btn-open-folder");
    if (btnFolder) {
      btnFolder.addEventListener("click", async () => {
        try {
          await invoke("open_config_folder");
          this.showToast("Opened profiles directory");
        } catch {
          this.showToast("Could not open config directory");
        }
      });
    }

    const btnReload = document.getElementById("btn-reload-profiles");
    if (btnReload) {
      btnReload.addEventListener("click", async () => {
        try {
          await invoke("reload_profiles");
          await this.loadProfiles();
          this.showToast("Profiles reloaded from disk");
        } catch {
          this.showToast("Reloaded profiles (local)");
        }
      });
    }
  }

  private setupAccordion(): void {
    const headers = document.querySelectorAll(".accordion-header");
    headers.forEach((header) => {
      header.addEventListener("click", () => {
        const group = header.closest(".accordion-group");
        if (group) {
          group.classList.toggle("expanded");
        }
      });
    });
  }

  private setupActionLibrary(): void {
    const categories: Record<string, string> = {
      system: "list-system-actions",
      navigation: "list-navigation-actions",
      shortcuts: "list-shortcut-actions",
      launchers: "list-launcher-actions",
    };

    Object.entries(categories).forEach(([cat, elId]) => {
      const container = document.getElementById(elId);
      if (!container) return;
      container.innerHTML = "";

      const items = ACTION_LIBRARY.filter((a) => a.category === cat);
      items.forEach((item) => {
        const card = document.createElement("div");
        card.className = "action-card";
        card.setAttribute("data-action-id", item.id);
        card.setAttribute("data-label", item.label.toLowerCase());
        card.setAttribute("data-desc", item.description.toLowerCase());
        card.setAttribute("data-badge", (item.badge || "").toLowerCase());

        card.innerHTML = `
          <div class="action-card-icon">
            ${this.getSvgIcon(item.icon)}
          </div>
          <div class="action-card-details">
            <span class="action-card-title">${item.label}</span>
            <span class="action-card-desc">${item.description}</span>
          </div>
          ${item.badge ? `<span class="action-card-badge">${item.badge}</span>` : ""}
        `;

        card.addEventListener("click", () => this.assignActionToSelectedSlot(item));
        container.appendChild(card);
      });
    });

    const countBadge = document.getElementById("action-count-badge");
    if (countBadge) {
      countBadge.textContent = `${ACTION_LIBRARY.length} Actions`;
    }
  }

  private getSvgIcon(name: string): string {
    switch (name) {
      case "play-pause":
        return `<svg viewBox="0 0 24 24"><polygon points="5 3 19 12 5 21 5 3"></polygon></svg>`;
      case "lock":
        return `<svg viewBox="0 0 24 24"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect><path d="M7 11V7a5 5 0 0 1 10 0v4"></path></svg>`;
      case "folder":
        return `<svg viewBox="0 0 24 24"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>`;
      case "copy":
        return `<svg viewBox="0 0 24 24"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>`;
      case "paste":
        return `<svg viewBox="0 0 24 24"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"></path><rect x="8" y="2" width="8" height="4" rx="1" ry="1"></rect></svg>`;
      case "undo":
        return `<svg viewBox="0 0 24 24"><polyline points="1 4 1 10 7 10"></polyline><path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"></path></svg>`;
      case "redo":
        return `<svg viewBox="0 0 24 24"><polyline points="23 4 23 10 17 10"></polyline><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path></svg>`;
      case "screenshot":
        return `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"></circle><line x1="22" y1="12" x2="18" y2="12"></line><line x1="6" y1="12" x2="2" y2="12"></line><line x1="12" y1="6" x2="12" y2="2"></line><line x1="12" y1="22" x2="12" y2="18"></line></svg>`;
      case "emoji":
        return `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"></circle><path d="M8 14s1.5 2 4 2 4-2 4-2"></path><line x1="9" y1="9" x2="9.01" y2="9"></line><line x1="15" y1="9" x2="15.01" y2="9"></line></svg>`;
      case "sparkles":
        return `<svg viewBox="0 0 24 24"><path d="M12 2v20M2 12h20M4.93 4.93l14.14 14.14M4.93 19.07l14.14-14.14"></path></svg>`;
      case "search":
        return `<svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line></svg>`;
      case "terminal":
        return `<svg viewBox="0 0 24 24"><polyline points="4 17 10 11 4 5"></polyline><line x1="12" y1="19" x2="20" y2="19"></line></svg>`;
      case "layout":
        return `<svg viewBox="0 0 24 24"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect><line x1="3" y1="9" x2="21" y2="9"></line><line x1="9" y1="21" x2="9" y2="9"></line></svg>`;
      case "new-note":
        return `<svg viewBox="0 0 24 24"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path></svg>`;
      case "code":
        return `<svg viewBox="0 0 24 24"><polyline points="16 18 22 12 16 6"></polyline><polyline points="8 6 2 12 8 18"></polyline></svg>`;
      default:
        return `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path></svg>`;
    }
  }

  private assignActionToSelectedSlot(item: LibraryActionItem): void {
    if (this.selectedSlotIndex === null) {
      this.showToast("Please click a slot on the ring first to assign this action");
      return;
    }

    if (!this.activeProfile) return;

    this.pushUndo();

    const slotIndex = this.selectedSlotIndex;
    const existing = this.activeProfile.slices[slotIndex];
    const newSlice: SliceConfig = {
      id: existing ? existing.id : `slice_${Date.now()}_${slotIndex}`,
      label: item.label,
      icon: item.icon,
      action: { ...item.action },
    };

    this.activeProfile.slices[slotIndex] = newSlice;
    this.applyActiveProfile();
    this.selectSlot(slotIndex);
    this.syncActiveProfile();
    this.showToast(`Assigned "${item.label}" to Slot ${slotIndex + 1}`);
  }

  private setupSearch(): void {
    const input = document.getElementById("action-search-input") as HTMLInputElement;
    const clearBtn = document.getElementById("btn-clear-search");
    if (!input) return;

    input.addEventListener("input", () => {
      const q = input.value.trim().toLowerCase();
      if (clearBtn) clearBtn.style.display = q.length > 0 ? "block" : "none";

      const cards = document.querySelectorAll(".action-card");
      let visibleCount = 0;

      cards.forEach((card) => {
        const title = card.getAttribute("data-label") || "";
        const desc = card.getAttribute("data-desc") || "";
        const badge = card.getAttribute("data-badge") || "";

        const match = q.length === 0 || title.includes(q) || desc.includes(q) || badge.includes(q);
        (card as HTMLElement).style.display = match ? "flex" : "none";
        if (match) visibleCount++;
      });

      const countBadge = document.getElementById("action-count-badge");
      if (countBadge) {
        countBadge.textContent = `${visibleCount} Actions`;
      }
    });

    if (clearBtn) {
      clearBtn.addEventListener("click", () => {
        input.value = "";
        input.dispatchEvent(new Event("input"));
        input.focus();
      });
    }
  }

  private setupTriggerSettings(): void {
    const radios = document.querySelectorAll('input[name="trigger-mode"]') as NodeListOf<HTMLInputElement>;
    const holdSub = document.getElementById("subsettings-hold");
    const comboSub = document.getElementById("subsettings-combo");
    const cornerSub = document.getElementById("subsettings-corner");

    const updateVisibility = (mode: string) => {
      if (holdSub) holdSub.style.display = mode === "Hold" ? "flex" : "none";
      if (comboSub) comboSub.style.display = mode === "Combo" ? "flex" : "none";
      if (cornerSub) cornerSub.style.display = mode === "CornerHover" ? "flex" : "none";
    };

    radios.forEach((r) => {
      r.addEventListener("change", () => {
        if (r.checked) updateVisibility(r.value);
      });
    });

    const holdRange = document.getElementById("hold-delay-range") as HTMLInputElement;
    const holdVal = document.getElementById("hold-delay-val");
    if (holdRange && holdVal) {
      holdRange.addEventListener("input", () => {
        holdVal.textContent = `${holdRange.value} ms`;
      });
    }

    const cornerRange = document.getElementById("corner-delay-range") as HTMLInputElement;
    const cornerVal = document.getElementById("corner-delay-val");
    if (cornerRange && cornerVal) {
      cornerRange.addEventListener("input", () => {
        cornerVal.textContent = `${cornerRange.value} ms`;
      });
    }

    // Load trigger mode from Tauri backend
    this.loadTriggerMode();

    const saveBtn = document.getElementById("btn-save-trigger");
    if (saveBtn) {
      saveBtn.addEventListener("click", async () => {
        const checked = document.querySelector('input[name="trigger-mode"]:checked') as HTMLInputElement;
        const mode = checked ? checked.value : "Hold";

        let triggerPayload: TriggerMode;

        if (mode === "Hold") {
          const btn = (document.getElementById("hold-button-select") as HTMLSelectElement).value;
          const delay = parseInt(holdRange.value, 10) || 200;
          triggerPayload = { mode: "Hold", button: btn, hold_ms: delay };
        } else if (mode === "Combo") {
          const mods: string[] = [];
          if ((document.getElementById("mod-ctrl") as HTMLInputElement)?.checked) mods.push("Control");
          if ((document.getElementById("mod-shift") as HTMLInputElement)?.checked) mods.push("Shift");
          if ((document.getElementById("mod-alt") as HTMLInputElement)?.checked) mods.push("Alt");
          if ((document.getElementById("mod-win") as HTMLInputElement)?.checked) mods.push("Win");
          const btn = (document.getElementById("combo-button-select") as HTMLSelectElement).value;
          triggerPayload = { mode: "Combo", modifiers: mods, button: btn };
        } else {
          const corner = (document.getElementById("corner-select") as HTMLSelectElement).value;
          const dwell = parseInt(cornerRange.value, 10) || 400;
          triggerPayload = { mode: "CornerHover", corner, dwell_ms: dwell };
        }

        try {
          await invoke("set_trigger_mode", { mode: triggerPayload });
          this.showToast(`Trigger mode updated to: ${mode}`);
        } catch {
          this.showToast(`Saved trigger settings (${mode})`);
        }
      });
    }
  }

  private async loadTriggerMode(): Promise<void> {
    try {
      const modeData = await invoke<TriggerMode>("get_trigger_mode");
      if (modeData && modeData.mode) {
        const radio = document.querySelector(`input[name="trigger-mode"][value="${modeData.mode}"]`) as HTMLInputElement;
        if (radio) {
          radio.checked = true;
          radio.dispatchEvent(new Event("change"));
        }
      }
    } catch {
      // Keep default
    }
  }

  private setupModals(): void {
    // Add Profile Modal
    const btnAddProfile = document.getElementById("btn-add-profile");
    const modalProfile = document.getElementById("modal-add-profile");
    const btnCloseProfile = document.getElementById("btn-close-modal-profile");
    const btnCancelProfile = document.getElementById("btn-cancel-add-profile");
    const btnConfirmProfile = document.getElementById("btn-confirm-add-profile");

    const openProfileModal = () => {
      if (modalProfile) modalProfile.style.display = "flex";
      const nameInput = document.getElementById("input-profile-name") as HTMLInputElement;
      if (nameInput) {
        nameInput.value = "";
        nameInput.focus();
      }
    };

    const closeProfileModal = () => {
      if (modalProfile) modalProfile.style.display = "none";
    };

    if (btnAddProfile) btnAddProfile.addEventListener("click", openProfileModal);
    if (btnCloseProfile) btnCloseProfile.addEventListener("click", closeProfileModal);
    if (btnCancelProfile) btnCancelProfile.addEventListener("click", closeProfileModal);

    if (btnConfirmProfile) {
      btnConfirmProfile.addEventListener("click", () => {
        const nameInput = document.getElementById("input-profile-name") as HTMLInputElement;
        const targetInput = document.getElementById("input-process-target") as HTMLInputElement;
        const templateSelect = document.getElementById("select-profile-template") as HTMLSelectElement;

        const name = (nameInput?.value || "").trim();
        const target = (targetInput?.value || "").trim();
        if (!name) {
          this.showToast("Please enter a profile name");
          return;
        }

        let slices: SliceConfig[] = [];
        if (templateSelect.value === "default" && this.profiles.length > 0) {
          slices = JSON.parse(JSON.stringify(this.profiles[0].slices));
        } else if (templateSelect.value === "code") {
          const codeProf = this.profiles.find((p) => p.profile_name === "VS Code");
          slices = codeProf ? JSON.parse(JSON.stringify(codeProf.slices)) : [];
        } else {
          // Empty ring with 8 blank slots
          for (let i = 0; i < 8; i++) {
            slices.push({
              id: `slot_${i + 1}`,
              label: `Slot ${i + 1}`,
              icon: "options",
              action: { type: "Shortcut", keys: ["Escape"] },
            });
          }
        }

        const newProfile: ProfileConfig = {
          profile_name: name,
          process_targets: target ? [target] : [name.toLowerCase() + ".exe"],
          inner_radius: 26,
          outer_radius: 200,
          slices,
        };

        this.profiles.push(newProfile);
        this.activeProfile = newProfile;
        closeProfileModal();
        this.syncActiveProfile();
        this.switchProfile(name);
        this.showToast(`Created new profile: ${name}`);
      });
    }

    // Custom Action Modal
    const btnCustomAction = document.getElementById("btn-custom-action");
    const modalAction = document.getElementById("modal-custom-action");
    const btnCloseAction = document.getElementById("btn-close-modal-action");
    const btnCancelAction = document.getElementById("btn-cancel-custom-action");
    const btnConfirmAction = document.getElementById("btn-confirm-custom-action");
    const selectCustomType = document.getElementById("select-custom-type") as HTMLSelectElement;
    const groupKeys = document.getElementById("group-custom-keys");
    const groupValue = document.getElementById("group-custom-value");
    const labelValue = document.getElementById("label-custom-value");

    const openActionModal = () => {
      if (modalAction) modalAction.style.display = "flex";
      const labelInput = document.getElementById("input-custom-label") as HTMLInputElement;
      if (labelInput) {
        labelInput.value = "";
        labelInput.focus();
      }
    };

    const closeActionModal = () => {
      if (modalAction) modalAction.style.display = "none";
    };

    if (btnCustomAction) btnCustomAction.addEventListener("click", openActionModal);
    if (btnCloseAction) btnCloseAction.addEventListener("click", closeActionModal);
    if (btnCancelAction) btnCancelAction.addEventListener("click", closeActionModal);

    if (selectCustomType) {
      selectCustomType.addEventListener("change", () => {
        const t = selectCustomType.value;
        if (t === "Shortcut") {
          if (groupKeys) groupKeys.style.display = "flex";
          if (groupValue) groupValue.style.display = "none";
        } else {
          if (groupKeys) groupKeys.style.display = "none";
          if (groupValue) groupValue.style.display = "flex";
          if (labelValue) {
            labelValue.textContent = t === "OpenUrl" ? "Web URL:" : t === "OpenApp" ? "Application / Command:" : "Text to Type:";
          }
        }
      });
    }

    if (btnConfirmAction) {
      btnConfirmAction.addEventListener("click", () => {
        const labelInput = document.getElementById("input-custom-label") as HTMLInputElement;
        const keysInput = document.getElementById("input-custom-keys") as HTMLInputElement;
        const valInput = document.getElementById("input-custom-value") as HTMLInputElement;

        const label = (labelInput?.value || "").trim();
        if (!label) {
          this.showToast("Please provide an action label");
          return;
        }

        const type = selectCustomType.value;
        let action: ActionConfig;

        if (type === "Shortcut") {
          const keys = (keysInput?.value || "")
            .split(",")
            .map((k) => k.trim())
            .filter((k) => k.length > 0);
          action = { type: "Shortcut", keys: keys.length > 0 ? keys : ["Control", "k"] };
        } else if (type === "OpenUrl") {
          action = { type: "OpenUrl", url: valInput?.value.trim() || "https://google.com" };
        } else if (type === "OpenApp") {
          action = { type: "OpenApp", command: valInput?.value.trim() || "notepad.exe" };
        } else {
          action = { type: "TypeText", text: valInput?.value || "" };
        }

        const customItem: LibraryActionItem = {
          id: `custom_${Date.now()}`,
          label,
          description: `Custom ${type}`,
          icon: type === "Shortcut" ? "code" : type === "OpenUrl" ? "search" : "terminal",
          category: "shortcuts",
          badge: "Custom",
          action,
        };

        ACTION_LIBRARY.push(customItem);
        this.setupActionLibrary();
        closeActionModal();
        this.showToast(`Added custom action: "${label}"`);

        if (this.selectedSlotIndex !== null) {
          this.assignActionToSelectedSlot(customItem);
        }
      });
    }
  }

  private setupDockControls(): void {
    const btnUndo = document.getElementById("btn-undo-slot") as HTMLButtonElement;
    const btnRedo = document.getElementById("btn-redo-slot") as HTMLButtonElement;
    const btnClear = document.getElementById("btn-clear-slot") as HTMLButtonElement;
    const btnReset = document.getElementById("btn-reset-profile") as HTMLButtonElement;

    if (btnUndo) {
      btnUndo.addEventListener("click", () => {
        if (this.undoStack.length === 0 || !this.activeProfile) return;
        const current = JSON.parse(JSON.stringify(this.activeProfile.slices));
        this.redoStack.push(current);
        const prev = this.undoStack.pop();
        if (prev) {
          this.activeProfile.slices = prev;
          this.applyActiveProfile();
          this.syncActiveProfile();
          this.showToast("Action reverted");
        }
      });
    }

    if (btnRedo) {
      btnRedo.addEventListener("click", () => {
        if (this.redoStack.length === 0 || !this.activeProfile) return;
        const current = JSON.parse(JSON.stringify(this.activeProfile.slices));
        this.undoStack.push(current);
        const next = this.redoStack.pop();
        if (next) {
          this.activeProfile.slices = next;
          this.applyActiveProfile();
          this.syncActiveProfile();
          this.showToast("Action reapplied");
        }
      });
    }

    if (btnClear) {
      btnClear.addEventListener("click", () => {
        if (this.selectedSlotIndex === null || !this.activeProfile) return;
        this.pushUndo();
        const idx = this.selectedSlotIndex;
        this.activeProfile.slices[idx] = {
          id: `empty_${idx + 1}`,
          label: `Slot ${idx + 1}`,
          icon: "options",
          action: { type: "Shortcut", keys: ["Escape"] },
        };
        this.applyActiveProfile();
        this.selectSlot(idx);
        this.syncActiveProfile();
        this.showToast(`Cleared Slot ${idx + 1}`);
      });
    }

    if (btnReset) {
      btnReset.addEventListener("click", () => {
        if (!this.activeProfile) return;
        this.pushUndo();
        const base = MOCK_PROFILES.find((p) => p.profile_name === this.activeProfile?.profile_name);
        if (base) {
          this.activeProfile.slices = JSON.parse(JSON.stringify(base.slices));
        }
        this.applyActiveProfile();
        this.syncActiveProfile();
        this.showToast("Reset slices to profile defaults");
      });
    }
  }

  private pushUndo(): void {
    if (!this.activeProfile) return;
    this.undoStack.push(JSON.parse(JSON.stringify(this.activeProfile.slices)));
    this.redoStack = [];
    this.updateUndoRedoButtons();
  }

  private updateUndoRedoButtons(): void {
    const btnUndo = document.getElementById("btn-undo-slot") as HTMLButtonElement;
    const btnRedo = document.getElementById("btn-redo-slot") as HTMLButtonElement;
    if (btnUndo) btnUndo.disabled = this.undoStack.length === 0;
    if (btnRedo) btnRedo.disabled = this.redoStack.length === 0;
  }

  private showToast(msg: string, durationMs: number = 2400): void {
    const container = document.getElementById("toast-container");
    if (!container) return;

    const toast = document.createElement("div");
    toast.className = "toast";
    toast.innerHTML = `<span>${msg}</span>`;
    container.appendChild(toast);

    setTimeout(() => {
      toast.style.opacity = "0";
      toast.style.transform = "translateY(8px)";
      toast.style.transition = "all 0.2s ease";
      setTimeout(() => {
        if (toast.parentElement) toast.parentElement.removeChild(toast);
      }, 200);
    }, durationMs);
  }
}

// Instantiate app when DOM is ready
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", () => new ConfiguratorApp());
} else {
  new ConfiguratorApp();
}
