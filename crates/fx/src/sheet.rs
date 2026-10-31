use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct FrameRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// An animation in a sprite sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub frames: Vec<FrameRect>,
    pub fps: f32,
    pub looping: bool,
    /// Where the effect's origin is inside a frame.
    pub pivot: (f32, f32),
    /// When the piece under the effect must not be drawn, in seconds: the sheet
    /// shows that piece itself then.
    pub hidden: Option<(f32, f32)>,
}

#[derive(Deserialize)]
struct RawSheet {
    frames: Vec<RawFrame>,
    meta: RawMeta,
}

#[derive(Deserialize)]
struct RawFrame {
    frame: FrameRect,
}

#[derive(Deserialize)]
struct RawMeta {
    pixelfx: RawPixelfx,
    shaxmat: RawShaxmat,
}

#[derive(Deserialize)]
struct RawPixelfx {
    fps: f32,
    #[serde(rename = "loop")]
    looping: bool,
    pivot: (f32, f32),
}

#[derive(Deserialize)]
struct RawShaxmat {
    hidden: Option<(f32, f32)>,
}

impl Sheet {
    /// Parses the Aseprite-style JSON from `tools/fx/bake.mjs`.
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        let raw: RawSheet = serde_json::from_str(json)?;
        Ok(Sheet {
            frames: raw.frames.into_iter().map(|f| f.frame).collect(),
            fps: raw.meta.pixelfx.fps,
            looping: raw.meta.pixelfx.looping,
            pivot: raw.meta.pixelfx.pivot,
            hidden: raw.meta.shaxmat.hidden,
        })
    }

    pub fn duration(&self) -> f32 {
        self.frames.len() as f32 / self.fps
    }

    /// The frame to show `age` seconds in. Looping sheets wrap; others hold the last.
    pub fn frame_at(&self, age: f32) -> usize {
        let index = (age * self.fps) as usize;
        if self.looping {
            index % self.frames.len()
        } else {
            index.min(self.frames.len() - 1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub const JSON: &str = r#"{
        "frames": [
            {"filename": "a 0", "frame": {"x": 0, "y": 0, "w": 8, "h": 6}, "duration": 100},
            {"filename": "a 1", "frame": {"x": 8, "y": 0, "w": 8, "h": 6}, "duration": 100}
        ],
        "meta": {
            "pixelfx": {"fps": 10, "loop": false, "frames": 2, "frameSize": [8, 6],
                        "pivot": [4, 5], "columns": 2},
            "shaxmat": {"hidden": [0.1, 0.2]}
        }
    }"#;

    #[test]
    fn parses_a_sheet() {
        let sheet = Sheet::parse(JSON).unwrap();
        assert_eq!(sheet.frames.len(), 2);
        assert_eq!(sheet.frames[1].x, 8);
        assert_eq!(sheet.pivot, (4.0, 5.0));
        assert_eq!(sheet.hidden, Some((0.1, 0.2)));
        assert!((sheet.duration() - 0.2).abs() < 1e-6);
    }

    #[test]
    fn frames_hold_or_wrap() {
        let mut sheet = Sheet::parse(JSON).unwrap();
        assert_eq!(sheet.frame_at(0.05), 0);
        assert_eq!(sheet.frame_at(0.15), 1);
        assert_eq!(sheet.frame_at(5.0), 1);
        sheet.looping = true;
        assert_eq!(sheet.frame_at(0.25), 0);
    }
}
