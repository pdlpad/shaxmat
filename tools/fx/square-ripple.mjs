// A ripple that fits one 24x24 square, for legal move targets. The stock water
// ripple is 136 wide and made for a pond.
import { TAU, clamp, hex } from "../pixelfx/src/core/index.js";

export const meta = {
  name: "Square ripple",
  desc: "Rings spreading inside one board square.",
  tags: ["ripple", "chess", "loop"],
  size: [24, 24],
  origin: [12, 23],
  duration: 1,
  loop: true,
  fps: 30,
  options: { color: "#FCCB3A", core: "#080018", shape: "circle", rings: 2, seed: 1 },
};

const CENTER_Y = -11.5, MAX_RADIUS = 11;

function dots(shape, radius) {
  const n = Math.max(8, Math.round(radius * 6)), out = [];
  for (let i = 0; i < n; i++) {
    const a = (i / n) * TAU, c = Math.cos(a), s = Math.sin(a);
    const norm = shape === "diamond" ? Math.abs(c) + Math.abs(s) : shape === "square" ? Math.max(Math.abs(c), Math.abs(s)) : 1;
    out.push([c * radius / norm, s * radius / norm]);
  }
  return out;
}

export function draw(p, t, o) {
  const color = hex(o.color), core = hex(o.core), phase = (t % 1 + 1) % 1;
  p.rect(-2, CENTER_Y - 2.5, 5, 5, core);
  p.rect(-1, CENTER_Y - 1.5, 3, 3, color);
  for (let i = 0; i < o.rings; i++) {
    const age = (phase + i / o.rings) % 1, radius = 3 + age * (MAX_RADIUS - 3);
    const fade = clamp(1.2 - age * 1.2, 0, 1);
    for (const [x, y] of dots(o.shape, radius)) p.dot(x + 0.5, y + CENTER_Y, color, fade);
  }
}
