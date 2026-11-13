"""Writes the bitmaps of gxpad for Windows:

  toolbar.bmp  the 16 x 16 toolbar and tool box images in one strip, 32-bit with
               alpha, in the order of the Icon enumeration of src/common.h
  gxpad.ico    the program icon at 16, 32 and 48 pixels, in the BMP format that
               Windows XP reads

Requires Pillow. Run from this directory: python3 make_icons.py
"""
import math
import struct

from PIL import Image, ImageDraw

SS = 4  # drawing is done at SS times the size and scaled down

INK = (24, 40, 96, 255)      # lines of the geometry icons
RED = (205, 30, 30, 255)     # points
GRAY = (120, 128, 140, 255)  # helper lines
BLUE = (40, 90, 200, 255)


class Pen:
    """Draws an icon in a square of `size` pixels with coordinates from -1 to 1
    (y downwards), like the vector icons of gxpad."""

    def __init__(self, size):
        self.n = size * SS
        self.img = Image.new("RGBA", (self.n, self.n), (0, 0, 0, 0))
        self.d = ImageDraw.Draw(self.img)
        self.h = self.n * 0.5 * 0.86
        self.k = self.n / 16.0  # 1 unit of stroke = 1 pixel at 16 px

    def at(self, x, y):
        c = self.n / 2
        return (c + x * self.h, c + y * self.h)

    def line(self, a, b, w=1.4, col=INK):
        self.poly([a, b], w, col)

    def poly(self, pts, w=1.4, col=INK):
        ww = max(1, int(round(w * self.k)))
        self.d.line(pts, fill=col, width=ww, joint="curve")
        r = ww / 2
        for p in (pts[0], pts[-1]):
            self.d.ellipse([p[0] - r, p[1] - r, p[0] + r, p[1] + r], fill=col)

    def dashed(self, a, b, w=1.1, col=INK, dash=2.0, gap=1.6):
        L = math.dist(a, b)
        if L == 0:
            return
        ux, uy = (b[0] - a[0]) / L, (b[1] - a[1]) / L
        t, on = 0.0, True
        while t < L:
            s = (dash if on else gap) * self.k
            e = min(L, t + s)
            if on:
                self.line((a[0] + ux * t, a[1] + uy * t), (a[0] + ux * e, a[1] + uy * e), w, col)
            t, on = e, not on

    def circle(self, c, r, w=1.4, col=INK):
        ww = max(1, int(round(w * self.k)))
        self.d.ellipse([c[0] - r, c[1] - r, c[0] + r, c[1] + r], outline=col, width=ww)

    def dot(self, c, r=2.2, col=RED):
        r *= self.k
        self.d.ellipse([c[0] - r, c[1] - r, c[0] + r, c[1] + r], fill=col)

    def fill(self, pts, col, outline=None, w=1.0):
        self.d.polygon(pts, fill=col)
        if outline:
            self.poly(pts + [pts[0]], w, outline)

    def rect(self, x0, y0, x1, y1, fillc, outline=INK, w=1.0):
        self.fill([self.at(x0, y0), self.at(x1, y0), self.at(x1, y1), self.at(x0, y1)], fillc, outline, w)

    def arc(self, c, r, a0, a1, n=24):
        return [(c[0] + math.cos(a0 + (a1 - a0) * i / n) * r, c[1] - math.sin(a0 + (a1 - a0) * i / n) * r) for i in range(n + 1)]

    def arrow(self, tip, d, w=1.4, col=INK, l=3.2):
        L = math.hypot(*d) or 1
        dx, dy = d[0] / L, d[1] / L
        nx, ny = -dy, dx
        l *= self.k
        self.fill([tip, (tip[0] - dx * l + nx * l * 0.6, tip[1] - dy * l + ny * l * 0.6),
                   (tip[0] - dx * l - nx * l * 0.6, tip[1] - dy * l - ny * l * 0.6)], col)

    def curve(self, f, t0, t1, n=40, w=1.4, col=INK):
        self.poly([self.at(*f(t0 + (t1 - t0) * i / n)) for i in range(n + 1)], w, col)

    def done(self, size):
        return self.img.resize((size, size), Image.LANCZOS)


def ellipse_pts(p, a=0.9, b=0.55, rot=0.0):
    def f(t):
        x, y = a * math.cos(t), b * math.sin(t)
        return (x * math.cos(rot) - y * math.sin(rot), x * math.sin(rot) + y * math.cos(rot))
    return f


def icon(name, size=16):
    p = Pen(size)
    at, c = p.at, p.at(0, 0)
    h = p.h
    if name == "select":
        pts = [at(-0.5, -0.9), at(-0.5, 0.6), at(-0.15, 0.27), at(0.13, 0.88), at(0.36, 0.77), at(0.08, 0.17), at(0.55, 0.14)]
        p.fill(pts, (255, 255, 255, 255), (0, 0, 0, 255), 1.2)
    elif name == "point":
        p.circle(c, h * 0.62, 1.0, GRAY)
        p.dot(c, 2.6)
    elif name == "line":
        p.line(at(-0.7, 0.7), at(0.7, -0.7))
        p.dot(at(-0.7, 0.7))
        p.dot(at(0.7, -0.7))
    elif name == "circle":
        p.circle(c, h * 0.82)
        p.dot(c, 1.8)
    elif name == "circle3":
        p.circle(c, h * 0.8)
        for t in (0.3, 2.3, 4.2):
            p.dot((c[0] + math.cos(t) * h * 0.8, c[1] - math.sin(t) * h * 0.8), 2.0)
    elif name == "arc":
        p.poly(p.arc(at(-0.45, 0.55), h * 1.15, 0, math.pi / 2))
        p.dot(at(-0.45, 0.55), 1.8)
    elif name == "arc3":
        p.poly(p.arc(at(0, 0.6), h * 1.15, 0.12 * math.pi, 0.88 * math.pi))
        for t in (0.12, 0.5, 0.88):
            a = t * math.pi
            p.dot((at(0, 0.6)[0] + math.cos(a) * h * 1.15, at(0, 0.6)[1] - math.sin(a) * h * 1.15), 2.0)
    elif name == "ellipse":
        p.curve(ellipse_pts(p, 0.92, 0.5, -0.5), 0, 2 * math.pi)
    elif name == "hyperbola":
        for s in (-1, 1):
            p.curve(lambda u, s=s: (s * 0.25 * math.cosh(u), 0.62 * math.sinh(u)), -1.3, 1.3, 24)
    elif name == "parabola":
        p.curve(lambda x: (x, 0.75 - 1.6 * x * x), -0.85, 0.85, 24)
        p.dot(at(0, 0.25), 1.8)
    elif name == "ellipse_foci":
        p.curve(ellipse_pts(p), 0, 2 * math.pi)
        p.dot(at(-0.55, 0), 2.0)
        p.dot(at(0.55, 0), 2.0)
    elif name == "hyperbola_foci":
        for s in (-1, 1):
            p.curve(lambda u, s=s: (s * 0.4 * math.cosh(u), 0.65 * math.sinh(u)), -1.1, 1.1, 20)
            p.dot(at(s * 0.88, 0), 2.0)
    elif name == "parabola_focus":
        p.curve(lambda y: (-0.2 + 0.85 * y * y, y), -0.95, 0.95, 24)
        p.dot(at(0.12, 0), 2.0)
        p.dashed(at(-0.62, -0.95), at(-0.62, 0.95), 1.1, GRAY)
    elif name == "function":
        p.line(at(-0.95, 0.85), at(0.95, 0.85), 0.9, GRAY)
        p.line(at(-0.85, 0.95), at(-0.85, -0.95), 0.9, GRAY)
        p.curve(lambda x: (x, -0.1 - 0.55 * math.sin(x * 3.3)), -0.85, 0.95, 30, 1.4, BLUE)
    elif name in ("dimension", "distance"):
        p.line(at(-0.85, -0.6), at(-0.85, 0.6), 1.0)
        p.line(at(0.85, -0.6), at(0.85, 0.6), 1.0)
        p.line(at(-0.7, 0), at(0.7, 0), 1.2)
        p.arrow(at(-0.8, 0), (-1, 0))
        p.arrow(at(0.8, 0), (1, 0))
    elif name == "midpoint":
        p.line(at(-0.9, 0.5), at(0.9, -0.5))
        p.dot(at(-0.9, 0.5), 1.6, INK)
        p.dot(at(0.9, -0.5), 1.6, INK)
        p.dot(c, 2.6)
    elif name == "intersect":
        p.poly(p.arc(at(0, 1.6), h * 1.9, 0.18 * math.pi, 0.82 * math.pi))
        p.line(at(-0.9, 0.8), at(0.9, -0.8))
        p.dot(at(0, -0.3), 2.6)
    elif name == "perp_line":
        p.line(at(-0.9, 0.6), at(0.9, 0.6), 1.1, GRAY)
        p.line(at(0.1, 0.95), at(0.1, -0.95))
        p.poly([at(0.1, 0.3), at(0.4, 0.3), at(0.4, 0.6)], 1.0, GRAY)
        p.dot(at(0.1, -0.35), 2.2)
    elif name == "par_line":
        p.line(at(-0.9, 0.75), at(0.4, -0.75), 1.1, GRAY)
        p.line(at(-0.4, 0.75), at(0.9, -0.75))
        p.dot(at(0.25, 0), 2.2)
    elif name == "perp_bisector":
        p.line(at(-0.85, 0.45), at(0.85, 0.45), 1.1, GRAY)
        p.dot(at(-0.85, 0.45), 1.9)
        p.dot(at(0.85, 0.45), 1.9)
        p.line(at(0, 0.95), at(0, -0.95))
    elif name == "angle_bisector":
        p.line(at(-0.85, 0.75), at(0.9, 0.75), 1.1, GRAY)
        p.line(at(-0.85, 0.75), at(0.45, -0.85), 1.1, GRAY)
        p.line(at(-0.85, 0.75), at(0.95, -0.25))
    elif name == "tangent_line":
        p.circle(at(0.25, 0.3), h * 0.55, 1.1, GRAY)
        p.line(at(-0.9, -0.82), at(0.85, -0.22))
        p.dot(at(-0.9, -0.82), 2.0)
    elif name == "reflect":
        p.dashed(at(0, -0.95), at(0, 0.95), 1.1, GRAY)
        p.dot(at(-0.6, -0.3), 2.4)
        p.circle(at(0.6, -0.3), 2.4 * p.k, 1.1, RED)
        p.poly(p.arc(at(0, 0.15), h * 0.6, 0.2 * math.pi, 0.8 * math.pi, 10), 1.0, INK)
    elif name == "foci":
        p.curve(ellipse_pts(p), 0, 2 * math.pi, 40, 1.1, GRAY)
        p.dot(at(-0.55, 0), 2.3)
        p.dot(at(0.55, 0), 2.3)
    elif name == "directrix":
        p.curve(lambda y: (-0.35 + 0.9 * y * y, y), -0.9, 0.9, 24, 1.1, GRAY)
        p.dot(c, 1.9)
        p.line(at(-0.75, -0.95), at(-0.75, 0.95))
    elif name == "asymptotes":
        p.line(at(-0.9, -0.75), at(0.9, 0.75))
        p.line(at(-0.9, 0.75), at(0.9, -0.75))
        for s in (-1, 1):
            p.curve(lambda u, s=s: (s * 0.35 * math.cosh(u), 0.3 * math.sinh(u)), -1.2, 1.2, 16, 1.0, GRAY)
    elif name == "coincident":
        p.circle(at(-0.27, 0), h * 0.45, 1.2)
        p.circle(at(0.27, 0), h * 0.45, 1.2)
        p.dot(c, 2.0)
    elif name == "on_curve":
        p.poly(p.arc(at(0, 1.4), h * 1.6, 0.2 * math.pi, 0.8 * math.pi))
        p.dot(at(0, -0.2), 2.6)
    elif name == "midpoint_con":
        p.line(at(-0.9, 0.5), at(0.9, -0.5))
        p.dot(c, 2.6)
        p.line(at(-0.3, -0.6), at(0.3, 0.6), 1.0)
    elif name == "horizontal":
        p.line(at(-0.85, 0), at(0.85, 0), 1.8)
        p.dot(at(-0.85, 0), 2.0)
        p.dot(at(0.85, 0), 2.0)
    elif name == "vertical":
        p.line(at(0, -0.85), at(0, 0.85), 1.8)
        p.dot(at(0, -0.85), 2.0)
        p.dot(at(0, 0.85), 2.0)
    elif name == "parallel":
        p.line(at(-0.6, 0.8), at(0, -0.8), 1.6)
        p.line(at(0, 0.8), at(0.6, -0.8), 1.6)
    elif name == "perpendicular":
        p.line(at(-0.8, 0.7), at(0.8, 0.7), 1.6)
        p.line(at(0, 0.7), at(0, -0.8), 1.6)
        p.poly([at(0, 0.35), at(0.35, 0.35), at(0.35, 0.7)], 1.0)
    elif name == "collinear":
        p.line(at(-0.9, 0.6), at(-0.1, 0.07), 1.6)
        p.line(at(0.15, -0.1), at(0.9, -0.6), 1.6)
        p.dashed(at(-0.9, 0.6), at(0.9, -0.6), 0.9, GRAY)
    elif name == "equal":
        p.line(at(-0.7, -0.3), at(0.7, -0.3), 1.8)
        p.line(at(-0.7, 0.3), at(0.7, 0.3), 1.8)
    elif name == "tangent":
        p.circle(at(0, 0.2), h * 0.6, 1.4)
        p.line(at(-0.95, -0.42), at(0.95, -0.42), 1.4, BLUE)
    elif name == "concentric":
        p.circle(c, h * 0.85, 1.3)
        p.circle(c, h * 0.4, 1.3)
        p.dot(c, 1.5)
    elif name == "symmetric":
        p.dashed(at(0, -0.95), at(0, 0.95), 1.1, GRAY)
        p.dot(at(-0.6, 0), 2.6)
        p.dot(at(0.6, 0), 2.6)
    elif name == "lock":
        p.poly(p.arc(at(0, -0.25), h * 0.42, 0, math.pi, 16), 1.6, GRAY)
        p.rect(-0.62, -0.1, 0.62, 0.85, (230, 180, 40, 255), (140, 100, 0, 255))
        p.dot(at(0, 0.35), 1.4, (90, 60, 0, 255))
    elif name == "construction":
        p.dashed(at(-0.9, 0.9), at(0.9, -0.9), 1.6, INK, 3.4, 2.6)
    elif name == "infinite":
        p.line(at(-1.0, 0.45), at(1.0, -0.45))
        p.dot(at(-0.35, 0.16), 1.8)
        p.dot(at(0.35, -0.16), 1.8)
        p.arrow(at(-1.0, 0.45), (-1, 0.45), 1.0, INK, 2.6)
        p.arrow(at(1.0, -0.45), (1, -0.45), 1.0, INK, 2.6)
    elif name == "hdist":
        p.line(at(-0.85, -0.8), at(-0.85, 0.8), 1.0)
        p.line(at(0.85, -0.8), at(0.85, 0.8), 1.0)
        p.line(at(-0.7, 0.3), at(0.7, 0.3), 1.2)
        p.arrow(at(-0.8, 0.3), (-1, 0))
        p.arrow(at(0.8, 0.3), (1, 0))
        p.line(at(-0.4, -0.5), at(0.4, -0.5), 1.0, GRAY)
    elif name == "vdist":
        p.line(at(-0.8, -0.85), at(0.8, -0.85), 1.0)
        p.line(at(-0.8, 0.85), at(0.8, 0.85), 1.0)
        p.line(at(-0.3, -0.7), at(-0.3, 0.7), 1.2)
        p.arrow(at(-0.3, -0.8), (0, -1))
        p.arrow(at(-0.3, 0.8), (0, 1))
        p.line(at(0.5, -0.4), at(0.5, 0.4), 1.0, GRAY)
    elif name == "length":
        p.line(at(-0.85, 0.55), at(0.85, 0.55), 1.4)
        p.dot(at(-0.85, 0.55), 1.8)
        p.dot(at(0.85, 0.55), 1.8)
        p.line(at(-0.65, -0.3), at(0.65, -0.3), 1.1)
        p.arrow(at(-0.85, -0.3), (-1, 0), 1, INK, 2.8)
        p.arrow(at(0.85, -0.3), (1, 0), 1, INK, 2.8)
    elif name == "radius":
        p.circle(c, h * 0.85, 1.0, GRAY)
        p.line(c, at(0.5, -0.5), 1.3)
        p.arrow(at(0.6, -0.6), (1, -1))
        p.dot(c, 1.6, INK)
    elif name == "diameter":
        p.circle(c, h * 0.85, 1.0, GRAY)
        p.line(at(-0.5, 0.5), at(0.5, -0.5), 1.3)
        p.arrow(at(0.6, -0.6), (1, -1))
        p.arrow(at(-0.6, 0.6), (-1, 1))
    elif name == "angle":
        p.line(at(-0.8, 0.7), at(0.9, 0.7))
        p.line(at(-0.8, 0.7), at(0.5, -0.8))
        p.poly(p.arc(at(-0.8, 0.7), h * 0.95, 0, 0.85, 12), 1.1, RED)
    elif name in ("coord_x", "coord_y"):
        p.line(at(-0.9, 0.85), at(0.95, 0.85), 1.0, GRAY)
        p.line(at(-0.85, 0.9), at(-0.85, -0.95), 1.0, GRAY)
        q = at(0.45, -0.35)
        if name == "coord_x":
            p.dashed(q, at(0.45, 0.85), 1.0, GRAY)
            p.line(at(-0.85, 0.45), at(0.45, 0.45), 1.4)
        else:
            p.dashed(q, at(-0.85, -0.35), 1.0, GRAY)
            p.line(at(0.05, -0.35), at(0.05, 0.85), 1.4)
        p.dot(q, 2.2)
    elif name in ("axis_a", "axis_b"):
        p.curve(ellipse_pts(p), 0, 2 * math.pi, 40, 1.0, GRAY)
        p.line(c, at(0.9, 0) if name == "axis_a" else at(0, -0.55), 1.5)
        p.dot(c, 1.6)
    elif name == "new":
        p.fill([at(-0.62, -0.92), at(0.25, -0.92), at(0.66, -0.5), at(0.66, 0.95), at(-0.62, 0.95)], (255, 255, 255, 255), (90, 90, 90, 255), 1.0)
        p.fill([at(0.25, -0.92), at(0.25, -0.5), at(0.66, -0.5)], (215, 215, 215, 255), (90, 90, 90, 255), 1.0)
    elif name == "open":
        p.fill([at(-0.95, -0.55), at(-0.45, -0.55), at(-0.3, -0.38), at(0.6, -0.38), at(0.6, 0.8), at(-0.95, 0.8)], (230, 190, 70, 255), (150, 110, 20, 255), 1.0)
        p.fill([at(-0.7, -0.05), at(0.95, -0.05), at(0.6, 0.8), at(-0.95, 0.8)], (250, 215, 110, 255), (150, 110, 20, 255), 1.0)
    elif name == "save":
        p.fill([at(-0.88, -0.88), at(0.7, -0.88), at(0.88, -0.7), at(0.88, 0.88), at(-0.88, 0.88)], (60, 90, 170, 255), (20, 40, 100, 255), 1.0)
        p.rect(-0.55, -0.88, 0.5, -0.2, (240, 240, 240, 255), (200, 200, 200, 255), 0.6)
        p.rect(0.2, -0.8, 0.38, -0.32, (60, 90, 170, 255), None)
        p.rect(-0.6, 0.25, 0.6, 0.88, (220, 220, 225, 255), (150, 150, 160, 255), 0.6)
    elif name == "print":
        p.rect(-0.55, -0.92, 0.55, -0.15, (255, 255, 255, 255), (100, 100, 100, 255))
        p.rect(-0.95, -0.25, 0.95, 0.55, (190, 190, 195, 255), (80, 80, 85, 255))
        p.rect(-0.6, 0.4, 0.6, 0.92, (255, 255, 255, 255), (100, 100, 100, 255))
        p.dot(at(0.7, -0.05), 1.0, (40, 180, 40, 255))
    elif name in ("undo", "redo"):
        flip = -1 if name == "undo" else 1
        pts = [(c[0] - flip * (q[0] - c[0]), q[1]) for q in p.arc(at(0, 0.18), h * 0.62, -0.5 * math.pi, 0.75 * math.pi, 18)]
        p.poly(pts, 1.9, BLUE)
        p.arrow(pts[-1], (pts[-1][0] - pts[-3][0], pts[-1][1] - pts[-3][1]), 1, BLUE, 4.2)
    elif name == "delete":
        p.line(at(-0.6, -0.6), at(0.6, 0.6), 2.4, (210, 30, 30, 255))
        p.line(at(-0.6, 0.6), at(0.6, -0.6), 2.4, (210, 30, 30, 255))
    elif name in ("zoom_in", "zoom_out"):
        p.line(at(0.25, 0.25), at(0.85, 0.85), 2.6, (110, 80, 40, 255))
        p.d.ellipse([*at(-0.85, -0.85), *at(0.35, 0.35)], fill=(225, 240, 255, 255), outline=(60, 60, 70, 255), width=int(1.2 * p.k))
        p.line(at(-0.5, -0.25), at(0.0, -0.25), 1.4, (0, 0, 0, 255))
        if name == "zoom_in":
            p.line(at(-0.25, -0.5), at(-0.25, 0.0), 1.4, (0, 0, 0, 255))
    elif name == "fit":
        for sx, sy in ((-1, -1), (1, -1), (-1, 1), (1, 1)):
            k = at(0.85 * sx, 0.85 * sy)
            p.poly([(k[0] - 0.5 * h * sx, k[1]), k, (k[0], k[1] - 0.5 * h * sy)], 1.5, BLUE)
        p.rect(-0.35, -0.35, 0.35, 0.35, (255, 255, 255, 255), INK)
    elif name == "properties":
        p.rect(-0.9, -0.85, 0.9, 0.85, (255, 255, 255, 255), (90, 90, 90, 255))
        p.rect(-0.9, -0.85, 0.9, -0.5, (40, 90, 200, 255), (90, 90, 90, 255))
        for y in (-0.2, 0.15, 0.5):
            p.line(at(-0.6, y), at(-0.4, y), 1.2, (90, 90, 90, 255))
            p.line(at(-0.2, y), at(0.6, y), 1.2, GRAY)
    elif name == "help":
        p.d.ellipse([*at(-0.92, -0.92), *at(0.92, 0.92)], fill=(40, 90, 200, 255))
        p.poly(p.arc(at(0, -0.25), h * 0.32, math.pi * 1.0, -0.4 * math.pi, 14) + [at(0, 0.25)], 1.7, (255, 255, 255, 255))
        p.dot(at(0, 0.55), 1.3, (255, 255, 255, 255))
    elif name == "variable":
        p.line(at(-0.8, -0.5), at(-0.1, 0.5), 1.6)
        p.line(at(-0.1, -0.5), at(-0.8, 0.5), 1.6)
        p.line(at(0.2, -0.15), at(0.85, -0.15), 1.2, RED)
        p.line(at(0.2, 0.2), at(0.85, 0.2), 1.2, RED)
    elif name == "copy":
        p.rect(-0.85, -0.85, 0.3, 0.45, (255, 255, 255, 255), (90, 90, 90, 255))
        p.rect(-0.3, -0.3, 0.85, 0.92, (255, 255, 255, 255), (90, 90, 90, 255))
        for y in (0.0, 0.3, 0.6):
            p.line(at(-0.1, y), at(0.6, y), 0.9, GRAY)
    else:
        raise ValueError(name)
    return p.done(size)


# the order of the Icon enumeration in src/common.h
NAMES = """select point line circle circle3 arc arc3 ellipse hyperbola parabola
ellipse_foci hyperbola_foci parabola_focus function dimension
midpoint intersect perp_line par_line perp_bisector angle_bisector tangent_line
reflect foci directrix asymptotes
coincident on_curve midpoint_con horizontal vertical parallel perpendicular
collinear equal tangent concentric symmetric lock construction infinite
distance hdist vdist length radius diameter angle coord_x coord_y
axis_a axis_b
new open save print undo redo delete zoom_in zoom_out fit
properties help variable copy""".split()


def bmp32(img):
    """A 32-bit BMP with a BITMAPINFOHEADER and straight alpha, rows bottom-up."""
    w, h = img.size
    px = img.load()
    rows = bytearray()
    for y in range(h - 1, -1, -1):
        for x in range(w):
            r, g, b, a = px[x, y]
            rows += bytes((b, g, r, a))
    info = struct.pack("<IiiHHIIiiII", 40, w, h, 1, 32, 0, len(rows), 2835, 2835, 0, 0)
    head = struct.pack("<2sIHHI", b"BM", 14 + len(info) + len(rows), 0, 0, 14 + len(info))
    return head + info + bytes(rows)


def ico(images):
    """An .ico of BMP entries (32-bit with alpha and an AND mask), which XP reads."""
    entries, blobs = [], []
    offset = 6 + 16 * len(images)
    for img in images:
        w, h = img.size
        px = img.load()
        xor = bytearray()
        for y in range(h - 1, -1, -1):
            for x in range(w):
                r, g, b, a = px[x, y]
                xor += bytes((b, g, r, a))
        stride = ((w + 31) // 32) * 4
        mask = bytearray()
        for y in range(h - 1, -1, -1):
            row = bytearray(stride)
            for x in range(w):
                if px[x, y][3] < 128:
                    row[x // 8] |= 0x80 >> (x % 8)
            mask += row
        info = struct.pack("<IiiHHIIiiII", 40, w, h * 2, 1, 32, 0, len(xor) + len(mask), 0, 0, 0, 0)
        blob = info + bytes(xor) + bytes(mask)
        entries.append(struct.pack("<BBBBHHII", w % 256, h % 256, 0, 0, 1, 32, len(blob), offset))
        blobs.append(blob)
        offset += len(blob)
    return struct.pack("<HHH", 0, 1, len(images)) + b"".join(entries) + b"".join(blobs)


def app_icon(size):
    """A drafting triangle with its corners marked and an angle dimension."""
    p = Pen(size)
    at = p.at
    tri = [at(-0.85, 0.75), at(0.85, 0.75), at(-0.35, -0.85)]
    p.fill(tri, (255, 250, 220, 255), (24, 40, 96, 255), 1.5)
    p.poly(p.arc(at(-0.85, 0.75), p.h * 0.6, 0, 1.27, 16), 1.2, (128, 0, 160, 255))
    for q in tri:
        p.dot(q, 1.9)
    return p.done(size)


def main():
    strip = Image.new("RGBA", (16 * len(NAMES), 16), (0, 0, 0, 0))
    for i, n in enumerate(NAMES):
        strip.paste(icon(n), (16 * i, 0))
    with open("toolbar.bmp", "wb") as f:
        f.write(bmp32(strip))
    with open("gxpad.ico", "wb") as f:
        f.write(ico([app_icon(s) for s in (16, 32, 48)]))
    print(len(NAMES), "icons")


if __name__ == "__main__":
    main()
