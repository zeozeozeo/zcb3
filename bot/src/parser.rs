use anyhow::{Context, Result};
use rand::Rng;
use serde_json::Value;
use std::io::Cursor;

use crate::{Timings, VolumeSettings};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum ClickType {
    HardClick,
    HardRelease,
    Click,
    Release,
    SoftClick,
    SoftRelease,
    MicroClick,
    MicroRelease,
    #[default]
    None,
}

impl ClickType {
    /// * `time` - time between clicks
    ///
    /// # Returns
    ///
    /// The click type and the volume offset.
    pub fn from_time(
        time: f32,
        timings: Timings,
        is_click: bool,
        vol: VolumeSettings,
    ) -> (Self, f32) {
        let rand_var = rand::thread_rng().gen_range(-vol.volume_var..=vol.volume_var);
        let vol_offset =
            if vol.enabled && time < vol.spam_time && !(!vol.change_releases_volume && !is_click) {
                let offset = (vol.spam_time - time) * vol.spam_vol_offset_factor;
                (offset.clamp(0.0, vol.max_spam_vol_offset) + rand_var) * vol.global_volume
            } else {
                rand_var * vol.global_volume
            };

        let typ = if time > timings.hard {
            if is_click {
                Self::HardClick
            } else {
                Self::HardRelease
            }
        } else if time > timings.regular {
            if is_click {
                Self::Click
            } else {
                Self::Release
            }
        } else if time > timings.soft {
            if is_click {
                Self::SoftClick
            } else {
                Self::SoftRelease
            }
        } else {
            if is_click {
                Self::MicroClick
            } else {
                Self::MicroRelease
            }
        };
        (typ, vol_offset)
    }

    /// Order of which clicks should be selected depending on the actual click type
    #[rustfmt::skip]
    pub fn preferred(self) -> [Self; 4] {
        // import all enum variants to scope
        use ClickType::*;

        // this is perfect
        match self {
            HardClick =>    [HardClick,    Click,        SoftClick,   MicroClick  ],
            HardRelease =>  [HardRelease,  Release,      SoftRelease, MicroRelease],
            Click =>        [Click,        HardClick,    SoftClick,   MicroClick  ],
            Release =>      [Release,      HardRelease,  SoftRelease, MicroRelease],
            SoftClick =>    [SoftClick,    MicroClick,   Click,       HardClick   ],
            SoftRelease =>  [SoftRelease,  MicroRelease, Release,     HardRelease ],
            MicroClick =>   [MicroClick,   SoftClick,    Click,       HardClick   ],
            MicroRelease => [MicroRelease, SoftRelease,  Release,     HardRelease ],
            None =>         [None,         None,         None,        None        ],
        }
    }

    pub const fn is_release(self) -> bool {
        matches!(
            self,
            ClickType::HardRelease
                | ClickType::Release
                | ClickType::SoftRelease
                | ClickType::MicroRelease
        )
    }

    pub const fn is_click(self) -> bool {
        !self.is_release()
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
    /// Volume offset of the action.
    pub vol_offset: f32,
}

impl Action {
    pub const fn new(time: f32, player: Player, click: ClickType, vol_offset: f32) -> Self {
        Self {
            time,
            player,
            click,
            vol_offset,
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
    timings: Timings,
    vol_settings: VolumeSettings,
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
    /// .echo files
    EchoBin,
    /// .thyst files
    Amethyst,
    /// .osr files
    OsuReplay,
}

impl MacroType {
    pub fn guess_format(filename: &str) -> Result<Self> {
        use MacroType::*;
        let ext = filename
            .split('.')
            .last()
            .context("replay file has no extension")?;

        Ok(match ext {
            "json" => {
                if filename.ends_with(".mhr.json") {
                    MegaHack
                } else {
                    TasBot
                }
            }
            "zbf" => Zbot,
            "replay" => Obot2,
            "ybf" => Ybotf,
            "mhr" => MhrBin,
            "echo" => EchoBin,
            "thyst" => Amethyst,
            "osr" => OsuReplay,
            _ => anyhow::bail!("unknown replay format"),
        })
    }
}

impl Macro {
    pub const SUPPORTED_EXTENSIONS: &[&'static str] = &[
        "json", "mhr.json", "mhr", "zbf", "replay", "ybf", "echo", "thyst", "osr",
    ];

    pub fn parse(
        typ: MacroType,
        data: &[u8],
        timings: Timings,
        vol_settings: VolumeSettings,
    ) -> Result<Self> {
        log::info!("parsing replay, strlen {}, replay type {typ:?}", data.len());

        let mut replay = Self::default();
        replay.timings = timings;
        replay.vol_settings = vol_settings;

        match typ {
            MacroType::MegaHack => replay.parse_mhr(data)?,
            MacroType::TasBot => replay.parse_tasbot(data)?,
            MacroType::Zbot => replay.parse_zbf(data)?,
            MacroType::Obot2 => replay.parse_obot2(data)?,
            MacroType::Ybotf => replay.parse_ybotf(data)?,
            MacroType::MhrBin => replay.parse_mhrbin(data)?,
            MacroType::EchoBin => replay.parse_echobin(data)?,
            MacroType::Amethyst => replay.parse_amethyst(data)?,
            MacroType::OsuReplay => replay.parse_osr(data)?,
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
        // if action is the same, skip it
        if down == self.prev_action.0.is_click() {
            return;
        }

        let delta = time - self.prev_time.0;
        let (typ, vol_offset) = ClickType::from_time(delta, self.timings, down, self.vol_settings);

        self.prev_time.0 = time;
        self.prev_action.0 = typ;
        self.actions
            .push(Action::new(time, Player::One, typ, vol_offset))
    }

    // .0 is changed to .1 here, because it's the second player
    fn process_action_p2(&mut self, time: f32, down: bool) {
        if down == self.prev_action.1.is_click() {
            return;
        }

        let delta = time - self.prev_time.1;
        let (typ, vol_offset) = ClickType::from_time(delta, self.timings, down, self.vol_settings);

        self.prev_time.1 = time;
        self.prev_action.1 = typ;
        self.actions
            .push(Action::new(time, Player::Two, typ, vol_offset))
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
        use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
        let mut cursor = Cursor::new(data);

        let magic = cursor.read_u32::<BigEndian>()?;
        if magic != 0x4841434B {
            // HACK
            log::error!("invalid mhrbin magic: {}", magic);
            return Err(anyhow::anyhow!("unknown mhrbin magic: {}", magic));
        }

        cursor.set_position(12);
        self.fps = cursor.read_u32::<LittleEndian>()? as f32;
        log::debug!("fps: {}", self.fps);
        cursor.set_position(28);
        let num_actions = cursor.read_u32::<LittleEndian>()?;
        log::debug!("num_actions: {}", num_actions);

        for _ in 0..num_actions {
            // let format = cursor.read_u16::<LittleEndian>()?;
            // if format != 0x0002 {
            //     log::error!("xpos replays not supported, because they don't store frames");
            //     return Err(anyhow::anyhow!(
            //         "xpos replays not supported, because they don't store frames"
            //     ));
            // }
            cursor.set_position(cursor.position() + 2);
            let down = cursor.read_u8()? == 1;
            let p1 = cursor.read_u8()? == 0;
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

    fn parse_echobin(&mut self, data: &[u8]) -> Result<()> {
        use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
        let mut cursor = Cursor::new(data);

        let magic = cursor.read_u32::<BigEndian>()?;
        if magic != 0x4D455441 {
            log::error!("invalid echobin magic: {}", magic);
            return Err(anyhow::anyhow!("unknown echobin magic: {}", magic));
        }

        let replay_type = cursor.read_u32::<BigEndian>()?;
        let action_size;
        if replay_type == 0x44424700 {
            action_size = 24;
        } else {
            action_size = 6;
        }
        cursor.set_position(24);
        self.fps = cursor.read_f32::<LittleEndian>()?;
        cursor.set_position(48);

        for _ in (48..data.len()).step_by(action_size).enumerate() {
            let frame = cursor.read_u32::<LittleEndian>()?;
            let down = cursor.read_u8()? == 1;
            let p1 = cursor.read_u8()? == 0;
            let time = frame as f32 / self.fps;

            if p1 {
                self.process_action_p1(time, down);
            } else {
                self.process_action_p2(time, down);
            }
        }

        Ok(())
    }

    /// Amethyst stores macros like this:
    ///
    /// ```no_run
    /// /* for player1 clicks */
    /// {num actions}
    /// {action time}...
    /// /* for player1 releases */
    /// {num actions}
    /// {action time}...
    /// /* for player2 clicks */
    /// {num actions}
    /// {action time}...
    /// /* for player2 releases */
    /// {num actions}
    /// {action time}...
    /// ```
    fn parse_amethyst(&mut self, data: &[u8]) -> Result<()> {
        let data = String::from_utf8(data.to_vec())?;
        let mut lines = data.split('\n');

        let mut get_times = |player, is_clicks| -> Result<Vec<(Player, bool, f32)>> {
            let num: usize = lines.next().context("unexpected EOF")?.parse()?;
            let mut times: Vec<(Player, bool, f32)> = Vec::with_capacity(num);
            for _ in 0..num {
                let time: f32 = lines.next().context("unexpected EOF")?.parse()?;
                times.push((player, is_clicks, time));
            }
            Ok(times)
        };

        let mut actions = get_times(Player::One, true)?;
        actions.extend(get_times(Player::One, false)?);
        actions.extend(get_times(Player::Two, true)?);
        actions.extend(get_times(Player::Two, false)?);
        actions.sort_by(|a, b| a.2.total_cmp(&b.2)); // sort actions by time

        for action in actions {
            if action.0 == Player::One {
                self.process_action_p1(action.2, action.1);
            } else {
                self.process_action_p2(action.2, action.1);
            }
        }

        Ok(())
    }

    // https://osu.ppy.sh/wiki/en/Client/File_formats/osr_%28file_format%29
    fn parse_osr(&mut self, data: &[u8]) -> Result<()> {
        use byteorder::{LittleEndian, ReadBytesExt};
        let mut cursor = Cursor::new(data);

        self.fps = 1000.0;

        cursor.set_position(5);
        let bm_md5_exists = cursor.read_u8()? == 0x0b;
        if bm_md5_exists {
            let str_len = leb128::read::unsigned(&mut cursor)?;
            cursor.set_position(cursor.position() + str_len);
        }

        let player_name_exists = cursor.read_u8()? == 0x0b;
        if player_name_exists {
            let str_len = leb128::read::unsigned(&mut cursor)?;
            cursor.set_position(cursor.position() + str_len);
        }

        let replay_md5_exists = cursor.read_u8()? == 0x0b;
        if replay_md5_exists {
            let str_len = leb128::read::unsigned(&mut cursor)?;
            cursor.set_position(cursor.position() + str_len);
        }

        cursor.set_position(cursor.position() + 20); // skip 8 bytes
        let mods = cursor.read_i32::<LittleEndian>()?;
        let speed;
        if mods & (1 << 6) != 0 {
            // dt
            speed = 1.5;
        } else if mods & (1 << 8) != 0 {
            // ht
            speed = 0.75;
        } else {
            // nm
            speed = 1.0;
        }

        let life_graph_exists = cursor.read_u8()? == 0x0b;
        if life_graph_exists {
            let str_len = leb128::read::unsigned(&mut cursor)?;
            cursor.set_position(cursor.position() + str_len);
        }

        cursor.set_position(cursor.position() + 8); // skip 8 bytes

        let data_len = cursor.read_u32::<LittleEndian>()?;
        let data =
            &data[cursor.position() as usize..(cursor.position() + data_len as u64) as usize];

        let mut decompressed_data = Vec::new();

        // try to decompress with lzma or lzma2
        if lzma_rs::lzma_decompress(&mut Cursor::new(data), &mut decompressed_data).is_err() {
            decompressed_data.clear();
            lzma_rs::lzma2_decompress(&mut Cursor::new(data), &mut decompressed_data)?;
        }

        let data_str = String::from_utf8(decompressed_data)?;

        let entries = data_str.split(',');
        let mut current_time = 0;

        for entry in entries {
            let params = entry.split('|');
            let vec_params = params.collect::<Vec<&str>>();
            let delta_time = vec_params[0].parse::<i64>()?;
            current_time += delta_time;
            let time = (current_time as f32 * speed) / self.fps;

            let keys = vec_params[1].parse::<i32>()?;

            if keys & (1 << 0) != 0 {
                // m1
                self.process_action_p1(time, true);
            }
            if keys & (1 << 1) != 0 {
                // m2
                self.process_action_p2(time, true);
            }
        }

        Ok(())
    }
}
