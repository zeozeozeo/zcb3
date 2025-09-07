use crate::{f32_range, Timings, VolumeSettings};
use anyhow::{Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use ijson::IValue;
use indexmap::IndexMap;
use serde::Deserialize;
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Cursor, Read, Seek, SeekFrom},
};

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
        time: f64,
        timings: Timings,
        is_click: bool,
        vol: VolumeSettings,
    ) -> (Self, f32) {
        let rand_var = f32_range(-vol.volume_var..=vol.volume_var);
        let vol_offset =
            if vol.enabled && time < vol.spam_time && !(!vol.change_releases_volume && !is_click) {
                let offset = (vol.spam_time - time) as f32 * vol.spam_vol_offset_factor;
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

    /// Order of which clicks should be selected depending on the actual click type
    pub fn preferred(self) -> [Self; 8] {
        use ClickType::*;

        match self {
            HardClick => [
                HardClick,
                Click,
                SoftClick,
                MicroClick,
                HardRelease,
                Release,
                SoftRelease,
                MicroRelease,
            ],
            HardRelease => [
                HardRelease,
                Release,
                SoftRelease,
                MicroRelease,
                HardClick,
                Click,
                SoftClick,
                MicroClick,
            ],
            Click => [
                Click,
                HardClick,
                SoftClick,
                MicroClick,
                Release,
                HardRelease,
                SoftRelease,
                MicroRelease,
            ],
            Release => [
                Release,
                HardRelease,
                SoftRelease,
                MicroRelease,
                Click,
                HardClick,
                SoftClick,
                MicroClick,
            ],
            SoftClick => [
                SoftClick,
                MicroClick,
                Click,
                HardClick,
                SoftRelease,
                MicroRelease,
                Release,
                HardRelease,
            ],
            SoftRelease => [
                SoftRelease,
                MicroRelease,
                Release,
                HardRelease,
                SoftClick,
                MicroClick,
                Click,
                HardClick,
            ],
            MicroClick => [
                MicroClick,
                SoftClick,
                Click,
                HardClick,
                MicroRelease,
                SoftRelease,
                Release,
                HardRelease,
            ],
            MicroRelease => [
                MicroRelease,
                SoftRelease,
                Release,
                HardRelease,
                MicroClick,
                SoftClick,
                Click,
                HardClick,
            ],
            None => [None, None, None, None, None, None, None, None],
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

#[derive(Clone, Copy, Debug)]
pub enum Click {
    /// Regular player click.
    Regular(ClickType),
    /// Platformer left click.
    Left(ClickType),
    /// Platformer right click.
    Right(ClickType),
}

impl Default for Click {
    fn default() -> Self {
        Self::Regular(ClickType::None)
    }
}

impl Click {
    pub const fn click_type(self) -> ClickType {
        match self {
            Click::Regular(typ) | Click::Left(typ) | Click::Right(typ) => typ,
        }
    }

    pub const fn is_click(self) -> bool {
        self.click_type().is_click()
    }

    pub const fn is_release(self) -> bool {
        self.click_type().is_release()
    }

    const fn from_button_and_typ(button: Button, typ: ClickType) -> Self {
        match button {
            Button::Push | Button::Release => Self::Regular(typ),
            Button::LeftPush | Button::LeftRelease => Self::Left(typ),
            Button::RightPush | Button::RightRelease => Self::Right(typ),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Button {
    Push,
    Release,
    LeftPush,
    LeftRelease,
    RightPush,
    RightRelease,
}

impl Button {
    #[inline]
    const fn from_down(down: bool) -> Self {
        if down {
            Self::Push
        } else {
            Self::Release
        }
    }

    #[inline]
    const fn from_left_down(down: bool) -> Self {
        if down {
            Self::LeftPush
        } else {
            Self::LeftRelease
        }
    }

    #[inline]
    const fn from_right_down(down: bool) -> Self {
        if down {
            Self::RightPush
        } else {
            Self::RightRelease
        }
    }

    #[inline]
    const fn from_button_idx(idx: i32, down: bool) -> Self {
        match idx {
            3 => Self::from_right_down(down),
            2 => Self::from_left_down(down),
            _ => Self::from_down(down),
        }
    }

    #[inline]
    const fn is_down(self) -> bool {
        matches!(self, Self::Push | Self::LeftPush | Self::RightPush)
    }

    // const fn is_left(self) -> bool {
    //     matches!(self, Self::LeftPush | Self::LeftRelease)
    // }
    //
    // const fn is_right(self) -> bool {
    //     matches!(self, Self::RightPush | Self::RightRelease)
    // }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Action {
    /// Time since the replay was started (in seconds).
    pub time: f64,
    /// What player this action is for.
    pub player: Player,
    /// Click type for this player.
    pub click: Click,
    /// Volume offset of the action.
    pub vol_offset: f32,
    /// Frame.
    pub frame: u32,
}

impl Action {
    pub const fn new(time: f64, player: Player, click: Click, vol_offset: f32, frame: u32) -> Self {
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
    pub fps_change: Option<f64>,
}

#[derive(Clone, Debug, Default)]
pub struct Replay {
    /// Framerate of the replay.
    pub fps: f64,
    /// Duration of the replay (in seconds).
    pub duration: f64,
    /// Actions used for generating clicks.
    pub actions: Vec<Action>,
    /// Whether to populate the `extended` vector.
    pub extended_data: bool,
    /// Action data used for converting replays.
    pub extended: Vec<ExtendedAction>,

    // used for determining the click type
    prev_action: (Option<ClickType>, Option<ClickType>),
    prev_time: (f64, f64),

    // used for generating additional click info
    timings: Timings,
    vol_settings: VolumeSettings,

    /// Whether to sort actions.
    sort_actions: bool,
    pub override_fps: Option<f64>,
    discard_deaths: bool,
    swap_players: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum ReplayType {
    /// .mhr.json files
    Mhr,
    /// .json files
    TasBot,
    /// .zbf files
    Zbot,
    /// OmegaBot 3 and OmegaBot 2 .replay files
    Obot,
    /// yBot frame files (no extension)
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
    /// ReplayBot .replay files
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
    /// yBot 2 .ybot files
    Ybot2,
    /// xdBot .xd files
    XdBot,
    /// GDReplayFormat .gdr files (GDMegaOverlay)
    Gdr,
    /// qBot .qb files
    Qbot,
    /// RBot .rbot files
    Rbot,
    /// Zephyrus (OpenHack) .zr files
    Zephyrus,
    /// ReplayEngine 2 .re2 files
    ReplayEngine2,
    /// ReplayEngine 3 .re3 files
    ReplayEngine3,
    /// Silicate .slc files
    Silicate,
    /// Silicate .slc2 files
    Silicate2,
    /// GDReplayFormat 2 .gdr2 files
    Gdr2,
    /// uvBot .uv files
    UvBot,
    // TCBot .tcm files
    TcBot,
}

impl ReplayType {
    pub fn guess_format(filename: &str) -> Result<Self> {
        use ReplayType::*;
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
                } else if filename.ends_with(".gdr.json") {
                    Gdr
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
            "ybot" => Ybot2,
            "xd" => XdBot,
            "gdr" => Gdr,
            "qb" => Qbot,
            "rbot" => Rbot,
            "zr" => Zephyrus,
            "re2" => ReplayEngine2,
            "re3" => ReplayEngine3,
            "slc" => Silicate,
            "slc2" => Silicate2,
            "gdr2" => Gdr2,
            "uv" => UvBot,
            "tcm" => TcBot,
            _ => anyhow::bail!("unknown replay format"),
        })
    }
}

// /// Reads a type `T` as raw bytes from the reader.
// macro_rules! read_t {
//     ($t:ty, $reader:ident) => {{
//         let mut buf = [0u8; ::std::mem::size_of::<$t>()];
//         $reader.read_exact(&mut buf)?;
//         unsafe { ::std::mem::transmute(buf) }
//     }};
// }

impl Replay {
    pub const SUPPORTED_EXTENSIONS: &'static [&'static str] = &[
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
        "ybot",
        // "gatobot",
        "xd",
        "gdr",
        "qb",
        "rbot",
        "zr",
        "re2",
        "re3",
        "slc",
        "slc2",
        "gdr2",
        "uv",
        "tcm",
    ];

    pub fn build() -> Self {
        Self::default()
    }

    pub fn with_timings(mut self, timings: Timings) -> Self {
        self.timings = timings;
        self
    }

    pub fn with_override_fps(mut self, override_fps: Option<f64>) -> Self {
        self.override_fps = override_fps;
        self
    }

    pub fn with_vol_settings(mut self, vol_settings: VolumeSettings) -> Self {
        self.vol_settings = vol_settings;
        self
    }

    pub fn with_extended(mut self, extended: bool) -> Self {
        self.extended_data = extended;
        self
    }

    pub fn with_sort_actions(mut self, sort_actions: bool) -> Self {
        self.sort_actions = sort_actions;
        self
    }

    pub fn with_discard_deaths(mut self, discard_deaths: bool) -> Self {
        self.discard_deaths = discard_deaths;
        self
    }

    pub fn with_swap_players(mut self, swap_players: bool) -> Self {
        self.swap_players = swap_players;
        self
    }

    #[inline]
    pub fn has_actions(&self) -> bool {
        !self.actions.is_empty()
    }

    pub fn parse<R: Read + Seek>(mut self, typ: ReplayType, reader: R) -> Result<Self> {
        log::info!("parsing replay, replay type {typ:?}");

        match typ {
            ReplayType::Mhr => self.parse_mhr(reader)?,
            ReplayType::TasBot => self.parse_tasbot(reader)?,
            ReplayType::Zbot => self.parse_zbf(reader)?,
            ReplayType::Obot => self.parse_obot2(reader)?, // will also handle obot3 and replaybot replays
            ReplayType::Ybotf => self.parse_ybotf(reader)?,
            ReplayType::MhrBin => self.parse_mhrbin(reader)?,
            ReplayType::Echo => self.parse_echo(reader)?, // will handle all 3 replay versions
            ReplayType::Amethyst => self.parse_amethyst(reader)?,
            ReplayType::OsuReplay => self.parse_osr(reader)?,
            ReplayType::Gdmo => self.parse_gdmo(reader)?,
            ReplayType::ReplayBot => self.parse_replaybot(reader)?,
            ReplayType::Rush => self.parse_rush(reader)?,
            ReplayType::Kdbot => self.parse_kdbot(reader)?,
            ReplayType::Txt => self.parse_plaintext(reader)?,
            ReplayType::ReplayEngine => self.parse_re(reader)?,
            ReplayType::Ddhor => self.parse_ddhor(reader)?,
            ReplayType::Xbot => self.parse_xbot(reader)?,
            ReplayType::Ybot2 => self.parse_ybot2(reader)?,
            ReplayType::XdBot => self.parse_xdbot(reader)?,
            ReplayType::Gdr => self.parse_gdr(reader)?,
            ReplayType::Qbot => self.parse_qbot(reader)?,
            ReplayType::Rbot => self.parse_rbot(reader)?,
            ReplayType::Zephyrus => self.parse_zephyrus(reader)?,
            ReplayType::ReplayEngine2 => self.parse_re2(reader)?,
            ReplayType::ReplayEngine3 => self.parse_re3(reader)?,
            ReplayType::Gdr2 => self.parse_gdr2(reader)?,
            ReplayType::Silicate => self.parse_slc(reader)?,
            ReplayType::Silicate2 => self.parse_slc2(reader)?,
            // MacroType::GatoBot => self.parse_gatobot(reader)?,
            ReplayType::UvBot => self.parse_uvbot(reader)?,
            ReplayType::TcBot => self.parse_tcm(reader)?,
        }

        // sort actions by time / frame
        if self.sort_actions {
            self.sort_actions();
        }

        if let Some(last) = self.actions.last() {
            self.duration = last.time;
        }

        log::debug!(
            "replay fps: {}; replay duration: {:?}s",
            self.fps,
            self.duration
        );

        Ok(self)
    }

    /// Sorts actions by time / frame.
    pub fn sort_actions(&mut self) -> &mut Self {
        self.actions.sort_by(|a, b| a.time.total_cmp(&b.time));
        self.extended.sort_by(|a, b| a.frame.cmp(&b.frame));
        self
    }

    fn process_action_p1(&mut self, time: f64, button: Button, frame: u32) {
        let down = button.is_down();
        if !down && self.actions.is_empty() {
            return;
        }
        // if action is the same, skip it
        if let Some(typ) = self.prev_action.0 {
            if down == typ.is_click() {
                return;
            }
        }

        let delta = time - self.prev_time.0;
        let (typ, vol_offset) = ClickType::from_time(delta, self.timings, down, self.vol_settings);
        // println!("ctyp: {typ:?}");

        self.prev_time.0 = time;
        self.prev_action.0 = Some(typ);
        self.actions.push(Action::new(
            time,
            if self.swap_players {
                Player::Two
            } else {
                Player::One
            },
            Click::from_button_and_typ(button, typ),
            vol_offset,
            frame,
        ))
    }

    // .0 is changed to .1 here, because it's the second player
    fn process_action_p2(&mut self, time: f64, button: Button, frame: u32) {
        let down = button.is_down();
        if !down && self.actions.is_empty() {
            return;
        }
        if let Some(typ) = self.prev_action.1 {
            if down == typ.is_click() {
                return;
            }
        }

        let delta = time - self.prev_time.1;
        let (typ, vol_offset) = ClickType::from_time(delta, self.timings, down, self.vol_settings);

        self.prev_time.1 = time;
        self.prev_action.1 = Some(typ);
        self.actions.push(Action::new(
            time,
            if self.swap_players {
                Player::One
            } else {
                Player::Two
            },
            Click::from_button_and_typ(button, typ),
            vol_offset,
            frame,
        ))
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
            // FIXME: we probably shouldn't do this for converting replays
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

    pub fn filter_actions<F>(&self, player: Player, func: F)
    where
        F: FnMut(&ExtendedAction),
    {
        self.extended
            .iter()
            .filter(|a| {
                (a.player2 && player == Player::Two) || (!a.player2 && player == Player::One)
            })
            .for_each(func)
    }

    /// Returns the last frame in the replay. If extended actions are disabled, this
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
    fn fps_change(&mut self, fps_change: f64) {
        if let Some(last) = self.extended.last_mut() {
            last.fps_change = Some(fps_change);
        }
    }

    fn get_fps(&self, actual: f64) -> f64 {
        if let Some(override_fps) = self.override_fps {
            override_fps
        } else {
            actual
        }
    }

    fn parse_ybotf<R: Read>(&mut self, mut reader: R) -> Result<()> {
        self.fps = self.get_fps(reader.read_f32::<LittleEndian>()? as f64);
        let num_actions = reader.read_i32::<LittleEndian>()?;

        for _ in (12..12 + num_actions * 8).step_by(8) {
            let frame = reader.read_u32::<LittleEndian>()?;
            let state = reader.read_u32::<LittleEndian>()?;
            let down = (state & 0b10) == 2;
            let p2 = (state & 0b01) == 1;
            let time = frame as f64 / self.fps;

            if p2 {
                self.process_action_p2(time, Button::from_down(down), frame);
                self.extended_p2(down, frame, 0., 0., 0., 0.);
            } else {
                self.process_action_p1(time, Button::from_down(down), frame);
                self.extended_p1(down, frame, 0., 0., 0., 0.);
            }
        }

        Ok(())
    }

    /// Will also handle obot3 and replaybot replays.
    fn parse_obot2<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        #[derive(Deserialize)]
        pub enum Obot2Location {
            XPos,
            Frame(u32),
        }
        #[derive(Deserialize, PartialEq)]
        enum Obot2ReplayType {
            XPos,
            Frame,
        }
        #[derive(Deserialize, PartialEq, Clone, Copy)]
        enum Obot2ClickType {
            None,
            FpsChange(f32),
            Player1Down,
            Player1Up,
            Player2Down,
            Player2Up,
        }
        #[derive(Deserialize)]
        struct Obot2Click {
            location: Obot2Location,
            click_type: Obot2ClickType,
        }
        #[derive(Deserialize)]
        struct Obot2Replay {
            initial_fps: f32,
            _current_fps: f32,
            replay_type: Obot2ReplayType,
            _current_click: usize,
            clicks: Vec<Obot2Click>,
        }
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        reader.seek(SeekFrom::Start(0))?;

        // check if its a replaybot macro
        if &data[..4] == b"RPLY" {
            return self.parse_replaybot(reader);
        }
        // check if its a obot3 macro
        let Ok(decoded) = bincode::deserialize::<Obot2Replay>(&data) else {
            return self.parse_obot3(reader);
        };

        if decoded.replay_type == Obot2ReplayType::XPos {
            log::error!("xpos replays not supported, because they doesn't store frames");
            anyhow::bail!("xpos replays not supported, because they doesn't store frames")
        };

        self.fps = self.get_fps(decoded.initial_fps as f64);

        for action in decoded.clicks {
            let frame = match action.location {
                Obot2Location::Frame(frame) => frame,
                _ => {
                    log::warn!("got xpos action while replay type is frame, skipping");
                    continue;
                }
            };
            let time = frame as f64 / self.fps;
            match action.click_type {
                Obot2ClickType::Player1Down => {
                    self.process_action_p1(time, Button::from_down(true), frame);
                    self.extended_p1(true, frame, 0., 0., 0., 0.);
                }
                Obot2ClickType::Player1Up => {
                    self.process_action_p1(time, Button::from_down(false), frame);
                    self.extended_p1(false, frame, 0., 0., 0., 0.);
                }
                Obot2ClickType::Player2Down => {
                    self.process_action_p2(time, Button::from_down(true), frame);
                    self.extended_p2(true, frame, 0., 0., 0., 0.);
                }
                Obot2ClickType::Player2Up => {
                    self.process_action_p2(time, Button::from_down(false), frame);
                    self.extended_p2(false, frame, 0., 0., 0., 0.);
                }
                Obot2ClickType::FpsChange(fps) => {
                    self.fps = self.get_fps(fps as _);
                    self.fps_change(fps as _);
                }
                Obot2ClickType::None => {}
            }
        }

        Ok(())
    }

    fn parse_zbf<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        let len = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;

        let delta = reader.read_f32::<LittleEndian>()?;
        let mut speedhack = reader.read_f32::<LittleEndian>()?;
        if speedhack == 0.0 {
            log::error!("zbf speedhack is 0.0, defaulting to 1.0");
            speedhack = 1.0; // reset to 1 so we don't get Infinity as fps
        }
        self.fps = self.get_fps(1.0 / delta as f64 / speedhack as f64);

        for _ in (8..len).step_by(6).enumerate() {
            let frame = reader.read_i32::<LittleEndian>()?;
            let down = reader.read_u8()? == 0x31;
            let p1 = reader.read_u8()? == 0x31;
            let time = frame as f64 / self.fps;

            if p1 {
                self.process_action_p1(time, Button::from_down(down), frame as _);
                self.extended_p1(down, frame as u32, 0., 0., 0., 0.);
            } else {
                self.process_action_p2(time, Button::from_down(down), frame as _);
                self.extended_p2(down, frame as u32, 0., 0., 0., 0.);
            }
        }

        Ok(())
    }

    /// Also handles MHR json replays.
    fn parse_tasbot<R: Read + Seek>(&mut self, reader: R) -> Result<()> {
        let v: IValue = serde_json::from_reader(reader)?;

        // check if it's a mhr replay, because maybe someone renamed .mhr.json
        // to .json by accident
        if let Some(meta) = v.get("meta") {
            if let Some(fps) = meta.get("fps") {
                if fps.to_f64().is_some() {
                    return self.parse_mhr_from_ivalue(v);
                }
            }
        }

        self.fps = self.get_fps(
            v.get("fps")
                .context("couldn't get 'fps' field")?
                .to_f64()
                .context("couldn't convert 'fps' field to float")?,
        );
        let events = v
            .get("macro")
            .context("couldn't get 'macro' field")?
            .as_array()
            .context("'macro' is not an array")?;

        let mut prev_action = (0, 0);

        for ev in events {
            let frame = ev
                .get("frame")
                .context("couldn't get 'frame' field")?
                .to_u64()
                .context("'frame' is not a number")?;
            let time = frame as f64 / self.fps;

            let p1 = ev
                .get("player_1")
                .context("couldn't get 'player_1' field")?
                .get("click")
                .context("couldn't get p1 'click' field")?
                .to_i64()
                .context("p1 'click' field is not a number")?;
            let p2 = ev
                .get("player_2")
                .context("couldn't get 'player_2' field")?
                .get("click")
                .context("couldn't get p2 'click' field")?
                .to_i64()
                .context("p2 'click' field is not a number")?;
            let x = ev
                .get("player_1")
                .map(|v| {
                    v.get("x_position")
                        .map(|v| v.to_f64().unwrap_or(0.) as f32)
                        .unwrap_or(0.0)
                })
                .unwrap_or(0.0);

            // 0 = nothing, 1 = click, 2 = release
            if p1 != 0 {
                if p1 == 1 && prev_action.0 == 1 {
                    // if the previous frame also was a click, this actually means there
                    // was a release before this, if i understand this correctly
                    self.process_action_p1(time, Button::Release, frame as _);
                    self.extended_p1(false, frame as u32, x, 0., 0., 0.);
                }
                self.process_action_p1(time, Button::from_down(p1 == 1), frame as _);
                self.extended_p1(p1 == 1, frame as u32, x, 0., 0., 0.);
            }
            if p2 != 0 {
                if p2 == 1 && prev_action.1 == 1 {
                    // same thing for p2
                    self.process_action_p2(time, Button::Release, frame as _);
                    self.extended_p2(false, frame as u32, x, 0., 0., 0.);
                }
                self.process_action_p2(time, Button::from_down(p2 == 1), frame as _);
                self.extended_p2(p2 == 1, frame as u32, x, 0., 0., 0.);
            }

            prev_action = (p1, p2);
        }

        Ok(())
    }

    fn parse_mhr_from_ivalue(&mut self, v: IValue) -> Result<()> {
        self.fps = self.get_fps(
            v.get("meta")
                .context("failed to get 'meta' field")?
                .get("fps")
                .context("failed to get 'fps' field")?
                .to_f64()
                .context("'fps' field is not a float")?,
        );

        let events = v
            .get("events")
            .context("failed to get 'events' field")?
            .as_array()
            .context("'events' field is not an array")?;

        for ev in events {
            let frame = ev
                .get("frame")
                .context("failed to get 'frame' field")?
                .to_u64()
                .context("'frame' field is not a number")?;
            let time = frame as f64 / self.fps;

            let down = if let Some(down) = ev.get("down") {
                down.to_bool().context("'down' field is not a bool")?
            } else {
                continue;
            };

            // 'p2' always seems to be true if it exists, but we'll still query the value just to be safe
            let p2 = if let Some(p2) = ev.get("p2") {
                p2.to_bool().context("couldn't get 'p2' field")?
            } else {
                false
            };

            let y_accel = ev
                .get("a")
                .map(|v| v.to_f32().unwrap_or(0.0))
                .unwrap_or(0.0);
            let x = ev
                .get("x")
                .map(|v| v.to_f32().unwrap_or(0.0))
                .unwrap_or(0.0);
            let y = ev
                .get("y")
                .map(|v| v.to_f32().unwrap_or(0.0))
                .unwrap_or(0.0);
            let rot = ev
                .get("r")
                .map(|v| v.to_f32().unwrap_or(0.0))
                .unwrap_or(0.0);

            if p2 {
                self.process_action_p2(time, Button::from_down(down), frame as _);
                self.extended_p2(down, frame as u32, x, y, y_accel, rot)
            } else {
                self.process_action_p1(time, Button::from_down(down), frame as _);
                self.extended_p1(down, frame as u32, x, y, y_accel, rot)
            }
        }

        Ok(())
    }

    fn parse_mhr<R: Read + Seek>(&mut self, reader: R) -> Result<()> {
        let v: serde_json::Result<IValue> = serde_json::from_reader(reader);
        self.parse_mhr_from_ivalue(v?)
    }

    fn parse_mhrbin<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        reader.seek(SeekFrom::Start(0))?;

        // if it's a json replay, load from json instead
        if serde_json::from_slice::<IValue>(&data).is_ok() {
            return self.parse_mhr(reader);
        }

        use byteorder::BigEndian;

        let magic = reader.read_u32::<BigEndian>()?;
        if magic != 0x4841434B {
            // HACK
            log::error!("invalid mhrbin magic: {}", magic);
            anyhow::bail!("unknown mhrbin magic: {}", magic)
        }

        reader.seek(SeekFrom::Start(12))?;
        self.fps = self.get_fps(reader.read_u32::<LittleEndian>()? as f64);
        log::debug!("fps: {}", self.fps);
        reader.seek(SeekFrom::Start(28))?;
        let num_actions = reader.read_u32::<LittleEndian>()?;
        log::debug!("num_actions: {}", num_actions);

        for _ in 0..num_actions {
            reader.seek(SeekFrom::Current(2))?;
            let down = reader.read_u8()? == 1;
            let p1 = reader.read_u8()? == 0;
            let frame = reader.read_u32::<LittleEndian>()?;
            let time = frame as f64 / self.fps;
            // skip 24 bytes
            reader.seek(SeekFrom::Current(24))?;

            if p1 {
                self.process_action_p1(time, Button::from_down(down), frame);
                self.extended_p1(down, frame, 0., 0., 0., 0.); // TODO: parse all vars
            } else {
                self.process_action_p2(time, Button::from_down(down), frame);
                self.extended_p2(down, frame, 0., 0., 0., 0.); // TODO: parse all vars
            }
        }

        Ok(())
    }

    /// Parses the new Echo replay format.
    fn parse_echobin<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        use byteorder::BigEndian;

        let len = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;

        let magic = reader.read_u32::<BigEndian>()?;
        if magic != 0x4D455441 {
            log::error!("invalid echobin magic: {}", magic);
            anyhow::bail!("unknown echobin magic: {}", magic)
        }

        let replay_type = reader.read_u32::<BigEndian>()?;
        let action_size = if replay_type == 0x44424700 { 24 } else { 6 };
        let dbg = action_size == 24;
        reader.seek(SeekFrom::Start(24))?;
        self.fps = self.get_fps(reader.read_f32::<LittleEndian>()? as f64);
        reader.seek(SeekFrom::Start(48))?;

        for _ in (48..len).step_by(action_size) {
            let frame = reader.read_u32::<LittleEndian>()?;
            let down = reader.read_u8()? == 1;
            let p1 = reader.read_u8()? == 0;
            let time = frame as f64 / self.fps;

            // read extra vars (only saved in debug mode)
            let x = if dbg {
                reader.read_f32::<LittleEndian>()?
            } else {
                0.
            };
            let y_accel = if dbg {
                reader.read_f64::<LittleEndian>()?
            } else {
                0.
            };
            let _x_accel = if dbg {
                reader.read_f64::<LittleEndian>()?
            } else {
                0.
            };
            let y = if dbg {
                reader.read_f32::<LittleEndian>()?
            } else {
                0.
            };
            let rot = if dbg {
                reader.read_f32::<LittleEndian>()?
            } else {
                0.
            };

            if p1 {
                self.process_action_p1(time, Button::from_down(down), frame);
                self.extended_p1(down, frame, x, y, y_accel as _, rot);
            } else {
                self.process_action_p2(time, Button::from_down(down), frame);
                self.extended_p2(down, frame, x, y, y_accel as _, rot);
            }
        }

        Ok(())
    }

    /// Parses the old Echo json format.
    fn parse_echo_old(&mut self, v: IValue) -> Result<()> {
        self.fps = self.get_fps(
            v.get("FPS")
                .context("couldn't get 'FPS' field")?
                .to_f64()
                .context("'FPS' field is not a float")?,
        );
        let starting_frame = v
            .get("Starting Frame")
            .map(|v| v.to_u64().unwrap_or(0))
            .unwrap_or(0);

        for action in v
            .get("Echo Replay")
            .context("failed to get 'Echo Replay' field")?
            .as_array()
            .context("'Echo Replay' field is not an array")?
        {
            let frame = action
                .get("Frame")
                .context("couldn't get 'Frame' field")?
                .to_u64()
                .context("'Frame' field is not a number")?
                + starting_frame;
            let time = frame as f64 / self.fps;
            let p2 = action
                .get("Player 2")
                .context("couldn't get 'Player 2' field")?
                .to_bool()
                .context("'Player 2' field is not a bool")?;
            let down = action
                .get("Hold")
                .context("couldn't get 'Hold' field")?
                .to_bool()
                .context("'Hold' field is not a bool")?;

            let x = action
                .get("X Position")
                .map(|v| v.to_f32().unwrap_or(0.0))
                .unwrap_or(0.0);
            let y = action
                .get("Y Position")
                .map(|v| v.to_f32().unwrap_or(0.0))
                .unwrap_or(0.0);
            let y_accel = action
                .get("Y Acceleration")
                .map(|v| v.to_f32().unwrap_or(0.0))
                .unwrap_or(0.0);
            let rot = action
                .get("Rotation")
                .map(|v| v.to_f32().unwrap_or(0.0))
                .unwrap_or(0.0);

            if p2 {
                self.process_action_p2(time, Button::from_down(down), frame as _);
                self.extended_p2(down, frame as u32, x, y, y_accel, rot);
            } else {
                self.process_action_p1(time, Button::from_down(down), frame as _);
                self.extended_p1(down, frame as u32, x, y, y_accel, rot);
            }
        }
        Ok(())
    }

    /// Parses .echo files (both old json and new binary formats).
    fn parse_echo<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        reader.seek(SeekFrom::Start(0))?;

        let Ok(v) = serde_json::from_slice::<IValue>(&data) else {
            return self.parse_echobin(reader); // can't parse json, parse binary
        };

        // try parsing old json format
        if self.parse_echo_old(v.clone()).is_ok() {
            return Ok(());
        } else {
            self.actions.clear();
            self.extended.clear();
        }

        // parse new json format
        self.fps = self.get_fps(
            v.get("fps")
                .context("no 'fps' field")?
                .to_f64()
                .context("'fps' field is not a float")?,
        );
        for action in v
            .get("inputs")
            .context("no 'inputs' field")?
            .as_array()
            .context("'inputs' field is not an array")?
        {
            let frame = action
                .get("frame")
                .context("no 'frame' field")?
                .to_u64()
                .context("'frame' field is not a number")?;
            let time = frame as f64 / self.fps;
            let down = action
                .get("holding")
                .context("no 'holding' field")?
                .to_bool()
                .context("'holding' field is not a bool")?;
            let p2 = if let Some(p2) = action.get("player_2") {
                p2.to_bool().unwrap_or(false)
            } else {
                false
            };
            let x = action
                .get("x_position")
                .map(|v| v.to_f64().unwrap_or(0.0))
                .unwrap_or(0.0);
            let y_accel = action
                .get("y_vel")
                .map(|v| v.to_f64().unwrap_or(0.0))
                .unwrap_or(0.0);
            // let _x_accel = action.get("x_vel").context("no 'x_vel' field").to_f64().unwrap_or(0.);
            let rot = action
                .get("rotation")
                .map(|v| v.to_f64().unwrap_or(0.0))
                .unwrap_or(0.0);

            if p2 {
                self.process_action_p2(time, Button::from_down(down), frame as _);
                self.extended_p2(down, frame as _, x as _, 0., y_accel as _, rot as _);
            } else {
                self.process_action_p1(time, Button::from_down(down), frame as _);
                self.extended_p1(down, frame as _, x as _, 0., y_accel as _, rot as _);
            }
        }

        Ok(())
    }

    // Amethyst stores replays like this:
    //
    // ```
    // /* for player1 clicks */
    // {num actions}
    // {action time}...
    // /* for player1 releases */
    // {num actions}
    // {action time}...
    // /* for player2 clicks */
    // {num actions}
    // {action time}...
    // /* for player2 releases */
    // {num actions}
    // {action time}...
    // ```
    fn parse_amethyst<R: Read>(&mut self, mut reader: R) -> Result<()> {
        let mut data = String::new();
        reader.read_to_string(&mut data)?;
        let mut lines = data.split('\n');

        let mut get_times = |player, is_clicks| -> Result<Vec<(Player, bool, f64)>> {
            let num: usize = lines.next().context("unexpected EOF")?.parse()?;
            let mut times: Vec<(Player, bool, f64)> = Vec::with_capacity(num);
            for _ in 0..num {
                let time = lines.next().context("unexpected EOF")?.parse()?;
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
                self.process_action_p1(
                    action.2,
                    Button::from_down(action.1),
                    (action.2 * self.fps) as _,
                );
                self.extended_p1(action.1, (action.2 * self.fps) as u32, 0., 0., 0., 0.);
            } else {
                self.process_action_p2(
                    action.2,
                    Button::from_down(action.1),
                    (action.2 * self.fps) as _,
                );
                self.extended_p2(action.1, (action.2 * self.fps) as u32, 0., 0., 0., 0.);
            }
        }

        Ok(())
    }

    // https://osu.ppy.sh/wiki/en/Client/File_formats/osr_%28file_format%29
    fn parse_osr<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        let len = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;
        let mut data = vec![0; len as usize];
        reader.read_exact(&mut data)?;
        let mut cursor = Cursor::new(&data);

        self.fps = self.get_fps(1000.0);

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
        let speed = if mods & (1 << 6) != 0 {
            // dt
            1.5
        } else if mods & (1 << 8) != 0 {
            // ht
            0.75
        } else {
            // nm
            1.0
        };

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
            let time = current_time as f64 / self.fps / speed;

            let keys = vec_params[3].parse::<i32>()?;

            // bit 1 = M1 in standard, left kan in taiko, k1 in mania
            // bit 2 = M2 in standard, left kat in taiko, k2 in mania
            let p1_down = keys & (1 << 0) != 0;
            let p2_down = keys & (1 << 1) != 0;
            self.process_action_p1(time, Button::from_down(p1_down), (time * self.fps) as _);
            self.process_action_p2(time, Button::from_down(p2_down), (time * self.fps) as _);
            self.extended_p1(p1_down, (time * self.fps) as u32, 0., 0., 0., 0.);
            self.extended_p2(p2_down, (time * self.fps) as u32, 0., 0., 0., 0.);
        }

        Ok(())
    }

    fn parse_gdmo_22<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        use std::mem::size_of;
        log::info!("trying to parse 2.2 gdmo macro");

        #[repr(C)]
        struct GdmoAction {
            time: f64,
            key: i32,
            press: bool,
            player1: bool,
        }
        #[repr(C)]
        #[derive(Copy, Clone)]
        struct PlayerCheckpoint {
            y_vel: f64,
            x_vel: f64,
            x_pos: f32,
            y_pos: f32,
            node_x_pos: f32,
            node_y_pos: f32,
            rotation: f32,
            // ignored fields
            // rotation_rate: f32,
            // random_properties: [f32; 2268],
        }
        #[repr(C)]
        #[derive(Copy, Clone)]
        struct Correction {
            time: f64,
            player1: bool,
            checkpoint: PlayerCheckpoint,
        }

        let num_actions = reader.read_u32::<LittleEndian>()?;
        self.fps = self.get_fps(240.0);

        for _ in 0..num_actions {
            let mut buf = [0; size_of::<GdmoAction>()];
            reader.read_exact(&mut buf)?;
            let action: GdmoAction = unsafe { std::mem::transmute(buf) };
            let frame = (action.time * self.fps as f64) as u32;
            if action.player1 {
                self.process_action_p1(action.time, Button::from_down(action.press), frame);
            } else {
                self.process_action_p2(action.time, Button::from_down(action.press), frame);
            }
        }

        let num_corrections = reader.read_u32::<LittleEndian>()?;
        if num_corrections == 0 {
            return Ok(());
        }
        let current_pos = reader.stream_position()?;
        let end = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(current_pos))?;
        let correction_size = (end - current_pos) / num_corrections as u64;
        log::debug!("correction size: {correction_size}");
        if correction_size != 0x23a8 && correction_size != 56 {
            anyhow::bail!(
                "invalid correction size {correction_size}, expected {} or 56",
                0x23a8
            )
        }
        log::debug!("reading {num_corrections} corrections");

        for _ in 0..num_corrections {
            let mut buf = vec![0; correction_size as usize];
            reader.read_exact(&mut buf)?;
            let correction: Correction = unsafe { *(buf.as_ptr() as *const Correction) };
            let frame = (correction.time * self.fps as f64) as u32;
            let push = self
                .actions
                .binary_search_by(|a| a.frame.cmp(&frame))
                .unwrap_or(0);
            let push = self
                .actions
                .get(push)
                .map(|a| a.click.is_click())
                .unwrap_or(false);

            if correction.player1 {
                self.extended_p1(
                    push,
                    frame,
                    correction.checkpoint.x_pos,
                    correction.checkpoint.y_pos,
                    correction.checkpoint.y_vel as f32,
                    correction.checkpoint.rotation,
                );
            } else {
                self.extended_p2(
                    push,
                    frame,
                    correction.checkpoint.x_pos,
                    correction.checkpoint.y_pos,
                    correction.checkpoint.y_vel as f32,
                    correction.checkpoint.rotation,
                );
            }
        }

        let current_pos = reader.stream_position()?;
        log::debug!("cur: {current_pos}, end: {end}");
        if current_pos != end {
            reader.seek(SeekFrom::Start(0))?;
            anyhow::bail!(
                "didn't read entire file, {} leftover bytes",
                end - current_pos
            );
        }
        log::info!("parsed 2.2 gdmo macro");

        Ok(())
    }

    // https://github.com/maxnut/GDMegaOverlay/blob/3bc9c191e3fcdde838b0f69f8411af782afa3ba7/src/Replay.cpp#L124-L140
    fn parse_gdmo<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        // identify if its a 2.2 gdmo macro
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        reader.seek(SeekFrom::Start(0))?;
        if self
            .parse_gdmo_22(reader)
            .map_err(|e| log::error!("failed to parse 2.2 gdmo macro: {e}"))
            .is_ok()
        {
            return Ok(());
        }
        let mut reader = Cursor::new(data);

        use std::mem::size_of;
        self.fps = self.get_fps(reader.read_f32::<LittleEndian>()? as f64);

        let num_actions = reader.read_u32::<LittleEndian>()?;
        let _num_frame_captures = reader.read_u32::<LittleEndian>()?;

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
            reader.read_exact(&mut buf)?;
            let action: GdmoAction = unsafe { std::mem::transmute(buf) };

            let time = action.frame as f64 / self.fps;
            if action.player2 {
                self.process_action_p2(time, Button::from_down(action.press), action.frame);
                self.extended_p2(
                    action.press,
                    action.frame,
                    action.px,
                    action.py,
                    action.y_accel as f32,
                    0.,
                );
            } else {
                self.process_action_p1(time, Button::from_down(action.press), action.frame);
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

    fn parse_replaybot<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        const REPLAYBOT_MAGIC: [u8; 4] = *b"RPLY";
        let len = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;

        // check if its a version 2 frame replay
        let mut magicbuf = [0; 4];
        if reader.read_exact(&mut magicbuf).is_err() || magicbuf != REPLAYBOT_MAGIC {
            anyhow::bail!(
                "old replaybot replay format is not supported, as it does not store frames"
            )
        }

        let version = reader.read_u8()?;
        if version != 2 {
            anyhow::bail!("unsupported replaybot version {version} (only v2 is supported, because v1 doesn't store frames)")
        }
        if reader.read_u8()? != 1 {
            anyhow::bail!("only frame replays are supported")
        }

        self.fps = self.get_fps(reader.read_f32::<LittleEndian>()? as f64);
        for _ in (10..len).step_by(5) {
            let frame = reader.read_u32::<LittleEndian>()?;
            let time = frame as f64 / self.fps;
            let state = reader.read_u8()?;
            let down = state & 0x1 != 0;
            let player2 = state >> 1 != 0;

            if player2 {
                self.process_action_p2(time, Button::from_down(down), frame);
                self.extended_p2(down, frame, 0., 0., 0., 0.);
            } else {
                self.process_action_p1(time, Button::from_down(down), frame);
                self.extended_p1(down, frame, 0., 0., 0., 0.);
            }
        }

        Ok(())
    }

    fn parse_rush<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        let len = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;

        self.fps = self.get_fps(reader.read_i16::<LittleEndian>()? as f64);

        for _ in (2..len).step_by(5) {
            let frame = reader.read_i32::<LittleEndian>()?;
            let time = frame as f64 / self.fps;
            let state = reader.read_u8()?;
            let down = (state & 1) != 0;
            let p2 = (state >> 1) != 0;

            if p2 {
                self.process_action_p2(time, Button::from_down(down), frame as _);
                self.extended_p2(down, frame as u32, 0., 0., 0., 0.);
            } else {
                self.process_action_p1(time, Button::from_down(down), frame as _);
                self.extended_p1(down, frame as u32, 0., 0., 0., 0.);
            }
        }

        Ok(())
    }

    fn parse_kdbot<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        let len = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;

        self.fps = self.get_fps(reader.read_f32::<LittleEndian>()? as f64);

        for _ in (4..len).step_by(6) {
            let frame = reader.read_i32::<LittleEndian>()?;
            let time = frame as f64 / self.fps;
            let down = reader.read_u8()? == 1;
            let p2 = reader.read_u8()? == 1;

            if p2 {
                self.process_action_p2(time, Button::from_down(down), frame as _);
                self.extended_p2(down, frame as u32, 0., 0., 0., 0.);
            } else {
                self.process_action_p1(time, Button::from_down(down), frame as _);
                self.extended_p1(down, frame as u32, 0., 0., 0., 0.);
            }
        }

        Ok(())
    }

    fn parse_plaintext<R: Read>(&mut self, reader: R) -> Result<()> {
        let mut reader = BufReader::new(reader);
        {
            let mut fps_string = String::new();
            reader.read_line(&mut fps_string)?;
            self.fps = self.get_fps(fps_string.trim().parse()?);
        }

        for (i, line) in reader.lines().enumerate() {
            let line = line?;
            let mut split = line.trim().split(' ');
            if split.clone().count() < 4 {
                log::warn!("plaintext: line {i} length < 3, skipping");
                continue;
            }
            let frame: f64 = split.next().unwrap().parse()?;
            let time = frame / self.fps;
            let down = split.next().unwrap().parse::<u8>()? == 1;
            let pbutton: i32 = split.next().unwrap().parse()?;
            let p2 = split.next().unwrap().parse::<u8>()? == 0;
            let b = Button::from_button_idx(pbutton, down);

            if p2 {
                self.process_action_p2(time, b, frame as _);
                self.extended_p2(down, frame as u32, 0., 0., 0., 0.);
            } else {
                self.process_action_p1(time, b, frame as _);
                self.extended_p1(down, frame as u32, 0., 0., 0., 0.);
            }
        }
        Ok(())
    }

    fn parse_obot3<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        #[derive(Deserialize)]
        enum Obot3ClickType {
            None,
            Player1Down,
            Player1Up,
            Player2Down,
            Player2Up,
            FpsChange(f32),
        }

        #[derive(Deserialize)]
        struct Obot3Click {
            frame: u32,
            click_type: Obot3ClickType,
        }

        #[derive(Deserialize)]
        struct Obot3Replay {
            initial_fps: f32,
            _current_fps: f32,
            clicks: Vec<Obot3Click>,
        }
        let mut deserializer = dlhn::Deserializer::new(&mut reader);
        let Ok(replay) = Obot3Replay::deserialize(&mut deserializer) else {
            reader.seek(SeekFrom::Start(0))?;
            return self.parse_obot2(reader);
        };

        self.fps = self.get_fps(replay.initial_fps as f64);

        for action in replay.clicks {
            let time = action.frame as f64 / self.fps;
            match action.click_type {
                Obot3ClickType::Player1Down => {
                    self.process_action_p1(time, Button::from_down(true), action.frame);
                    self.extended_p1(true, action.frame, 0., 0., 0., 0.);
                }
                Obot3ClickType::Player1Up => {
                    self.process_action_p1(time, Button::from_down(false), action.frame);
                    self.extended_p1(false, action.frame, 0., 0., 0., 0.);
                }
                Obot3ClickType::Player2Down => {
                    self.process_action_p2(time, Button::from_down(true), action.frame);
                    self.extended_p2(true, action.frame, 0., 0., 0., 0.);
                }
                Obot3ClickType::Player2Up => {
                    self.process_action_p2(time, Button::from_down(false), action.frame);
                    self.extended_p2(false, action.frame, 0., 0., 0., 0.);
                }
                Obot3ClickType::FpsChange(fps) => {
                    self.fps = self.get_fps(fps as _);
                    self.fps_change(fps as _);
                }
                Obot3ClickType::None => {}
            }
        }

        Ok(())
    }

    fn parse_re<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        use std::mem::size_of;
        let file_len = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;

        self.fps = self.get_fps(reader.read_f32::<LittleEndian>()? as f64);
        let num_frame_actions = reader.read_u32::<LittleEndian>()?;
        let num_actions = reader.read_u32::<LittleEndian>()?;

        #[derive(Default, Clone)]
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
        #[repr(C)]
        struct ActionDataNew {
            frame: u32,
            hold: bool,
            button: i32,
            player2: bool,
        }
        const DEFAULT_ACTION: ActionDataNew = ActionDataNew {
            frame: 0,
            hold: false,
            button: 0,
            player2: false,
        };

        // read frame data
        let mut frame_datas: Vec<FrameData> = vec![];
        for _ in 0..num_frame_actions {
            let mut buf = [0; size_of::<FrameData>()];
            reader.read_exact(&mut buf)?;
            frame_datas.push(unsafe { std::mem::transmute(buf) });
        }

        // detect action data type (there are actually 2 versions of replayengine v1,
        // so we have to handle both)
        let action_data_size =
            (file_len - reader.stream_position()?) as usize / num_actions as usize;
        log::debug!("predicted action data size: {action_data_size}");
        if action_data_size != size_of::<ActionData>()
            && action_data_size != size_of::<ActionDataNew>()
        {
            anyhow::bail!("unknown action data type (length: {action_data_size})");
        }
        let is_new = action_data_size == size_of::<ActionDataNew>();

        // hash action datas
        let mut actions = HashMap::new();
        for _ in 0..num_actions {
            let action = if is_new {
                let mut buf = [0; size_of::<ActionDataNew>()];
                reader.read_exact(&mut buf)?;
                unsafe { std::mem::transmute(buf) }
            } else {
                let mut buf = [0; size_of::<ActionData>()];
                reader.read_exact(&mut buf)?;
                let action: ActionData = unsafe { std::mem::transmute(buf) };
                ActionDataNew {
                    frame: action.frame,
                    hold: action.hold,
                    button: 1,
                    player2: action.player2,
                }
            };
            actions.insert(action.frame, action);
        }

        for frame_data in frame_datas {
            // to get the button we need to lookup the action hashmap
            let action = actions.get(&frame_data.frame).unwrap_or(&DEFAULT_ACTION);
            let time = frame_data.frame as f64 / self.fps;
            let button = Button::from_button_idx(action.button, action.hold);
            if action.player2 {
                self.process_action_p2(time, button, frame_data.frame);
                self.extended_p2(
                    action.hold,
                    frame_data.frame,
                    frame_data.x,
                    frame_data.y,
                    frame_data.y_accel as _,
                    frame_data.rot,
                );
            } else {
                self.process_action_p1(time, button, frame_data.frame);
                self.extended_p1(
                    action.hold,
                    frame_data.frame,
                    frame_data.x,
                    frame_data.y,
                    frame_data.y_accel as _,
                    frame_data.rot,
                );
            }
        }

        Ok(())
    }

    fn parse_ddhor<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        const DDHOR_MAGIC: &[u8; 4] = b"DDHR";
        let len = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;

        let mut magicbuf = [0; DDHOR_MAGIC.len()];
        if reader.read_exact(&mut magicbuf).is_err() || magicbuf != *DDHOR_MAGIC {
            anyhow::bail!(
                "ddhor json is not supported, as it doesn't store frames.\n\
                           try using an older ddhor version with frame mode"
            );
        }

        self.fps = self.get_fps(reader.read_i16::<LittleEndian>()? as f64);
        let num_p1 = reader.read_i32::<LittleEndian>()?; // num p1 actions
        let _num_p2 = reader.read_i32::<LittleEndian>()?; // num p2 actions

        for i in (14..len).step_by(5) {
            let frame = reader.read_f32::<LittleEndian>()? as f64;
            let time = frame / self.fps;
            let down = reader.read_u8()? == 0;
            let p2 = i - 14 >= num_p1 as u64 * 5;

            if p2 {
                self.process_action_p2(time, Button::from_down(down), frame as _);
                self.extended_p2(down, frame as u32, 0., 0., 0., 0.);
            } else {
                self.process_action_p1(time, Button::from_down(down), frame as _);
                self.extended_p1(down, frame as u32, 0., 0., 0., 0.);
            }
        }

        Ok(())
    }

    fn parse_xbot<R: Read>(&mut self, mut reader: R) -> Result<()> {
        let mut string = String::new();
        reader.read_to_string(&mut string)?;
        let mut lines = string.split('\n');

        self.fps = self.get_fps(
            lines
                .next()
                .context("first fps line doesn't exist, did you select an empty file?")?
                .trim()
                .parse::<u64>()? as f64,
        );

        if lines.next().context("second line doesn't exist")?.trim() != "frames" {
            anyhow::bail!("the xBot parser only supports xBot Frame replays");
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
            let time = frame as f64 / self.fps;

            if player2 {
                self.process_action_p2(time, Button::from_down(down), frame);
                self.extended_p2(down, frame, 0., 0., 0., 0.);
            } else {
                self.process_action_p1(time, Button::from_down(down), frame);
                self.extended_p1(down, frame, 0., 0., 0., 0.);
            }
        }

        Ok(())
    }

    fn parse_ybot2<R: Read + Seek>(&mut self, reader: R) -> Result<()> {
        use ybot_fmt::*;
        let mut replay = Macro::open(reader)?;
        let date = replay.get(Meta::DATE)?;
        let presses = replay.get(Meta::PRESSES)?;
        let frames = replay.get(Meta::FRAMES)?;
        self.fps = self.get_fps(replay.get(Meta::FPS)? as f64);
        let total_presses = replay.get(Meta::TOTAL_PRESSES)?;
        let version = replay.version();
        log::info!(
            "ybot2 replay: version {version}, {presses} presses, \
            {frames} frames, {} FPS, {total_presses} total presses",
            self.fps
        );

        // log datetime
        use chrono::prelude::*;
        log::info!(
            "replay created at {} (UNIX timestamp: {date})",
            Utc.timestamp_opt(date, 0).unwrap()
        );

        let mut frame = 0;
        for timed_action in replay.actions() {
            let timed_action = timed_action?;
            frame += timed_action.delta;

            match timed_action.action {
                Action::Button(p1, push, button) => {
                    let time = frame as f64 / self.fps;
                    let b = match button {
                        PlayerButton::Jump => Button::from_down(push),
                        PlayerButton::Left => Button::from_left_down(push),
                        PlayerButton::Right => Button::from_right_down(push),
                    };
                    if p1 {
                        self.process_action_p1(time, b, frame as u32);
                        self.extended_p1(push, frame as u32, 0.0, 0.0, 0.0, 0.0);
                    } else {
                        self.process_action_p2(time, b, frame as u32);
                        self.extended_p2(push, frame as u32, 0.0, 0.0, 0.0, 0.0);
                    }
                }
                Action::FPS(fps) => {
                    self.fps = self.get_fps(fps as _);
                    self.fps_change(fps as _);
                }
            }
        }

        Ok(())
    }

    fn parse_xdbot<R: Read>(&mut self, reader: R) -> Result<()> {
        let reader = BufReader::new(reader);

        self.fps = if let Some(override_fps) = self.override_fps {
            override_fps
        } else {
            240.0
        };

        // first line: fps
        // action: frame|holding|button|player1|pos_only|p1_xpos|p1_ypos|p1_upsideDown|p1_rotation|p1_xSpeed|p1_ySpeed|p2_xpos|p2_ypos|p2_upsideDown|p2_rotation|p2_xSpeed|p2_ySpeed
        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }
            let mut split = line.split('|');

            // if the line only contains the fps, set the fps
            if split.clone().count() == 1 {
                self.fps = if let Some(override_fps) = self.override_fps {
                    override_fps
                } else {
                    split.next().context("failed to get fps")?.parse::<f64>()?
                };
                continue;
            }

            let frame = split
                .next()
                .context("failed to get frame")?
                .parse::<u32>()?;
            let push = split
                .next()
                .context("failed to get holding state")?
                .parse::<u8>()?
                == 1;
            let button = split
                .next()
                .context("failed to get button state")?
                .parse::<u8>()?;
            let p1 = split
                .next()
                .context("failed to get player1 state")?
                .parse::<u8>()?
                == 1;
            split.next(); // pos_only
            let x = if let Some(x) = split.next() {
                x.parse::<f32>()?
            } else {
                0.0
            };
            let y = if let Some(y) = split.next() {
                y.parse::<f32>()?
            } else {
                0.0
            };
            let b = if button == 3 {
                Button::from_right_down(push)
            } else if button == 2 {
                Button::from_left_down(push)
            } else {
                Button::from_down(push)
            };
            if p1 {
                self.process_action_p1(frame as f64 / self.fps, b, frame as u32);
                self.extended_p1(push, frame as u32, x, y, 0.0, 0.0);
            } else {
                self.process_action_p2(frame as f64 / self.fps, b, frame as u32);
                self.extended_p2(push, frame as u32, x, y, 0.0, 0.0);
            }
        }
        Ok(())
    }

    fn parse_gdr<R: Read>(&mut self, mut reader: R) -> Result<()> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        let replay = gdr::Replay::from_slice(&data)?;
        self.fps = if let Some(override_fps) = self.override_fps {
            override_fps
        } else {
            replay.framerate as f64
        };
        for input in &replay.inputs {
            let time = if input.correction.time != 0.0 {
                input.correction.time as f64
            } else {
                input.frame as f64 / self.fps
            };
            if input.player2 {
                self.process_action_p2(time, Button::from_down(input.down), input.frame);
                self.extended_p2(
                    input.down,
                    input.frame,
                    input.correction.x_pos,
                    input.correction.y_pos,
                    input.correction.y_vel,
                    input.correction.rotation,
                );
            } else {
                self.process_action_p1(time, Button::from_down(input.down), input.frame);
                self.extended_p1(
                    input.down,
                    input.frame,
                    input.correction.x_pos,
                    input.correction.y_pos,
                    input.correction.y_vel,
                    input.correction.rotation,
                );
            }
        }
        Ok(())
    }

    fn parse_qbot<R: Read>(&mut self, mut reader: R) -> Result<()> {
        #[derive(Deserialize)]
        enum PlayerButton {
            Jump = 1,
            Left = 2,
            Right = 3,
        }
        #[derive(Deserialize)]
        enum Action {
            Button {
                is_p2: bool,
                push: bool,
                button: PlayerButton,
            },
            FPS(f32),
        }
        #[derive(Deserialize, Default)]
        struct Position {
            x: f32,
            y: f32,
            rotate: f32,
        }
        #[derive(Deserialize)]
        struct Click {
            frame: u32,
            time: f64,
            action: Action,
            _x_vel: f64,
            y_vel: f64,
            position: Option<Position>,
        }
        #[derive(Deserialize)]
        struct Replay {
            initial_fps: f32,
            _fps: f32,
            _index: usize,
            clicks: Vec<Click>,
        }
        let mut deserializer = dlhn::Deserializer::new(&mut reader);
        let replay = Replay::deserialize(&mut deserializer)?;
        self.fps = self.get_fps(replay.initial_fps as _);
        for click in replay.clicks {
            match click.action {
                Action::Button {
                    is_p2,
                    push,
                    button,
                } => {
                    let p = click.position.unwrap_or_default();
                    let b = match button {
                        PlayerButton::Jump => Button::from_down(push),
                        PlayerButton::Left => Button::from_left_down(push),
                        PlayerButton::Right => Button::from_right_down(push),
                    };
                    let time = if click.time != 0.0 {
                        click.time
                    } else {
                        click.frame as f64 / self.fps
                    };
                    if is_p2 {
                        self.process_action_p2(time, b, click.frame);
                        self.extended_p2(push, click.frame, p.x, p.y, click.y_vel as f32, p.rotate);
                    } else {
                        self.process_action_p1(time, b, click.frame);
                        self.extended_p1(push, click.frame, p.x, p.y, click.y_vel as f32, p.rotate);
                    }
                }
                Action::FPS(new_fps) => {
                    self.fps = self.get_fps(new_fps as _);
                    self.fps_change(new_fps as _);
                }
            }
        }
        Ok(())
    }

    fn parse_rbot_gz<R: Read>(&mut self, reader: R) -> Result<()> {
        use flate2::read::GzDecoder;
        let mut d = GzDecoder::new(reader);
        let fps = d.read_u32::<LittleEndian>()?;
        self.fps = if let Some(override_fps) = self.override_fps {
            override_fps
        } else {
            fps as f64
        };

        let num_actions = d.read_u32::<LittleEndian>()?;
        for _ in 0..num_actions {
            let frame = d.read_u32::<LittleEndian>()?;
            let hold = d.read_u8()? != 0;
            let p2 = d.read_u8()? != 0;
            if p2 {
                self.process_action_p2(frame as f64 / self.fps, Button::from_down(hold), frame);
                // self.extended_p2(hold, frame, 0.0, 0.0, 0.0, 0.0);
            } else {
                self.process_action_p1(frame as f64 / self.fps, Button::from_down(hold), frame);
                // self.extended_p1(hold, frame, 0.0, 0.0, 0.0, 0.0);
            }
        }

        let num_positions = d.read_u32::<LittleEndian>()?;
        for _ in 0..num_positions {
            let frame = d.read_u32::<LittleEndian>()?;
            let p2 = d.read_u8()? != 0;
            let x = d.read_f32::<LittleEndian>()?;
            let y = d.read_f32::<LittleEndian>()?;
            let rot = d.read_f32::<LittleEndian>()?;

            // find action to get hold state (FIXME: very inefficent)
            let hold = if let Ok(idx) = self.actions.binary_search_by(|a| a.frame.cmp(&frame)) {
                self.actions[idx].click.is_click()
            } else {
                false
            };

            if p2 {
                self.extended_p2(hold, frame, x, y, 0.0, rot);
            } else {
                self.extended_p1(hold, frame, x, y, 0.0, rot);
            }
        }

        Ok(())
    }

    fn parse_rbot<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        let magic = reader.read_u16::<LittleEndian>()?;
        reader.seek(SeekFrom::Start(0))?;
        if magic == 0x8b1f {
            return self.parse_rbot_gz(reader);
        }

        let fps = reader.read_u32::<LittleEndian>()?;
        self.fps = if let Some(override_fps) = self.override_fps {
            override_fps
        } else {
            fps as f64
        };
        let num_actions = reader.read_u32::<LittleEndian>()?;
        for _ in 0..num_actions {
            let frame = reader.read_u32::<LittleEndian>()?;
            let push = reader.read_u8()? != 0;
            let p1 = reader.read_u8()? != 0;
            if p1 {
                self.process_action_p1(frame as f64 / self.fps, Button::from_down(push), frame);
                self.extended_p1(push, frame, 0.0, 0.0, 0.0, 0.0);
            } else {
                self.process_action_p2(frame as f64 / self.fps, Button::from_down(push), frame);
                self.extended_p2(push, frame, 0.0, 0.0, 0.0, 0.0);
            }
        }
        Ok(())
    }

    // note that Zephyrus does not write structs directly into replays,
    // so just reading the structs wouldn't work!
    fn parse_zephyrus<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        #[derive(Default, Debug)]
        struct Header {
            magic: u16,
            version: u8,
            fps: u32,
            num_actions: u32,
            num_frame_fixes: u32,
        }

        #[derive(Default)]
        struct Action {
            frame: u32,
            flags: u8,
        }

        #[derive(Default)]
        struct PlayerData {
            x: f32,
            y: f32,
            y_speed: f64,
            rot: f32,
        }

        #[derive(Default)]
        struct FrameFix {
            frame: u32,
            player1: PlayerData,
            player2_exists: bool,
            player2: PlayerData,
        }

        // read hdr
        let mut header = Header::default();
        header.magic = reader.read_u16::<LittleEndian>()?;
        header.version = reader.read_u8()?;
        header.fps = reader.read_u32::<LittleEndian>()?;
        header.num_actions = reader.read_u32::<LittleEndian>()?;
        header.num_frame_fixes = reader.read_u32::<LittleEndian>()?;
        log::debug!("zephyrus header: {header:?}");

        if header.magic != 0x525a {
            anyhow::bail!("invalid zephyrus replay magic {:#x}", header.magic);
        }
        if header.version != 2 {
            anyhow::bail!(
                "invalid zephyrus replay version {} (expected 2)",
                header.version
            );
        }

        self.fps = self.get_fps(header.fps as f64);

        // read actions
        for _ in 0..header.num_actions {
            let mut action = Action::default();
            action.frame = reader.read_u32::<LittleEndian>()?;
            action.flags = reader.read_u8()?;

            let player2 = (action.flags & 0b10000000) != 0;
            let push = (action.flags & 0b01000000) != 0;
            let button = Button::from_button_idx(((action.flags & 0b00110000) >> 4) as i32, push);
            let time = action.frame as f64 / self.fps;
            if player2 {
                self.process_action_p2(time, button, action.frame);
            } else {
                self.process_action_p1(time, button, action.frame);
            }
        }

        macro_rules! read_player_data {
            () => {{
                let mut data = PlayerData::default();
                data.x = reader.read_f32::<LittleEndian>()?;
                data.y = reader.read_f32::<LittleEndian>()?;
                data.y_speed = reader.read_f64::<LittleEndian>()?;
                data.rot = reader.read_f32::<LittleEndian>()?;
                data
            }};
        }

        // read frame fixes
        for _ in 0..header.num_frame_fixes {
            let mut fix = FrameFix::default();
            fix.frame = reader.read_u32::<LittleEndian>()?;
            fix.player1 = read_player_data!();
            fix.player2_exists = reader.read_u8()? != 0;
            if fix.player2_exists {
                fix.player2 = read_player_data!();
            }

            // find button state
            let push = if let Ok(idx) = self.actions.binary_search_by(|a| a.frame.cmp(&fix.frame)) {
                self.actions
                    .get(idx)
                    .map(|a| a.click.is_click())
                    .unwrap_or(false)
            } else {
                false
            };

            self.extended_p1(
                push,
                fix.frame,
                fix.player1.x,
                fix.player1.y,
                fix.player1.y_speed as f32,
                fix.player1.rot,
            );
            if fix.player2_exists {
                self.extended_p2(
                    push,
                    fix.frame,
                    fix.player2.x,
                    fix.player2.y,
                    fix.player2.y_speed as f32,
                    fix.player2.rot,
                );
            }
        }

        Ok(())
    }

    fn parse_re2<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        #[repr(C)]
        struct FrameData {
            frame: u32,
            hold: bool,
            button: i32,
            player2: bool,
        }

        // ensure magic
        const RE2_MAGIC: [u8; 3] = *b"RE2";
        let mut magicbuf = [0u8; 3];
        if reader.read_exact(&mut magicbuf).is_err() || magicbuf != RE2_MAGIC {
            anyhow::bail!(format!(
                "invalid re2 magic (got: {magicbuf:?}, expect: {RE2_MAGIC:?})"
            ))
        }

        // all re2 replays are 240 fps
        self.fps = self.get_fps(240.0);

        let num_actions = reader.read_u32::<LittleEndian>()?;
        for _ in 0..num_actions {
            let mut buf = [0; size_of::<FrameData>()];
            reader.read_exact(&mut buf)?;
            let action: FrameData = unsafe { std::mem::transmute(buf) };
            let time = action.frame as f64 / self.fps;
            let button = Button::from_button_idx(action.button, action.hold);
            if action.player2 {
                self.process_action_p2(time, button, action.frame);
                self.extended_p2(action.hold, action.frame, 0.0, 0.0, 0.0, 0.0);
            } else {
                self.process_action_p1(time, button, action.frame);
                self.extended_p1(action.hold, action.frame, 0.0, 0.0, 0.0, 0.0);
            }
        }

        Ok(())
    }

    fn parse_slc<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        // compare slc2 header
        let mut header_buf = [0u8; 4];
        if reader
            .read_exact(&mut header_buf)
            .is_ok_and(|_| header_buf == [0x53, 0x49, 0x4C, 0x4C])
        {
            reader.seek(SeekFrom::Start(0))?;
            return self.parse_slc2(reader);
        }

        reader.seek(SeekFrom::Start(0))?;

        self.fps = self.get_fps(reader.read_f64::<LittleEndian>()?);
        let num_actions = reader.read_u32::<LittleEndian>()?;
        for _ in 0..num_actions {
            let action = reader.read_u32::<LittleEndian>()?;
            // first 28 bits - frame
            // 29th bit - p2 (1 = p2, 0 = p1)
            // 30-31st bits - button (1 = click, 2 = left, 3 = right, 0 is reserved)
            // 32nd bit - down (1 = down, 0 = up)
            let frame = action >> 4;
            let player2 = (action & 0b1000) != 0;
            let down = (action & 0b0001) != 0;
            let button = Button::from_button_idx(((action & 0b0110) >> 1) as i32, down);
            let time = frame as f64 / self.fps;
            if player2 {
                self.process_action_p2(time, button, frame);
                self.extended_p2(down, frame, 0.0, 0.0, 0.0, 0.0);
            } else {
                self.process_action_p1(time, button, frame);
                self.extended_p1(down, frame, 0.0, 0.0, 0.0, 0.0);
            }
        }

        // read seed for the fun of it
        if let Ok(seed) = reader.read_u64::<LittleEndian>() {
            log::info!("silicate: seed: {seed}");
        } else {
            log::info!("silicate: no seed stored in macro");
        }
        Ok(())
    }

    fn parse_re3<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        // a bit similar to re1, but the p1 and p2 actions are stored separately
        self.fps = self.get_fps(reader.read_f32::<LittleEndian>()? as f64);

        // mirrors https://github.com/TobyAdd/GDH/blob/088b5accb04cddcbd09cac29b2e9850ebcea5c60/src/replayEngine.hpp#L11-L27
        #[repr(C)]
        #[derive(Default)]
        struct FrameData {
            frame: u32,
            x: f32,
            y: f32,
            rot: f32,
            y_accel: f64,
            player2: bool,
        }
        #[repr(C)]
        #[derive(Default)]
        struct ActionData {
            frame: u32,
            down: bool,
            button: i32,
            player1: bool,
        }

        #[derive(Default)]
        struct AmalgamatedActionDatas {
            p1_frame: Option<FrameData>,
            p2_frame: Option<FrameData>,
            p1_action: Option<ActionData>,
            p2_action: Option<ActionData>,
        }

        let p1_size = reader.read_u32::<LittleEndian>()? as usize;
        let p2_size = reader.read_u32::<LittleEndian>()? as usize;
        let p1_input_size = reader.read_u32::<LittleEndian>()? as usize;
        let p2_input_size = reader.read_u32::<LittleEndian>()? as usize;

        // an indexmap to store our amalgamated actions
        let mut amalgamated_action_datas: IndexMap<u32, AmalgamatedActionDatas> = IndexMap::new();

        // read p1 frame datas
        for _ in 0..p1_size {
            let mut buf = [0; size_of::<FrameData>()];
            reader.read_exact(&mut buf)?;
            let frame_data: FrameData = unsafe { std::mem::transmute(buf) };
            if let Some(action_data) = amalgamated_action_datas.get_mut(&frame_data.frame) {
                action_data.p1_frame = Some(frame_data);
            } else {
                amalgamated_action_datas.insert(
                    frame_data.frame,
                    AmalgamatedActionDatas {
                        p1_frame: Some(frame_data),
                        ..Default::default()
                    },
                );
            }
        }

        // read p2 frame datas
        for _ in 0..p2_size {
            let mut buf = [0; size_of::<FrameData>()];
            reader.read_exact(&mut buf)?;
            let frame_data: FrameData = unsafe { std::mem::transmute(buf) };
            if let Some(action_data) = amalgamated_action_datas.get_mut(&frame_data.frame) {
                action_data.p2_frame = Some(frame_data);
            } else {
                amalgamated_action_datas.insert(
                    frame_data.frame,
                    AmalgamatedActionDatas {
                        p2_frame: Some(frame_data),
                        ..Default::default()
                    },
                );
            }
        }

        // now, add action datas into our amalgamation

        // read p1 action datas
        for _ in 0..p1_input_size {
            let mut buf = [0; size_of::<ActionData>()];
            reader.read_exact(&mut buf)?;
            let action: ActionData = unsafe { std::mem::transmute(buf) };
            if let Some(action_data) = amalgamated_action_datas.get_mut(&action.frame) {
                action_data.p1_action = Some(action);
            } else {
                amalgamated_action_datas.insert(
                    action.frame,
                    AmalgamatedActionDatas {
                        p1_action: Some(action),
                        ..Default::default()
                    },
                );
            }
        }

        // read p2 action datas
        for _ in 0..p2_input_size {
            let mut buf = [0; size_of::<ActionData>()];
            reader.read_exact(&mut buf)?;
            let action: ActionData = unsafe { std::mem::transmute(buf) };
            if let Some(action_data) = amalgamated_action_datas.get_mut(&action.frame) {
                action_data.p2_action = Some(action);
            } else {
                amalgamated_action_datas.insert(
                    action.frame,
                    AmalgamatedActionDatas {
                        p2_action: Some(action),
                        ..Default::default()
                    },
                );
            }
        }

        // now sort by frame, since everything is separate in 4 chunks
        amalgamated_action_datas.sort_by(|k1, _, k2, _| k1.cmp(&k2));

        // now we can process the amalgamation
        for (frame, action_data) in amalgamated_action_datas {
            let AmalgamatedActionDatas {
                p1_frame,
                p2_frame,
                p1_action,
                p2_action,
            } = &action_data;

            let time = frame as f64 / self.fps;

            let down = if let Some(ac) = p1_action {
                let button = Button::from_button_idx(ac.button, ac.down);
                self.process_action_p1(time, button, frame);
                ac.down
            } else if let Some(ac) = p2_action {
                let button = Button::from_button_idx(ac.button, ac.down);
                self.process_action_p2(time, button, frame);
                ac.down
            } else {
                false
            };

            // add extended
            if let Some(p1_frame) = p1_frame {
                self.extended_p1(
                    down,
                    frame,
                    p1_frame.x,
                    p1_frame.y,
                    p1_frame.y_accel as _,
                    p1_frame.rot,
                );
            }
            if let Some(p2_frame) = p2_frame {
                self.extended_p2(
                    down,
                    frame,
                    p2_frame.x,
                    p2_frame.y,
                    p2_frame.y_accel as _,
                    p2_frame.rot,
                );
            }
        }

        Ok(())
    }

    fn parse_gdr2<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        let mut buf = vec![];
        reader.read_to_end(&mut buf)?;
        let replay = gdr2::Replay::import_data(&buf)?;

        self.fps = self.get_fps(replay.framerate);

        log::info!("gdr2 replay author: {}", replay.author);
        log::info!("gdr2 replay description: {}", replay.description);
        log::info!("gdr2 replay game version: {}", replay.game_version);
        log::info!("gdr2 replay duration: {}", replay.duration);
        log::info!("gdr2 replay seed: {}", replay.seed);
        log::info!("gdr2 bot info: {:?}", replay.bot_info);
        log::info!("gdr2 level info: {:?}", replay.level_info);

        let start_frame = if self.discard_deaths {
            replay.deaths.last().copied().unwrap_or(0)
        } else {
            0
        };

        log::info!(
            "gdr2 replay start frame: {} (discard deaths: {})",
            start_frame,
            self.discard_deaths
        );

        for input in &replay.inputs {
            if input.frame < start_frame {
                continue;
            }
            let time = input.frame as f64 / self.fps;
            let button = Button::from_button_idx(input.button as _, input.down);
            let p = input.physics.clone().unwrap_or_default();
            if input.player2 {
                self.process_action_p2(time, button, input.frame as _);
                self.extended_p2(
                    input.down,
                    input.frame as _,
                    p.x_position,
                    p.y_position,
                    p.y_velocity as _,
                    p.rotation,
                );
            } else {
                self.process_action_p1(time, button, input.frame as _);
                self.extended_p1(
                    input.down,
                    input.frame as _,
                    p.x_position,
                    p.y_position,
                    p.y_velocity as _,
                    p.rotation,
                );
            }
        }

        Ok(())
    }

    fn parse_slc2<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        use slc_oxide::{input::InputData, meta::Meta, replay::Replay};

        #[repr(C, packed)]
        #[derive(Debug)]
        struct SilicateMeta {
            seed: u64,
            _reserved: [u8; 56],
        }

        impl Meta for SilicateMeta {
            fn size() -> u64 {
                size_of::<SilicateMeta>() as _
            }

            fn from_bytes(bytes: &[u8]) -> Self {
                let mut seed_buf = [0u8; 8];
                seed_buf.copy_from_slice(&bytes[0..8]);
                Self {
                    seed: u64::from_le_bytes(seed_buf),
                    _reserved: [0u8; 56],
                }
            }

            fn to_bytes(&self) -> Box<[u8]> {
                let mut buf = vec![];
                buf.extend_from_slice(&self.seed.to_le_bytes());
                buf.extend_from_slice(&[0u8; 56]);
                buf.into()
            }
        }

        let replay = Replay::<SilicateMeta>::read(&mut reader)?;
        log::info!("slc2: meta: {:?}", replay.meta);
        self.fps = self.get_fps(replay.tps);

        let start = if self.discard_deaths {
            // find the last death
            replay
                .inputs
                .iter()
                .rposition(|i| {
                    matches!(
                        i.data,
                        InputData::Restart | InputData::RestartFull | InputData::Death
                    )
                })
                .unwrap_or(0)
        } else {
            0
        };

        for input in replay.inputs.iter().skip(start) {
            let time = input.frame as f64 / self.fps;
            match &input.data {
                InputData::TPS(tps) => {
                    self.fps = self.get_fps(*tps);
                    self.fps_change(*tps);
                }
                InputData::Player(player) => {
                    let button = Button::from_button_idx(player.button as _, player.hold);
                    if player.player_2 {
                        self.process_action_p2(time, button, input.frame as _);
                        self.extended_p2(player.hold, input.frame as _, 0.0, 0.0, 0.0, 0.0);
                    } else {
                        self.process_action_p1(time, button, input.frame as _);
                        self.extended_p1(player.hold, input.frame as _, 0.0, 0.0, 0.0, 0.0);
                    }
                }
                _ => (),
            }
        }

        Ok(())
    }

    /* gato
    fn parse_gatobot<R: Read>(&mut self, mut reader: R) -> Result<()> {
        use base64::{engine::general_purpose, Engine as _};
        use flate2::read::GzDecoder;

        let text = String::from_utf8(data.to_vec())?;
        if !text.starts_with("H4sIAAAAAAAA") {
            anyhow::bail!("corrupted gatobot replay (must start with 'H4sIAAAAAAAA')");
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

    fn parse_uvbot<R: Read>(&mut self, mut reader: R) -> Result<()> {
        let mut magic = [0; 5];
        reader.read_exact(&mut magic)?;
        if magic != "UVBOT".as_bytes() {
            anyhow::bail!(format!(
                "invalid uvbot magic (got: {magic:?}, expect: UVBOT)"
            ))
        }

        let version = reader.read_u8()?;
        if version != 1 && version != 2 {
            anyhow::bail!(format!(
                "invalid uvbot version (got: {version:?}, expect: 1 or 2)"
            ))
        }

        if version == 1 {
            self.fps = self.get_fps(240.0);
        } else {
            let tps = reader.read_f32::<LittleEndian>()?;
            self.fps = self.get_fps(tps as _);
        }

        // mix all inputs

        struct InputAction {
            player_2: bool,
            button: Button,
        }

        struct PhysicsAction {
            x: f32,
            y: f32,
            rotation: f32,
            y_velocity: f64,
        }

        #[derive(Default)]
        struct Action {
            input: Option<InputAction>,
            p1_physics: Option<PhysicsAction>,
            p2_physics: Option<PhysicsAction>,
        }

        let mut actions: IndexMap<u64, Action> = IndexMap::new();

        let input_actions = reader.read_i32::<LittleEndian>()?;
        let physics_p1_actions = reader.read_i32::<LittleEndian>()?;
        let physics_p2_actions = reader.read_i32::<LittleEndian>()?;

        for _ in 0..input_actions {
            let frame = reader.read_u64::<LittleEndian>()?;
            let flags = reader.read_u8()?;

            let hold = (flags & 1) != 0;
            let button = (flags >> 1) % 3;
            let player_2 = (flags >> 1) > 2;

            let input_action = InputAction {
                player_2: player_2,
                button: match button {
                    0 => Button::from_down(hold),
                    1 => Button::from_left_down(hold),
                    2 => Button::from_right_down(hold),
                    _ => todo!(),
                },
            };

            if let Some(action) = actions.get_mut(&frame) {
                action.input = Some(input_action);
            } else {
                actions.insert(
                    frame,
                    Action {
                        input: Some(input_action),
                        ..Default::default()
                    },
                );
            }
        }

        for _ in 0..physics_p1_actions {
            let frame = reader.read_u64::<LittleEndian>()?;
            let x = reader.read_f32::<LittleEndian>()?;
            let y = reader.read_f32::<LittleEndian>()?;
            let rotation = reader.read_f32::<LittleEndian>()?;
            let y_velocity = reader.read_f64::<LittleEndian>()?;

            let physics_action = PhysicsAction {
                x: x,
                y: y,
                rotation: rotation,
                y_velocity: y_velocity,
            };

            if let Some(action) = actions.get_mut(&frame) {
                action.p1_physics = Some(physics_action);
            } else {
                actions.insert(
                    frame,
                    Action {
                        p1_physics: Some(physics_action),
                        ..Default::default()
                    },
                );
            }
        }

        for _ in 0..physics_p2_actions {
            let frame = reader.read_u64::<LittleEndian>()?;
            let x = reader.read_f32::<LittleEndian>()?;
            let y = reader.read_f32::<LittleEndian>()?;
            let rotation = reader.read_f32::<LittleEndian>()?;
            let y_velocity = reader.read_f64::<LittleEndian>()?;

            let physic_action = PhysicsAction {
                x: x,
                y: y,
                rotation: rotation,
                y_velocity: y_velocity,
            };

            if let Some(action) = actions.get_mut(&frame) {
                action.p2_physics = Some(physic_action);
            } else {
                actions.insert(
                    frame,
                    Action {
                        p2_physics: Some(physic_action),
                        ..Default::default()
                    },
                );
            }
        }

        reader.read_exact(&mut magic)?;
        if magic != "TOBVU".as_bytes() {
            anyhow::bail!(format!(
                "invalid uvbot magic (got: {magic:?}, expect: TOBVU)"
            ))
        }

        actions.sort_by(|k1, _, k2, _| k1.cmp(&k2));

        for (frame, action) in actions {
            let Action {
                input,
                p1_physics,
                p2_physics,
            } = action;

            let time = frame as f64 / self.fps;

            let down = if let Some(ref input) = input {
                match input.button {
                    Button::Push => true,
                    Button::LeftPush => true,
                    Button::RightPush => true,
                    _ => false,
                }
            } else {
                false
            };

            if let Some(input) = input {
                if !input.player_2 {
                    self.process_action_p1(time, input.button, frame as _);
                } else {
                    self.process_action_p2(time, input.button, frame as _);
                }
            }

            if let Some(p1_physics) = p1_physics {
                self.extended_p1(
                    down,
                    frame as _,
                    p1_physics.x,
                    p1_physics.y,
                    p1_physics.y_velocity as _,
                    p1_physics.rotation,
                );
            }

            if let Some(p2_physics) = p2_physics {
                self.extended_p2(
                    down,
                    frame as _,
                    p2_physics.x,
                    p2_physics.y,
                    p2_physics.y_velocity as _,
                    p2_physics.rotation,
                );
            }
        }

        Ok(())
    }

    fn parse_tcm<R: Read + Seek>(&mut self, mut reader: R) -> Result<()> {
        let replay = tcm::DynamicReplay::from_reader(&mut reader)?;

        self.fps = self.get_fps(replay.meta.tps() as _);

        for input in &replay.inputs {
            let time = input.frame as f64 / self.fps;
            use tcm::input::{Input::*, TpsInput};

            match &input.input {
                Restart(_) => {
                    if self.discard_deaths {
                        self.actions.clear();
                        self.extended.clear();
                    }
                }
                Tps(TpsInput { tps }) => {
                    let tps = *tps as f64;
                    self.fps = self.get_fps(tps);
                    self.fps_change(tps);
                }
                Vanilla(v) => {
                    if v.player2 {
                        self.process_action_p1(
                            time,
                            Button::from_button_idx(v.button as _, v.push),
                            input.frame as _,
                        );
                        self.extended_p1(v.push, input.frame as _, 0.0, 0.0, 0.0, 0.0);
                    } else {
                        self.process_action_p2(
                            time,
                            Button::from_button_idx(v.button as _, v.push),
                            input.frame as _,
                        );
                        self.extended_p2(v.push, input.frame as _, 0.0, 0.0, 0.0, 0.0);
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }
}
