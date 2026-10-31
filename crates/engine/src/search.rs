use shaxmat_core::shakmaty::EnPassantMode;
use shaxmat_core::shakmaty::zobrist::Zobrist64;
use shaxmat_core::{Chess, Color, Move, MoveList, Position, Role};

use crate::tt::{Bound, TranspositionTable};
use crate::{Clock, Engine, Limits, ScoredMove, SearchPosition, evaluate};

/// Score of being checkmated right now, negated. A mate in `n` plies scores
/// `MATE - n`.
pub const MATE: i32 = 30_000;
/// Scores beyond this mean a forced mate.
pub const MATE_BOUND: i32 = MATE - MAX_PLY as i32;

const INFINITY: i32 = MATE + 1;
const DRAW: i32 = 0;
const MAX_PLY: usize = 128;
const MAX_DEPTH: u32 = 64;
const DEFAULT_TT_BITS: u32 = 19;
/// How many nodes pass between looks at the clock.
const CLOCK_CHECK_INTERVAL: u64 = 1024;
const FIFTY_MOVE_HALFMOVES: u32 = 100;
/// A new iteration takes several times the previous one, so it is not started
/// after this share of the time budget is gone.
const ITERATION_TIME_SHARE: f64 = 0.5;

const TT_MOVE_ORDER: i32 = 1 << 24;
const CAPTURE_ORDER: i32 = 1 << 22;
const KILLER_ORDER: i32 = 1 << 21;
const HISTORY_LIMIT: i32 = 1 << 20;

const NULL_MOVE_MIN_DEPTH: i32 = 3;
const NULL_MOVE_REDUCTION: i32 = 2;
const LMR_MIN_DEPTH: i32 = 3;
const LMR_MIN_MOVES: usize = 3;
const LMR_DEEP_DEPTH: i32 = 6;
const LMR_LATE_MOVES: usize = 10;

/// Negamax alpha-beta search. See [`Limits`] for how to bound it. The transposition
/// table carries over between calls; [`Searcher::clear`] resets it for a new game.
pub struct Searcher {
    clock: Box<dyn Clock>,
    tt: TranspositionTable,
    /// Repetition keys of the game and the current search path.
    path: Vec<u64>,
    nodes: u64,
    node_limit: u64,
    deadline_ms: Option<u64>,
    /// End of the current time slice, if the host is slicing.
    slice_end_ms: Option<u64>,
    /// Depth 1 runs to the end whatever the limits, so there is always a move.
    can_stop: bool,
    stopped: bool,
    /// The search stopped for the slice, not for good.
    paused: bool,
    task: Option<RootTask>,
    completed_depth: u32,
    killers: Box<[[Option<Move>; 2]; MAX_PLY + 1]>,
    /// Indexed by color, moving role and target square.
    history: Vec<i32>,
}

impl Searcher {
    pub fn new(clock: Box<dyn Clock>) -> Self {
        Searcher {
            clock,
            tt: TranspositionTable::new(DEFAULT_TT_BITS),
            path: Vec::new(),
            nodes: 0,
            node_limit: u64::MAX,
            deadline_ms: None,
            slice_end_ms: None,
            can_stop: false,
            stopped: false,
            paused: false,
            task: None,
            completed_depth: 0,
            killers: Box::new([[None; 2]; MAX_PLY + 1]),
            history: vec![0; 2 * 6 * 64],
        }
    }

    pub fn clear(&mut self) {
        self.tt.clear();
    }

    /// Nodes visited by the last search.
    pub fn nodes(&self) -> u64 {
        self.nodes
    }

    /// The last fully searched depth of the last search.
    pub fn completed_depth(&self) -> u32 {
        self.completed_depth
    }

    /// Sets up a search to be driven by [`Searcher::step`].
    pub fn start(&mut self, input: &SearchPosition, limits: &Limits) {
        let position = &input.position;
        let legal = position.legal_moves();
        let top_n = match limits.top_n {
            0 => legal.len(),
            n => n.min(legal.len()),
        };

        let start_ms = self.clock.now_ms();
        self.nodes = 0;
        self.node_limit = limits.nodes.unwrap_or(u64::MAX);
        self.deadline_ms = limits.time_ms.map(|ms| start_ms + ms);
        self.can_stop = false;
        self.completed_depth = 0;
        self.killers.fill([None; 2]);
        self.history.fill(0);
        self.path.clear();
        self.path.extend(input.history.iter().map(|key| key.0));

        let unlimited = limits.time_ms.is_none() && limits.nodes.is_none();
        let default_depth = if unlimited {
            Limits::DEFAULT_DEPTH
        } else {
            MAX_DEPTH
        };

        self.task = (!legal.is_empty()).then(|| RootTask {
            position: position.clone(),
            order: legal
                .iter()
                .map(|&mv| ScoredMove { mv, score: 0 })
                .collect(),
            best: Vec::new(),
            partial: Vec::with_capacity(top_n + 1),
            next_move: 0,
            depth: 1,
            max_depth: limits.depth.unwrap_or(default_depth).clamp(1, MAX_DEPTH),
            top_n,
            time_ms: limits.time_ms,
            start_ms,
        });
    }

    /// Runs the started search for about `slice_ms`, or to the end with `None`.
    /// Returns the result once done, and `None` while there is work left, so a
    /// single-threaded host can keep drawing frames in between.
    pub fn step(&mut self, slice_ms: Option<u64>) -> Option<Vec<ScoredMove>> {
        let Some(mut task) = self.task.take() else {
            return Some(Vec::new());
        };
        self.slice_end_ms = slice_ms.map(|ms| self.clock.now_ms() + ms);
        self.stopped = false;
        self.paused = false;

        while task.depth <= task.max_depth {
            self.search_root(&mut task);
            if self.paused {
                self.task = Some(task);
                return None;
            }
            if self.stopped {
                break;
            }
            self.completed_depth = task.depth;
            self.can_stop = true;
            task.finish_iteration();

            let forced = task.order.len() == 1 || task.best[0].score.abs() >= MATE_BOUND;
            if forced && task.top_n == 1 {
                break;
            }
            if let Some(time_ms) = task.time_ms {
                let spent = self.clock.now_ms() - task.start_ms;
                if spent as f64 > time_ms as f64 * ITERATION_TIME_SHARE {
                    break;
                }
            }
        }
        Some(task.best)
    }

    fn run(&mut self, input: &SearchPosition, limits: &Limits) -> Vec<ScoredMove> {
        self.start(input, limits);
        self.step(None).unwrap_or_default()
    }

    /// Scores the root moves, picking up where the last pause left off. Only the
    /// best `top_n` get an exact score: each other move only has to prove it stays
    /// below the current last place.
    fn search_root(&mut self, task: &mut RootTask) {
        let depth = task.depth as i32;
        while task.next_move < task.order.len() {
            let mv = task.order[task.next_move].mv;
            let alpha = if task.partial.len() == task.top_n {
                task.partial[task.top_n - 1].score
            } else {
                -INFINITY
            };
            let mut next = task.position.clone();
            next.play_unchecked(mv);
            let mut score = -INFINITY;
            if alpha > -INFINITY {
                score = -self.negamax(&next, depth - 1, -alpha - 1, -alpha, 1);
            }
            if alpha == -INFINITY || (score > alpha && !self.stopped) {
                score = -self.negamax(&next, depth - 1, -INFINITY, -alpha, 1);
            }
            if self.stopped {
                return;
            }
            if score > alpha {
                let at = task.partial.partition_point(|r| r.score >= score);
                task.partial.insert(at, ScoredMove { mv, score });
                task.partial.truncate(task.top_n);
            }
            task.next_move += 1;
        }
    }

    fn negamax(
        &mut self,
        position: &Chess,
        depth: i32,
        mut alpha: i32,
        beta: i32,
        ply: usize,
    ) -> i32 {
        self.count_node();
        if self.stopped {
            return DRAW;
        }

        let key = position.zobrist_hash::<Zobrist64>(EnPassantMode::Legal).0;
        if self.is_repetition(position, key) || position.is_insufficient_material() {
            return DRAW;
        }

        let in_check = position.is_check();
        let depth = depth + i32::from(in_check);
        if depth <= 0 || ply >= MAX_PLY {
            return self.quiescence(position, alpha, beta, ply);
        }

        let moves = position.legal_moves();
        if moves.is_empty() {
            return if in_check { ply as i32 - MATE } else { DRAW };
        }
        if position.halfmoves() >= FIFTY_MOVE_HALFMOVES {
            return DRAW;
        }

        let is_pv = beta - alpha > 1;
        let entry = self.tt.probe(key);
        if !is_pv
            && let Some(entry) = entry
            && entry.depth >= depth
        {
            let score = score_from_tt(entry.score, ply);
            let usable = match entry.bound {
                Bound::Exact => true,
                Bound::Lower => score >= beta,
                Bound::Upper => score <= alpha,
            };
            if usable {
                return score;
            }
        }

        if !is_pv && !in_check && self.null_move_cuts(position, key, depth, beta, ply) {
            return beta;
        }
        if self.stopped {
            return DRAW;
        }

        let original_alpha = alpha;
        let mut best_score = -INFINITY;
        let mut best_move = None;
        let tt_move = entry.and_then(|e| e.mv);
        let turn = position.turn();
        let mut ordered = OrderedMoves::new(moves, |mv| self.order_score(mv, tt_move, turn, ply));

        self.path.push(key);
        let mut searched = 0;
        while let Some(mv) = ordered.next() {
            let mut next = position.clone();
            next.play_unchecked(mv);
            let quiet = !is_noisy(mv);

            let score = if searched == 0 {
                -self.negamax(&next, depth - 1, -beta, -alpha, ply + 1)
            } else {
                // A late quiet move is probably bad: look less deep, and with a
                // null window. Anything that beats alpha gets a proper search.
                let reduction = if quiet && !in_check && !next.is_check() {
                    late_move_reduction(depth, searched)
                } else {
                    0
                };
                let probe =
                    -self.negamax(&next, depth - 1 - reduction, -alpha - 1, -alpha, ply + 1);
                if probe > alpha && (reduction > 0 || probe < beta) {
                    -self.negamax(&next, depth - 1, -beta, -alpha, ply + 1)
                } else {
                    probe
                }
            };
            searched += 1;
            if self.stopped {
                self.path.pop();
                return DRAW;
            }

            if score > best_score {
                best_score = score;
                best_move = Some(mv);
            }
            alpha = alpha.max(score);
            if alpha >= beta {
                if quiet {
                    self.reward_quiet_move(mv, turn, depth, ply);
                }
                break;
            }
        }
        self.path.pop();

        let bound = if best_score >= beta {
            Bound::Lower
        } else if best_score > original_alpha {
            Bound::Exact
        } else {
            Bound::Upper
        };
        self.tt
            .store(key, best_move, score_to_tt(best_score, ply), depth, bound);
        best_score
    }

    /// If passing still leaves us above `beta`, a real move would too. Not used
    /// with only pawns on the board, where passing can be the best move.
    fn null_move_cuts(
        &mut self,
        position: &Chess,
        key: u64,
        depth: i32,
        beta: i32,
        ply: usize,
    ) -> bool {
        let board = position.board();
        let pieces = board.by_color(position.turn()) & !board.pawns() & !board.kings();
        if depth < NULL_MOVE_MIN_DEPTH
            || pieces.is_empty()
            || beta.abs() >= MATE_BOUND
            || evaluate(position) < beta
        {
            return false;
        }
        let Ok(passed) = position.clone().swap_turn() else {
            return false;
        };
        let reduction = NULL_MOVE_REDUCTION + depth / 6;
        self.path.push(key);
        let score = -self.negamax(&passed, depth - 1 - reduction, -beta, -beta + 1, ply + 1);
        self.path.pop();
        score >= beta && !self.stopped
    }

    /// The hash move, then captures and promotions by MVV-LVA, then killers, then
    /// quiet moves by history.
    fn order_score(&self, mv: Move, tt_move: Option<Move>, turn: Color, ply: usize) -> i32 {
        if Some(mv) == tt_move {
            TT_MOVE_ORDER
        } else if is_noisy(mv) {
            capture_order(mv)
        } else if self.killers[ply].contains(&Some(mv)) {
            KILLER_ORDER
        } else {
            self.history[history_index(mv, turn)]
        }
    }

    fn reward_quiet_move(&mut self, mv: Move, turn: Color, depth: i32, ply: usize) {
        let killers = &mut self.killers[ply];
        if killers[0] != Some(mv) {
            killers[1] = killers[0];
            killers[0] = Some(mv);
        }
        let history = &mut self.history[history_index(mv, turn)];
        *history += depth * depth;
        if *history >= HISTORY_LIMIT {
            self.history.iter_mut().for_each(|value| *value /= 2);
        }
    }

    /// Plays out captures and queen promotions, and every reply to a check, until
    /// the position is quiet.
    fn quiescence(&mut self, position: &Chess, mut alpha: i32, beta: i32, ply: usize) -> i32 {
        self.count_node();
        if self.stopped {
            return DRAW;
        }
        if ply >= MAX_PLY {
            return evaluate(position);
        }

        let mut moves = position.legal_moves();
        let in_check = position.is_check();
        if moves.is_empty() {
            return if in_check { ply as i32 - MATE } else { DRAW };
        }

        let mut best_score = -INFINITY;
        if !in_check {
            best_score = evaluate(position);
            if best_score >= beta {
                return best_score;
            }
            alpha = alpha.max(best_score);
            moves.retain(|mv| mv.is_capture() || mv.promotion() == Some(Role::Queen));
        }

        let mut ordered = OrderedMoves::new(moves, capture_order);
        while let Some(mv) = ordered.next() {
            let mut next = position.clone();
            next.play_unchecked(mv);
            let score = -self.quiescence(&next, -beta, -alpha, ply + 1);
            if self.stopped {
                return DRAW;
            }
            best_score = best_score.max(score);
            alpha = alpha.max(score);
            if alpha >= beta {
                break;
            }
        }
        best_score
    }

    /// True if the position already occurred in the game or the search path. Moves
    /// behind the last irreversible one cannot repeat, so the scan stops there.
    fn is_repetition(&self, position: &Chess, key: u64) -> bool {
        let reversible = position.halfmoves() as usize;
        self.path
            .iter()
            .rev()
            .take(reversible)
            .skip(1)
            .step_by(2)
            .any(|&earlier| earlier == key)
    }

    fn count_node(&mut self) {
        self.nodes += 1;
        if self.can_stop && self.nodes >= self.node_limit {
            self.stopped = true;
        }
        if !self.nodes.is_multiple_of(CLOCK_CHECK_INTERVAL) {
            return;
        }
        let now = self.clock.now_ms();
        if self.can_stop && self.deadline_ms.is_some_and(|deadline| now >= deadline) {
            self.stopped = true;
        } else if self.slice_end_ms.is_some_and(|end| now >= end) {
            self.stopped = true;
            self.paused = true;
        }
    }
}

/// A root search in progress.
struct RootTask {
    position: Chess,
    /// All legal moves, in search order.
    order: Vec<ScoredMove>,
    /// The result of the last finished iteration.
    best: Vec<ScoredMove>,
    /// The result so far of the running iteration.
    partial: Vec<ScoredMove>,
    next_move: usize,
    depth: u32,
    max_depth: u32,
    top_n: usize,
    time_ms: Option<u64>,
    start_ms: u64,
}

impl RootTask {
    /// Puts the best moves first and moves on to the next depth.
    fn finish_iteration(&mut self) {
        self.best = std::mem::take(&mut self.partial);
        let rest: Vec<ScoredMove> = self
            .order
            .iter()
            .filter(|m| !self.best.iter().any(|b| b.mv == m.mv))
            .copied()
            .collect();
        self.order = self.best.iter().copied().chain(rest).collect();
        self.next_move = 0;
        self.depth += 1;
    }
}

impl Engine for Searcher {
    fn best_move(&mut self, position: &SearchPosition, limits: &Limits) -> Option<Move> {
        let limits = limits.with_top_n(1);
        self.run(position, &limits).first().map(|scored| scored.mv)
    }

    fn analyze(&mut self, position: &SearchPosition, limits: &Limits) -> Vec<ScoredMove> {
        self.run(position, limits)
    }
}

/// Mate scores in the table count from the stored node, not from the root.
fn score_to_tt(score: i32, ply: usize) -> i32 {
    if score >= MATE_BOUND {
        score + ply as i32
    } else if score <= -MATE_BOUND {
        score - ply as i32
    } else {
        score
    }
}

fn score_from_tt(score: i32, ply: usize) -> i32 {
    if score >= MATE_BOUND {
        score - ply as i32
    } else if score <= -MATE_BOUND {
        score + ply as i32
    } else {
        score
    }
}

/// Yields moves best-first by selection, so a cutoff skips sorting the rest.
struct OrderedMoves {
    moves: MoveList,
    scores: [i32; 256],
    next: usize,
}

impl OrderedMoves {
    fn new(moves: MoveList, order: impl Fn(Move) -> i32) -> Self {
        let mut scores = [0; 256];
        for (score, &mv) in scores.iter_mut().zip(moves.iter()) {
            *score = order(mv);
        }
        OrderedMoves {
            moves,
            scores,
            next: 0,
        }
    }

    fn next(&mut self) -> Option<Move> {
        let len = self.moves.len();
        if self.next == len {
            return None;
        }
        let best = (self.next..len).max_by_key(|&i| self.scores[i])?;
        self.moves.swap(self.next, best);
        self.scores.swap(self.next, best);
        self.next += 1;
        Some(self.moves[self.next - 1])
    }
}

fn is_noisy(mv: Move) -> bool {
    mv.is_capture() || mv.is_promotion()
}

/// MVV-LVA. Quiet moves get 0; only quiescence search meets them, in check.
fn capture_order(mv: Move) -> i32 {
    let victim = match mv {
        Move::EnPassant { .. } => Some(Role::Pawn),
        _ => mv.capture(),
    };
    if victim.is_none() && !mv.is_promotion() {
        return 0;
    }
    let gain = victim.map_or(0, |role| role as i32) + mv.promotion().map_or(0, |role| role as i32);
    CAPTURE_ORDER + 16 * gain - mv.role() as i32
}

fn history_index(mv: Move, turn: Color) -> usize {
    let role = mv.role() as usize - 1;
    (turn as usize * 6 + role) * 64 + usize::from(mv.to())
}

fn late_move_reduction(depth: i32, searched: usize) -> i32 {
    if depth < LMR_MIN_DEPTH || searched < LMR_MIN_MOVES {
        0
    } else if depth >= LMR_DEEP_DEPTH && searched >= LMR_LATE_MOVES {
        2
    } else {
        1
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use shaxmat_core::{Game, parse_fen};

    use super::*;
    use crate::NativeClock;

    fn searcher() -> Searcher {
        Searcher::new(Box::new(NativeClock::default()))
    }

    fn best(fen: &str, depth: u32) -> ScoredMove {
        let position = SearchPosition::from(&parse_fen(fen).unwrap());
        searcher().analyze(&position, &Limits::depth(depth).with_top_n(1))[0]
    }

    fn san(fen: &str, mv: Move) -> String {
        let mut game = Game::from_fen(fen).unwrap();
        game.play(mv).unwrap();
        game.sans().remove(0)
    }

    #[test]
    fn finds_mate_in_one() {
        let fen = "6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1";
        let found = best(fen, 2);
        assert_eq!(san(fen, found.mv), "Ra8#");
        assert_eq!(found.score, MATE - 1);
    }

    #[test]
    fn finds_mate_in_three() {
        // Philidor's smothered mate pattern: 1. Qg8+ Rxg8 2. Nf7#.
        let fen = "r5rk/6pp/7N/3Q4/8/8/8/6K1 w - - 0 1";
        let found = best(fen, 5);
        assert!(found.score > MATE_BOUND, "score {}", found.score);
    }

    #[test]
    fn takes_a_hanging_queen() {
        let fen = "4k3/8/8/3q4/4P3/8/8/4K3 w - - 0 1";
        assert_eq!(san(fen, best(fen, 3).mv), "exd5");
    }

    #[test]
    fn quiescence_sees_a_protected_pawn() {
        // Qxd5 wins a pawn and loses the queen, a line a depth-1 search must see.
        let fen = "3rk3/8/8/3p4/8/8/8/3QK3 w - - 0 1";
        assert_ne!(san(fen, best(fen, 1).mv), "Qxd5");
    }

    #[test]
    fn reports_no_move_when_the_game_is_over() {
        let mated = SearchPosition::from(&parse_fen("R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1").unwrap());
        let mut engine = searcher();
        assert_eq!(engine.best_move(&mated, &Limits::depth(3)), None);
        assert!(engine.analyze(&mated, &Limits::depth(3)).is_empty());
    }

    #[test]
    fn avoids_a_repetition_when_winning() {
        // A queen up, White must not go back to c2, where the game has already been.
        let mut game = Game::from_fen("7k/8/8/8/8/8/2Q5/K7 w - - 0 1").unwrap();
        for san in ["Qc8+", "Kh7", "Qc2+", "Kh8", "Qc8+", "Kh7"] {
            game.play_san(san).unwrap();
        }
        let scored = searcher().analyze(&(&game).into(), &Limits::depth(4).with_top_n(1))[0];
        game.play(scored.mv).unwrap();
        assert_ne!(game.sans().last().unwrap(), "Qc2+");
        assert!(scored.score > 300);
    }

    #[test]
    fn analyze_returns_sorted_exact_top_moves() {
        let fen = "r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3";
        let position = SearchPosition::from(&parse_fen(fen).unwrap());
        let all = searcher().analyze(&position, &Limits::depth(3));
        let top = searcher().analyze(&position, &Limits::depth(3).with_top_n(4));
        assert_eq!(all.len(), parse_fen(fen).unwrap().legal_moves().len());
        assert_eq!(top.len(), 4);
        assert!(top.windows(2).all(|pair| pair[0].score >= pair[1].score));
        let all_scores: Vec<i32> = all.iter().take(4).map(|m| m.score).collect();
        let top_scores: Vec<i32> = top.iter().map(|m| m.score).collect();
        assert_eq!(all_scores, top_scores);
    }

    #[test]
    fn respects_the_time_limit() {
        let position = SearchPosition::from(
            &parse_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
                .unwrap(),
        );
        let clock = NativeClock::default();
        let start = clock.now_ms();
        let mut engine = searcher();
        assert!(engine.best_move(&position, &Limits::time_ms(100)).is_some());
        assert!(clock.now_ms() - start < 300);
        assert!(engine.completed_depth() >= 1);
    }

    #[test]
    fn sliced_search_reaches_the_same_depth() {
        let fen = "r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3";
        let position = SearchPosition::from(&parse_fen(fen).unwrap());
        let mut engine = searcher();
        engine.start(&position, &Limits::depth(7).with_top_n(1));
        let mut slices = 0;
        let result = loop {
            slices += 1;
            if let Some(result) = engine.step(Some(1)) {
                break result;
            }
        };
        assert!(slices > 1);
        assert_eq!(engine.completed_depth(), 7);
        assert!(parse_fen(fen).unwrap().is_legal(result[0].mv));
    }

    #[test]
    fn sliced_search_keeps_the_time_limit() {
        let position = SearchPosition::from(&Chess::default());
        let clock = NativeClock::default();
        let mut engine = searcher();
        engine.start(&position, &Limits::time_ms(50));
        while engine.step(Some(1)).is_none() {}
        assert!(clock.now_ms() < 250);
        assert!(engine.completed_depth() >= 1);
    }

    #[test]
    fn respects_the_node_limit_but_always_finds_a_move() {
        let position = SearchPosition::from(&Chess::default());
        let mut engine = searcher();
        let limits = Limits {
            nodes: Some(1),
            ..Limits::default()
        };
        assert!(engine.best_move(&position, &limits).is_some());
        assert_eq!(engine.completed_depth(), 1);
    }
}
