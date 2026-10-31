#!/usr/bin/env node
// Renders effect drafts on our own board with our own pieces, one numbered grid per
// game event, for choosing a direction by eye.
//
//   node tools/fx/drafts.mjs            every event
//   node tools/fx/drafts.mjs check mate only these
//
// Needs tools/pixelfx (see tools/fx/README.md) and ffmpeg. Output: records/stage5/.

import { readFileSync, mkdirSync } from "node:fs";
import { inflateSync } from "node:zlib";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { DRAFTS } from "./draft-list.mjs";

const HERE = dirname(fileURLToPath(import.meta.url)), ROOT = join(HERE, "..", "..");
const PIXELFX = join(HERE, "..", "pixelfx");
const importFx = (path) => import(pathToFileURL(join(PIXELFX, path)).href);
const { createFrame, clearFrame, createFx, composite, actorAlphaOf } = await importFx("src/core/index.js");
const { writeVideo, save, canvas } = await importFx("tools/lib/fxlib.mjs");

const SQUARE = 24, FPS = 30;
const abgr = (hex) => {
  const v = parseInt(hex.slice(1), 16);
  return (255 << 24 | (v & 255) << 16 | (v >> 8 & 255) << 8 | v >> 16) >>> 0;
};
const LIGHT = abgr("#F8F0E0"), DARK = abgr("#2C4C90");

// tools/gen_sprites.py writes unfiltered PNGs, so filter type 0 is all this has to read
function readPieces() {
  const file = readFileSync(join(ROOT, "assets", "pieces.png"));
  const width = file.readUInt32BE(16), height = file.readUInt32BE(20);
  const chunks = [];
  for (let at = 8; at < file.length; ) {
    const size = file.readUInt32BE(at);
    if (file.toString("latin1", at + 4, at + 8) === "IDAT") chunks.push(file.subarray(at + 8, at + 8 + size));
    at += 12 + size;
  }
  const raw = inflateSync(Buffer.concat(chunks)), stride = width * 4 + 1;
  return (x, y) => raw.readUInt32LE(y * stride + 1 + x * 4);
}

const pixelAt = readPieces();
const ORDER = "PNBRQK";

/** A piece as a pixelfx actor, with its feet at the origin. */
function pieceActor(name) {
  const column = ORDER.indexOf(name[1]), row = name[0] === "w" ? 0 : 1;
  const pixels = new Uint32Array(SQUARE * SQUARE), data = new Uint8Array(SQUARE * SQUARE);
  let top = SQUARE;
  for (let y = 0; y < SQUARE; y++)
    for (let x = 0; x < SQUARE; x++) {
      const c = pixelAt(column * SQUARE + x, row * SQUARE + y);
      if (c >>> 24) ((pixels[y * SQUARE + x] = c), (data[y * SQUARE + x] = 1), (top = Math.min(top, y)));
    }
  const y = -SQUARE;
  return { mask: { w: SQUARE, h: SQUARE, data }, pixels, x: -SQUARE / 2, y, w: SQUARE, h: SQUARE, k: 2, head: [0, y + top], chest: [0, -12] };
}

function drawActor(frame, actor, left, top, alpha) {
  if (alpha <= 0) return;
  for (let y = 0; y < SQUARE; y++)
    for (let x = 0; x < SQUARE; x++) {
      const c = actor.pixels[y * SQUARE + x];
      if (c >>> 24) frame.color[(top + y) * frame.w + left + x] = c;
    }
}

function boardBackground(w, h, flat) {
  const bg = new Uint32Array(w * h);
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++)
      bg[y * w + x] = flat ?? ((Math.floor(x / SQUARE) + Math.floor(y / SQUARE)) % 2 ? DARK : LIGHT);
  return bg;
}

const modules = new Map();
async function load(id) {
  if (!modules.has(id)) modules.set(id, await (id.startsWith("custom/") ? import(pathToFileURL(join(HERE, id.slice(7) + ".mjs")).href) : importFx(`src/fx/${id}.js`)));
  return modules.get(id);
}

/** One draft: parts running on a clock, over the board. */
async function renderDraft(draft, event, index) {
  const { w, h, seconds } = event, n = Math.round(seconds * FPS);
  const speed = draft.speed ?? 1;
  const bg = boardBackground(w, h, event.flat);
  const frame = createFrame(w, h);
  const parts = [];
  for (const part of draft.parts) {
    const mod = await load(part.id);
    const at = part.at ?? event.pieces?.[0]?.at ?? [0, 0];
    const options = { ...part.options };
    const actor = part.piece ?? event.piece;
    if (mod.meta.actor && actor) options.actor = pieceActor(actor);
    const x = event.screen ? 0 : at[0] * SQUARE + SQUARE / 2, y = event.screen ? 0 : at[1] * SQUARE + SQUARE - 1;
    parts.push({ mod, fx: null, part, options, x, y, start: part.t ?? 0, at });
  }

  const frames = [];
  for (let i = 0; i < n; i++) {
    const t = (i / FPS) * speed;
    clearFrame(frame);
    const live = [];
    for (const p of parts) {
      if (t < p.start) continue;
      if (!p.fx) p.fx = createFx(p.mod, { x: p.x, y: p.y, ...p.options });
      else p.fx.update(speed / FPS);
      live.push(p);
    }
    for (const p of live) p.fx.draw(frame, "back");
    for (const piece of event.pieces ?? []) {
      const hider = live.find((p) => p.mod.meta.actor && p.at[0] === piece.at[0] && p.at[1] === piece.at[1]);
      const hidden = parts.find((p) => p.part.hides && p.at[0] === piece.at[0] && p.at[1] === piece.at[1] && t >= p.start);
      const alpha = hidden ? 0 : hider ? actorAlphaOf(hider.mod, hider.fx.t, hider.options) : 1;
      drawActor(frame, pieceActor(piece.name), piece.at[0] * SQUARE, piece.at[1] * SQUARE, alpha);
    }
    for (const p of live) p.fx.draw(frame, "front");
    const rgba = composite(frame, new Uint8ClampedArray(w * h * 4), bg);
    frames.push(rgba);
  }
  return frames;
}

async function renderEvent(name) {
  const event = DRAFTS[name];
  const columns = event.columns ?? 4, rows = Math.ceil(event.drafts.length / columns);
  const gap = 2, w = columns * (event.w + gap) - gap, h = rows * (event.h + gap + 10) - gap;
  const grids = [];
  const rendered = [];
  for (let i = 0; i < event.drafts.length; i++) rendered.push(await renderDraft(event.drafts[i], event, i));
  const n = rendered[0].length;
  for (let f = 0; f < n; f++) {
    const c = canvas(w, h, [8, 0, 24, 255]);
    rendered.forEach((frames, i) => {
      const x = (i % columns) * (event.w + gap), y = Math.floor(i / columns) * (event.h + gap + 10);
      c.paste(frames[f], event.w, event.h, x, y + 10);
      c.text(`${i + 1} ${event.drafts[i].label}`, x + 1, y + 2, 1, [248, 240, 224]);
    });
    grids.push({ t: f / FPS, color: c.data });
  }
  const out = join(ROOT, "records", "stage5");
  mkdirSync(out, { recursive: true });
  const k = event.scale ?? 2;
  writeVideo(join(out, `${name}.gif`), { w, h, fps: FPS, frames: grids }, k);
  console.log(name, `${event.drafts.length} drafts`, `${w}x${h}`);
}

const names = process.argv.length > 2 ? process.argv.slice(2) : Object.keys(DRAFTS);
for (const name of names) await renderEvent(name);
