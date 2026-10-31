// What to try for each game event. Colors come from assets/palette.toml.

const GOLD = ["#3A2410", "#8A5A14", "#FCCB3A", "#FFE98A", "#FFFBE0"];
const SKY = ["#080018", "#02103A", "#2C4C90", "#7A9AD8", "#F8F0E0", "#FFFBE0"];
const FIRE = ["#080018", "#D8402C", "#C8801C", "#FCCB3A", "#FFFBE0"];

const one = (label, id, options, speed) => ({ label, speed, parts: [{ id, options }] });

const CELLS = [[1, 1], [2, 2], [3, 3]];
const onCells = (label, options) => ({
  label,
  parts: CELLS.map((at) => ({ id: "custom/square-ripple", at, options })),
});

export const DRAFTS = {
  select: {
    w: 120, h: 96, seconds: 2, piece: "wQ", pieces: [{ name: "wQ", at: [2, 3] }],
    drafts: [
      one("halo gold", "holy/halo", { palette: GOLD }),
      one("halo sky", "holy/halo", { palette: SKY }),
      one("blessing", "holy/blessing", { palette: GOLD }),
      one("charge", "arcane/spell-charge", { palette: GOLD }),
      one("rune", "arcane/rune-circle", { palette: GOLD }),
      one("shield", "holy/divine-shield", { palette: GOLD }),
      one("heal", "holy/heal", { palette: GOLD }),
      one("haste", "status/haste", { palette: GOLD }),
    ],
  },
  targets: {
    w: 120, h: 96, seconds: 2, piece: null, pieces: [{ name: "wB", at: [0, 0] }],
    drafts: [
      onCells("circle gold", { color: "#FCCB3A" }),
      onCells("circle sky", { color: "#7A9AD8", core: "#F8F0E0" }),
      onCells("circle ink", { color: "#080018", core: "#FCCB3A" }),
      onCells("diamond", { shape: "diamond" }),
      onCells("square", { shape: "square" }),
      onCells("one ring", { rings: 1 }),
      { label: "stock gold", parts: [{ id: "water/ripples", at: [2, 2], options: { palette: GOLD } }] },
      { label: "stock sky", parts: [{ id: "water/ripples", at: [2, 2], options: { palette: SKY } }] },
    ],
  },
  capture: {
    w: 120, h: 96, seconds: 1.6, piece: "bN", pieces: [{ name: "bN", at: [2, 3] }],
    drafts: [
      one("poof x2", "spawn/poof", {}, 2),
      one("poof gold x2", "spawn/poof", { palette: GOLD }, 2),
      { label: "flash+poof", speed: 2, parts: [{ id: "impacts/hit-flash" }, { id: "spawn/poof", t: 0.3, hides: true }] },
      one("burn x2", "spawn/dissolve-burn", { palette: FIRE }, 2),
      one("burn x3", "spawn/dissolve-burn", {}, 3),
      one("shatter x2", "spawn/shatter-death", {}, 2),
      { label: "spark+poof", speed: 2, parts: [{ id: "impacts/hit-spark", options: { palette: GOLD } }, { id: "spawn/poof", t: 0.1, hides: true }] },
      one("crit x1.5", "impacts/crit-hit", { palette: GOLD }, 1.5),
    ],
  },
  castle: {
    w: 120, h: 72, seconds: 2, piece: "wK", pieces: [{ name: "wK", at: [1, 2] }],
    drafts: [
      { label: "out/in x2", speed: 2, parts: [{ id: "arcane/blink-out", at: [1, 2] }, { id: "arcane/blink-in", t: 0.7, at: [3, 2] }] },
      { label: "gold x2", speed: 2, parts: [{ id: "arcane/blink-out", at: [1, 2], options: { palette: GOLD } }, { id: "arcane/blink-in", t: 0.7, at: [3, 2], options: { palette: GOLD } }] },
      { label: "sky x2", speed: 2, parts: [{ id: "arcane/blink-out", at: [1, 2], options: { palette: SKY } }, { id: "arcane/blink-in", t: 0.7, at: [3, 2], options: { palette: SKY } }] },
      { label: "gold x3", speed: 3, parts: [{ id: "arcane/blink-out", at: [1, 2], options: { palette: GOLD } }, { id: "arcane/blink-in", t: 0.7, at: [3, 2], options: { palette: GOLD } }] },
      { label: "default x1", speed: 1, parts: [{ id: "arcane/blink-out", at: [1, 2] }, { id: "arcane/blink-in", t: 0.7, at: [3, 2] }] },
      { label: "gold x1.5", speed: 1.5, parts: [{ id: "arcane/blink-out", at: [1, 2], options: { palette: GOLD } }, { id: "arcane/blink-in", t: 0.7, at: [3, 2], options: { palette: GOLD } }] },
    ],
  },
  check: {
    w: 168, h: 168, seconds: 2, piece: "bK", pieces: [{ name: "bK", at: [3, 6] }],
    columns: 3, scale: 1,
    drafts: [
      one("strike x2", "lightning/lightning-strike", {}, 2),
      one("strike gold x2", "lightning/lightning-strike", { palette: GOLD }, 2),
      one("strike sky x3", "lightning/lightning-strike", { palette: SKY }, 3),
      one("zap", "lightning/zap-hit", {}),
      one("thunderclap", "lightning/thunderclap", { palette: GOLD }),
      one("electrified", "lightning/electrified", { palette: GOLD }),
    ],
  },
  mate: {
    w: 240, h: 192, seconds: 4, piece: "bK", pieces: [{ name: "bK", at: [4, 6] }],
    columns: 3, scale: 1,
    drafts: [
      one("disintegrate", "spawn/disintegrate", {}),
      one("dis gold", "spawn/disintegrate", { palette: GOLD, accent: FIRE.slice(1, 4) }),
      { label: "boom+dis", parts: [{ id: "explosions/big-explosion" }, { id: "spawn/disintegrate" }] },
      { label: "boom x2+dis", speed: 2, parts: [{ id: "explosions/big-explosion" }, { id: "spawn/disintegrate" }] },
      { label: "small+dis", parts: [{ id: "explosions/small-explosion", t: 0.45 }, { id: "spawn/disintegrate" }] },
      { label: "plasma+dis", parts: [{ id: "explosions/plasma-explosion", t: 0.45, options: { palette: GOLD } }, { id: "spawn/disintegrate", options: { palette: GOLD } }] },
    ],
  },
  promotion: {
    w: 120, h: 144, seconds: 2.5, piece: "wQ", pieces: [{ name: "wQ", at: [2, 5] }],
    drafts: [
      { label: "beam+bless", parts: [{ id: "spawn/respawn-beam" }, { id: "holy/blessing", t: 1.5 }] },
      { label: "gold", parts: [{ id: "spawn/respawn-beam", options: { palette: GOLD } }, { id: "holy/blessing", t: 1.5, options: { palette: GOLD } }] },
      { label: "gold x2", speed: 2, parts: [{ id: "spawn/respawn-beam", options: { palette: GOLD } }, { id: "holy/blessing", t: 1.5, options: { palette: GOLD } }] },
      { label: "sky x2", speed: 2, parts: [{ id: "spawn/respawn-beam", options: { palette: SKY } }, { id: "holy/blessing", t: 1.5, options: { palette: SKY } }] },
      one("pillar", "holy/holy-pillar", { palette: GOLD }),
      one("sunburst", "holy/sunburst", { palette: GOLD }),
    ],
  },
  hint: {
    w: 192, h: 96, seconds: 1.2, piece: null, pieces: [{ name: "wN", at: [1, 3] }],
    columns: 3,
    drafts: [
      { label: "beam", parts: [{ id: "arcane/arcane-beam", at: [1, 3] }] },
      { label: "beam gold", parts: [{ id: "arcane/arcane-beam", at: [1, 3], options: { palette: GOLD } }] },
      { label: "beam sky", parts: [{ id: "arcane/arcane-beam", at: [1, 3], options: { palette: SKY } }] },
      { label: "missiles", parts: [{ id: "arcane/arcane-missiles", at: [1, 3], options: { palette: GOLD } }] },
      { label: "arc", parts: [{ id: "lightning/spark-arc", at: [4, 3], options: { palette: GOLD } }] },
      { label: "orb", parts: [{ id: "arcane/arcane-orb", at: [4, 3], options: { palette: GOLD } }] },
    ],
  },
  blunder: {
    w: 192, h: 144, seconds: 1.1, piece: null, pieces: [{ name: "wQ", at: [3, 4] }],
    columns: 3,
    drafts: [
      { label: "pow", parts: [{ id: "explosions/cartoon-pow", at: [3, 4] }] },
      { label: "pow gold", parts: [{ id: "explosions/cartoon-pow", at: [3, 4], options: { palette: GOLD } }] },
      { label: "pow x1.5", speed: 1.5, parts: [{ id: "explosions/cartoon-pow", at: [3, 4], options: { palette: FIRE } }] },
      { label: "!", parts: [{ id: "ui/exclamation", at: [3, 4] }] },
      { label: "crit", parts: [{ id: "impacts/crit-hit", at: [3, 4], options: { palette: FIRE } }] },
      { label: "frame", parts: [{ id: "impacts/impact-frame", at: [3, 4] }] },
    ],
  },
  brilliant: {
    w: 240, h: 168, seconds: 3, piece: null, pieces: [{ name: "wN", at: [4, 5] }],
    columns: 3, scale: 1,
    drafts: [
      { label: "ach+confetti", parts: [{ id: "ui/confetti", at: [4, 6] }, { id: "ui/achievement", at: [4, 1], options: { palette: GOLD } }] },
      { label: "confetti", parts: [{ id: "ui/confetti", at: [4, 6] }] },
      { label: "quest", parts: [{ id: "ui/quest-complete", at: [4, 3] }] },
      { label: "level up", parts: [{ id: "ui/level-up", at: [4, 5] }] },
      { label: "firework", parts: [{ id: "explosions/firework-burst", at: [4, 3], options: { palette: GOLD } }] },
      { label: "sunburst", parts: [{ id: "holy/sunburst", at: [4, 5], options: { palette: GOLD } }] },
    ],
  },
  transition: {
    w: 320, h: 180, seconds: 2.2, screen: true, piece: null, pieces: [], columns: 3, scale: 1,
    drafts: [
      one("iris", "transitions/iris", { color: "#02103A", palette: GOLD }),
      one("diamond", "transitions/diamond-wipe", { color: "#02103A", palette: GOLD }),
      one("star", "transitions/star-iris", { color: "#02103A", palette: GOLD }),
      one("dissolve", "transitions/pixel-dissolve", { color: "#02103A" }),
      one("shatter", "transitions/shatter-wipe", { color: "#02103A" }),
      one("slash", "transitions/slash-wipe", { color: "#02103A", palette: GOLD }),
    ],
  },
};
