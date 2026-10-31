//! What plays on which game event. Pieces are named as in the sheets: `wQ`, `bK`.

/// The longest any effect but mate may hold up clicks.
pub const MAX_BLOCK_SECONDS: f32 = 0.4;
/// Mate ends the game, so its effect may take its time.
pub const MATE_BLOCK_SECONDS: f32 = 2.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Capture { square: u8, piece: String },
    Castle { from: u8, to: u8, king: String },
    Check { king_square: u8 },
    Mate { king_square: u8, king: String },
    Promotion { square: u8, piece: String },
    Blunder { square: u8 },
    Brilliant { square: u8 },
    Transition,
}

/// Where a sheet plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Square(u8),
    /// Centered over the board, above it.
    Banner,
    /// Filling the screen.
    Screen,
}

/// One sheet to start.
#[derive(Debug, Clone, PartialEq)]
pub struct Cue {
    pub sheet: String,
    pub place: Place,
    pub delay: f32,
    pub hide_before_start: bool,
}

fn cue(sheet: impl Into<String>, place: Place, delay: f32) -> Cue {
    Cue {
        sheet: sheet.into(),
        place,
        delay,
        hide_before_start: false,
    }
}

/// The sheets an event starts.
pub fn cues(event: &Event) -> Vec<Cue> {
    match event {
        Event::Capture { square, piece } => {
            let place = Place::Square(*square);
            vec![
                cue(format!("hit-flash-{piece}"), place, 0.0),
                cue("poof", place, 0.15),
            ]
        }
        Event::Castle { from, to, king } => vec![
            cue(format!("blink-out-{king}"), Place::Square(*from), 0.0),
            Cue {
                hide_before_start: true,
                ..cue(format!("blink-in-{king}"), Place::Square(*to), 0.35)
            },
        ],
        Event::Check { king_square } => vec![cue("lightning", Place::Square(*king_square), 0.0)],
        Event::Mate { king_square, king } => {
            let place = Place::Square(*king_square);
            vec![
                cue(format!("disintegrate-{king}"), place, 0.0),
                cue("explosion", place, 0.4),
            ]
        }
        Event::Promotion { square, piece } => {
            let place = Place::Square(*square);
            vec![
                cue(format!("respawn-{piece}"), place, 0.0),
                cue("blessing", place, 0.75),
            ]
        }
        Event::Blunder { square } => vec![cue("pow", Place::Square(*square), 0.0)],
        Event::Brilliant { square } => vec![
            cue("confetti", Place::Square(*square), 0.0),
            cue("achievement", Place::Banner, 0.0),
        ],
        Event::Transition => vec![cue("iris", Place::Screen, 0.0)],
    }
}

/// How long an event holds up clicks.
pub fn block_seconds(event: &Event) -> f32 {
    match event {
        Event::Mate { .. } => MATE_BLOCK_SECONDS,
        Event::Capture { .. } | Event::Brilliant { .. } => 0.2,
        Event::Check { .. } => 0.3,
        Event::Castle { .. }
        | Event::Promotion { .. }
        | Event::Blunder { .. }
        | Event::Transition => MAX_BLOCK_SECONDS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_events() -> Vec<Event> {
        let piece = "bQ".to_owned();
        vec![
            Event::Capture {
                square: 1,
                piece: piece.clone(),
            },
            Event::Castle {
                from: 4,
                to: 6,
                king: "wK".to_owned(),
            },
            Event::Check { king_square: 4 },
            Event::Mate {
                king_square: 4,
                king: "bK".to_owned(),
            },
            Event::Promotion { square: 60, piece },
            Event::Blunder { square: 2 },
            Event::Brilliant { square: 2 },
            Event::Transition,
        ]
    }

    #[test]
    fn every_event_plays_something() {
        for event in all_events() {
            assert!(!cues(&event).is_empty(), "{event:?}");
        }
    }

    #[test]
    fn only_mate_may_block_input_past_the_limit() {
        for event in all_events() {
            let seconds = block_seconds(&event);
            if matches!(event, Event::Mate { .. }) {
                assert!(seconds > MAX_BLOCK_SECONDS);
            } else {
                assert!(seconds <= MAX_BLOCK_SECONDS, "{event:?}");
            }
        }
    }

    #[test]
    fn the_castled_king_stays_hidden_until_it_blinks_in() {
        let event = Event::Castle {
            from: 4,
            to: 6,
            king: "wK".to_owned(),
        };
        let cues = cues(&event);
        assert_eq!(cues[1].sheet, "blink-in-wK");
        assert_eq!(cues[1].place, Place::Square(6));
        assert!(cues[1].hide_before_start);
    }
}
