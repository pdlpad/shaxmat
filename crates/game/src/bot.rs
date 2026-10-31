//! Lets the bot think without stopping the frame loop: in a thread natively, in
//! time slices between frames on WASM, which has no threads.

use shaxmat_core::{Game, Move};
use shaxmat_engine::{Clock, Engine, LevelEngine, Limits, MATE_BOUND, SearchPosition};

/// The bot accepts a draw unless its own score is better than this.
const DRAW_ACCEPT_SCORE: i32 = 0;
const DRAW_OFFER_DEPTH: u32 = 4;

/// Thinking time per frame on WASM.
#[cfg(target_arch = "wasm32")]
const SLICE_MS: u64 = 8;

pub struct Bot {
    /// Away only while a thread holds it.
    engine: Option<LevelEngine>,
    #[cfg(not(target_arch = "wasm32"))]
    reply: Option<std::sync::mpsc::Receiver<(LevelEngine, Option<Move>)>>,
    thinking: bool,
}

impl Bot {
    pub fn new(level: usize, seed: u64) -> Self {
        Bot {
            engine: Some(LevelEngine::new(level, seed, new_clock())),
            #[cfg(not(target_arch = "wasm32"))]
            reply: None,
            thinking: false,
        }
    }

    pub fn is_thinking(&self) -> bool {
        self.thinking
    }

    /// Asks for a move. The answer comes from [`Bot::poll`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn think(&mut self, game: &Game) {
        let position = SearchPosition::from(game);
        let mut engine = self.engine.take().expect("the bot is idle");
        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mv = engine.best_move(&position, &Limits::default());
            // A dropped receiver only means the game was abandoned.
            let _ = send.send((engine, mv));
        });
        self.reply = Some(receive);
        self.thinking = true;
    }

    /// The bot's move once ready.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn poll(&mut self) -> Option<Move> {
        let (engine, mv) = self.reply.as_ref()?.try_recv().ok()?;
        self.engine = Some(engine);
        self.reply = None;
        self.thinking = false;
        mv
    }

    /// Asks for a move. The answer comes from [`Bot::poll`].
    #[cfg(target_arch = "wasm32")]
    pub fn think(&mut self, game: &Game) {
        let engine = self.engine.as_mut().expect("the bot is idle");
        engine.start(&SearchPosition::from(game), &Limits::default());
        self.thinking = true;
    }

    /// Thinks for one slice, and gives the bot's move once ready.
    #[cfg(target_arch = "wasm32")]
    pub fn poll(&mut self) -> Option<Move> {
        if !self.thinking {
            return None;
        }
        let engine = self.engine.as_mut().expect("the bot is on this thread");
        let mv = engine.step(Some(SLICE_MS))?;
        self.thinking = false;
        mv
    }

    /// Whether the bot takes a draw now. Must be called while it is idle.
    pub fn accepts_draw(&mut self, game: &Game) -> bool {
        let engine = self.engine.as_mut().expect("the bot is idle");
        let limits = Limits::depth(DRAW_OFFER_DEPTH).with_top_n(1);
        let scored = engine.analyze(&SearchPosition::from(game), &limits);
        // The offer comes on the player's turn, so the score is the player's.
        let bot_score = -scored.first().map_or(0, |m| m.score);
        bot_score <= DRAW_ACCEPT_SCORE && bot_score > -MATE_BOUND
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn new_clock() -> Box<dyn Clock> {
    Box::new(shaxmat_engine::NativeClock::default())
}

#[cfg(target_arch = "wasm32")]
fn new_clock() -> Box<dyn Clock> {
    struct FrameClock;

    impl Clock for FrameClock {
        fn now_ms(&self) -> u64 {
            (macroquad::time::get_time() * 1000.0) as u64
        }
    }

    Box::new(FrameClock)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use shaxmat_core::Position;

    use super::*;

    fn wait_for_move(bot: &mut Bot) -> Move {
        loop {
            if let Some(mv) = bot.poll() {
                return mv;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    #[test]
    fn answers_with_a_legal_move_and_can_be_reused() {
        let mut game = Game::new();
        let mut bot = Bot::new(2, 1);
        for _ in 0..4 {
            assert!(!bot.is_thinking());
            bot.think(&game);
            assert!(bot.is_thinking());
            let mv = wait_for_move(&mut bot);
            assert!(game.position().is_legal(mv));
            game.play(mv).unwrap();
        }
    }

    #[test]
    fn dropping_a_thinking_bot_is_harmless() {
        let mut bot = Bot::new(8, 1);
        bot.think(&Game::new());
    }

    #[test]
    fn draw_depends_on_who_stands_better() {
        let mut bot = Bot::new(1, 1);
        // The player to move has a queen against nothing: the bot is lost, and glad.
        let winning = Game::from_fen("7k/8/8/8/8/8/2Q5/K7 w - - 0 1").unwrap();
        assert!(bot.accepts_draw(&winning));
        // The player has a bare king: the bot keeps playing.
        let losing = Game::from_fen("7k/8/8/8/8/8/3q4/K7 w - - 0 1").unwrap();
        assert!(!bot.accepts_draw(&losing));
    }
}
