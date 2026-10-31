#!/usr/bin/env node
// Bakes the chosen effects into assets/fx/: a transparent PNG and an Aseprite-style
// JSON per sheet, plus manifest.rs, which embeds them all in the game.
//
//   node tools/fx/bake.mjs
//
// Needs tools/pixelfx, see tools/fx/README.md.

import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { inflateSync } from "node:zlib";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url)), ROOT = join(HERE, "..", "..");
const PIXELFX = join(HERE, "..", "pixelfx"), OUT = join(ROOT, "assets", "fx");
const importFx = (path) => import(pathToFileURL(join(PIXELFX, path)).href);
const { createFrame, clearFrame, createFx, composite, actorAlphaOf } = await importFx("src/core/index.js");
const { encodePNG } = await importFx("tools/lib/png.mjs");

const SQUARE = 24;
const GOLD = ["#3A2410", "#8A5A14", "#FCCB3A", "#FFE98A", "#FFFBE0"];
// Gold that still shows on cream squares.
const AMBER = ["#3A2410", "#8A5A14", "#C8801C", "#FCCB3A", "#FFE98A"];
const FIRE = ["#080018", "#D8402C", "#C8801C", "#FCCB3A", "#FFFBE0"];
const NAVY = "#02103A";
const BODIES = ["wP", "wN", "wB", "wR", "wQ", "bP", "bN", "bB", "bR", "bQ"];

const pieceFile = readFileSync(join(ROOT, "assets", "pieces.png"));
function piecePixels() {
  const width = pieceFile.readUInt32BE(16), chunks = [];
  for (let at = 8; at < pieceFile.length; ) {
    const size = pieceFile.readUInt32BE(at);
    if (pieceFile.toString("latin1", at + 4, at + 8) === "IDAT") chunks.push(pieceFile.subarray(at + 8, at + 8 + size));
    at += 12 + size;
  }
  const raw = inflateSync(Buffer.concat(chunks)), stride = width * 4 + 1;
  return (x, y) => raw.readUInt32LE(y * stride + 1 + x * 4);
}
const pixelAt = piecePixels();

function pieceActor(name) {
  const column = "PNBRQK".indexOf(name[1]), row = name[0] === "w" ? 0 : 1;
  const pixels = new Uint32Array(SQUARE * SQUARE), data = new Uint8Array(SQUARE * SQUARE);
  let top = SQUARE;
  for (let y = 0; y < SQUARE; y++)
    for (let x = 0; x < SQUARE; x++) {
      const c = pixelAt(column * SQUARE + x, row * SQUARE + y);
      if (c >>> 24) ((pixels[y * SQUARE + x] = c), (data[y * SQUARE + x] = 1), (top = Math.min(top, y)));
    }
  return { mask: { w: SQUARE, h: SQUARE, data }, pixels, x: -SQUARE / 2, y: -SQUARE, w: SQUARE, h: SQUARE, k: 2, head: [0, top - SQUARE], chest: [0, -12] };
}

/** An actor with no body, for effects whose light should not paint a piece. */
function noActor() {
  const data = new Uint8Array(SQUARE * SQUARE);
  return { mask: { w: SQUARE, h: SQUARE, data }, pixels: new Uint32Array(SQUARE * SQUARE), x: -SQUARE / 2, y: -SQUARE, w: SQUARE, h: SQUARE, k: 2, head: [0, -SQUARE], chest: [0, -12] };
}

// name, effect, and how to play it. speed: faster than authored. skip: seconds cut
// from the start. until: authored second to stop at. fade: seconds of fade at the end.
// lift: pixels to move the effect down, onto a piece of our height.
const SHEETS = [
  { name: "select-halo", id: "holy/halo", options: { palette: AMBER }, actor: "none", lift: 24, loop: true, fps: 15 },
  { name: "target-ripple", file: "square-ripple.mjs", options: { color: "#080018", core: "#FCCB3A" }, loop: true },
  { name: "poof", id: "spawn/poof", speed: 2 },
  ...BODIES.map((body) => ({ name: `hit-flash-${body}`, id: "impacts/hit-flash", actor: body, speed: 2 })),
  ...["wK", "bK"].flatMap((body) => [
    { name: `blink-out-${body}`, id: "arcane/blink-out", actor: body, speed: 2, options: { palette: GOLD } },
    { name: `blink-in-${body}`, id: "arcane/blink-in", actor: body, speed: 2, options: { palette: GOLD } },
    { name: `disintegrate-${body}`, id: "spawn/disintegrate", actor: body, fps: 20 },
  ]),
  { name: "lightning", id: "lightning/lightning-strike", options: { palette: GOLD }, speed: 2, skip: 0.36, until: 2.2, fade: 0.15 },
  { name: "explosion", id: "explosions/big-explosion", speed: 2, until: 5, fade: 0.4, fps: 15 },
  ...["wQ", "wR", "wB", "wN", "bQ", "bR", "bB", "bN"].map((body) => ({
    name: `respawn-${body}`, id: "spawn/respawn-beam", actor: body, speed: 2, fps: 20, options: { palette: GOLD },
  })),
  { name: "blessing", id: "holy/blessing", actor: "none", options: { palette: GOLD }, speed: 2, until: 1.6, fade: 0.2, fps: 20 },
  { name: "hint-beam", id: "arcane/arcane-beam", options: { palette: GOLD }, loop: true, fps: 15 },
  { name: "pow", id: "explosions/cartoon-pow", options: { palette: FIRE }, speed: 1.5 },
  { name: "confetti", id: "ui/confetti", fps: 20 },
  { name: "achievement", id: "ui/achievement", options: { palette: GOLD, title: "", text: "" }, fps: 20 },
  { name: "iris", id: "transitions/iris", options: { color: NAVY, palette: GOLD, focus: [160, 90] }, speed: 3 },
];

async function load(sheet) {
  if (sheet.file) return import(pathToFileURL(join(HERE, sheet.file)).href);
  return importFx(`src/fx/${sheet.id}.js`);
}

async function bake(sheet) {
  const mod = await load(sheet), { meta } = mod;
  const [W, H] = meta.size, [OX, OY] = meta.origin;
  const fps = sheet.fps ?? meta.fps, speed = sheet.speed ?? 1, skip = sheet.skip ?? 0;
  const until = sheet.until ?? meta.duration;
  const n = Math.max(1, Math.round(((until - skip) / speed) * fps));
  const options = { ...sheet.options };
  if (sheet.actor === "none") options.actor = noActor();
  else if (sheet.actor) options.actor = pieceActor(sheet.actor);
  const inst = createFx(mod, { x: OX, y: OY, ...options });
  for (let t = 0; t < skip; t += 1 / 120) inst.update(1 / 120);

  const frame = createFrame(W, H), images = [];
  let left = W, right = 0, top = H, bottom = 0, hiddenFrom = null, hiddenTo = null;
  for (let i = 0; i < n; i++) {
    if (i > 0) inst.update(speed / fps);
    clearFrame(frame);
    inst.draw(frame);
    const rgba = composite(frame, new Uint8ClampedArray(W * H * 4), null);
    const t = i / fps, fadeFrom = (until - (sheet.fade ?? 0) - skip) / speed;
    if (sheet.fade && t > fadeFrom) {
      const k = 1 - (t - fadeFrom) / (sheet.fade / speed);
      for (let a = 3; a < rgba.length; a += 4) rgba[a] *= k;
    }
    images.push(rgba);
    for (let y = 0; y < H; y++)
      for (let x = 0; x < W; x++)
        if (rgba[(y * W + x) * 4 + 3]) {
          left = Math.min(left, x), right = Math.max(right, x);
          top = Math.min(top, y), bottom = Math.max(bottom, y);
        }
    if (meta.actor && sheet.actor !== "none" && mod.actorAlpha && actorAlphaOf(mod, inst.t, inst.options) <= 0) {
      hiddenFrom ??= t;
      hiddenTo = t + 1 / fps;
    }
  }

  const w = Math.max(1, right - left + 1), h = Math.max(1, bottom - top + 1);
  const columns = Math.ceil(Math.sqrt((n * h) / w)), rows = Math.ceil(n / columns);
  const sheetW = columns * w, sheetH = rows * h, out = new Uint8ClampedArray(sheetW * sheetH * 4);
  const frames = [];
  images.forEach((rgba, i) => {
    const x0 = (i % columns) * w, y0 = Math.floor(i / columns) * h;
    for (let y = 0; y < h; y++)
      out.set(rgba.subarray(((y + top) * W + left) * 4, ((y + top) * W + left + w) * 4), ((y0 + y) * sheetW + x0) * 4);
    frames.push({ filename: `${sheet.name} ${i}`, frame: { x: x0, y: y0, w, h }, duration: Math.round(1000 / fps) });
  });
  writeFileSync(join(OUT, `${sheet.name}.png`), encodePNG(sheetW, sheetH, out));
  writeFileSync(join(OUT, `${sheet.name}.json`), JSON.stringify({
    frames,
    meta: {
      app: "https://github.com/patch-gg/pixelfx", image: `${sheet.name}.png`, format: "RGBA8888",
      size: { w: sheetW, h: sheetH },
      pixelfx: { id: sheet.id ?? sheet.name, fps, loop: !!sheet.loop, frames: n, frameSize: [w, h], pivot: [OX - left, OY - top - (sheet.lift ?? 0)], columns },
      shaxmat: { hidden: hiddenFrom === null ? null : [hiddenFrom, hiddenTo] },
    },
  }));
  console.log(sheet.name, `${n} frames ${w}x${h}`);
}

rmSync(OUT, { recursive: true, force: true });
mkdirSync(OUT, { recursive: true });
for (const sheet of SHEETS) await bake(sheet);
const lines = SHEETS.map(({ name }) =>
  `    ("${name}", include_bytes!("${name}.png"), include_str!("${name}.json")),`);
writeFileSync(join(OUT, "manifest.rs"),
  `// Written by tools/fx/bake.mjs.\n&[\n${lines.join("\n")}\n]\n`);
