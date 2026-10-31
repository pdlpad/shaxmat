//! Effects for SHAXMAT: reads the sprite sheets baked from pixelfx, keeps time for
//! the ones playing, and decides what plays on which game event. It knows nothing
//! about graphics or input: squares are plain numbers and drawing is up to the host.

mod events;
mod scene;
mod sheet;

pub use events::{Cue, Event, MATE_BLOCK_SECONDS, MAX_BLOCK_SECONDS, Place, block_seconds, cues};
pub use scene::{Instance, Scene, Tag};
pub use sheet::{FrameRect, Sheet};
