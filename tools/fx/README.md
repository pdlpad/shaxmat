# Effects tooling

Effects come from [pixelfx](https://github.com/patch-gg/pixelfx), which is not vendored.
To rebuild them, check it out at the commit used:

```bash
git clone https://github.com/patch-gg/pixelfx tools/pixelfx
```
```bash
git -C tools/pixelfx checkout a841596
```

- `node tools/fx/drafts.mjs [event...]` renders numbered drafts on our board into
  `records/stage5/`. The candidates are in `draft-list.mjs`.
- `node tools/fx/bake.mjs` bakes the chosen effects into `assets/fx/`, recolored from
  `assets/palette.toml`, with the pieces from `assets/pieces.png` as actors. It also
  writes `manifest.rs`, which embeds them in the game.

`square-ripple.mjs` is our own effect: the stock ripple is far wider than a square.

What plays on which event is in `crates/fx/src/events.rs`.
