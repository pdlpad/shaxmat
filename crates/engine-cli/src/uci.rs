//! UCI, both ways: talking to an external engine, and serving a level so that
//! the client can be tested end to end without Stockfish.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use shaxmat_core::shakmaty::CastlingMode;
use shaxmat_core::shakmaty::uci::UciMove;
use shaxmat_core::{Game, Move};
use shaxmat_engine::{Engine, LevelEngine, Limits, NativeClock, SearchPosition};

pub const STOCKFISH_ENV: &str = "STOCKFISH_PATH";

/// An engine process spoken to over UCI.
pub struct UciEngine {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    movetime_ms: u64,
}

impl UciEngine {
    /// Starts Stockfish at `STOCKFISH_PATH`, limited to `elo`.
    ///
    /// # Panics
    /// If the variable is unset or the process cannot be talked to.
    pub fn stockfish(elo: u32, movetime_ms: u64) -> Self {
        let path = std::env::var(STOCKFISH_ENV).expect("calibrate checks the variable first");
        let mut engine = Self::spawn(Command::new(path), movetime_ms);
        engine.send("setoption name UCI_LimitStrength value true");
        engine.send(&format!("setoption name UCI_Elo value {elo}"));
        engine.send("setoption name Hash value 16");
        engine.send("setoption name Threads value 1");
        engine.send("ucinewgame");
        engine.wait_ready();
        engine
    }

    pub fn spawn(mut command: Command, movetime_ms: u64) -> Self {
        let mut process = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the UCI engine starts");
        let stdin = process.stdin.take().expect("stdin is piped");
        let stdout = BufReader::new(process.stdout.take().expect("stdout is piped"));
        let mut engine = UciEngine {
            process,
            stdin,
            stdout,
            movetime_ms,
        };
        engine.send("uci");
        engine.read_until("uciok");
        engine
    }

    /// Asks for a move in the current game position.
    pub fn best_move(&mut self, game: &Game) -> Option<Move> {
        let moves: Vec<String> = game
            .moves()
            .iter()
            .map(|mv| mv.to_uci(CastlingMode::Standard).to_string())
            .collect();
        self.send(&format!("position startpos moves {}", moves.join(" ")));
        self.send(&format!("go movetime {}", self.movetime_ms));
        let line = self.read_until("bestmove");
        let text = line.split_whitespace().nth(1)?;
        let parsed: UciMove = text.parse().ok()?;
        parsed.to_move(game.position()).ok()
    }

    fn wait_ready(&mut self) {
        self.send("isready");
        self.read_until("readyok");
    }

    fn send(&mut self, line: &str) {
        writeln!(self.stdin, "{line}").expect("the UCI engine accepts input");
        self.stdin.flush().expect("the UCI engine accepts input");
    }

    fn read_until(&mut self, prefix: &str) -> String {
        let mut line = String::new();
        loop {
            line.clear();
            let read = self
                .stdout
                .read_line(&mut line)
                .expect("the UCI engine answers");
            assert!(read > 0, "the UCI engine quit while waiting for {prefix}");
            if line.starts_with(prefix) {
                return line.trim().to_owned();
            }
        }
    }
}

impl Drop for UciEngine {
    fn drop(&mut self) {
        let _ = writeln!(self.stdin, "quit");
        let _ = self.stdin.flush();
        let _ = self.process.wait();
    }
}

/// Answers UCI on stdin and stdout as a fixed level.
pub fn serve(level: usize) {
    let mut engine = LevelEngine::new(level, 1, Box::new(NativeClock::default()));
    let mut game = Game::new();
    let mut out = std::io::stdout();
    for line in std::io::stdin().lock().lines() {
        let line = line.expect("stdin is readable");
        let mut words = line.split_whitespace();
        let reply = match words.next() {
            Some("uci") => "id name SHAXMAT\nuciok".to_owned(),
            Some("isready") => "readyok".to_owned(),
            Some("position") => {
                game = Game::new();
                for text in words.skip_while(|word| *word != "moves").skip(1) {
                    let mv = text
                        .parse::<UciMove>()
                        .ok()
                        .and_then(|uci| uci.to_move(game.position()).ok())
                        .expect("the client sends legal moves");
                    game.play(mv).expect("the client sends legal moves");
                }
                continue;
            }
            Some("go") => {
                let mv = engine
                    .best_move(&SearchPosition::from(&game), &Limits::default())
                    .map_or("0000".to_owned(), |mv| {
                        mv.to_uci(CastlingMode::Standard).to_string()
                    });
                format!("bestmove {mv}")
            }
            Some("quit") => return,
            _ => continue,
        };
        writeln!(out, "{reply}").expect("stdout is writable");
        out.flush().expect("stdout is writable");
    }
}
