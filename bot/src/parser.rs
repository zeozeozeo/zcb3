use crate::{Timings, VolumeSettings};
use anyhow::{Context, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use rand::Rng;
use serde_json::Value;
use std::io::{Cursor, Read, Write};

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
        } else if is_click {
            Self::MicroClick
        } else {
            Self::MicroRelease
        };
        (typ, vol_offset)
    }

    /// Returns the opposite click type. For example, every click will be translated to a release,
    /// and every release will be translated into a click.
    ///
    /// None will always be translated to None.
    #[inline]
    pub fn opposite(self) -> Self {
        use ClickType::*;
        match self {
            HardClick => HardRelease,
            Click => Release,
            SoftClick => SoftRelease,
            MicroClick => MicroRelease,
            HardRelease => HardClick,
            Release => Click,
            SoftRelease => SoftClick,
            MicroRelease => MicroClick,
            None => None,
        }
    }

    /// Order of which clicks should be selected depending on the actual click type
    #[rustfmt::skip]
    pub fn preferred(self) -> [Self; 8] {
        use ClickType::*;

        // this is perfect
        match self {
            HardClick =>    [HardClick,    Click,        SoftClick,   MicroClick  , HardRelease,  Release,      SoftRelease, MicroRelease],
            HardRelease =>  [HardRelease,  Release,      SoftRelease, MicroRelease, HardRelease,  Release,      SoftRelease, MicroRelease],
            Click =>        [Click,        HardClick,    SoftClick,   MicroClick  , Release,      HardRelease,  SoftRelease, MicroRelease],
            Release =>      [Release,      HardRelease,  SoftRelease, MicroRelease, Release,      HardRelease,  SoftRelease, MicroRelease],
            SoftClick =>    [SoftClick,    MicroClick,   Click,       HardClick   , SoftRelease,  MicroRelease, Release,     HardRelease ],
            SoftRelease =>  [SoftRelease,  MicroRelease, Release,     HardRelease , SoftRelease,  MicroRelease, Release,     HardRelease ],
            MicroClick =>   [MicroClick,   SoftClick,    Click,       HardClick   , MicroRelease, SoftRelease,  Release,     HardRelease ],
            MicroRelease => [MicroRelease, SoftRelease,  Release,     HardRelease , MicroRelease, SoftRelease,  Release,     HardRelease ],
            None =>         [None,         None,         None,        None        , None,         None,         None,        None        ],
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
    /// Frame.
    pub frame: u32,
}

impl Action {
    pub const fn new(
        time: f32,
        player: Player,
        click: ClickType,
        vol_offset: f32,
        frame: u32,
    ) -> Self {
        Self {
            time,
            player,
            click,
            vol_offset,
            frame,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Player {
    #[default]
    One,
    Two,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ExtendedAction {
    pub player2: bool,
    pub down: bool,
    pub frame: u32,
    pub x: f32,
    pub y: f32,
    pub y_accel: f32,
    pub rot: f32,
    pub fps_change: Option<f32>,
}

#[derive(Clone, Debug, Default)]
pub struct Macro {
    /// Framerate of the macro.
    pub fps: f32,
    /// Duration of the macro (in seconds).
    pub duration: f32,
    /// Actions used for generating clicks.
    pub actions: Vec<Action>,
    /// Whether to populate the `extended` vector.
    pub extended_data: bool,
    /// Action data used for converting macros.
    pub extended: Vec<ExtendedAction>,

    // used for determining the click type
    prev_action: (Option<ClickType>, Option<ClickType>),
    prev_time: (f32, f32),

    // used for generating additional click info
    timings: Timings,
    vol_settings: VolumeSettings,
}

#[derive(Clone, Copy, Debug)]
pub enum MacroType {
    /// .mhr.json files
    Mhr,
    /// .json files
    TasBot,
    /// .zbf files
    Zbot,
    /// OmegaBot 3 and OmegaBot 2 .replay files
    Obot,
    /// Ybot frame files (no extension)
    Ybotf,
    /// .mhr files
    MhrBin,
    /// .echo files (new binary format, new json format and old json format)
    Echo,
    /// .thyst files
    Amethyst,
    /// .osr files
    OsuReplay,
    /// GDMegaOverlay .macro files
    Gdmo,
    /// ReplayBot .replay files (rename to .replaybot)
    ReplayBot,
    /// Rush .rsh files
    Rush,
    /// KDBot .kd files
    Kdbot,
    /// Text files generated by mat's macro converter
    Txt,
    /// ReplayEngine .re files
    ReplayEngine,
    /// DDHOR .ddhor files
    Ddhor,
    /// Xbot Frame .xbot files
    Xbot,
    // GatoBot .gatobot files
    // GatoBot,
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
                    Mhr
                } else if filename.ends_with(".echo.json") {
                    Echo
                } else {
                    TasBot
                }
            }
            "zbf" => Zbot,
            "replay" => Obot,
            "ybf" => Ybotf,
            "mhr" => MhrBin,
            "echo" => Echo, // the parser will also handle the old echo format
            "thyst" => Amethyst,
            "osr" => OsuReplay,
            "macro" => Gdmo,
            "replaybot" => ReplayBot,
            "rsh" => Rush,
            "kd" => Kdbot,
            "txt" => Txt,
            "re" => ReplayEngine,
            "ddhor" => Ddhor,
            "xbot" => Xbot,
            // "gatobot" => GatoBot,
            _ => anyhow::bail!("unknown replay format"),
        })
    }
}

use serde::{Deserialize, Serialize};

// structs that are serialized by obot2 using [`bincode`]

#[derive(Serialize, Deserialize)]
pub enum Obot2Location {
    XPos(u32),
    Frame(u32),
}
#[derive(Serialize, Deserialize, PartialEq)]
enum Obot2ReplayType {
    XPos,
    Frame,
}
#[derive(Serialize, Deserialize, PartialEq, Clone, Copy)]
enum Obot2ClickType {
    None,
    FpsChange(f32),
    Player1Down,
    Player1Up,
    Player2Down,
    Player2Up,
}
#[derive(Serialize, Deserialize)]
struct Obot2Click {
    location: Obot2Location,
    click_type: Obot2ClickType,
}
#[derive(Serialize, Deserialize)]
struct Obot2Replay {
    initial_fps: f32,
    current_fps: f32,
    replay_type: Obot2ReplayType,
    current_click: usize,
    clicks: Vec<Obot2Click>,
}

// structs that are serialized by obot3 using [`dlhn`]

#[derive(Serialize, Deserialize)]
enum Obot3ClickType {
    None,
    Player1Down,
    Player1Up,
    Player2Down,
    Player2Up,
    FpsChange(f32),
}

#[derive(Serialize, Deserialize)]
struct Obot3Click {
    frame: u32,
    click_type: Obot3ClickType,
}

#[derive(Serialize, Deserialize)]
struct Obot3Replay {
    initial_fps: f32,
    current_fps: f32,
    clicks: Vec<Obot3Click>,
}

impl Macro {
    pub const SUPPORTED_EXTENSIONS: &[&'static str] = &[
        "json",
        "mhr.json",
        "mhr",
        "zbf",
        "replay",
        "ybf",
        "echo",
        "echo.json",
        "thyst",
        "osr",
        "macro",
        "replaybot",
        "rsh",
        "kd",
        "txt",
        "re",
        "ddhor",
        "xbot",
        // "gatobot",
    ];

    pub fn parse(
        typ: MacroType,
        data: &[u8],
        timings: Timings,
        vol_settings: VolumeSettings,
        extended: bool,
        sort_actions: bool,
    ) -> Result<Self> {
        log::info!("parsing replay, strlen {}, replay type {typ:?}", data.len());

        let mut replay = Macro {
            timings,
            vol_settings,
            extended_data: extended,
            ..Default::default()
        };

        match typ {
            MacroType::Mhr => replay.parse_mhr(data)?,
            MacroType::TasBot => replay.parse_tasbot(data)?,
            MacroType::Zbot => replay.parse_zbf(data)?,
            MacroType::Obot => replay.parse_obot2(data)?, // will also handle obot3 replays
            MacroType::Ybotf => replay.parse_ybotf(data)?,
            MacroType::MhrBin => replay.parse_mhrbin(data)?,
            MacroType::Echo => replay.parse_echo(data)?, // will handle all 3 replay versions
            MacroType::Amethyst => replay.parse_amethyst(data)?,
            MacroType::OsuReplay => replay.parse_osr(data)?,
            MacroType::Gdmo => replay.parse_gdmo(data)?,
            MacroType::ReplayBot => replay.parse_replaybot(data)?,
            MacroType::Rush => replay.parse_rush(data)?,
            MacroType::Kdbot => replay.parse_kdbot(data)?,
            MacroType::Txt => replay.parse_txt(data)?,
            MacroType::ReplayEngine => replay.parse_re(data)?,
            MacroType::Ddhor => replay.parse_ddhor(data)?,
            MacroType::Xbot => replay.parse_xbot(data)?,
            // MacroType::GatoBot => replay.parse_gatobot(data)?,
        }

        // sort actions by time / frame
        if sort_actions {
            replay.sort_actions();
        }

        if let Some(last) = replay.actions.last() {
            replay.duration = last.time;
        }

        log::info!(
            "macro fps: {}; macro duration: {:?}",
            replay.fps,
            replay.duration
        );

        Ok(replay)
    }

    /// Sorts actions by time / frame.
    pub fn sort_actions(&mut self) {
        self.actions.sort_by(|a, b| a.time.total_cmp(&b.time));
        self.extended.sort_by(|a, b| a.frame.cmp(&b.frame));
    }

    pub fn write<W: Write>(&self, typ: MacroType, writer: W) -> Result<()> {
        match typ {
            MacroType::Mhr => self.write_mhr(writer)?,
            MacroType::TasBot => self.write_tasbot(writer)?,
            MacroType::Zbot => self.write_zbf(writer)?,
            MacroType::Obot => self.write_obot2(writer)?,
            MacroType::Ybotf => self.write_ybotf(writer)?,
            MacroType::Echo => self.write_echo(writer)?,
            MacroType::Amethyst => self.write_amethyst(writer)?,
            _ => anyhow::bail!("unsupported format"),
        }
        Ok(())
    }

    fn process_action_p1(&mut self, time: f32, down: bool, frame: u32) {
        // if action is the same, skip it
        if let Some(typ) = self.prev_action.0 {
            if down == typ.is_click() {
                return;
            }
        }

        let delta = time - self.prev_time.0;
        let (typ, vol_offset) = ClickType::from_time(delta, self.timings, down, self.vol_settings);

        self.prev_time.0 = time;
        self.prev_action.0 = Some(typ);
        self.actions
            .push(Action::new(time, Player::One, typ, vol_offset, frame))
    }

    // .0 is changed to .1 here, because it's the second player
    fn process_action_p2(&mut self, time: f32, down: bool, frame: u32) {
        if let Some(typ) = self.prev_action.1 {
            if down == typ.is_click() {
                return;
            }
        }

        let delta = time - self.prev_time.1;
        let (typ, vol_offset) = ClickType::from_time(delta, self.timings, down, self.vol_settings);

        self.prev_time.1 = time;
        self.prev_action.1 = Some(typ);
        self.actions
            .push(Action::new(time, Player::Two, typ, vol_offset, frame))
    }

    fn extended_p1(&mut self, down: bool, frame: u32, x: f32, y: f32, y_accel: f32, rot: f32) {
        if self.extended_data {
            self.extended.push(ExtendedAction {
                player2: false,
                down,
                frame,
                x,
                y,
                y_accel,
                rot,
                fps_change: None,
            });
        }
    }

    fn extended_p2(&mut self, down: bool, frame: u32, x: f32, y: f32, y_accel: f32, rot: f32) {
        if self.extended_data {
            // if x is 0.0, try to get the x position from the first player
            let x = if x == 0. {
                if let Some(last) = self.get_last_extended(Player::One) {
                    last.x
                } else {
                    x
                }
            } else {
                x
            };

            self.extended.push(ExtendedAction {
                player2: true,
                down,
                frame,
                x,
                y,
                y_accel,
                rot,
                fps_change: None,
            });
        }
    }

    fn get_last_extended(&self, player: Player) -> Option<ExtendedAction> {
        // iterate from the back and find the last action for this player
        for action in self.extended.iter().rev() {
            if player != Player::Two && action.player2 {
                continue;
            }
            return Some(*action);
        }
        None
    }

    /// Returns the last frame in the macro. If extended actions are disabled, this
    /// always returns 0.
    #[inline]
    pub fn last_frame(&self) -> u32 {
        if let Some(last) = self.extended.last() {
            last.frame
        } else {
            0
        }
    }

    #[inline]
    fn fps_change(&mut self, fps_change: f32) {
        if let Some(last) = self.extended.last_mut() {
            last.fps_change = Some(fps_change);
        }
    }

    fn parse_ybotf(&mut self, data: &[u8]) -> Result<()> {
        // honestly i don't know if this works
        let mut cursor = Cursor::new(data);

        self.fps = cursor.read_f32::<LittleEndian>()?;
        let num_actions = cursor.read_i32::<LittleEndian>()?;

        for _ in (12..12 + num_actions * 8).step_by(8) {
            let frame = cursor.read_u32::<LittleEndian>()?;
            let state = cursor.read_u32::<LittleEndian>()?;
            let down = (state & 0b10) == 2;
            let p2 = (state & 0b01) == 1;
            let time = frame as f32 / self.fps;

            if p2 {
                self.process_action_p2(time, down, frame);
                self.extended_p2(down, frame, 0., 0., 0., 0.);
            } else {
                self.process_action_p1(time, down, frame);
                self.extended_p1(down, frame, 0., 0., 0., 0.);
            }
        }

        Ok(())
    }

    fn write_ybotf<W: Write>(&self, mut writer: W) -> Result<()> {
        writer.write_f32::<LittleEndian>(self.fps)?; // fps
        writer.write_i32::<LittleEndian>(self.extended.len() as i32)?; // num actions

        for action in &self.extended {
            writer.write_u32::<LittleEndian>(action.frame)?;
            let state = action.player2 as u32 + action.down as u32 * 2;
            writer.write_u32::<LittleEndian>(state)?;
        }

        Ok(())
    }

    /// Will also handle obot3 replays.
    fn parse_obot2(&mut self, data: &[u8]) -> Result<()> {
        let Ok(decoded) = bincode::deserialize::<Obot2Replay>(data) else {
            return self.parse_obot3(data);
        };

        if decoded.replay_type == Obot2ReplayType::XPos {
            log::error!("xpos replays not supported, because they doesn't store frames");
            anyhow::bail!("xpos replays not supported, because they doesn't store frames")
        };

        self.fps = decoded.initial_fps;
        let mut current_fps = self.fps;

        for action in decoded.clicks {
            let frame = match action.location {
                Obot2Location::Frame(frame) => frame,
                _ => {
                    log::warn!("got xpos action while replay type is frame, skipping");
                    continue;
                }
            };
            let time = frame as f32 / current_fps;
            match action.click_type {
                Obot2ClickType::Player1Down => {
                    self.process_action_p1(time, true, frame);
                    self.extended_p1(true, frame, 0., 0., 0., 0.);
                }
                Obot2ClickType::Player1Up => {
                    self.process_action_p1(time, false, frame);
                    self.extended_p1(false, frame, 0., 0., 0., 0.);
                }
                Obot2ClickType::Player2Down => {
                    self.process_action_p2(time, true, frame);
                    self.extended_p2(true, frame, 0., 0., 0., 0.);
                }
                Obot2ClickType::Player2Up => {
                    self.process_action_p2(time, false, frame);
                    self.extended_p2(false, frame, 0., 0., 0., 0.);
                }
                Obot2ClickType::FpsChange(fps) => {
                    current_fps = fps;
                    self.fps_change(fps);
                }
                Obot2ClickType::None => {}
            }
        }

        Ok(())
    }

    fn write_obot2<W: Write>(&self, writer: W) -> Result<()> {
        let mut clicks = vec![];
        let mut prev_click_type = None;
        for action in &self.extended {
            let click_type = if action.player2 {
                if action.down {
                    Obot2ClickType::Player2Down
                } else {
                    Obot2ClickType::Player2Up
                }
            } else if action.down {
                Obot2ClickType::Player1Down
            } else {
                Obot2ClickType::Player1Up
            };
            if let Some(prev_click_type) = prev_click_type {
                if prev_click_type == click_type {
                    continue;
                }
            }
            prev_click_type = Some(click_type);
            clicks.push(Obot2Click {
                location: Obot2Location::Frame(action.frame),
                click_type,
            })
        }
        let replay = Obot2Replay {
            initial_fps: self.fps,
            current_fps: self.fps,
            replay_type: Obot2ReplayType::Frame,
            current_click: 0,
            clicks,
        };
        // obot2 uses bincode for serialization
        bincode::serialize_into(writer, &replay)?;
        Ok(())
    }

    fn parse_zbf(&mut self, data: &[u8]) -> Result<()> {
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
                self.process_action_p1(time, down, frame as _);
                self.extended_p1(down, frame as u32, 0., 0., 0., 0.);
            } else {
                self.process_action_p2(time, down, frame as _);
                self.extended_p2(down, frame as u32, 0., 0., 0., 0.);
            }
        }

        Ok(())
    }

    fn write_zbf<W: Write>(&self, mut writer: W) -> Result<()> {
        writer.write_f32::<LittleEndian>(1.0 / self.fps)?; // delta
        writer.write_f32::<LittleEndian>(1.0)?; // speedhack
                                                // 1.0 / delta / speedhack = fps
        for action in &self.extended {
            writer.write_i32::<LittleEndian>(action.frame as i32)?;
            writer.write_u8(if action.down { 0x31 } else { 0x30 })?;
            writer.write_u8(if action.player2 { 0x30 } else { 0x31 })?; // p1
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

            self.process_action_p1(time, p1 == 1, frame as _);
            self.process_action_p2(time, p2 == 1, frame as _);

            self.extended_p1(
                p1 == 1,
                frame as u32,
                ev["player_1"]["x_position"].as_f64().unwrap_or(0.) as f32,
                0.,
                0.,
                0.,
            );
            self.extended_p2(
                p2 == 1,
                frame as u32,
                ev["player_2"]["x_position"].as_f64().unwrap_or(0.) as f32,
                0.,
                0.,
                0.,
            );
        }

        Ok(())
    }

    fn write_tasbot<W: Write>(&self, writer: W) -> Result<()> {
        #[derive(Default, Serialize)]
        struct TasbotPlayerAction {
            click: i32,
            x_position: f32,
        }
        #[derive(Default, Serialize)]
        struct TasbotAction {
            frame: u32,
            player_1: TasbotPlayerAction,
            player_2: TasbotPlayerAction,
        }
        #[derive(Serialize)]
        struct TasbotReplay {
            fps: f32,
            #[serde(rename = "macro")]
            r#macro: Vec<TasbotAction>,
        }

        // create a replay array, try to predict capacity
        let mut replay: Vec<TasbotAction> = Vec::with_capacity(self.actions.len() / 2);
        let mut skip_action = false;

        for (i, action) in self.extended.iter().enumerate() {
            if skip_action {
                skip_action = false;
                continue;
            }

            let player_2 = if let Some(next) = self.extended.get(i + 1) {
                if next.player2 && next.frame == action.frame {
                    skip_action = true;
                    TasbotPlayerAction {
                        click: next.down as i32,
                        x_position: next.x,
                    }
                } else {
                    TasbotPlayerAction::default()
                }
            } else {
                TasbotPlayerAction::default()
            };

            replay.push(TasbotAction {
                frame: action.frame,
                player_1: TasbotPlayerAction {
                    click: action.down as i32,
                    x_position: action.x,
                },
                player_2,
            });
        }
        let replay = TasbotReplay {
            fps: self.fps,
            r#macro: replay,
        };
        serde_json::to_writer_pretty(writer, &replay)?;
        Ok(())
    }

    fn parse_mhr(&mut self, data: &[u8]) -> Result<()> {
        let v: serde_json::Result<Value> = serde_json::from_slice(data);

        // if we can't parse the JSON, try parsing the binary format
        if v.is_err() && self.parse_mhrbin(data).is_ok() {
            return Ok(());
        } else {
            // failed to parse binary format
            self.actions.clear();
            self.extended.clear();
        }

        let v = v?;

        self.fps = v["meta"]["fps"]
            .as_f64()
            .context("couldn't get 'fps' field (does 'meta' exist?)")? as f32;

        let events = v["events"]
            .as_array()
            .context("couldn't get 'events' array")?;

        for ev in events {
            let frame = ev["frame"].as_u64().context("couldn't get 'frame' field")?;
            let time = frame as f32 / self.fps;

            let Some(down) = ev["down"].as_bool() else {
                continue;
            };

            // 'p2' always seems to be true if it exists, but we'll still query the value just to be safe
            let p2 = if let Some(p2) = ev.get("p2") {
                p2.as_bool().context("couldn't get 'p2' field")?
            } else {
                false
            };

            let y_accel = ev["a"].as_f64().unwrap_or(0.) as f32;
            let x = ev["x"].as_f64().unwrap_or(0.) as f32;
            let y = ev["y"].as_f64().unwrap_or(0.) as f32;
            let rot = ev["r"].as_f64().unwrap_or(0.) as f32;

            if p2 {
                self.process_action_p2(time, down, frame as _);
                self.extended_p2(down, frame as u32, x, y, y_accel, rot)
            } else {
                self.process_action_p1(time, down, frame as _);
                self.extended_p1(down, frame as u32, x, y, y_accel, rot)
            }
        }

        Ok(())
    }

    fn write_mhr<W: Write>(&self, writer: W) -> Result<()> {
        #[derive(Serialize)]
        struct MhrMeta {
            fps: i32,
        }
        fn is_false(b: &bool) -> bool {
            !b
        }
        #[derive(Serialize)]
        struct MhrEvent {
            a: f32,
            down: bool,
            frame: u32,
            #[serde(skip_serializing_if = "is_false")]
            p2: bool,
            r: f32,
            x: f32,
            y: f32,
        }
        #[derive(Serialize)]
        struct MhrReplay {
            #[serde(rename = "_")]
            version: String,
            events: Vec<MhrEvent>,
            meta: MhrMeta,
        }

        let events: Vec<MhrEvent> = self
            .extended
            .iter()
            .map(|action| MhrEvent {
                a: action.y_accel,
                down: action.down,
                frame: action.frame,
                p2: action.player2,
                r: action.rot,
                x: action.x,
                y: action.y,
            })
            .collect();
        let replay = MhrReplay {
            version: String::from("Mega Hack v7.1.1.3 Replay"),
            events,
            meta: MhrMeta {
                fps: self.fps as i32, // TODO: do we need this to be an int?
            },
        };
        serde_json::to_writer_pretty(writer, &replay)?;
        Ok(())
    }

    fn parse_mhrbin(&mut self, data: &[u8]) -> Result<()> {
        // if it's a json macro, load from json instead
        if serde_json::from_slice::<Value>(data).is_ok() {
            return self.parse_mhr(data);
        }

        use byteorder::BigEndian;
        let mut cursor = Cursor::new(data);

        let magic = cursor.read_u32::<BigEndian>()?;
        if magic != 0x4841434B {
            // HACK
            log::error!("invalid mhrbin magic: {}", magic);
            anyhow::bail!("unknown mhrbin magic: {}", magic)
        }

        cursor.set_position(12);
        self.fps = cursor.read_u32::<LittleEndian>()? as f32;
        log::debug!("fps: {}", self.fps);
        cursor.set_position(28);
        let num_actions = cursor.read_u32::<LittleEndian>()?;
        log::debug!("num_actions: {}", num_actions);

        for _ in 0..num_actions {
            cursor.set_position(cursor.position() + 2);
            let down = cursor.read_u8()? == 1;
            let p1 = cursor.read_u8()? == 0;
            let frame = cursor.read_u32::<LittleEndian>()?;
            let time = frame as f32 / self.fps;
            // skip 24 bytes
            cursor.set_position(cursor.position() + 24);

            if p1 {
                self.process_action_p1(time, down, frame);
                self.extended_p1(down, frame, 0., 0., 0., 0.); // TODO: parse all vars
            } else {
                self.process_action_p2(time, down, frame);
                self.extended_p2(down, frame, 0., 0., 0., 0.); // TODO: parse all vars
            }
        }

        Ok(())
    }

    /// Parses the new Echo macro format.
    fn parse_echobin(&mut self, data: &[u8]) -> Result<()> {
        use byteorder::BigEndian;
        let mut cursor = Cursor::new(data);

        let magic = cursor.read_u32::<BigEndian>()?;
        if magic != 0x4D455441 {
            log::error!("invalid echobin magic: {}", magic);
            anyhow::bail!("unknown echobin magic: {}", magic)
        }

        let replay_type = cursor.read_u32::<BigEndian>()?;
        let action_size = if replay_type == 0x44424700 { 24 } else { 6 };
        let dbg = action_size == 24;
        cursor.set_position(24);
        self.fps = cursor.read_f32::<LittleEndian>()?;
        cursor.set_position(48);

        for _ in (48..data.len()).step_by(action_size) {
            let frame = cursor.read_u32::<LittleEndian>()?;
            let down = cursor.read_u8()? == 1;
            let p1 = cursor.read_u8()? == 0;
            let time = frame as f32 / self.fps;

            // read extra vars (only saved in debug mode)
            let x = if dbg {
                cursor.read_f32::<LittleEndian>()?
            } else {
                0.
            };
            let y_accel = if dbg {
                cursor.read_f64::<LittleEndian>()?
            } else {
                0.
            };
            let _x_accel = if dbg {
                cursor.read_f64::<LittleEndian>()?
            } else {
                0.
            };
            let y = if dbg {
                cursor.read_f32::<LittleEndian>()?
            } else {
                0.
            };
            let rot = if dbg {
                cursor.read_f32::<LittleEndian>()?
            } else {
                0.
            };

            if p1 {
                self.process_action_p1(time, down, frame);
                self.extended_p1(down, frame, x, y, y_accel as _, rot);
            } else {
                self.process_action_p2(time, down, frame);
                self.extended_p2(down, frame, x, y, y_accel as _, rot);
            }
        }

        Ok(())
    }

    /// Parses the old Echo json format.
    fn parse_echo_old(&mut self, v: Value) -> Result<()> {
        self.fps = v["FPS"].as_f64().context("couldn't get 'FPS' field")? as f32;
        let starting_frame = v["Starting Frame"].as_u64().unwrap_or(0);

        for action in v["Echo Replay"]
            .as_array()
            .context("couldn't get 'Echo Replay' field")?
        {
            let frame = action["Frame"]
                .as_u64()
                .context("couldn't get 'Frame' field")?
                + starting_frame;
            let time = frame as f32 / self.fps;
            let p2 = action["Player 2"]
                .as_bool()
                .context("couldn't get 'Player 2' field")?;
            let down = action["Hold"]
                .as_bool()
                .context("couldn't get 'Hold' field")?;

            let x = action["X Position"].as_f64().unwrap_or(0.) as f32;
            let y = action["Y Position"].as_f64().unwrap_or(0.) as f32;
            let y_accel = action["Y Acceleration"].as_f64().unwrap_or(0.) as f32;
            let rot = action["Rotation"].as_f64().unwrap_or(0.) as f32;

            if p2 {
                self.process_action_p2(time, down, frame as _);
                self.extended_p2(down, frame as u32, x, y, y_accel, rot);
            } else {
                self.process_action_p1(time, down, frame as _);
                self.extended_p1(down, frame as u32, x, y, y_accel, rot);
            }
        }
        Ok(())
    }

    /// Parses .echo files (both old json and new binary formats).
    fn parse_echo(&mut self, data: &[u8]) -> Result<()> {
        let Ok(v) = serde_json::from_slice::<Value>(data) else {
            return self.parse_echobin(data); // can't parse json, parse binary
        };

        // try parsing old json format
        if self.parse_echo_old(v.clone()).is_ok() {
            return Ok(());
        } else {
            self.actions.clear();
            self.extended.clear();
        }

        // parse new json format
        self.fps = v["fps"].as_f64().context("no 'fps' field")? as f32;
        for action in v["inputs"].as_array().context("no 'inputs' field")? {
            let frame = action["frame"].as_u64().context("no 'frame' field")?;
            let time = frame as f32 / self.fps;
            let down = action["holding"].as_bool().context("no 'holding' field")?;
            let p2 = if let Some(p2) = action["player_2"].as_bool() {
                p2
            } else {
                false
            };
            let x = action["x_position"].as_f64().unwrap_or(0.);
            let y_accel = action["y_vel"].as_f64().unwrap_or(0.);
            // let _x_accel = action["x_vel"].as_f64().unwrap_or(0.);
            let rot = action["rotation"].as_f64().unwrap_or(0.);

            if p2 {
                self.process_action_p2(time, down, frame as _);
                self.extended_p2(down, frame as _, x as _, 0., y_accel as _, rot as _);
            } else {
                self.process_action_p1(time, down, frame as _);
                self.extended_p1(down, frame as _, x as _, 0., y_accel as _, rot as _);
            }
        }

        Ok(())
    }

    fn write_echo<W: Write>(&self, writer: W) -> Result<()> {
        #[derive(Serialize)]
        struct EchoAction {
            #[serde(rename = "Hold")]
            hold: bool,
            #[serde(rename = "Player 2")]
            player2: bool,
            #[serde(rename = "Frame")]
            frame: u32,
            #[serde(rename = "X Position")]
            x_position: f32,
            #[serde(rename = "Y Position")]
            y_position: f32,
            #[serde(rename = "Y Acceleration")]
            y_accel: f32,
            #[serde(rename = "Rotation")]
            rotation: f32,
            #[serde(rename = "Action")]
            action: bool,
        }
        #[derive(Serialize)]
        struct EchoReplay {
            #[serde(rename = "FPS")]
            fps: f32,
            #[serde(rename = "Starting Frame")]
            starting_frame: u8,
            #[serde(rename = "Type")]
            typ: String,
            #[serde(rename = "Echo Replay")]
            echo_replay: Vec<EchoAction>,
        }

        let echo_replay: Vec<EchoAction> = self
            .extended
            .iter()
            .map(|action| EchoAction {
                hold: action.down,
                frame: action.frame,
                player2: action.player2,
                x_position: action.x,
                y_position: action.y,
                y_accel: action.y_accel,
                rotation: action.rot,
                action: true,
            })
            .collect();
        let replay = EchoReplay {
            fps: self.fps,
            starting_frame: 0,
            typ: String::from("Frames"),
            echo_replay,
        };
        serde_json::to_writer_pretty(writer, &replay)?;
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
                self.process_action_p1(action.2, action.1, (action.2 * self.fps) as _);
                self.extended_p1(action.1, (action.2 * self.fps) as u32, 0., 0., 0., 0.);
            } else {
                self.process_action_p2(action.2, action.1, (action.2 * self.fps) as _);
                self.extended_p2(action.1, (action.2 * self.fps) as u32, 0., 0., 0., 0.);
            }
        }

        Ok(())
    }

    fn write_amethyst<W: Write>(&self, _writer: W) -> Result<()> {
        let mut prev_down = false;
        for action in &self.extended {
            if action.down != prev_down {
                prev_down = action.down;
            }
        }
        Ok(())
    }

    // https://osu.ppy.sh/wiki/en/Client/File_formats/osr_%28file_format%29
    fn parse_osr(&mut self, data: &[u8]) -> Result<()> {
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

        cursor.set_position(cursor.position() + 19);
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
            if vec_params.len() != 4 {
                continue; // this is probably the last action
            }
            let delta_time = vec_params[0].parse::<i64>()?;
            if delta_time == -12345 {
                continue; // -12345 is reserved for the rng seed of the replay
            }
            current_time += delta_time;
            let time = current_time as f32 / self.fps / speed;

            let keys = vec_params[3].parse::<i32>()?;

            // bit 1 = M1 in standard, left kan in taiko, k1 in mania
            // bit 2 = M2 in standard, left kat in taiko, k2 in mania
            let p1_down = keys & (1 << 0) != 0;
            let p2_down = keys & (1 << 1) != 0;
            self.process_action_p1(time, p1_down, (time * self.fps) as _);
            self.process_action_p2(time, p2_down, (time * self.fps) as _);
            self.extended_p1(p1_down, (time * self.fps) as u32, 0., 0., 0., 0.);
            self.extended_p2(p2_down, (time * self.fps) as u32, 0., 0., 0., 0.);
        }

        Ok(())
    }

    // https://github.com/maxnut/GDMegaOverlay/blob/3bc9c191e3fcdde838b0f69f8411af782afa3ba7/src/Replay.cpp#L124-L140
    fn parse_gdmo(&mut self, data: &[u8]) -> Result<()> {
        use std::mem::size_of;

        let mut cursor = Cursor::new(data);
        self.fps = cursor.read_f32::<LittleEndian>()?;

        let num_actions = cursor.read_u32::<LittleEndian>()?;
        let _num_frame_captures = cursor.read_u32::<LittleEndian>()?;

        #[repr(C)]
        struct GdmoAction {
            press: bool,
            player2: bool,
            frame: u32,
            y_accel: f64,
            px: f32,
            py: f32,
        }

        for _ in 0..num_actions {
            let mut buf = [0; size_of::<GdmoAction>()];
            cursor.read_exact(&mut buf)?;
            let action: GdmoAction = unsafe { std::mem::transmute(buf) };

            let time = action.frame as f32 / self.fps;
            if action.player2 {
                self.process_action_p2(time, action.press, action.frame);
                self.extended_p2(
                    action.press,
                    action.frame,
                    action.px,
                    action.py,
                    action.y_accel as f32,
                    0.,
                );
            } else {
                self.process_action_p1(time, action.press, action.frame);
                self.extended_p1(
                    action.press,
                    action.frame,
                    action.px,
                    action.py,
                    action.y_accel as f32,
                    0.,
                );
            }
        }

        Ok(())
    }

    fn parse_replaybot(&mut self, data: &[u8]) -> Result<()> {
        const REPLAYBOT_MAGIC: &[u8; 4] = b"RPLY";

        let mut cursor = Cursor::new(data);
        let mut magic = [0u8; 4];
        cursor.read_exact(&mut magic)?;

        // check if its a version 2 frame macro
        if magic != *REPLAYBOT_MAGIC {
            anyhow::bail!(
                "old replaybot macro format is not supported, as it does not store frames"
            )
        }
        let version = cursor.read_u8()?;
        if version != 2 {
            anyhow::bail!("unsupported replaybot version {version} (only v2 is supported, because v1 doesn't store frames)")
        }
        if cursor.read_u8()? != 1 {
            anyhow::bail!("only frame replays are supported")
        }

        self.fps = cursor.read_f32::<LittleEndian>()?;
        cursor.set_position(cursor.position() + 4); // skip 4 bytes
        for _ in (10..data.len()).step_by(5) {
            let frame = cursor.read_u32::<LittleEndian>()?;
            let time = frame as f32 / self.fps;
            let state = cursor.read_u8()?;
            let down = state & 0x1 != 0;
            let player2 = state >> 1 != 0;

            if player2 {
                self.process_action_p2(time, down, frame);
                self.extended_p2(down, frame, 0., 0., 0., 0.);
            } else {
                self.process_action_p1(time, down, frame);
                self.extended_p1(down, frame, 0., 0., 0., 0.);
            }
        }

        Ok(())
    }

    fn parse_rush(&mut self, data: &[u8]) -> Result<()> {
        let mut cursor = Cursor::new(data);
        self.fps = cursor.read_i16::<LittleEndian>()? as f32;

        for _ in (2..data.len()).step_by(5) {
            let frame = cursor.read_i32::<LittleEndian>()?;
            let time = frame as f32 / self.fps;
            let state = cursor.read_u8()?;
            let down = (state & 1) != 0;
            let p2 = (state >> 1) != 0;

            if p2 {
                self.process_action_p2(time, down, frame as _);
                self.extended_p2(down, frame as u32, 0., 0., 0., 0.);
            } else {
                self.process_action_p1(time, down, frame as _);
                self.extended_p1(down, frame as u32, 0., 0., 0., 0.);
            }
        }

        Ok(())
    }

    fn parse_kdbot(&mut self, data: &[u8]) -> Result<()> {
        let mut cursor = Cursor::new(data);
        self.fps = cursor.read_f32::<LittleEndian>()?;

        for _ in (4..data.len()).step_by(6) {
            let frame = cursor.read_i32::<LittleEndian>()?;
            let time = frame as f32 / self.fps;
            let down = cursor.read_u8()? == 1;
            let p2 = cursor.read_u8()? == 1;

            if p2 {
                self.process_action_p2(time, down, frame as _);
                self.extended_p2(down, frame as u32, 0., 0., 0., 0.);
            } else {
                self.process_action_p1(time, down, frame as _);
                self.extended_p1(down, frame as u32, 0., 0., 0., 0.);
            }
        }

        Ok(())
    }

    fn parse_txt(&mut self, data: &[u8]) -> Result<()> {
        let lines = String::from_utf8(data.to_vec())?;
        let mut lines = lines.split('\n');
        self.fps = lines.next().context("failed to get fps")?.parse()?;

        for line in lines {
            let mut split = line.split(' ');
            if split.clone().count() != 3 {
                continue;
            }
            let frame_or_xpos: f32 = split.next().unwrap().parse()?;
            let time = frame_or_xpos / self.fps;
            let down = split.next().unwrap().parse::<u8>()? == 1;
            let p2 = split.next().unwrap().parse::<u8>()? == 1;

            if p2 {
                self.process_action_p2(time, down, frame_or_xpos as _);
                self.extended_p2(down, frame_or_xpos as u32, 0., 0., 0., 0.);
            } else {
                self.process_action_p1(time, down, frame_or_xpos as _);
                self.extended_p1(down, frame_or_xpos as u32, 0., 0., 0., 0.);
            }
        }

        Ok(())
    }

    /// Will also handle obot2 replays.
    fn parse_obot3(&mut self, mut data: &[u8]) -> Result<()> {
        let mut deserializer = dlhn::Deserializer::new(&mut data);
        let Ok(replay) = Obot3Replay::deserialize(&mut deserializer) else {
            return self.parse_obot2(data);
        };

        self.fps = replay.initial_fps;
        let mut current_fps = self.fps;

        for action in replay.clicks {
            let time = action.frame as f32 / current_fps;
            match action.click_type {
                Obot3ClickType::Player1Down => {
                    self.process_action_p1(time, true, action.frame);
                    self.extended_p1(true, action.frame, 0., 0., 0., 0.);
                }
                Obot3ClickType::Player1Up => {
                    self.process_action_p1(time, false, action.frame);
                    self.extended_p1(false, action.frame, 0., 0., 0., 0.);
                }
                Obot3ClickType::Player2Down => {
                    self.process_action_p2(time, true, action.frame);
                    self.extended_p2(true, action.frame, 0., 0., 0., 0.);
                }
                Obot3ClickType::Player2Up => {
                    self.process_action_p2(time, false, action.frame);
                    self.extended_p2(false, action.frame, 0., 0., 0., 0.);
                }
                Obot3ClickType::FpsChange(fps) => {
                    current_fps = fps;
                    self.fps_change(fps);
                }
                Obot3ClickType::None => {}
            }
        }

        Ok(())
    }

    fn parse_re(&mut self, data: &[u8]) -> Result<()> {
        use std::mem::size_of;
        let mut cursor = Cursor::new(data);

        self.fps = cursor.read_f32::<LittleEndian>()?;
        let num_actions = cursor.read_i32::<LittleEndian>()?;
        let num_actions2 = cursor.read_i32::<LittleEndian>()?;

        #[repr(C)]
        struct FrameData {
            frame: u32,
            x: f32,
            y: f32,
            rot: f32,
            y_accel: f64,
            player2: bool,
        }
        #[repr(C)]
        struct ActionData {
            frame: u32,
            hold: bool,
            player2: bool,
        }

        // read action data
        let prev_pos = cursor.position();
        cursor.set_position(num_actions as u64 * size_of::<FrameData>() as u64);
        let mut actions: Vec<ActionData> =
            Vec::with_capacity(num_actions2 as usize * size_of::<ActionData>());

        for _ in 0..num_actions2 {
            let mut buf = [0; size_of::<ActionData>()];
            cursor.read_exact(&mut buf)?;
            actions.push(unsafe { std::mem::transmute(buf) });
        }

        // read frame data
        cursor.set_position(prev_pos);
        for _ in 0..num_actions {
            let mut buf = [0; size_of::<FrameData>()];
            cursor.read_exact(&mut buf)?;
            let data: FrameData = unsafe { std::mem::transmute(buf) };

            // find action for this frame
            let action = actions
                .iter()
                .find(|a| a.frame == data.frame)
                .context(format!("failed to find action for frame {}", data.frame))?;

            let time = data.frame as f32 / self.fps;

            if data.player2 {
                self.process_action_p2(time, action.hold, data.frame);
                self.extended_p2(
                    action.hold,
                    data.frame,
                    data.x,
                    data.y,
                    data.y_accel as f32,
                    data.rot,
                );
            } else {
                self.process_action_p1(time, action.hold, data.frame);
                self.extended_p1(
                    action.hold,
                    data.frame,
                    data.x,
                    data.y,
                    data.y_accel as f32,
                    data.rot,
                );
            }
        }

        Ok(())
    }

    fn parse_ddhor(&mut self, data: &[u8]) -> Result<()> {
        const DDHOR_MAGIC: &[u8; 4] = b"DDHR";

        let mut cursor = Cursor::new(data);
        let mut magicbuf = [0; DDHOR_MAGIC.len()];
        cursor.read_exact(&mut magicbuf)?;

        if magicbuf != *DDHOR_MAGIC {
            anyhow::bail!(
                "ddhor json is not supported, as it doesn't store frames.\n\
                           try using an older ddhor version with frame mode"
            );
        }

        self.fps = cursor.read_i16::<LittleEndian>()? as f32;
        let num_p1 = cursor.read_i32::<LittleEndian>()?; // num p1 actions
        let _num_p2 = cursor.read_i32::<LittleEndian>()?; // num p2 actions

        for i in (14..data.len()).step_by(5) {
            let frame = cursor.read_f32::<LittleEndian>()?;
            let time = frame / self.fps;
            let down = cursor.read_u8()? == 0;
            let p2 = i - 14 >= num_p1 as usize * 5;

            if p2 {
                self.process_action_p2(time, down, frame as _);
                self.extended_p2(down, frame as u32, 0., 0., 0., 0.);
            } else {
                self.process_action_p1(time, down, frame as _);
                self.extended_p1(down, frame as u32, 0., 0., 0., 0.);
            }
        }

        Ok(())
    }

    fn parse_xbot(&mut self, data: &[u8]) -> Result<()> {
        let text = String::from_utf8(data.to_vec())?;
        let mut lines = text.split('\n');

        self.fps = lines
            .next()
            .context("first fps line doesn't exist, did you select an empty file?")?
            .trim()
            .parse::<u64>()? as f32;

        if lines.next().context("second line doesn't exist")?.trim() != "frames" {
            anyhow::bail!("the xBot parser only supports xBot Frame macros");
        }

        for (i, line) in lines.enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let i = i + 1;
            let mut splitted = line.trim().split(' ');
            let state: u8 = splitted
                .next()
                .context(format!("failed to get input state at line {i}"))?
                .parse()?;
            let frame: u32 = splitted
                .next()
                .context(format!("failed to get raw position at line {i}"))?
                .parse()?;

            // state:
            // 0 - release
            // 1 - down
            // 2 - p2 release
            // 3 - p2 down
            let player2 = state > 1;
            let down = state % 2 == 1;
            let time = frame as f32 / self.fps;

            if player2 {
                self.process_action_p2(time, down, frame);
                self.extended_p2(down, frame, 0., 0., 0., 0.);
            } else {
                self.process_action_p1(time, down, frame);
                self.extended_p1(down, frame, 0., 0., 0., 0.);
            }
        }

        Ok(())
    }

    /* gato
    fn parse_gatobot(&mut self, data: &[u8]) -> Result<()> {
        use base64::{engine::general_purpose, Engine as _};
        use flate2::read::GzDecoder;

        let text = String::from_utf8(data.to_vec())?;
        if !text.starts_with("H4sIAAAAAAAA") {
            anyhow::bail!("corrupted gatobot macro (must start with 'H4sIAAAAAAAA')");
        }

        let mut base64_decoded = general_purpose::URL_SAFE_NO_PAD.decode(text)?;

        // data is xored with key 11
        base64_decoded.iter_mut().for_each(|x| *x ^= 11);

        let mut decoder = GzDecoder::new(base64_decoded.as_slice());
        let mut decoded_str = String::new();
        decoder.read_to_string(&mut decoded_str)?;

        for action in decoded_str.split(';') {
            let mut splitted = action.split('_');
            let frame = splitted.next().context("no frame value")?;
            let data = splitted.next().context("no saved data")?;
            for (player, player_actions) in data.split('~').enumerate() {}
        }
        Ok(())
    }
    */
}
