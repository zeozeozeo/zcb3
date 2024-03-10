//! An implementation of [GDReplayFormat](https://github.com/maxnut/GDReplayFormat) in Rust.
//!
//! Supports JSON and [MessagePack](https://msgpack.org) encoding.

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct BotInfo {
    #[serde(default = "String::new")]
    pub name: String,
    #[serde(default = "String::new")]
    pub version: String,
}

impl BotInfo {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct LevelInfo {
    pub id: u32,
    pub name: String,
}

impl LevelInfo {
    pub fn new(id: u32, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct Correction {
    #[serde(rename = "nodeXPos", default = "f32::default")]
    pub node_x_pos: f32,
    #[serde(rename = "nodeYPos", default = "f32::default")]
    pub node_y_pos: f32,
    #[serde(default = "bool::default")]
    pub player2: bool,
    #[serde(default = "f32::default")]
    pub rotation: f32,
    #[serde(rename = "rotationRate", default = "f32::default")]
    pub rotation_rate: f32,
    #[serde(default = "f32::default")]
    pub time: f32,
    #[serde(rename = "xPos", default = "f32::default")]
    pub x_pos: f32,
    #[serde(rename = "xVel", default = "f32::default")]
    pub x_vel: f32,
    #[serde(rename = "yPos", default = "f32::default")]
    pub y_pos: f32,
    #[serde(rename = "yVel", default = "f32::default")]
    pub y_vel: f32,
}

// "2p": false,
// "btn": 1,
// "correction": {
//     "nodeXPos": 1403.365478515625,
//     "nodeYPos": 567.1383666992188,
//     "player2": false,
//     "rotation": 443.49798583984375,
//     "rotationRate": -415.3846130371094,
//     "time": 5.591680086666608,
//     "xPos": 1403.365478515625,
//     "xVel": 0,
//     "yPos": 567.1383666992188,
//     "yVel": 6.552
// },
// "down": true,
// "frame": 1342
#[derive(Serialize, Deserialize)]
pub struct Input {
    #[serde(rename = "2p")]
    pub player2: bool,
    #[serde(rename = "btn")]
    pub button: i32,
    #[serde(default = "Correction::default")]
    pub correction: Correction,
    pub down: bool,
    pub frame: u32,
}

impl Input {
    #[inline]
    pub fn new(frame: u32, button: i32, player2: bool, down: bool) -> Self {
        Self {
            frame,
            button,
            player2,
            down,
            correction: Correction::default(),
        }
    }

    #[inline]
    pub fn hold(frame: u32, button: i32, player2: bool) -> Self {
        Self::new(frame, button, player2, true)
    }

    #[inline]
    pub fn release(frame: u32, button: i32, player2: bool) -> Self {
        Self::new(frame, button, player2, false)
    }
}

const fn default_framerate() -> f32 {
    240.0
}

#[derive(Serialize, Deserialize)]
pub struct Replay {
    #[serde(default = "String::new")]
    pub author: String,
    #[serde(default = "String::new")]
    pub description: String,
    #[serde(default = "f32::default")]
    pub duration: f32,
    #[serde(rename = "gameVersion")]
    pub game_version: f32,
    #[serde(default = "f32::default")]
    pub version: f32,
    #[serde(default = "default_framerate")]
    pub framerate: f32,
    #[serde(default = "i32::default")]
    pub seed: i32,
    #[serde(default = "i32::default")]
    pub coins: i32,
    #[serde(default = "bool::default")]
    pub ldm: bool,
    #[serde(default = "BotInfo::default")]
    pub bot: BotInfo,
    #[serde(default = "LevelInfo::default")]
    pub level: LevelInfo,
    #[serde(default = "Vec::new")]
    pub inputs: Vec<Input>,
}

impl Default for Replay {
    fn default() -> Self {
        Self {
            author: String::new(),
            description: String::new(),
            duration: 0.0,
            game_version: 0.0,
            version: 1.0,
            framerate: 240.0,
            seed: 0,
            coins: 0,
            ldm: false,
            bot: BotInfo::default(),
            level: LevelInfo::default(),
            inputs: Vec::new(),
        }
    }
}

impl Replay {
    pub fn from_slice(data: &[u8]) -> Result<Self, serde_json::Error> {
        rmp_serde::from_slice(data)
            .map_err(|e| {
                log::warn!("failed to parse messagepack GDR file: {e}")
            })
            .or_else(|_| serde_json::from_slice(data))
    }

    #[inline]
    pub fn frame_for_time(&self, time: f32) -> u32 {
        (time * self.framerate) as u32
    }
}
