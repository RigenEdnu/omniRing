import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { CanvasRenderer, RingConfig, SliceConfig } from "./canvas_renderer";

console.log("omniRing overlay script loaded");

export interface RadialPlacement {
  cursor_x: number;
  cursor_y: number;
  center_x: number;
  center_y: number;
  window_x: number;
  window_y: number;
  window_width: number;
  window_height: number;
  relative_center_x: number;
  relative_center_y: number;
  radius: number;
}

export interface RadialMoveData {
  current_x: number;
  current_y: number;
  delta_x: number;
  delta_y: number;
}

export interface RadialReleaseData {
  current_x: number;
  current_y: number;
}

export interface GestureEventPayload {
  type: "Tap" | "Flick" | "FlickComplete" | "RadialOpen" | "RadialMove" | "RadialRelease" | "Cancelled";
  payload?: unknown;
}

export interface ProcessInfo {
  pid: number;
  process_name: string;
  executable_path: string;
  window_title?: string | null;
}

let activePlacement: RadialPlacement | null = null;
let renderer: CanvasRenderer | null = null;

function resolveMovePayload(rawPayload: unknown): RadialMoveData | null {
  if (!rawPayload || typeof rawPayload !== "object") return null;
  const p = rawPayload as Record<string, unknown>;

  // Check if wrapped in { type: "RadialMove", payload: { ... } }
  if (p.payload && typeof p.payload === "object") {
    const inner = p.payload as Record<string, unknown>;
    if (typeof inner.current_x === "number") {
      return inner as unknown as RadialMoveData;
    }
  }

  if (typeof p.current_x === "number") {
    return p as unknown as RadialMoveData;
  }

  return null;
}

function resolveReleasePayload(rawPayload: unknown): RadialReleaseData | null {
  if (!rawPayload || typeof rawPayload !== "object") return null;
  const p = rawPayload as Record<string, unknown>;

  if (p.payload && typeof p.payload === "object") {
    const inner = p.payload as Record<string, unknown>;
    if (typeof inner.current_x === "number") {
      return inner as unknown as RadialReleaseData;
    }
  }

  if (typeof p.current_x === "number") {
    return p as unknown as RadialReleaseData;
  }

  return null;
}

window.addEventListener("DOMContentLoaded", async () => {
  const canvas = document.getElementById("ring-canvas") as HTMLCanvasElement;
  if (!canvas) return;

  // Set high-DPI canvas dimensions
  const dpr = window.devicePixelRatio || 1;
  const displaySize = 600;
  canvas.width = displaySize * dpr;
  canvas.height = displaySize * dpr;
  canvas.style.width = `${displaySize}px`;
  canvas.style.height = `${displaySize}px`;

  const ctx = canvas.getContext("2d");
  if (ctx) {
    ctx.scale(dpr, dpr);
  }

  renderer = new CanvasRenderer(canvas);
  renderer.setCenter(displaySize / 2, displaySize / 2);
  renderer.clear();

  // Listen for radial menu open event (payload carries clamped RadialPlacement)
  try {
    await listen<RadialPlacement | GestureEventPayload>("omniring://radial-open", (event) => {
      console.log("Radial menu opened:", event.payload);
      const payload = event.payload as unknown as Record<string, unknown> | null;

      if (payload && typeof payload.relative_center_x === "number") {
        activePlacement = payload as unknown as RadialPlacement;
        renderer?.setCenter(activePlacement.relative_center_x, activePlacement.relative_center_y);
      } else {
        activePlacement = null;
        renderer?.setCenter(displaySize / 2, displaySize / 2);
      }

      renderer?.openWithAnimation(80);
    });

    // Listen for cursor displacement while radial menu is active
    await listen<GestureEventPayload | RadialMoveData>("omniring://radial-move", (event) => {
      if (!renderer) return;
      const moveData = resolveMovePayload(event.payload);
      if (!moveData) return;

      let canvasCursorX: number;
      let canvasCursorY: number;

      if (activePlacement) {
        const dx = moveData.current_x - activePlacement.center_x;
        const dy = moveData.current_y - activePlacement.center_y;
        canvasCursorX = activePlacement.relative_center_x + dx;
        canvasCursorY = activePlacement.relative_center_y + dy;
      } else {
        const center = renderer.getCenter();
        canvasCursorX = center.x + moveData.delta_x;
        canvasCursorY = center.y + moveData.delta_y;
      }

      const hoveredIndex = renderer.hitTest(canvasCursorX, canvasCursorY);
      renderer.setActiveSlice(hoveredIndex);
      renderer.setCursorPosition(canvasCursorX, canvasCursorY);
      renderer.render();
    });

    // Listen for radial release / slice confirmation
    await listen<GestureEventPayload | RadialReleaseData>("omniring://radial-release", async (event) => {
      if (!renderer) return;
      const releaseData = resolveReleasePayload(event.payload);
      console.log("Radial menu release received:", releaseData);

      const confirmedSlice: SliceConfig | null = renderer.getActiveSliceData();
      const confirmedIndex = renderer.getActiveSlice();

      if (confirmedSlice && confirmedIndex >= 0 && confirmedSlice.action) {
        console.log(`[Slice Confirmed] Index: ${confirmedIndex}, Label: "${confirmedSlice.label}", Action:`, confirmedSlice.action);

        if (confirmedSlice.action.type === "Submenu") {
          const targetRing = confirmedSlice.action.target_ring_id || "root";
          const switched = renderer.switchToRing(targetRing);
          if (switched) {
            console.log(`[Submenu Navigated] Showing ring: ${targetRing}`);
            return; // Maintain overlay visibility for the child ring
          }
        } else {
          // Terminal action (Shortcut, TypeText, OpenApp, OpenUrl, Sequence)
          try {
            await invoke("execute_action", { action: confirmedSlice.action });
          } catch (err) {
            console.error("Failed to execute action:", err);
          }
        }
      } else {
        // Deadzone release: if inside a submenu, return to root ring instead of closing
        if (renderer.getActiveRingId() !== "root") {
          renderer.resetToRoot();
          renderer.openWithAnimation(60);
          return;
        }
        console.log("[Slice Released in Deadzone] No action triggered");
      }

      // Hide overlay for terminal actions or root deadzone release
      try {
        await invoke("hide_overlay");
      } catch (_) {}

      activePlacement = null;
      renderer.clear();
    });

    // Listen for directional flick gestures
    await listen<GestureEventPayload>("omniring://flick", (event) => {
      console.log("Directional flick gesture detected:", event.payload);
    });

    // Listen for tap gestures
    await listen<GestureEventPayload>("omniring://tap", (event) => {
      console.log("Tap gesture detected:", event.payload);
    });

    // Listen for cancelled gestures
    await listen<GestureEventPayload>("omniring://cancelled", () => {
      console.log("Gesture cancelled");
      activePlacement = null;
      renderer?.clear();
    });

    // Listen for dynamic profile change events
    await listen<RingConfig>("omniring://profile-changed", (event) => {
      console.log("[Profile Changed Event Received]:", event.payload);
      if (event.payload && renderer) {
        renderer.setConfig(event.payload);
        renderer.render();
      }
    });

    // Listen for foreground application changes
    await listen<ProcessInfo>("omniring://foreground-changed", (event) => {
      console.log("[Foreground Process Changed]:", event.payload);
    });

    // Listen for omniRing enabled/disabled state toggles
    await listen<boolean>("omniring://enabled-changed", (event) => {
      console.log("[omniRing Enabled Changed]:", event.payload);
      if (!event.payload) {
        renderer?.clear();
      }
    });

    // Fetch initial active profile from Tauri backend on startup
    try {
      const activeProf = await invoke<RingConfig>("get_active_profile");
      if (activeProf && renderer) {
        console.log("[Initial Active Profile Loaded]:", activeProf.profile_name);
        renderer.setConfig(activeProf);
        renderer.render();
      }
    } catch (e) {
      console.log("Could not invoke get_active_profile (running in browser/mock mode)");
    }
  } catch (err) {
    console.warn("Tauri event listener registration skipped (running in browser/mock mode):", err);
  }

  // DOM Mouse event listeners for standalone browser testing & interactive preview
  canvas.addEventListener("mousemove", (e: MouseEvent) => {
    if (!renderer) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const hoveredIndex = renderer.hitTest(x, y);
    renderer.setActiveSlice(hoveredIndex);
    renderer.setCursorPosition(x, y);
    renderer.render();
  });

  canvas.addEventListener("mouseleave", () => {
    if (!renderer) return;
    renderer.setActiveSlice(-1);
    renderer.setCursorPosition(null, null);
    renderer.render();
  });

  canvas.addEventListener("mouseup", async () => {
    if (!renderer) return;
    const confirmed = renderer.getActiveSliceData();
    if (confirmed && confirmed.action) {
      console.log("[Browser Click Confirmed]:", confirmed.label, confirmed.action);
      if (confirmed.action.type === "Submenu") {
        renderer.switchToRing(confirmed.action.target_ring_id || "root");
      } else {
        try {
          await invoke("execute_action", { action: confirmed.action });
        } catch (_) {}
      }
    } else {
      if (renderer.getActiveRingId() !== "root") {
        renderer.resetToRoot();
        renderer.openWithAnimation(60);
      }
    }
  });

  // Initial draw preview for browser mode if not inside Tauri overlay
  renderer.render();
});

// App & Tray control helpers callable from frontend
export async function openConfigFolder(): Promise<void> {
  return await invoke("open_config_folder");
}

export async function isOmniEnabled(): Promise<boolean> {
  return await invoke("is_omni_enabled");
}

export async function toggleOmniEnabled(): Promise<boolean> {
  return await invoke("toggle_omni_enabled");
}

export async function setOmniEnabled(enabled: boolean): Promise<void> {
  return await invoke("set_omni_enabled", { enabled });
}

export async function exitApp(): Promise<void> {
  return await invoke("exit_app");
}


