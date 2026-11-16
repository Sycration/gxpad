// The part of the plane the canvas shows: a center in world units and a scale in CSS pixels per unit,
// with y pointing up in the world and down on the screen.

export class View {
  constructor() {
    this.cx = 0;
    this.cy = 0;
    this.scale = 60;
    this.w = 800;
    this.h = 600;
  }

  sx(x) {
    return this.w / 2 + (x - this.cx) * this.scale;
  }

  sy(y) {
    return this.h / 2 - (y - this.cy) * this.scale;
  }

  /** Screen position [x, y] of a world position. */
  toScreen(p) {
    return [this.sx(p[0]), this.sy(p[1])];
  }

  /** World position of a screen position. */
  toWorld(x, y) {
    return [this.cx + (x - this.w / 2) / this.scale, this.cy - (y - this.h / 2) / this.scale];
  }

  /** The world window: lower-left and upper-right corners and the size of a pixel. */
  window() {
    const hw = this.w / 2 / this.scale, hh = this.h / 2 / this.scale;
    return { x0: this.cx - hw, y0: this.cy - hh, x1: this.cx + hw, y1: this.cy + hh, px: 1 / this.scale };
  }

  /** Zooms by factor `f`, keeping the world point under screen position (x, y) in place. */
  zoomAt(x, y, f) {
    const before = this.toWorld(x, y);
    this.scale = Math.min(1e5, Math.max(0.2, this.scale * f));
    const after = this.toWorld(x, y);
    this.cx += before[0] - after[0];
    this.cy += before[1] - after[1];
  }

  /** Moves the content by (dx, dy) screen pixels. */
  panBy(dx, dy) {
    this.cx -= dx / this.scale;
    this.cy += dy / this.scale;
  }

  /** Shows the world box [x0, y0, x1, y1] with margins in pixels (left, top, right, bottom). */
  fit(box, m = [40, 40, 40, 40]) {
    const [x0, y0, x1, y1] = box;
    const w = Math.max(x1 - x0, 1), hh = Math.max(y1 - y0, 1);
    const aw = Math.max(this.w - m[0] - m[2], 50), ah = Math.max(this.h - m[1] - m[3], 50);
    this.scale = Math.min(2000, Math.max(2, Math.min(aw / w, ah / hh)));
    // the center of the free area, which the margins shift off the center of the canvas
    this.cx = (x0 + x1) / 2 - (m[0] - m[2]) / 2 / this.scale;
    this.cy = (y0 + y1) / 2 + (m[1] - m[3]) / 2 / this.scale;
  }

  key() {
    return `${this.cx},${this.cy},${this.scale},${this.w},${this.h}`;
  }
}
