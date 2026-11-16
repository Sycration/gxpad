// sgweb's icons: 24×24 SVG drawn with the current text color. Strokes are paths; `D` marks a filled dot.

const pts = (f, a, b, n) => {
  let d = "";
  for (let i = 0; i <= n; i++) {
    const [x, y] = f(a + ((b - a) * i) / n);
    d += (i ? "L" : "M") + x.toFixed(2) + " " + y.toFixed(2);
  }
  return d;
};
const P = (d, extra = "") => `<path d="${d}"${extra}/>`;
const C = (x, y, r) => `<circle cx="${x}" cy="${y}" r="${r}"/>`;
const D = (x, y, r = 1.9) => `<circle class="f" cx="${x}" cy="${y}" r="${r}"/>`;
const dash = (d) => P(d, ' stroke-dasharray="2.2 2.2"');
const thin = (d) => P(d, ' class="t"');
const ellipse = (rx, ry, rot = 0) => `<ellipse cx="12" cy="12" rx="${rx}" ry="${ry}" transform="rotate(${rot} 12 12)"/>`;
const hyp = (s, a, b, cx = 12) => pts((u) => [cx + s * a * Math.cosh(u), 12 + b * Math.sinh(u)], -1.25, 1.25, 16);
const parab = (k, x0, y0) => pts((y) => [x0 + k * (y - y0) * (y - y0), y], 3, 21, 18);
const arrows = (x0, y, x1) => P(`M${x0} ${y}H${x1}M${x0 + 3} ${y - 2.5}L${x0} ${y}l3 2.5M${x1 - 3} ${y - 2.5}L${x1} ${y}l-3 2.5`);

const ICONS = {
  select: P("M6 3.5v15l4-3.6 2.6 5.6 2.6-1.2-2.6-5.5 5.4-.4z"),
  point: D(12, 12, 3.2) + thin("M12 4v3M12 17v3M4 12h3M17 12h3"),
  line: P("M5 19L19 5") + D(5, 19, 2.2) + D(19, 5, 2.2),
  circle: C(12, 12, 8) + D(12, 12) + thin("M12 12l5.6-5.6"),
  circle3: C(12, 12, 8) + D(4.6, 15) + D(19.4, 15) + D(12, 4),
  arc: P("M5 18A10 10 0 0 1 18 5.5") + D(16, 17, 1.6) + D(5, 18, 2) + D(18, 5.5, 2),
  arc3: P("M4 17A9 9 0 0 1 20 17") + D(4, 17) + D(12, 8) + D(20, 17),
  ellipse: ellipse(9, 5, -25),
  hyperbola: P(hyp(1, 3, 6.5)) + P(hyp(-1, 3, 6.5)),
  parabola: P(pts((x) => [x, 20 - 0.21 * (x - 12) * (x - 12)], 4, 20, 18)) + D(12, 14.5, 1.5),
  ellipseFoci: ellipse(9, 5.5) + D(5.2, 12, 1.6) + D(18.8, 12, 1.6) + C(12, 6.5, 1.6),
  hyperbolaFoci: P(hyp(1, 3.5, 6)) + P(hyp(-1, 3.5, 6)) + D(3, 12, 1.6) + D(21, 12, 1.6),
  parabolaFocus: P(parab(0.06, 7, 12)) + D(10.5, 12, 1.6) + dash("M3.5 3v18"),
  function: P("M3 16c3 0 3-9 6-9s3 10 6 10 3-6 6-6") + thin("M3 20h18"),
  midpoint: P("M4 18L20 6") + C(12, 12, 2.6) + thin("M9 8l6 8"),
  intersect: P("M3.5 17C8 4 16 4 20.5 17") + P("M4 7l16 10") + D(12, 12, 2.2),
  perpLine: thin("M3 17h18") + P("M12 3v18") + thin("M12 13h4v4") + D(12, 7, 1.9),
  parLine: thin("M3 18L15 6") + P("M9 18L21 6") + D(15, 12, 1.9),
  perpBisector: P("M4 16h16") + D(4, 16) + D(20, 16) + dash("M12 3v18"),
  angleBisector: P("M4 19h17M4 19L13 4") + dash("M4 19l16-8"),
  tangentLine: C(14, 14, 6) + P("M3 4.5l17.2 3.3") + D(3, 4.5, 1.9),
  reflect: dash("M12 3v18") + D(6, 9, 2.2) + C(18, 9, 2.2) + thin("M7.5 15q4.5 3 9 0"),
  foci: ellipse(9, 5.5) + D(5.5, 12) + D(18.5, 12),
  directrix: P(parab(0.06, 9, 12)) + D(12.5, 12, 1.5) + P("M4 3v18"),
  asymptotes: thin("M3 5l18 14M3 19L21 5") + P(hyp(1, 3.5, 3.2)) + P(hyp(-1, 3.5, 3.2)),
  measure: thin("M3 6v12M21 6v12") + arrows(4, 12, 20),
  distance: thin("M3 6v12M21 6v12") + arrows(4, 12, 20),
  hdist: thin("M3 4v16M21 4v16") + arrows(4, 15, 20) + thin("M8 8h8"),
  vdist: thin("M4 3h16M4 21h16") + P("M9 4v16M6.5 7L9 4l2.5 3M6.5 17L9 20l2.5-3") + thin("M16 8v8"),
  coordX: thin("M3 20h18M4 3v18") + D(16, 8) + dash("M16 8v12") + P("M4 15h12"),
  coordY: thin("M3 20h18M4 3v18") + D(16, 8) + dash("M16 8H4") + P("M10 8v12"),
  radius: thin('M12 12m-8.5 0a8.5 8.5 0 1 0 17 0a8.5 8.5 0 1 0-17 0') + P("M12 12l5.5-5.5M14.2 6.6l3.3-.1-.1 3.3") + D(12, 12, 1.5),
  diameter: thin("M12 12m-8.5 0a8.5 8.5 0 1 0 17 0a8.5 8.5 0 1 0-17 0") + P("M6 18L18 6M14.7 6.1l3.3-.1-.1 3.3M9.3 17.9L6 18l.1-3.3"),
  angle: P("M4 19h17M4 19L14 4") + thin("M12 19a8 8 0 0 0-3.5-6.6"),
  axisA: ellipse(9, 5.5) + P("M12 12h9") + D(12, 12, 1.5),
  axisB: ellipse(9, 5.5) + P("M12 12V6.5") + D(12, 12, 1.5),
  coincident: C(9, 12, 4.5) + C(15, 12, 4.5) + D(12, 12, 1.6),
  onCurve: P("M3 18C7 7 17 7 21 18") + D(12, 9.8, 2.6),
  horizontal: P("M4 12h16", ' stroke-width="2.2"') + D(4, 12, 2.2) + D(20, 12, 2.2),
  vertical: P("M12 4v16", ' stroke-width="2.2"') + D(12, 4, 2.2) + D(12, 20, 2.2),
  parallel: P("M5 20L11 4M13 20l6-16"),
  perpendicular: P("M4 19h16M12 19V4") + thin("M12 15h4v4"),
  collinear: P("M3 18l7-5M14 10l7-5") + dash("M3 18L21 5"),
  equal: P("M5 9h14M5 15h14", ' stroke-width="2.2"'),
  tangent: C(12, 14, 6) + P("M3 8h18"),
  concentric: C(12, 12, 8.5) + C(12, 12, 4) + D(12, 12, 1.4),
  symmetric: dash("M12 3v18") + D(5.5, 12, 2.6) + D(18.5, 12, 2.6),
  lock: `<rect class="f" x="5.5" y="11" width="13" height="9" rx="2"/>` + P("M8.5 11V8a3.5 3.5 0 0 1 7 0v3"),
  unlock: `<rect class="f" x="5.5" y="11" width="13" height="9" rx="2"/>` + P("M8.5 11V8a3.5 3.5 0 0 1 6.8-1.2"),
  construction: dash("M4 20L20 4"),
  infinite: P("M2 12h20") + P("M5 9.5L2 12l3 2.5M19 9.5l3 2.5-3 2.5"),
  edit: P("M15 5l4 4L9 19H5v-4z") + thin("M13 7l4 4"),
  driving: arrows(3, 12, 21) + D(12, 12, 2.4),
  measured: thin("M3 6v12M21 6v12") + P("M7 12h10", ' stroke-dasharray="2 2"') + P("M9 9.5L6.5 12 9 14.5M15 9.5l2.5 2.5-2.5 2.5"),
  supplement: P("M3 17h18M12 17l5-11") + thin("M7 17a5 5 0 0 1 7-4.5"),
  trash: P("M4.5 7h15M10 4h4M6.5 7l1 13h9l1-13M10 10.5v6M14 10.5v6"),
  undo: P("M9 14L4.5 9.5 9 5") + P("M4.5 9.5H14a5.5 5.5 0 0 1 0 11h-3"),
  redo: P("M15 14l4.5-4.5L15 5") + P("M19.5 9.5H10a5.5 5.5 0 0 0 0 11h3"),
  plus: P("M12 5v14M5 12h14"),
  close: P("M6 6l12 12M18 6L6 18"),
  menu: P("M4 7h16M4 12h16M4 17h16"),
  more: D(5.5, 12, 1.7) + D(12, 12, 1.7) + D(18.5, 12, 1.7),
  chevron: P("M7 10l5 5 5-5"),
  chevronRight: P("M10 7l5 5-5 5"),
  chevronLeft: P("M14 7l-5 5 5 5"),
  help: C(12, 12, 9) + P("M9.5 9.5a2.5 2.5 0 1 1 3.4 2.3c-.6.3-.9.8-.9 1.4v.6") + D(12, 17, 1.2),
  sun: C(12, 12, 4) + P("M12 2.5v2.5M12 19v2.5M2.5 12H5M19 12h2.5M5.3 5.3l1.8 1.8M16.9 16.9l1.8 1.8M5.3 18.7l1.8-1.8M16.9 7.1l1.8-1.8"),
  moon: P("M19.5 14.5A8 8 0 0 1 9.5 4.5a8 8 0 1 0 10 10z"),
  fit: P("M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5") + `<rect x="8.5" y="8.5" width="7" height="7" rx="1"/>`,
  eye: P("M2.5 12s3.5-6.5 9.5-6.5S21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z") + C(12, 12, 2.8),
  tag: P("M3.5 12.5V4h8.5l8.5 8.5-8.5 8.5z") + D(8, 8.5, 1.5),
  folder: P("M3 7a2 2 0 0 1 2-2h4l2 2.5h8a2 2 0 0 1 2 2V18a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"),
  save: P("M12 3.5v11M7.5 10L12 14.5 16.5 10") + P("M4 15.5V20h16v-4.5"),
  image: `<rect x="3" y="4.5" width="18" height="15" rx="2"/>` + P("M3 16l5-5 4 4 3-3 6 6") + D(15.5, 9, 1.6),
  link: P("M10 14a4.5 4.5 0 0 0 6.4 0l3-3a4.5 4.5 0 0 0-6.4-6.4L11.5 6") + P("M14 10a4.5 4.5 0 0 0-6.4 0l-3 3a4.5 4.5 0 0 0 6.4 6.4l1.5-1.5"),
  file: P("M6 3h8l4.5 4.5V21H6z") + P("M14 3v4.5h4.5"),
  copy: `<rect x="8" y="8" width="12" height="12" rx="2"/>` + P("M16 8V5.5A1.5 1.5 0 0 0 14.5 4h-9A1.5 1.5 0 0 0 4 5.5v9A1.5 1.5 0 0 0 5.5 16H8"),
  code: P("M8.5 7L3.5 12l5 5M15.5 7l5 5-5 5M13.5 4.5l-3 15"),
  latex: P("M3 13l2.5-1.5 3 7.5L13 4.5h8") + thin("M14.5 10l5 6M19.5 10l-5 6"),
  variable: P("M4 7l7 10M11 7L4 17") + thin("M14 10.5h6.5M14 14.5h6.5"),
  numeric: P("M9 4.5l-2 15M17 4.5l-2 15M5 9.5h15M4 15h15"),
  text: P("M5 6h14M5 10h14M5 14h9M5 18h11"),
  sliders: P("M4 7h10M18 7h2M4 17h4M12 17h8") + C(16, 7, 2) + C(10, 17, 2),
  result: P("M4 6h7M4 6l3.5 6L4 18h7") + P("M14 10h6M14 14h6"),
  warning: P("M12 4L2.5 20h19z") + P("M12 10v4.5") + D(12, 17.3, 1.1),
  grip: D(9, 6, 1.4) + D(15, 6, 1.4) + D(9, 12, 1.4) + D(15, 12, 1.4) + D(9, 18, 1.4) + D(15, 18, 1.4),
  zoomIn: C(10.5, 10.5, 6.5) + P("M15.5 15.5L21 21M8 10.5h5M10.5 8v5"),
  zoomOut: C(10.5, 10.5, 6.5) + P("M15.5 15.5L21 21M8 10.5h5"),
  check: P("M5 12.5l4.5 4.5L19 7.5"),
};

/** The SVG markup of icon `name`. */
export function icon(name, cls = "") {
  return `<svg class="ic ${cls}" viewBox="0 0 24 24" aria-hidden="true">${ICONS[name] ?? ICONS.help}</svg>`;
}

/** Icon `name` as an element. */
export function iconEl(name, cls = "") {
  const t = document.createElement("template");
  t.innerHTML = icon(name, cls);
  return t.content.firstElementChild;
}

export const iconNames = Object.keys(ICONS);
