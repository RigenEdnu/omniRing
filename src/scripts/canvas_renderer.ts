export interface ActionConfig {
  type: string;
  keys?: string[];
  url?: string;
  command?: string;
  text?: string;
  target_ring_id?: string;
  delay_ms?: number;
  actions?: ActionConfig[];
  [key: string]: unknown;
}

export interface SliceConfig {
  id: string;
  label: string;
  icon?: string;
  action?: ActionConfig;
}

export interface SubmenuConfig {
  title?: string;
  slices: SliceConfig[];
}

export interface RingConfig {
  profile_name?: string;
  inner_radius: number;
  outer_radius: number;
  slices: SliceConfig[];
  submenus?: Record<string, SubmenuConfig>;
}

export const DEFAULT_RING_CONFIG: RingConfig = {
  profile_name: "Default Global",
  inner_radius: 26,
  outer_radius: 200,
  slices: [
    {
      id: "slice_play_pause",
      label: "Play/Pause",
      icon: "play-pause",
      action: { type: "Shortcut", keys: ["MediaPlayPause"] },
    },
    {
      id: "slice_new_note",
      label: "New note",
      icon: "new-note",
      action: { type: "Shortcut", keys: ["Control", "n"] },
    },
    {
      id: "slice_explore_ai",
      label: "Explore AI",
      icon: "sparkles",
      action: { type: "OpenUrl", url: "https://chatgpt.com" },
    },
    {
      id: "slice_lock",
      label: "Lock workstation",
      icon: "lock",
      action: { type: "Shortcut", keys: ["Meta", "l"] },
    },
    {
      id: "slice_options",
      label: "Settings",
      icon: "options",
      action: { type: "OpenApp", command: "ms-settings:" },
    },
    {
      id: "slice_screenshot",
      label: "Screenshot",
      icon: "screenshot",
      action: { type: "Shortcut", keys: ["Meta", "Shift", "s"] },
    },
    {
      id: "slice_emoji",
      label: "Emoji",
      icon: "emoji",
      action: { type: "Shortcut", keys: ["Meta", "."] },
    },
    {
      id: "slice_finder",
      label: "Files",
      icon: "folder",
      action: { type: "Shortcut", keys: ["Meta", "e"] },
    },
  ],
};

function drawRoundedRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  radius: number
): void {
  if (typeof ctx.roundRect === "function") {
    ctx.roundRect(x, y, width, height, radius);
  } else {
    ctx.moveTo(x + radius, y);
    ctx.lineTo(x + width - radius, y);
    ctx.quadraticCurveTo(x + width, y, x + width, y + radius);
    ctx.lineTo(x + width, y + height - radius);
    ctx.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
    ctx.lineTo(x + radius, y + height);
    ctx.quadraticCurveTo(x, y + height, x, y + height - radius);
    ctx.lineTo(x, y + radius);
    ctx.quadraticCurveTo(x, y, x + radius, y);
  }
}

export class CanvasRenderer {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private centerX: number;
  private centerY: number;
  private config: RingConfig;
  private currentRingId: string = "root";
  private activeSliceIndex: number = -1;
  private cursorX: number | null = null;
  private cursorY: number | null = null;

  // Geometry parameters for Logitech Options+ Bubble UI
  public readonly hubRadius: number = 22;
  public readonly centerDeadzone: number = 26;
  public readonly nodeOrbitalRadius: number = 95;
  public readonly nodeRadius: number = 24;
  public readonly labelOrbitalRadius: number = 175;
  public readonly outerToleranceLimit: number = 240;

  private animProgress: number = 1.0;
  private animFrameId: number | null = null;

  constructor(canvas: HTMLCanvasElement, config: RingConfig = DEFAULT_RING_CONFIG) {
    this.canvas = canvas;
    const context = canvas.getContext("2d");
    if (!context) {
      throw new Error("Failed to get 2D context from canvas");
    }
    this.ctx = context;
    this.centerX = canvas.width / 2;
    this.centerY = canvas.height / 2;
    this.config = config;
  }

  public setCenter(x: number, y: number): void {
    this.centerX = x;
    this.centerY = y;
  }

  public getCenter(): { x: number; y: number } {
    return { x: this.centerX, y: this.centerY };
  }

  public setConfig(config: RingConfig): void {
    this.config = config;
    const slices = this.getCurrentSlices();
    if (this.activeSliceIndex >= slices.length) {
      this.activeSliceIndex = -1;
    }
  }

  public getConfig(): RingConfig {
    return this.config;
  }

  public getCurrentSlices(): SliceConfig[] {
    if (this.currentRingId !== "root" && this.config.submenus && this.config.submenus[this.currentRingId]) {
      return this.config.submenus[this.currentRingId].slices;
    }
    return this.config.slices;
  }

  public switchToRing(ringId: string): boolean {
    if (ringId === "root" || ringId === "") {
      this.currentRingId = "root";
      this.activeSliceIndex = -1;
      this.openWithAnimation(60);
      return true;
    }
    if (this.config.submenus && this.config.submenus[ringId]) {
      this.currentRingId = ringId;
      this.activeSliceIndex = -1;
      this.openWithAnimation(60);
      return true;
    }
    return false;
  }

  public getActiveRingId(): string {
    return this.currentRingId;
  }

  public resetToRoot(): void {
    this.currentRingId = "root";
    this.activeSliceIndex = -1;
  }

  public setActiveSlice(index: number | null): void {
    this.activeSliceIndex = index === null ? -1 : index;
  }

  public getActiveSlice(): number {
    return this.activeSliceIndex;
  }

  public getActiveSliceData(): SliceConfig | null {
    const slices = this.getCurrentSlices();
    if (this.activeSliceIndex >= 0 && this.activeSliceIndex < slices.length) {
      return slices[this.activeSliceIndex];
    }
    return null;
  }

  public setCursorPosition(x: number | null, y: number | null): void {
    this.cursorX = x;
    this.cursorY = y;
  }

  public clear(): void {
    if (this.animFrameId !== null) {
      cancelAnimationFrame(this.animFrameId);
      this.animFrameId = null;
    }
    this.activeSliceIndex = -1;
    this.cursorX = null;
    this.cursorY = null;
    this.currentRingId = "root";
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
  }

  /**
   * Helper to determine if cursor coordinate is within center dismiss hub.
   */
  public isCenterHub(x: number, y: number, isRelative: boolean = false): boolean {
    const dx = isRelative ? x : x - this.centerX;
    const dy = isRelative ? y : y - this.centerY;
    return Math.hypot(dx, dy) <= this.centerDeadzone;
  }

  /**
   * Calculates the slice angles for N slices.
   * Slice 0 is centered on top (-PI/2), corresponding to an Up gesture.
   */
  public getSliceAngleSpan(): number {
    const n = Math.max(1, this.getCurrentSlices().length);
    return (2 * Math.PI) / n;
  }

  public getStartAngleOffset(): number {
    const sliceAngle = this.getSliceAngleSpan();
    return -Math.PI / 2 - sliceAngle / 2;
  }

  public getSliceAngles(index: number): { start: number; end: number; mid: number } {
    const sliceAngle = this.getSliceAngleSpan();
    const startOffset = this.getStartAngleOffset();
    const start = startOffset + index * sliceAngle;
    const end = start + sliceAngle;
    const mid = start + sliceAngle / 2;
    return { start, end, mid };
  }

  /**
   * Hit test a coordinate (r, theta) adhering to Fitts's Law polar angles.
   * Returns slice index (0..n-1), or -1 if in center deadzone (< 26px) or beyond outer tolerance.
   */
  public hitTest(x: number, y: number, isRelative: boolean = false): number {
    const slices = this.getCurrentSlices();
    const n = slices.length;
    if (n === 0) return -1;

    const dx = isRelative ? x : x - this.centerX;
    const dy = isRelative ? y : y - this.centerY;
    const r = Math.hypot(dx, dy);

    // Dead zone check (inside center dismiss hub)
    if (r < this.centerDeadzone) {
      return -1;
    }

    // Outer boundary check
    if (r > this.outerToleranceLimit) {
      return -1;
    }

    const theta = Math.atan2(dy, dx);
    const startOffset = this.getStartAngleOffset();
    const sliceAngle = this.getSliceAngleSpan();

    let alpha = theta - startOffset;
    alpha = ((alpha % (2 * Math.PI)) + 2 * Math.PI) % (2 * Math.PI);

    let index = Math.floor(alpha / sliceAngle);
    if (index >= n) {
      index = n - 1;
    }
    return Math.max(0, index);
  }

  /**
   * Trigger fast entrance animation (<100ms fade-in & scale-up).
   */
  public openWithAnimation(durationMs: number = 80, onComplete?: () => void): void {
    if (typeof requestAnimationFrame === "undefined") {
      this.animProgress = 1.0;
      this.render();
      if (onComplete) onComplete();
      return;
    }
    if (this.animFrameId !== null) {
      cancelAnimationFrame(this.animFrameId);
    }
    const startTime = performance.now();
    this.animProgress = 0;

    const animate = (currentTime: number) => {
      const elapsed = currentTime - startTime;
      const t = Math.min(1.0, elapsed / durationMs);
      this.animProgress = 1 - Math.pow(1 - t, 3);
      this.render();

      if (t < 1.0) {
        this.animFrameId = requestAnimationFrame(animate);
      } else {
        this.animProgress = 1.0;
        this.animFrameId = null;
        this.render();
        if (onComplete) onComplete();
      }
    };

    this.animFrameId = requestAnimationFrame(animate);
  }

  /**
   * Renders the Logitech Options+ Bubble Nodes radial menu overlay on canvas.
   */
  public render(): void {
    const ctx = this.ctx;
    ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);

    const slices = this.getCurrentSlices();
    const n = slices.length;
    if (n === 0) return;

    ctx.save();

    // Apply scale and opacity animation if opening
    if (this.animProgress < 1.0) {
      ctx.translate(this.centerX, this.centerY);
      const scale = 0.85 + 0.15 * this.animProgress;
      ctx.scale(scale, scale);
      ctx.translate(-this.centerX, -this.centerY);
      ctx.globalAlpha = this.animProgress;
    }

    // 1. Draw subtle connecting guideline / orbital ring
    ctx.beginPath();
    ctx.arc(this.centerX, this.centerY, this.nodeOrbitalRadius, 0, Math.PI * 2);
    ctx.strokeStyle = "rgba(0, 0, 0, 0.06)";
    ctx.lineWidth = 1;
    ctx.stroke();

    // 2. Draw 8 Floating Pill Badges (outer orbital radius 175px)
    for (let i = 0; i < n; i++) {
      const { mid } = this.getSliceAngles(i);
      const slice = slices[i];
      const isActive = i === this.activeSliceIndex;

      const badgeX = this.centerX + this.labelOrbitalRadius * Math.cos(mid);
      const badgeY = this.centerY + this.labelOrbitalRadius * Math.sin(mid);

      ctx.save();
      ctx.font = isActive
        ? "600 12px -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif"
        : "500 12px -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif";

      const textWidth = ctx.measureText(slice.label).width;
      const pillWidth = Math.max(64, textWidth + 22);
      const pillHeight = 26;
      const pillRadius = 6;
      const rectX = badgeX - pillWidth / 2;
      const rectY = badgeY - pillHeight / 2;

      // Draw pill background with drop shadow
      ctx.beginPath();
      drawRoundedRect(ctx, rectX, rectY, pillWidth, pillHeight, pillRadius);

      if (isActive) {
        ctx.fillStyle = "#18181b";
        ctx.shadowColor = "rgba(94, 72, 232, 0.35)";
        ctx.shadowBlur = 12;
        ctx.shadowOffsetY = 4;
        ctx.fill();

        ctx.shadowBlur = 0;
        ctx.strokeStyle = "#5E48E8";
        ctx.lineWidth = 1.8;
        ctx.stroke();
      } else {
        ctx.fillStyle = "#ffffff";
        ctx.shadowColor = "rgba(0, 0, 0, 0.12)";
        ctx.shadowBlur = 8;
        ctx.shadowOffsetY = 2;
        ctx.fill();

        ctx.shadowBlur = 0;
        ctx.strokeStyle = "rgba(0, 0, 0, 0.08)";
        ctx.lineWidth = 1;
        ctx.stroke();
      }

      // Draw pill label text
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillStyle = isActive ? "#ffffff" : "#18181b";
      ctx.fillText(slice.label, badgeX, badgeY + 0.5);
      ctx.restore();
    }

    // 3. Draw 8 Action Bubble Nodes (inner orbital radius 95px)
    for (let i = 0; i < n; i++) {
      const { mid } = this.getSliceAngles(i);
      const slice = slices[i];
      const isActive = i === this.activeSliceIndex;

      const nodeX = this.centerX + this.nodeOrbitalRadius * Math.cos(mid);
      const nodeY = this.centerY + this.nodeOrbitalRadius * Math.sin(mid);
      const radius = isActive ? this.nodeRadius * 1.12 : this.nodeRadius;

      ctx.save();
      ctx.beginPath();
      ctx.arc(nodeX, nodeY, radius, 0, Math.PI * 2);

      if (isActive) {
        // High-contrast active bubble node with accent glow
        ctx.fillStyle = "#111827";
        ctx.shadowColor = "rgba(94, 72, 232, 0.65)";
        ctx.shadowBlur = 16;
        ctx.fill();

        ctx.lineWidth = 2.5;
        ctx.strokeStyle = "#5E48E8";
        ctx.stroke();
      } else {
        // Inactive dark circular node with subtle elevation shadow
        ctx.fillStyle = "#18181b";
        ctx.shadowColor = "rgba(0, 0, 0, 0.2)";
        ctx.shadowBlur = 6;
        ctx.shadowOffsetY = 2;
        ctx.fill();

        ctx.lineWidth = 1;
        ctx.strokeStyle = "rgba(255, 255, 255, 0.12)";
        ctx.stroke();
      }
      ctx.restore();

      // Render crisp white vector icon inside the node
      if (slice.icon) {
        this.drawVectorIcon(ctx, slice.icon, nodeX, nodeY, isActive);
      }
    }

    // 4. Draw Center Dismiss Hub (r = 22px) with '×' glyph
    ctx.save();
    ctx.beginPath();
    ctx.arc(this.centerX, this.centerY, this.hubRadius, 0, Math.PI * 2);

    const isCursorInHub = this.cursorX !== null && this.cursorY !== null && this.isCenterHub(this.cursorX, this.cursorY);
    ctx.fillStyle = isCursorInHub ? "#e2e8f0" : "#f1f5f9";
    ctx.shadowColor = "rgba(0, 0, 0, 0.1)";
    ctx.shadowBlur = 6;
    ctx.shadowOffsetY = 1;
    ctx.fill();

    ctx.lineWidth = 1.2;
    ctx.strokeStyle = isCursorInHub ? "#94a3b8" : "#cbd5e1";
    ctx.stroke();

    if (this.currentRingId !== "root") {
      // Submenu back indicator
      ctx.font = "600 11px system-ui, -apple-system, sans-serif";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillStyle = "#334155";
      ctx.fillText("Back", this.centerX, this.centerY);
    } else {
      // Central dismiss cross '×'
      const crossSize = 5;
      ctx.beginPath();
      ctx.moveTo(this.centerX - crossSize, this.centerY - crossSize);
      ctx.lineTo(this.centerX + crossSize, this.centerY + crossSize);
      ctx.moveTo(this.centerX + crossSize, this.centerY - crossSize);
      ctx.lineTo(this.centerX - crossSize, this.centerY + crossSize);
      ctx.lineWidth = 2.0;
      ctx.strokeStyle = isCursorInHub ? "#0f172a" : "#475569";
      ctx.lineCap = "round";
      ctx.stroke();
    }
    ctx.restore();

    // 5. Draw subtle cursor bead if cursor is tracking
    if (this.cursorX !== null && this.cursorY !== null && !isCursorInHub) {
      ctx.save();
      ctx.beginPath();
      ctx.arc(this.cursorX, this.cursorY, 3.5, 0, Math.PI * 2);
      ctx.fillStyle = "rgba(94, 72, 232, 0.85)";
      ctx.shadowColor = "rgba(94, 72, 232, 0.6)";
      ctx.shadowBlur = 6;
      ctx.fill();
      ctx.restore();
    }

    ctx.restore();
  }

  /**
   * Crisp vector icon renderer with 2px stroke matching Logitech Options+.
   */
  private drawVectorIcon(
    ctx: CanvasRenderingContext2D,
    iconName: string,
    x: number,
    y: number,
    isActive: boolean
  ): void {
    ctx.save();
    ctx.translate(x, y);
    ctx.strokeStyle = "#ffffff";
    ctx.fillStyle = "#ffffff";
    ctx.lineWidth = isActive ? 2.2 : 1.8;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";

    const name = iconName.toLowerCase();

    switch (name) {
      case "play":
      case "play-pause": {
        // Rightward playback triangle
        ctx.beginPath();
        ctx.moveTo(-4, -6);
        ctx.lineTo(6, 0);
        ctx.lineTo(-4, 6);
        ctx.closePath();
        ctx.fill();
        break;
      }
      case "note":
      case "new-note": {
        // Notepad with diagonal pencil
        ctx.strokeRect(-5, -6, 9, 12);
        ctx.beginPath();
        ctx.moveTo(1, -7);
        ctx.lineTo(6, -2);
        ctx.stroke();
        break;
      }
      case "sparkles":
      case "ai":
      case "explore-ai": {
        // Multi-point sparkles
        ctx.beginPath();
        ctx.moveTo(0, -7);
        ctx.quadraticCurveTo(0, 0, 7, 0);
        ctx.quadraticCurveTo(0, 0, 0, 7);
        ctx.quadraticCurveTo(0, 0, -7, 0);
        ctx.quadraticCurveTo(0, 0, 0, -7);
        ctx.closePath();
        ctx.fill();
        break;
      }
      case "lock":
      case "lock-workstation": {
        // Padlock
        ctx.beginPath();
        ctx.arc(0, -3, 4.5, Math.PI, 0, false);
        ctx.stroke();
        ctx.strokeRect(-6, -1, 12, 8);
        break;
      }
      case "options":
      case "settings":
      case "gear": {
        // Options / settings sliders
        ctx.beginPath();
        ctx.moveTo(-6, -4);
        ctx.lineTo(6, -4);
        ctx.moveTo(-6, 0);
        ctx.lineTo(6, 0);
        ctx.moveTo(-6, 4);
        ctx.lineTo(6, 4);
        ctx.stroke();
        // Slider knobs
        ctx.beginPath();
        ctx.arc(-2, -4, 2, 0, Math.PI * 2);
        ctx.arc(2, 0, 2, 0, Math.PI * 2);
        ctx.arc(-3, 4, 2, 0, Math.PI * 2);
        ctx.fill();
        break;
      }
      case "screenshot":
      case "camera": {
        // Dashed capture frame with crosshair
        ctx.strokeRect(-6, -5, 12, 10);
        ctx.beginPath();
        ctx.arc(0, 0, 2.5, 0, Math.PI * 2);
        ctx.stroke();
        break;
      }
      case "emoji":
      case "smile": {
        // Smiley face
        ctx.beginPath();
        ctx.arc(0, 0, 7, 0, Math.PI * 2);
        ctx.stroke();
        // Eyes
        ctx.beginPath();
        ctx.arc(-2.5, -2, 1, 0, Math.PI * 2);
        ctx.arc(2.5, -2, 1, 0, Math.PI * 2);
        ctx.fill();
        // Smile curve
        ctx.beginPath();
        ctx.arc(0, 1, 3.5, 0.2 * Math.PI, 0.8 * Math.PI, false);
        ctx.stroke();
        break;
      }
      case "files":
      case "folder":
      case "finder": {
        // Folder icon
        ctx.beginPath();
        ctx.moveTo(-7, -4);
        ctx.lineTo(-2, -4);
        ctx.lineTo(0, -2);
        ctx.lineTo(7, -2);
        ctx.lineTo(7, 5);
        ctx.lineTo(-7, 5);
        ctx.closePath();
        ctx.stroke();
        break;
      }
      case "copy": {
        ctx.strokeRect(-2, -6, 8, 9);
        ctx.strokeRect(-6, -3, 8, 9);
        break;
      }
      case "clipboard":
      case "paste": {
        ctx.strokeRect(-5, -4, 10, 11);
        ctx.strokeRect(-3, -7, 6, 3);
        break;
      }
      case "arrow-undo":
      case "undo": {
        ctx.beginPath();
        ctx.arc(1, 0, 5, 0, Math.PI * 1.25, true);
        ctx.stroke();
        ctx.beginPath();
        ctx.moveTo(-5, -4);
        ctx.lineTo(-5, 0);
        ctx.lineTo(-1, 0);
        ctx.stroke();
        break;
      }
      case "arrow-redo":
      case "redo": {
        ctx.beginPath();
        ctx.arc(-1, 0, 5, Math.PI, -Math.PI * 0.25, false);
        ctx.stroke();
        ctx.beginPath();
        ctx.moveTo(5, -4);
        ctx.lineTo(5, 0);
        ctx.lineTo(1, 0);
        ctx.stroke();
        break;
      }
      case "terminal": {
        ctx.beginPath();
        ctx.moveTo(-5, -4);
        ctx.lineTo(-1, -1);
        ctx.lineTo(-5, 2);
        ctx.stroke();
        ctx.beginPath();
        ctx.moveTo(1, 2);
        ctx.lineTo(5, 2);
        ctx.stroke();
        break;
      }
      case "code":
      case "code-outline": {
        ctx.beginPath();
        ctx.moveTo(-2, -5);
        ctx.lineTo(-6, 0);
        ctx.lineTo(-2, 5);
        ctx.moveTo(2, -5);
        ctx.lineTo(6, 0);
        ctx.lineTo(2, 5);
        ctx.stroke();
        break;
      }
      case "search": {
        ctx.beginPath();
        ctx.arc(-1, -2, 4.5, 0, Math.PI * 2);
        ctx.stroke();
        ctx.beginPath();
        ctx.moveTo(2.5, 1.5);
        ctx.lineTo(6, 5);
        ctx.stroke();
        break;
      }
      case "layout": {
        ctx.strokeRect(-6, -5, 12, 10);
        ctx.beginPath();
        ctx.moveTo(-2, -5);
        ctx.lineTo(-2, 5);
        ctx.stroke();
        break;
      }
      default: {
        ctx.beginPath();
        ctx.arc(0, 0, 4.5, 0, Math.PI * 2);
        ctx.fill();
        break;
      }
    }

    ctx.restore();
  }
}
