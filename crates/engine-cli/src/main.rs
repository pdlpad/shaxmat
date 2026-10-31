//! Self-play matches and strength calibration.
//!
//!   engine-cli match --a 5 --b 4 [--games 40] [--threads 15] [--seed 1]
//!   engine-cli ladder [--games 40] [--threads 15] [--seed 1] [--from 2] [--to 8]
//!   engine-cli calibrate [--games 40] [--threads 15] [--movetime 200] [--from 1] [--to 8]
//!   engine-cli bench [--depth 8]
//!   engine-cli uci --level 4
//!
//! `calibrate` reads the Stockfish executable from `STOCKFISH_PATH`.

use std::collections::HashMap;
use std::process::ExitCode;

use shaxmat_core::parse_fen;
use shaxmat_engine::{Engine, LEVEL_COUNT, Limits, NativeClock, Searcher};
use shaxmat_engine_cli::contest::{Player, elo_difference, run_match};
use shaxmat_engine_cli::{calibrate, uci};

const BENCH_POSITIONS: [&str; 4] = [
    "r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3",
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((command, rest)) = args.split_first() else {
        eprintln!("usage: engine-cli <match|ladder|calibrate|bench|uci> [--option value]...");
        return ExitCode::FAILURE;
    };
    let options = match parse_options(rest) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };
    let default_threads = std::thread::available_parallelism().map_or(1, |n| n.get().max(2) - 1);
    let games = options.get("games", 40);
    let threads = options.get("threads", default_threads);
    let seed = options.get("seed", 1) as u64;

    match command.as_str() {
        "match" => {
            let (a, b) = (options.get("a", 2), options.get("b", 1));
            print_header();
            print_match(a, b, games, threads, seed);
        }
        "ladder" => {
            print_header();
            for level in options.get("from", 2)..=options.get("to", LEVEL_COUNT) {
                print_match(level, level - 1, games, threads, seed);
            }
        }
        "calibrate" => {
            let movetime = options.get("movetime", 200) as u64;
            let levels = options.get("from", 1)..=options.get("to", LEVEL_COUNT);
            return calibrate::run(levels, games, threads, movetime, seed);
        }
        "bench" => bench(options.get("depth", 8) as u32),
        "uci" => uci::serve(options.get("level", 4)),
        other => {
            eprintln!("unknown command: {other}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

struct Options(HashMap<String, usize>);

impl Options {
    fn get(&self, name: &str, default: usize) -> usize {
        self.0.get(name).copied().unwrap_or(default)
    }
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut options = HashMap::new();
    for pair in args.chunks(2) {
        let [name, value] = pair else {
            return Err(format!("option {} has no value", pair[0]));
        };
        let name = name
            .strip_prefix("--")
            .ok_or_else(|| format!("expected an option, got {name}"))?;
        let value = value
            .parse()
            .map_err(|_| format!("--{name} needs a number, got {value}"))?;
        options.insert(name.to_owned(), value);
    }
    Ok(Options(options))
}

fn print_header() {
    println!("| Match | Wins | Draws | Losses | Score | Elo diff | Max ms/move | Avg plies |");
    println!("|---|---|---|---|---|---|---|---|");
}

fn print_match(a: usize, b: usize, games: usize, threads: usize, seed: u64) {
    let result = run_match(Player::Level(a), Player::Level(b), games, threads, seed);
    println!(
        "| L{a} vs L{b} | {} | {} | {} | {:.1}% | {:+.0} | {} | {:.0} |",
        result.wins,
        result.draws,
        result.losses,
        result.score() * 100.0,
        elo_difference(result.score(), games),
        result.max_move_ms,
        result.average_plies(),
    );
}

/// Prints speed and reached depth, without weakening.
fn bench(depth: u32) {
    println!("| Position | Depth | Nodes | ms | knps |");
    println!("|---|---|---|---|---|");
    let (mut total_nodes, mut total_ms) = (0, 0);
    for (i, fen) in BENCH_POSITIONS.iter().enumerate() {
        let position = parse_fen(fen).expect("bench positions are valid");
        let mut engine = Searcher::new(Box::new(NativeClock::default()));
        let start = std::time::Instant::now();
        engine.best_move(&(&position).into(), &Limits::depth(depth));
        let ms = start.elapsed().as_millis() as u64;
        total_nodes += engine.nodes();
        total_ms += ms;
        println!(
            "| {} | {} | {} | {ms} | {} |",
            i + 1,
            engine.completed_depth(),
            engine.nodes(),
            engine.nodes() / ms.max(1),
        );
    }
    println!("Total: {total_nodes} nodes in {total_ms} ms");
}
