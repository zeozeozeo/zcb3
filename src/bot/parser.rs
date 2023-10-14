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
    /// .replay files
    Obot2,
    /// Ybot frame files (no extension)
    Ybotf,
    /// .mhr files
    MhrBin,
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
            if Macro::parse(MacroType::Zbot, data, 0.0).is_ok() {
                return Ok(Self::Zbot);
            }
        } else if filename.ends_with(".replay") {
            if Macro::parse(MacroType::Obot2, data, 0.0).is_ok() {
                return Ok(Self::Obot2);
            }
        } else if filename.ends_with(".ybf") {
            if Macro::parse(MacroType::Ybotf, data, 0.0).is_ok() {
                return Ok(Self::Ybotf);
            }
        } else if filename.ends_with(".mhr") {
            if Macro::parse(MacroType::MhrBin, data, 0.0).is_ok() {
                return Ok(Self::MhrBin);
            }
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
            MacroType::Obot2 => replay.parse_obot2(data)?,
            MacroType::Ybotf => replay.parse_ybotf(data)?,
            MacroType::MhrBin => replay.parse_mhrbin(data)?,
        }

        if !replay.actions.is_empty() {
            replay.duration = replay.actions.last().unwrap().time;
        }

        log::info!(
            "macro fps: {}; macro duration: {:?}",
            replay.fps,
            replay.duration
        );

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

    fn parse_ybotf(&mut self, data: &[u8]) -> Result<()> {
        // honestly i don't know if this works
        use byteorder::{LittleEndian, ReadBytesExt};
        let mut cursor = Cursor::new(data);

        self.fps = cursor.read_f32::<LittleEndian>()?;
        let num_actions = cursor.read_i32::<LittleEndian>()?;

        for _ in (12..12 + num_actions * 8).step_by(8) {
            let frame = cursor.read_u32::<LittleEndian>()?;
            let what = cursor.read_u32::<LittleEndian>()?;
            let down = (what & 0b10) == 2;
            let p2 = (what & 0b01) == 1;
            let time = frame as f32 / self.fps;

            if p2 {
                self.process_action_p2(time, down);
            } else {
                self.process_action_p1(time, down);
            }
        }

        Ok(())
    }

    fn parse_obot2(&mut self, data: &[u8]) -> Result<()> {
        use serde::Deserialize;

        // structs that are serialized by obot using [`bincode`]
        #[derive(Deserialize, Debug, Clone, Copy)]
        pub enum OLocation {
            XPos(u32),
            Frame(u32),
        }
        #[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
        enum OReplayType {
            XPos,
            Frame,
        }
        #[derive(Deserialize, Debug, Clone, Copy)]
        enum OClickType {
            None,
            FpsChange(f32),
            Player1Down,
            Player1Up,
            Player2Down,
            Player2Up,
        }
        #[derive(Deserialize, Debug, Clone, Copy)]
        struct OClick {
            location: OLocation,
            click_type: OClickType,
        }
        #[derive(Deserialize, Debug, Clone)]
        struct OReplay {
            initial_fps: f32,
            _current_fps: f32,
            replay_type: OReplayType,
            _current_click: usize,
            clicks: Vec<OClick>,
        }

        let decoded: OReplay = bincode::deserialize(data)?;

        if decoded.replay_type == OReplayType::XPos {
            log::error!("xpos replays not supported, because they doesn't store frames");
            return Err(anyhow::anyhow!(
                "xpos replays not supported, because they doesn't store frames"
            ));
        };

        self.fps = decoded.initial_fps;
        let mut current_fps = self.fps;

        for action in decoded.clicks {
            let time = match action.location {
                OLocation::Frame(frame) => frame as f32 / current_fps,
                _ => {
                    log::warn!("got xpos action while replay type is frame, skipping");
                    continue;
                }
            };
            match action.click_type {
                OClickType::Player1Down => self.process_action_p1(time, true),
                OClickType::Player1Up => self.process_action_p1(time, false),
                OClickType::Player2Down => self.process_action_p2(time, true),
                OClickType::Player2Up => self.process_action_p2(time, false),
                OClickType::FpsChange(fps) => current_fps = fps,
                OClickType::None => {}
            }
        }

        Ok(())
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

            self.process_action_p1(time, p1 == 1);
            self.process_action_p2(time, p2 == 1);
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

    fn parse_mhrbin(&mut self, data: &[u8]) -> Result<()> {
        // this needs testing
        use byteorder::{LittleEndian, ReadBytesExt};
        let mut cursor = Cursor::new(data);

        cursor.set_position(12);
        self.fps = cursor.read_f32::<LittleEndian>()?;
        cursor.set_position(28);

        for _ in (28..data.len()).step_by(32).enumerate() {
            // skip 2 bytes
            cursor.set_position(cursor.position() + 2);
            let p1 = cursor.read_u8()? == 0;
            let down = cursor.read_u8()? == 1;
            let frame = cursor.read_u32::<LittleEndian>()?;
            let time = frame as f32 / self.fps;
            // skip 24 bytes
            cursor.set_position(cursor.position() + 24);

            if p1 {
                self.process_action_p1(time, down);
            } else {
                self.process_action_p2(time, down);
            }
        }

        Ok(())
    }
}
