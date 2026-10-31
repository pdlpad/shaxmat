# Engine calibration

Every change to the level parameters in `crates/engine/src/level.rs` gets an entry here.

Setup: Stockfish 19 with `UCI_LimitStrength`, single thread, 16 MB hash, `go movetime 200`.
Each match is 40 games from 20 balanced openings, each opening from both sides, draw after
400 plies. Reproduce with:

```bash
cargo run --release -p shaxmat-engine-cli --bin engine-cli -- ladder --games 40
```
```bash
STOCKFISH_PATH=/path/to/stockfish cargo run --release -p shaxmat-engine-cli --bin engine-cli -- calibrate --games 40
```

## How far to trust the numbers

- 40 games give roughly ±70-100 Elo at best. The same match with another seed moved L8
  from 2383 to 2245.
- `UCI_Elo` stops at 1320. L3 and L4 play that anchor directly; L1 and L2 are chained
  through self-play, which overstates differences, so they are lower bounds.
- Stockfish's limited levels blunder randomly, unlike these levels, so the low anchor is
  shaky. L4 scores 71% against Stockfish 1320, yet L5 (only 140 Elo higher on paper) beat
  it 40-0.
- `UCI_Elo` is an engine rating, not a human one. Human play on the game itself is the
  check that counts.

## Current parameters

| Level | Depth | Max ms | Noise cp | Slip | Top N | Window cp | Target | Estimate |
|---|---|---|---|---|---|---|---|---|
| 1 | 1 | 500 | 110 | 0.35 | 8 | 260 | 400 | 280 (chained) |
| 2 | 2 | 500 | 100 | 0.30 | 7 | 240 | 700 | 661 (chained) |
| 3 | 3 | 500 | 95 | 0.28 | 6 | 220 | 1000 | 1173 |
| 4 | 4 | 500 | 80 | 0.22 | 5 | 180 | 1300 | 1478 |
| 5 | 6 | 1000 | 35 | 0.09 | 4 | 100 | 1600 | 1609-1617 |
| 6 | 8 | 1000 | 20 | 0.05 | 3 | 60 | 1900 | 1874-1900 |
| 7 | 10 | 2000 | 0 | 0 | 1 | 0 | 2200 | 2191-2200 |
| 8 | none | 3000 | 0 | 0 | 1 | 0 | 2500 | 2245-2383 |

The task gives no Elo target for level 8; 2500 is only the anchor it is measured against.

## Current level matches

| Match | Wins | Draws | Losses | Score | Elo diff | Max ms/move |
|---|---|---|---|---|---|---|
| L2 vs L1 | 33 | 3 | 4 | 86.2% | +319 | 74 |
| L3 vs L2 | 32 | 1 | 7 | 81.2% | +255 | 228 |
| L4 vs L3 | 30 | 2 | 8 | 77.5% | +215 | 149 |
| L5 vs L4 | 40 | 0 | 0 | 100.0% | +759 | 1000 |
| L6 vs L5 | 34 | 3 | 3 | 88.8% | +359 | 1020 |
| L7 vs L6 | 33 | 3 | 4 | 86.2% | +319 | 2031 |
| L8 vs L7 | 26 | 11 | 3 | 78.8% | +228 | 3030 |

## History

### 2026-11-01: starting parameters

| Level | Noise cp | Slip | Top N | Window cp |
|---|---|---|---|---|
| 2 | 90 | 0.28 | 7 | 220 |
| 3 | 70 | 0.20 | 6 | 180 |
| 4 | 50 | 0.14 | 5 | 140 |

Levels 1 and 5-8 had the values in the table above. Results: L3 1276 (43.8% against
Stockfish 1320), L4 1478 (71.2%), L2 640 and L1 128 (chained), L5 1609 and 1617, L6 1900
and 1874, L7 2200 and 2191, L8 2383 and 2245 (two seeds).

### 2026-11-01: weaker L2-L4

L3 and L4 were 170-280 Elo above target and L1-L2 too far below them, so noise, slip and
window went up for L2-L4 to the current values. Results: L3 1173 (30.0%), L4 1478 (71.2%,
unchanged within noise), L2 661, L1 280. The L5-L4 gap widened to 40-0.

L4 is left 180 above target on purpose: given the shaky anchor, further tuning waits for
games played by hand.
