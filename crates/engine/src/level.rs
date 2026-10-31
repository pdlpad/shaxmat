//! Strength levels. Weak levels search shallowly and then choose imperfectly, but
//! the choice is filtered so that their mistakes look human: they fall for quiet
//! threats beyond their horizon and do not simply leave pieces hanging.

use shaxmat_core::{Move, Position};

use crate::rng::Rng;
use crate::{Clock, Engine, Limits, MATE_BOUND, ScoredMove, SearchPosition, Searcher};

pub const LEVEL_COUNT: usize = 8;

/// A candidate that loses more than this to a one-move capture sequence is
/// rejected as an implausible blunder.
const BLUNDER_GUARD_CP: i32 = 150;

/// Starting values, to be tuned by calibration. Record every change in
/// `docs/calibration.md`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelParams {
    pub depth: Option<u32>,
    pub time_ms: u64,
    pub target_elo: u32,
    /// Deviation of the noise added to each candidate's score, in centipawns.
    pub noise_cp: f64,
    /// Chance of picking any acceptable candidate instead of the noisy best.
    pub slip_chance: f64,
    /// How many moves are candidates at all.
    pub top_n: usize,
    /// How far below the best move a candidate may be, in centipawns.
    pub window_cp: i32,
}

const fn level(
    depth: Option<u32>,
    time_ms: u64,
    target_elo: u32,
    noise_cp: f64,
    slip_chance: f64,
    top_n: usize,
    window_cp: i32,
) -> LevelParams {
    LevelParams {
        depth,
        time_ms,
        target_elo,
        noise_cp,
        slip_chance,
        top_n,
        window_cp,
    }
}

const LEVELS: [LevelParams; LEVEL_COUNT] = [
    level(Some(1), 500, 400, 110.0, 0.35, 8, 260),
    level(Some(2), 500, 700, 100.0, 0.30, 7, 240),
    level(Some(3), 500, 1000, 95.0, 0.28, 6, 220),
    level(Some(4), 500, 1300, 80.0, 0.22, 5, 180),
    level(Some(6), 1000, 1600, 35.0, 0.09, 4, 100),
    level(Some(8), 1000, 1900, 20.0, 0.05, 3, 60),
    level(Some(10), 2000, 2200, 0.0, 0.0, 1, 0),
    level(None, 3000, 2500, 0.0, 0.0, 1, 0),
];

/// Parameters of a level from 1 to [`LEVEL_COUNT`].
///
/// # Panics
/// If `level` is out of range.
pub fn level_params(level: usize) -> LevelParams {
    assert!(
        (1..=LEVEL_COUNT).contains(&level),
        "level {level} out of range"
    );
    LEVELS[level - 1]
}

/// An engine playing at a fixed strength level.
pub struct LevelEngine {
    params: LevelParams,
    searcher: Searcher,
    rng: Rng,
    /// The position of a started search.
    pending: Option<SearchPosition>,
}

impl LevelEngine {
    pub fn new(level: usize, seed: u64, clock: Box<dyn Clock>) -> Self {
        LevelEngine {
            params: level_params(level),
            searcher: Searcher::new(clock),
            rng: Rng::new(seed),
            pending: None,
        }
    }

    pub fn params(&self) -> &LevelParams {
        &self.params
    }

    fn limits(&self) -> Limits {
        Limits {
            depth: self.params.depth,
            time_ms: Some(self.params.time_ms),
            nodes: None,
            top_n: self.params.top_n,
        }
    }

    /// Starts choosing a move, to be driven by [`LevelEngine::step`]. The level's own
    /// limits apply; `limits` can only tighten them.
    pub fn start(&mut self, position: &SearchPosition, limits: &Limits) {
        let own = self.limits();
        let limits = Limits {
            depth: tightest(own.depth, limits.depth),
            time_ms: tightest(own.time_ms, limits.time_ms),
            nodes: limits.nodes,
            top_n: own.top_n,
        };
        self.pending = Some(position.clone());
        self.searcher.start(position, &limits);
    }

    /// Thinks for about `slice_ms`, or to the end with `None`. Returns `None` while
    /// there is work left, then the chosen move, if any is legal.
    pub fn step(&mut self, slice_ms: Option<u64>) -> Option<Option<Move>> {
        let scored = self.searcher.step(slice_ms)?;
        let position = self.pending.take().expect("a search was started");
        if scored.len() <= 1 || position.position.legal_moves().len() == 1 {
            return Some(scored.first().map(|m| m.mv));
        }
        let candidates = self.candidates(&position, scored);
        Some(Some(self.choose(&candidates)))
    }

    /// The moves a level may play: close enough to the best, and not an outright
    /// blunder. The best move always qualifies.
    fn candidates(
        &mut self,
        position: &SearchPosition,
        scored: Vec<ScoredMove>,
    ) -> Vec<ScoredMove> {
        let best = scored[0];
        let shallow = self
            .searcher
            .analyze(position, &Limits::depth(1).with_top_n(0));
        let shallow_best = shallow[0].score;
        let window = self.params.window_cp;
        scored
            .into_iter()
            .filter(|candidate| {
                if candidate.mv == best.mv {
                    return true;
                }
                let shallow_score = shallow
                    .iter()
                    .find(|m| m.mv == candidate.mv)
                    .map_or(i32::MIN, |m| m.score);
                best.score - candidate.score <= window
                    && shallow_best - shallow_score <= BLUNDER_GUARD_CP
                    && candidate.score > -MATE_BOUND
            })
            .collect()
    }

    fn choose(&mut self, candidates: &[ScoredMove]) -> Move {
        if candidates.len() > 1 && self.rng.unit() < self.params.slip_chance {
            return candidates[self.rng.below(candidates.len())].mv;
        }
        let noise_cp = self.params.noise_cp;
        let mut best = candidates[0];
        let mut best_score = f64::MIN;
        for &candidate in candidates {
            let score = candidate.score as f64 + self.rng.gaussian() * noise_cp;
            if score > best_score {
                best_score = score;
                best = candidate;
            }
        }
        best.mv
    }
}

impl Engine for LevelEngine {
    /// The level's own limits apply; `limits` can only tighten them.
    fn best_move(&mut self, position: &SearchPosition, limits: &Limits) -> Option<Move> {
        self.start(position, limits);
        self.step(None).flatten()
    }

    fn analyze(&mut self, position: &SearchPosition, limits: &Limits) -> Vec<ScoredMove> {
        self.searcher.analyze(position, limits)
    }
}

fn tightest<T: Ord>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use shaxmat_core::{Color, Game, Role, Square, parse_fen};

    use super::*;
    use crate::NativeClock;

    fn engine(level: usize, seed: u64) -> LevelEngine {
        LevelEngine::new(level, seed, Box::new(NativeClock::default()))
    }

    #[test]
    fn levels_get_stronger_on_paper() {
        for pair in LEVELS.windows(2) {
            assert!(pair[0].target_elo < pair[1].target_elo);
            assert!(pair[0].noise_cp >= pair[1].noise_cp);
            assert!(pair[0].slip_chance >= pair[1].slip_chance);
        }
    }

    #[test]
    fn low_levels_vary_and_top_levels_do_not() {
        let position = SearchPosition::from(&Game::new());
        let moves = |level, limits: &Limits| -> std::collections::HashSet<_> {
            (0..30)
                .map(|seed| engine(level, seed).best_move(&position, limits).unwrap())
                .collect()
        };
        assert!(moves(1, &Limits::default()).len() > 1);
        assert_eq!(moves(7, &Limits::depth(3)).len(), 1);
    }

    #[test]
    fn weak_levels_do_not_hang_the_queen() {
        // The knight attacks the queen, and she can also take on f2 or e4.
        let position =
            parse_fen("rnb1kbnr/pppp1ppp/8/4p3/4P2q/5N2/PPPP1PPP/RNBQKB1R b KQkq - 0 3").unwrap();
        for level in 1..=6 {
            for seed in 0..10 {
                let mv = engine(level, seed)
                    .best_move(&(&position).into(), &Limits::depth(4))
                    .unwrap();
                let mut game = Game::from_position(position.clone());
                game.play(mv).unwrap();
                let hung = game
                    .legal_moves()
                    .iter()
                    .any(|reply| reply.capture() == Some(Role::Queen));
                assert!(!hung, "level {level} seed {seed} hung the queen");
            }
        }
    }

    #[test]
    fn only_weak_levels_walk_into_a_quiet_fork() {
        // Nc7+ forks king and queen, which takes three plies to see.
        let position = parse_fen("q3k3/pp6/8/1N6/8/8/PP6/4K3 b - - 0 1").unwrap();
        let fork_works = |mv: Move| {
            let mut game = Game::from_position(position.clone());
            game.play(mv).unwrap();
            let queen_stayed = game.position().board().role_at(Square::A8) == Some(Role::Queen);
            let check = game.play_san("Nc7+").is_ok();
            let board = game.position().board();
            let safe = board
                .attacks_to(Square::C7, Color::Black, board.occupied())
                .is_empty();
            queen_stayed && check && safe
        };
        let forked = |level, limits: Limits| {
            (0..30)
                .filter(|&seed| {
                    let mv = engine(level, seed)
                        .best_move(&(&position).into(), &limits)
                        .unwrap();
                    fork_works(mv)
                })
                .count()
        };
        assert!(forked(1, Limits::default()) > 0);
        assert_eq!(forked(7, Limits::depth(5)), 0);
    }
}
