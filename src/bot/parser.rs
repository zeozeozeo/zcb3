use anyhow::{Context, Error, Result};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum ClickType {
    /// Hard mouse click.
    Click,
    /// Hard mouse release.
    Release,
    /// Softclick (happens after at least 2 consecutive clicks).
    SoftClick,
    /// Soft release (happens after at least 2 consecutive releases).
    SoftRelease,
    /// No action.
    #[default]
    None,
}

impl ClickType {
    #[inline]
    pub fn is_soft(self) -> bool {
        self == ClickType::SoftClick || self == ClickType::SoftRelease
    }

    #[inline]
    pub fn is_click(self) -> bool {
        self == ClickType::Click || self == ClickType::SoftClick
    }

    #[inline]
    pub fn from_time_and_threshold(
        time: f32,
        last_time: f32,
        threshold: f32,
        down: bool,
        prev: Self,
    ) -> Self {
        // if mouse action was the same in current and previous frame, then no action was made
        if down == prev.is_click() {
            return ClickType::None;
        }
        match down {
            true => {
                // if time between current and previous action < threshold, click is considered soft
                if time - last_time < threshold {
                    ClickType::SoftClick
                } else {
                    ClickType::Click
                }
            }
            false => {
                // previous click has to be soft for the release to be considered soft
                // TODO: maybe there's a formula to make this sound more realistic?
                if prev.is_soft() {
                    ClickType::SoftRelease
                } else {
                    ClickType::Release
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Action {
    /// Time since the macro was started (in seconds).
    pub time: f32,
    /// Whether player 1 and player 2 mouse sound should be played.
    pub click: (ClickType, ClickType),
}

impl Action {
    pub fn new(time: f32, click: (ClickType, ClickType)) -> Self {
        Self { time, click }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Macro {
    pub fps: f32,
    /// Duration of the macro (in seconds).
    pub duration: f32,
    pub actions: Vec<Action>,
}

#[derive(Clone, Copy, Debug)]
pub enum MacroType {
    MegaHack,
    TasBot,
}

impl MacroType {
    pub fn guess_format(data: &str, filename: &str) -> Result<Self> {
        log::info!("guessing macro format, filename '{filename}'");
        if filename.ends_with(".json") {
            let v: Value = serde_json::from_str(data)?;

            if v.get("meta").is_some() && v.get("events").is_some() {
                return Ok(Self::MegaHack); // probably mega hack replay
            }
            if v.get("macro").is_some() && v.get("fps").is_some() {
                return Ok(Self::TasBot); // probably tasbot
            }
        }
        Err(Error::msg("failed to identify replay format"))
    }
}

impl Macro {
    pub fn parse(typ: MacroType, data: &str, soft_threshold: f32) -> Result<Self> {
        log::info!("parsing replay, strlen {}, replay type {typ:?}", data.len());
        let mut replay = Self::default();

        match typ {
            MacroType::MegaHack => replay.parse_mhr(data, soft_threshold)?,
            MacroType::TasBot => replay.parse_tasbot(data, soft_threshold)?,
        }

        if !replay.actions.is_empty() {
            replay.duration = replay.actions.last().unwrap().time;
        }

        Ok(replay)
    }

    fn parse_tasbot(&mut self, data: &str, threshold: f32) -> Result<()> {
        let v: Value = serde_json::from_str(data)?;
        self.fps = v["fps"].as_f64().context("couldn't get 'fps' field")? as f32;
        let events = v["macro"]
            .as_array()
            .context("couldn't get 'macro' array")?;

        let mut click = (ClickType::None, ClickType::None); // store mouse state of previous frame
        let get_click_type = |time: f32, prev_time: f32, down: bool, prev: ClickType| {
            ClickType::from_time_and_threshold(time, prev_time, threshold, down, prev)
        };
        let mut prev_time = (0.0f32, 0.0f32);

        for ev in events {
            let frame = ev["frame"].as_u64().context("couldn't get 'frame' field")?;
            let time = frame as f32 / self.fps;

            let player1 = ev["player_1"]["click"]
                .as_i64()
                .context("couldn't get 'click' field")?
                != 0;
            let player2 = ev["player_2"]["click"]
                .as_i64()
                .context("couldn't get 'click' field")?
                != 0;

            click.0 = get_click_type(time, prev_time.0, player1, click.0);
            prev_time.0 = time;

            click.1 = get_click_type(time, prev_time.1, player2, click.1);
            prev_time.1 = time;

            if click.0 != ClickType::None || click.1 != ClickType::None {
                self.actions.push(Action::new(time, click));
            }
        }

        Ok(())
    }

    fn parse_mhr(&mut self, data: &str, threshold: f32) -> Result<()> {
        let v: Value = serde_json::from_str(data)?;
        self.fps = v["meta"]["fps"]
            .as_f64()
            .context("couldn't get 'fps' field")? as f32;

        let events = v["events"]
            .as_array()
            .context("couldn't get 'events' array")?;

        let mut click = (ClickType::None, ClickType::None); // store mouse state of previous frame
        let mut next_p2 = false; // whether the next action refers to the player 2 mouse state
        let mut prev_time = (0.0f32, 0.0f32); // last time when an action was performed (for both players)
        let get_click_type = |time: f32, prev_time: f32, down: bool, prev: ClickType| {
            ClickType::from_time_and_threshold(time, prev_time, threshold, down, prev)
        };

        for ev in events {
            let time =
                ev["frame"].as_u64().context("couldn't get 'frame' field")? as f32 / self.fps;

            if let Some(d) = ev.get("down") {
                let d = d.as_bool().context("couldn't get 'down' field")?;
                if next_p2 {
                    // p2 action
                    click.1 = get_click_type(time, prev_time.1, d, click.1);
                    next_p2 = false; // next action is either another "p2" or it refers to player 1
                    prev_time.1 = time;
                } else {
                    // p1 action
                    click.0 = get_click_type(time, prev_time.0, d, click.0);
                    prev_time.0 = time;
                }
            }

            // "p2" always seems to be true, but for safety we'll query the value anyway
            if let Some(p2) = ev.get("p2") {
                next_p2 = p2.as_bool().context("couldn't get 'p2' field")?;
            }

            self.actions.push(Action::new(time, click));
        }

        Ok(())
    }
}
