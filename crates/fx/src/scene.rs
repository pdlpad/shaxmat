use crate::Sheet;

/// Why an instance plays, so that groups can be removed together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tag {
    /// Runs its course once.
    Event,
    /// Shows the current selection, for as long as it lasts.
    Selection,
    /// Shows a hint, until it is taken back.
    Hint,
}

/// One effect playing.
#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    pub sheet: usize,
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    /// Clockwise, in radians, around the pivot.
    pub rotation: f32,
    /// Shows only this much of the frame past the pivot, for beams.
    pub reach: Option<f32>,
    pub tag: Tag,
    /// The board square the effect sits on, if any.
    pub square: Option<u8>,
    /// Seconds left before it starts.
    pub delay: f32,
    /// Keep the piece hidden while waiting to start.
    pub hide_before_start: bool,
    pub age: f32,
}

impl Instance {
    pub fn new(sheet: usize, x: f32, y: f32, tag: Tag) -> Self {
        Instance {
            sheet,
            x,
            y,
            scale: 1.0,
            rotation: 0.0,
            reach: None,
            tag,
            square: None,
            delay: 0.0,
            hide_before_start: false,
            age: 0.0,
        }
    }
}

/// The sheets and whatever is playing.
pub struct Scene {
    names: Vec<String>,
    sheets: Vec<Sheet>,
    instances: Vec<Instance>,
    block_left: f32,
}

impl Scene {
    pub fn new(sheets: Vec<(String, Sheet)>) -> Self {
        let (names, sheets) = sheets.into_iter().unzip();
        Scene {
            names,
            sheets,
            instances: Vec::new(),
            block_left: 0.0,
        }
    }

    /// The index of a sheet.
    ///
    /// # Panics
    /// If no sheet has that name: the names are fixed at bake time.
    pub fn sheet_index(&self, name: &str) -> usize {
        self.names
            .iter()
            .position(|known| known == name)
            .unwrap_or_else(|| panic!("no sheet named {name}"))
    }

    pub fn sheet(&self, index: usize) -> &Sheet {
        &self.sheets[index]
    }

    pub fn spawn(&mut self, instance: Instance) {
        self.instances.push(instance);
    }

    pub fn remove(&mut self, tag: Tag) {
        self.instances.retain(|instance| instance.tag != tag);
    }

    /// Stops clicks for at least `seconds`.
    pub fn block_input(&mut self, seconds: f32) {
        self.block_left = self.block_left.max(seconds);
    }

    pub fn is_blocking_input(&self) -> bool {
        self.block_left > 0.0
    }

    pub fn update(&mut self, dt: f32) {
        self.block_left = (self.block_left - dt).max(0.0);
        let sheets = &self.sheets;
        self.instances.retain_mut(|instance| {
            if instance.delay > 0.0 {
                instance.delay -= dt;
                return true;
            }
            instance.age += dt;
            let sheet = &sheets[instance.sheet];
            sheet.looping || instance.age < sheet.duration()
        });
    }

    /// Instances that have started, oldest first.
    pub fn playing(&self) -> impl Iterator<Item = &Instance> {
        self.instances.iter().filter(|i| i.delay <= 0.0)
    }

    /// The running instance of a sheet, if any.
    pub fn find(&self, sheet: usize) -> Option<&Instance> {
        self.playing().find(|instance| instance.sheet == sheet)
    }

    /// Whether the piece on a square is being shown by an effect, or is yet to appear.
    pub fn hides(&self, square: u8) -> bool {
        self.instances
            .iter()
            .filter(|i| i.square == Some(square))
            .any(|instance| {
                if instance.delay > 0.0 {
                    return instance.hide_before_start;
                }
                self.sheets[instance.sheet]
                    .hidden
                    .is_some_and(|(from, to)| (from..to).contains(&instance.age))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FrameRect;

    fn sheet(looping: bool, hidden: Option<(f32, f32)>) -> Sheet {
        let frame = FrameRect {
            x: 0,
            y: 0,
            w: 4,
            h: 4,
        };
        Sheet {
            frames: vec![frame; 10],
            fps: 10.0,
            looping,
            pivot: (2.0, 2.0),
            hidden,
        }
    }

    fn scene() -> Scene {
        Scene::new(vec![
            ("once".to_owned(), sheet(false, Some((0.2, 0.5)))),
            ("loop".to_owned(), sheet(true, None)),
        ])
    }

    #[test]
    fn one_shots_end_and_loops_stay_until_removed() {
        let mut scene = scene();
        scene.spawn(Instance::new(0, 0.0, 0.0, Tag::Event));
        scene.spawn(Instance::new(1, 0.0, 0.0, Tag::Selection));
        scene.update(0.5);
        assert_eq!(scene.playing().count(), 2);
        scene.update(0.6);
        assert_eq!(scene.playing().count(), 1);
        scene.update(10.0);
        assert_eq!(scene.playing().count(), 1);
        scene.remove(Tag::Selection);
        assert_eq!(scene.playing().count(), 0);
    }

    #[test]
    fn delayed_instances_wait() {
        let mut scene = scene();
        let mut late = Instance::new(0, 0.0, 0.0, Tag::Event);
        late.delay = 0.3;
        scene.spawn(late);
        assert_eq!(scene.playing().count(), 0);
        scene.update(0.4);
        assert_eq!(scene.playing().count(), 1);
    }

    #[test]
    fn hides_a_piece_only_in_the_sheets_window() {
        let mut scene = scene();
        let mut instance = Instance::new(0, 0.0, 0.0, Tag::Event);
        instance.square = Some(12);
        scene.spawn(instance);
        assert!(!scene.hides(12));
        scene.update(0.3);
        assert!(scene.hides(12));
        assert!(!scene.hides(13));
        scene.update(0.3);
        assert!(!scene.hides(12));
    }

    #[test]
    fn hides_a_piece_before_a_delayed_start() {
        let mut scene = scene();
        let mut instance = Instance::new(1, 0.0, 0.0, Tag::Event);
        instance.square = Some(5);
        instance.delay = 0.2;
        instance.hide_before_start = true;
        scene.spawn(instance);
        assert!(scene.hides(5));
        scene.update(0.3);
        assert!(!scene.hides(5));
    }

    #[test]
    fn the_input_block_runs_out() {
        let mut scene = scene();
        assert!(!scene.is_blocking_input());
        scene.block_input(0.3);
        scene.block_input(0.1);
        assert!(scene.is_blocking_input());
        scene.update(0.2);
        assert!(scene.is_blocking_input());
        scene.update(0.2);
        assert!(!scene.is_blocking_input());
    }

    #[test]
    #[should_panic(expected = "no sheet named")]
    fn unknown_sheets_are_a_bug() {
        scene().sheet_index("nothing");
    }
}
