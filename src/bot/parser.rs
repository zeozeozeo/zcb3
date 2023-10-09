use anyhow::{Context, Result};
use serde_json::Value;
use std::io::Cursor;

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
    pub fn is_release(self) -> bool {
        self == ClickType::Release || self == ClickType::SoftRelease
    }

    pub fn hard_or_soft(time: f32, prev_time: f32, threshold: f32, down: bool, prev: Self) -> Self {
        match down {
            true => {
                // if time between current and previous action < threshold, click is considered soft
                if time - prev_time < threshold {
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
    /// What player this action is for.
    pub player: Player,
    /// Click type for this player.
    pub click: ClickType,
}

impl Action {
    pub const fn new(time: f32, player: Player, click: ClickType) -> Self {
        Self {
            time,
            player,
            click,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Player {
    #[default]
    One,
    Two,
}

#[derive(Clone, Debug, Default)]
pub struct Macro {
    pub fps: f32,
    /// Duration of the macro (in seconds).
    pub duration: f32,
    pub actions: Vec<Action>,

    prev_action: (ClickType, ClickType),
    prev_time: (f32, f32),
    soft_threshold: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum MacroType {
    /// .mhr.json files
    MegaHack,
    /// .json files
    TasBot,
    /// .zbf files
    Zbot,
}

impl MacroType {
    pub fn guess_format(data: &[u8], filename: &str) -> Result<Self> {
        log::info!("guessing macro format, filename '{filename}'");
        if filename.ends_with(".json") {
            let v: Value = serde_json::from_slice(data)?;

            if v.get("meta").is_some() && v.get("events").is_some() {
                return Ok(Self::MegaHack); // probably mega hack replay
            }
            if v.get("macro").is_some() && v.get("fps").is_some() {
                return Ok(Self::TasBot); // probably tasbot
            }
        } else if filename.ends_with(".zbf") {
            log::debug!("TODO: add zbf file validation");
            return Ok(Self::Zbot); // probably zbot replay (no validation yet)
        }
        Err(anyhow::anyhow!("failed to identify replay format"))
    }
}

impl Macro {
    pub fn parse(typ: MacroType, data: &[u8], soft_threshold: f32) -> Result<Self> {
        log::info!("parsing replay, strlen {}, replay type {typ:?}", data.len());
        let mut replay = Self::default();
        replay.soft_threshold = soft_threshold;

        match typ {
            MacroType::MegaHack => replay.parse_mhr(data)?,
            MacroType::TasBot => replay.parse_tasbot(data)?,
            MacroType::Zbot => replay.parse_zbf(data)?,
        }

        if !replay.actions.is_empty() {
            replay.duration = replay.actions.last().unwrap().time;
        }

        Ok(replay)
    }

    fn process_action_p1(&mut self, time: f32, down: bool) {
        if down == self.prev_action.0.is_click() {
            return;
        }

        let typ = ClickType::hard_or_soft(
            time,
            self.prev_time.0,
            self.soft_threshold,
            down,
            self.prev_action.0,
        );

        self.prev_time.0 = time;
        self.prev_action.0 = typ;
        self.actions.push(Action::new(time, Player::One, typ))
    }

    fn process_action_p2(&mut self, time: f32, down: bool) {
        if down == self.prev_action.1.is_click() {
            return;
        }

        let typ = ClickType::hard_or_soft(
            time,
            self.prev_time.1,
            self.soft_threshold,
            down,
            self.prev_action.1,
        );

        self.prev_time.1 = time;
        self.prev_action.1 = typ;
        self.actions.push(Action::new(time, Player::Two, typ))
    }

    fn parse_zbf(&mut self, data: &[u8]) -> Result<()> {
        use byteorder::{LittleEndian, ReadBytesExt};
        let mut cursor = Cursor::new(data);

        let delta = cursor.read_f32::<LittleEndian>()?;
        let mut speedhack = cursor.read_f32::<LittleEndian>()?;
        if speedhack == 0.0 {
            log::error!("zbf speedhack is 0.0, defaulting to 1.0");
            speedhack = 1.0; // reset to 1 so we don't get Infinity as fps
        }
        self.fps = 1.0 / delta / speedhack;

        for _ in (8..data.len()).step_by(6).enumerate() {
            let frame = cursor.read_i32::<LittleEndian>()?;
            let down = cursor.read_u8()? == 0x31;
            let p1 = cursor.read_u8()? == 0x31;
            let time = frame as f32 / self.fps;

            if p1 {
                self.process_action_p1(time, down);
            } else {
                self.process_action_p2(time, down);
            }
        }

        Ok(())
    }

    fn parse_tasbot(&mut self, data: &[u8]) -> Result<()> {
        let v: Value = serde_json::from_slice(data)?;
        self.fps = v["fps"].as_f64().context("couldn't get 'fps' field")? as f32;
        let events = v["macro"]
            .as_array()
            .context("couldn't get 'macro' array")?;

        for ev in events {
            let frame = ev["frame"].as_u64().context("couldn't get 'frame' field")?;
            let time = frame as f32 / self.fps;

            let p1 = ev["player_1"]["click"]
                .as_i64()
                .context("failed to get p1 'click' field")?;
            let p2 = ev["player_2"]["click"]
                .as_i64()
                .context("failed to get p2 'click' field")?;

            self.process_action_p1(time, p1 != 0);
            self.process_action_p2(time, p2 != 0);
        }

        Ok(())
    }

    fn parse_mhr(&mut self, data: &[u8]) -> Result<()> {
        let v: Value = serde_json::from_slice(data)?;
        self.fps = v["meta"]["fps"]
            .as_f64()
            .context("couldn't get 'fps' field (does 'meta' exist?)")? as f32;

        let events = v["events"]
            .as_array()
            .context("couldn't get 'events' array")?;

        let mut next_p2 = false;

        for ev in events {
            let frame = ev["frame"].as_u64().context("couldn't get 'frame' field")?;
            let time = frame as f32 / self.fps;

            let Some(down) = ev["down"].as_bool() else {
                continue;
            };

            if next_p2 {
                self.process_action_p2(time, down);
            } else {
                self.process_action_p1(time, down);
            }

            // 'p2' always seems to be true if it exists, but we'll still query the value just to be safe
            if let Some(p2) = ev.get("p2") {
                next_p2 = p2.as_bool().context("couldn't get 'p2' field")?;
            }
        }

        Ok(())
    }
}
