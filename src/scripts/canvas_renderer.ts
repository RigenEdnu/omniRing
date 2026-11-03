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
  inner_radius: 50,
  outer_radius: 140,
  slices: [
    {
      id: "slice_copy",
      label: "Copy",
      icon: "copy",
      action: { type: "Shortcut", keys: ["Control", "c"] },
    },
    {
      id: "slice_paste",
      label: "Paste",
      icon: "clipboard",
      action: { type: "Shortcut", keys: ["Control", "v"] },
    },
    {
      id: "slice_undo",
      label: "Undo",
      icon: "arrow-undo",
      action: { type: "Shortcut", keys: ["Control", "z"] },
    },
    {
      id: "slice_redo",
      label: "Redo",
      icon: "arrow-redo",
      action: { type: "Shortcut", keys: ["Control", "y"] },
    },
  ],
};

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
  private outerTolerance: number = 60;
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

  public setActiveSlice(index: number): void {
    this.activeSliceIndex = index;
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
   * Hit test a coordinate (r, theta) against the radial slices.
   * Returns slice index (0..n-1), or -1 if in dead zone (< inner_radius) or out of outer bound.
   */
  public hitTest(x: number, y: number, isRelative: boolean = false): number {
    const slices = this.getCurrentSlices();
    const n = slices.length;
    if (n === 0) return -1;

    const dx = isRelative ? x : x - this.centerX;
    const dy = isRelative ? y : y - this.centerY;
    const r = Math.hypot(dx, dy);

    // Dead zone check (inside inner radius)
    if (r < this.config.inner_radius) {
      return -1;
    }

    // Outer boundary check with tolerance
    if (r > this.config.outer_radius + this.outerTolerance) {
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
   * Renders the radial menu overlay on canvas.
   */
  public render(): void {
    const ctx = this.ctx;
    ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);

    const slices = this.getCurrentSlices();
    const n = slices.length;
    if (n === 0) return;

    ctx.save();

    // Apply scale and opacity animation if active
    if (this.animProgress < 1.0) {
      ctx.translate(this.centerX, this.centerY);
      const scale = 0.8 + 0.2 * this.animProgress;
      ctx.scale(scale, scale);
      ctx.translate(-this.centerX, -this.centerY);
      ctx.globalAlpha = this.animProgress;
    }

    const innerR = this.config.inner_radius;
    const outerR = this.config.outer_radius;

    // Draw slices
    for (let i = 0; i < n; i++) {
      const { start, end } = this.getSliceAngles(i);
      const isActive = i === this.activeSliceIndex;

      // Annular sector path
      ctx.beginPath();
      ctx.arc(this.centerX, this.centerY, outerR, start, end, false);
      ctx.arc(this.centerX, this.centerY, innerR, end, start, true);
      ctx.closePath();

      if (isActive) {
        ctx.fillStyle = "rgba(59, 130, 246, 0.85)";
        ctx.shadowColor = "rgba(59, 130, 246, 0.6)";
        ctx.shadowBlur = 16;
        ctx.fill();

        ctx.lineWidth = 2.5;
        ctx.strokeStyle = "rgba(255, 255, 255, 0.85)";
        ctx.stroke();

        ctx.shadowBlur = 0;
      } else {
        ctx.fillStyle = "rgba(20, 24, 34, 0.85)";
        ctx.fill();

        ctx.lineWidth = 1;
        ctx.strokeStyle = "rgba(255, 255, 255, 0.12)";
        ctx.stroke();
      }
    }

    // Draw separator lines between slices
    ctx.lineWidth = 1.5;
    ctx.strokeStyle = "rgba(255, 255, 255, 0.22)";
    for (let i = 0; i < n; i++) {
      const { start } = this.getSliceAngles(i);
      const x1 = this.centerX + innerR * Math.cos(start);
      const y1 = this.centerY + innerR * Math.sin(start);
      const x2 = this.centerX + outerR * Math.cos(start);
      const y2 = this.centerY + outerR * Math.sin(start);

      ctx.beginPath();
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
      ctx.stroke();
    }

    // Draw inner hub (center circle)
    ctx.beginPath();
    ctx.arc(this.centerX, this.centerY, innerR, 0, Math.PI * 2);
    ctx.fillStyle = "rgba(15, 18, 26, 0.94)";
    ctx.fill();
    ctx.lineWidth = 1.5;
    ctx.strokeStyle = "rgba(255, 255, 255, 0.25)";
    ctx.stroke();

    if (this.currentRingId !== "root") {
      ctx.save();
      ctx.font = "600 11px system-ui, -apple-system, sans-serif";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillStyle = "rgba(147, 197, 253, 0.9)";
      ctx.fillText("Back", this.centerX, this.centerY);
      ctx.restore();
    } else {
      // Center indicator dot
      ctx.beginPath();
      ctx.arc(this.centerX, this.centerY, 3.5, 0, Math.PI * 2);
      ctx.fillStyle = this.activeSliceIndex >= 0 ? "rgba(96, 165, 250, 0.9)" : "rgba(255, 255, 255, 0.5)";
      ctx.fill();
    }

    // Draw slice labels and icons
    const midR = (innerR + outerR) / 2;
    for (let i = 0; i < n; i++) {
      const { mid } = this.getSliceAngles(i);
      const slice = slices[i];
      const isActive = i === this.activeSliceIndex;

      const itemX = this.centerX + midR * Math.cos(mid);
      const itemY = this.centerY + midR * Math.sin(mid);

      const hasIcon = Boolean(slice.icon);
      const iconY = hasIcon ? itemY - 7 : itemY;
      const textY = hasIcon ? itemY + 16 : itemY + 4;

      if (hasIcon && slice.icon) {
        this.drawVectorIcon(ctx, slice.icon, itemX, iconY, isActive);
      }

      // Slice label text
      ctx.save();
      ctx.font = isActive
        ? "600 13px system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif"
        : "500 12px system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillStyle = isActive ? "#ffffff" : "rgba(240, 243, 246, 0.88)";
      if (isActive) {
        ctx.shadowColor = "rgba(0, 0, 0, 0.75)";
        ctx.shadowBlur = 4;
      }
      ctx.fillText(slice.label, itemX, textY);
      ctx.restore();
    }

    // Draw cursor indicator bead if cursor position is tracked
    if (this.cursorX !== null && this.cursorY !== null) {
      const cdx = this.cursorX - this.centerX;
      const cdy = this.cursorY - this.centerY;
      const dist = Math.hypot(cdx, cdy);

      if (dist >= innerR) {
        ctx.beginPath();
        ctx.arc(this.cursorX, this.cursorY, 4, 0, Math.PI * 2);
        ctx.fillStyle = "rgba(255, 255, 255, 0.9)";
        ctx.shadowColor = "rgba(96, 165, 250, 0.8)";
        ctx.shadowBlur = 8;
        ctx.fill();
        ctx.shadowBlur = 0;
      }
    }

    ctx.restore();
  }

  /**
   * Crisp vector icon renderer for common radial actions.
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
    ctx.strokeStyle = isActive ? "#ffffff" : "rgba(255, 255, 255, 0.85)";
    ctx.fillStyle = isActive ? "#ffffff" : "rgba(255, 255, 255, 0.85)";
    ctx.lineWidth = 1.6;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";

    const name = iconName.toLowerCase();

    switch (name) {
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
        ctx.stroke();
        break;
      }
    }

    ctx.restore();
  }
}
