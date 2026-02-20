use crate::{ExtendedAction, Player, Replay as ZcbReplay, ReplayType};
use anyhow::{Context, Result};
use byteorder::{LittleEndian, WriteBytesExt};
use indexmap::IndexMap;
use serde::Serialize;
use serde_json::{Map, Value};
use slc_oxide::input::InputData;
use slc_oxide::replay::Replay;
use std::collections::HashMap;
use std::io::{Cursor, Seek, Write};
use tcm::replay::ReplaySerializer;

pub struct Writer {
    fps: f64,
    duration: f64,
    actions: Vec<Action>,
    extended_map: HashMap<u32, ExtendedAction>,
}

#[derive(Clone, Copy)]
struct Action {
    time: f64,
    player: Player,
    down: bool,
    frame: u32,
}

impl Writer {
    pub fn new(replay: ZcbReplay) -> Self {
        let extended_map = replay.extended.iter().map(|e| (e.frame, *e)).collect();

        let actions = replay
            .actions
            .iter()
            .map(|a| Action {
                time: a.time,
                player: a.player,
                down: a.click.is_click(),
                frame: a.frame,
            })
            .collect();

        Self {
            fps: replay.fps,
            duration: replay.duration,
            actions,
            extended_map,
        }
    }

    pub fn write<W: Write + Seek>(&self, typ: ReplayType, writer: W) -> Result<W> {
        match typ {
            ReplayType::Mhr => self.write_mhr(writer),
            ReplayType::TasBot => self.write_tasbot(writer),
            ReplayType::Zbot => self.write_zbf(writer),
            ReplayType::Obot => self.write_obot2(writer),
            ReplayType::Ybotf => self.write_ybotf(writer),
            ReplayType::MhrBin => self.write_mhrbin(writer),
            ReplayType::Echo => self.write_echo(writer),
            ReplayType::Amethyst => self.write_amethyst(writer),
            ReplayType::OsuReplay => self.write_osr(writer),
            ReplayType::Gdmo => self.write_gdmo(writer),
            ReplayType::ReplayBot => self.write_replaybot(writer),
            ReplayType::Rush => self.write_rush(writer),
            ReplayType::Kdbot => self.write_kdbot(writer),
            ReplayType::Txt => self.write_plaintext(writer),
            ReplayType::ReplayEngine => self.write_re(writer),
            ReplayType::Ddhor => self.write_ddhor(writer),
            ReplayType::Xbot => self.write_xbot(writer),
            ReplayType::Ybot2 => self.write_ybot2(writer),
            ReplayType::XdBot => self.write_xdbot(writer),
            ReplayType::Gdr => self.write_gdr(writer),
            ReplayType::Qbot => self.write_qbot(writer),
            ReplayType::Rbot => self.write_rbot(writer),
            ReplayType::Zephyrus => self.write_zephyrus(writer),
            ReplayType::ReplayEngine2 => self.write_re2(writer),
            ReplayType::ReplayEngine3 => self.write_re3(writer),
            ReplayType::Gdr2 => self.write_gdr2(writer),
            ReplayType::Silicate => self.write_slc(writer),
            ReplayType::Silicate2 => self.write_slc2(writer),
            ReplayType::Silicate3 => self.write_slc3(writer),
            ReplayType::UvBot => self.write_uvbot(writer),
            ReplayType::TcBot => self.write_tcm(writer),
        }
    }

    fn get_extended(&self, frame: u32, _player2: bool) -> Option<ExtendedAction> {
        self.extended_map.get(&frame).copied()
    }

    fn write_mhr<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        let mut events = Vec::new();

        for action in &self.actions {
            let mut event = Map::new();
            event.insert("frame".to_string(), Value::from(action.frame as i64));
            event.insert("down".to_string(), Value::from(action.down));
            if action.player == Player::Two {
                event.insert("p2".to_string(), Value::from(true));
            }
            if let Some(ext) = self.get_extended(action.frame, action.player == Player::Two) {
                if ext.x != 0.0 {
                    event.insert("x".to_string(), Value::from(ext.x as f64));
                }
                if ext.y != 0.0 {
                    event.insert("y".to_string(), Value::from(ext.y as f64));
                }
                if ext.y_accel != 0.0 {
                    event.insert("a".to_string(), Value::from(ext.y_accel as f64));
                }
                if ext.rot != 0.0 {
                    event.insert("r".to_string(), Value::from(ext.rot as f64));
                }
            }
            events.push(event);
        }

        let output = serde_json::json!({
            "meta": {
                "fps": self.fps
            },
            "events": events
        });

        serde_json::to_writer_pretty(&mut writer, &output)?;
        Ok(writer)
    }

    fn write_tasbot<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        use std::collections::HashMap;

        let mut frame_actions: HashMap<u32, (Option<bool>, Option<bool>)> = HashMap::new();

        for action in &self.actions {
            let entry = frame_actions.entry(action.frame).or_insert((None, None));
            if action.player == Player::One {
                entry.0 = Some(action.down);
            } else {
                entry.1 = Some(action.down);
            }
        }

        let mut frames: Vec<u32> = frame_actions.keys().cloned().collect();
        frames.sort();

        let mut macro_events = Vec::new();

        for frame in frames {
            let (p1_down, p2_down) = frame_actions[&frame];

            let mut ev = serde_json::Map::new();
            ev.insert("frame".to_string(), serde_json::json!(frame));

            let mut p1 = serde_json::Map::new();
            let mut p2 = serde_json::Map::new();

            if let Some(down) = p1_down {
                p1.insert(
                    "click".to_string(),
                    serde_json::json!(if down { 1 } else { 2 }),
                );
                if let Some(ext) = self.get_extended(frame, false) {
                    p1.insert("x_position".to_string(), serde_json::json!(ext.x as f64));
                }
            } else {
                p1.insert("click".to_string(), serde_json::json!(0));
            }

            if let Some(down) = p2_down {
                p2.insert(
                    "click".to_string(),
                    serde_json::json!(if down { 1 } else { 2 }),
                );
                if let Some(ext) = self.get_extended(frame, true) {
                    p2.insert("x_position".to_string(), serde_json::json!(ext.x as f64));
                }
            } else {
                p2.insert("click".to_string(), serde_json::json!(0));
            }

            ev.insert("player_1".to_string(), p1.into());
            ev.insert("player_2".to_string(), p2.into());

            macro_events.push(ev);
        }

        let output = serde_json::json!({
            "fps": self.fps,
            "macro": macro_events
        });

        serde_json::to_writer_pretty(&mut writer, &output)?;
        Ok(writer)
    }

    fn write_zbf<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        let delta = 1.0 / self.fps as f32;
        writer.write_f32::<LittleEndian>(delta)?;
        writer.write_f32::<LittleEndian>(1.0f32)?; // speedhack

        for action in &self.actions {
            writer.write_i32::<LittleEndian>(action.frame as i32)?;
            writer.write_u8(if action.down { 0x31 } else { 0x30 })?;
            writer.write_u8(if action.player == Player::One {
                0x31
            } else {
                0x30
            })?;
        }

        Ok(writer)
    }

    fn write_obot2<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        #[derive(Serialize)]
        struct ObotClick {
            location: ObotLocation,
            click_type: ObotClickType,
        }

        #[derive(Serialize)]
        #[serde(untagged)]
        enum ObotLocation {
            Frame(u32),
        }

        #[derive(Serialize)]
        #[serde(untagged)]
        enum ObotClickType {
            Player1Down,
            Player1Up,
            Player2Down,
            Player2Up,
        }

        #[derive(Serialize)]
        struct ObotReplay {
            #[serde(rename = "initial_fps")]
            initial_fps: f32,
            #[serde(rename = "current_fps")]
            current_fps: f32,
            #[serde(rename = "replay_type")]
            replay_type: String,
            #[serde(rename = "current_click")]
            current_click: usize,
            clicks: Vec<ObotClick>,
        }

        let clicks: Vec<ObotClick> = self
            .actions
            .iter()
            .map(|a| ObotClick {
                location: ObotLocation::Frame(a.frame),
                click_type: if a.player == Player::One {
                    if a.down {
                        ObotClickType::Player1Down
                    } else {
                        ObotClickType::Player1Up
                    }
                } else if a.down {
                    ObotClickType::Player2Down
                } else {
                    ObotClickType::Player2Up
                },
            })
            .collect();

        let replay = ObotReplay {
            initial_fps: self.fps as f32,
            current_fps: self.fps as f32,
            replay_type: "Frame".to_string(),
            current_click: 0,
            clicks,
        };

        bincode::serialize_into(&mut writer, &replay)
            .context("failed to serialize obot2 replay")?;
        Ok(writer)
    }

    fn write_ybotf<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        writer.write_f32::<LittleEndian>(self.fps as f32)?;
        writer.write_i32::<LittleEndian>(self.actions.len() as i32)?;

        for action in &self.actions {
            writer.write_u32::<LittleEndian>(action.frame)?;
            let state = (if action.down { 2 } else { 0 })
                | (if action.player == Player::Two { 1 } else { 0 });
            writer.write_u32::<LittleEndian>(state)?;
        }

        Ok(writer)
    }

    fn write_mhrbin<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        use byteorder::BigEndian;

        writer.write_u32::<BigEndian>(0x4841434B)?; // HACK magic
        writer.write_u16::<BigEndian>(1)?; // version
        writer.write_u16::<BigEndian>(0)?; // padding
        writer.seek(std::io::SeekFrom::Start(12))?;
        writer.write_u32::<LittleEndian>(self.fps as u32)?;
        writer.seek(std::io::SeekFrom::Start(28))?;
        writer.write_u32::<LittleEndian>(self.actions.len() as u32)?;

        for action in &self.actions {
            writer.write_u16::<LittleEndian>(0)?; // padding
            writer.write_u8(if action.down { 1 } else { 0 })?;
            writer.write_u8(if action.player == Player::One { 0 } else { 1 })?;
            writer.write_u32::<LittleEndian>(action.frame)?;
            writer.write_all(&[0u8; 24])?; // padding
        }

        Ok(writer)
    }

    fn write_echo<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        let mut inputs = Vec::new();

        for action in &self.actions {
            let mut input = serde_json::Map::new();
            input.insert("frame".to_string(), serde_json::json!(action.frame));
            input.insert("holding".to_string(), serde_json::json!(action.down));

            if action.player == Player::Two {
                input.insert("player_2".to_string(), serde_json::json!(true));
            }

            if let Some(ext) = self.get_extended(action.frame, action.player == Player::Two) {
                if ext.x != 0.0 {
                    input.insert("x_position".to_string(), serde_json::json!(ext.x as f64));
                }
                if ext.y_accel != 0.0 {
                    input.insert("y_vel".to_string(), serde_json::json!(ext.y_accel as f64));
                }
                if ext.rot != 0.0 {
                    input.insert("rotation".to_string(), serde_json::json!(ext.rot as f64));
                }
            }

            inputs.push(input);
        }

        let output = serde_json::json!({
            "fps": self.fps,
            "inputs": inputs
        });

        serde_json::to_writer_pretty(&mut writer, &output)?;
        Ok(writer)
    }

    fn write_amethyst<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        let mut p1_clicks = Vec::new();
        let mut p1_releases = Vec::new();
        let mut p2_clicks = Vec::new();
        let mut p2_releases = Vec::new();

        for action in &self.actions {
            let time = action.frame as f64 / self.fps;
            if action.player == Player::One {
                if action.down {
                    p1_clicks.push(time.to_string());
                } else {
                    p1_releases.push(time.to_string());
                }
            } else {
                if action.down {
                    p2_clicks.push(time.to_string());
                } else {
                    p2_releases.push(time.to_string());
                }
            }
        }

        writeln!(writer, "{}", p1_clicks.len())?;
        for t in p1_clicks {
            writeln!(writer, "{}", t)?;
        }
        writeln!(writer, "{}", p1_releases.len())?;
        for t in p1_releases {
            writeln!(writer, "{}", t)?;
        }
        writeln!(writer, "{}", p2_clicks.len())?;
        for t in p2_clicks {
            writeln!(writer, "{}", t)?;
        }
        writeln!(writer, "{}", p2_releases.len())?;
        for t in p2_releases {
            writeln!(writer, "{}", t)?;
        }

        Ok(writer)
    }

    fn write_osr<W: Write + Seek>(&self, _writer: W) -> Result<W> {
        anyhow::bail!("osr writing not implemented yet")
    }

    fn write_gdmo<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        writer.write_f32::<LittleEndian>(self.fps as f32)?;
        writer.write_u32::<LittleEndian>(self.actions.len() as u32)?;
        writer.write_u32::<LittleEndian>(0)?; // num frame captures

        #[repr(C)]
        struct GdmoAction {
            press: bool,
            player2: bool,
            frame: u32,
            y_accel: f64,
            px: f32,
            py: f32,
        }

        for action in &self.actions {
            let ext = self.get_extended(action.frame, action.player == Player::Two);
            let (px, py, y_accel) = ext
                .map(|e| (e.x, e.y, e.y_accel as f64))
                .unwrap_or((0.0, 0.0, 0.0));

            let gdmo_action = GdmoAction {
                press: action.down,
                player2: action.player == Player::Two,
                frame: action.frame,
                y_accel,
                px,
                py,
            };

            let bytes = unsafe {
                std::slice::from_raw_parts(
                    &gdmo_action as *const GdmoAction as *const u8,
                    std::mem::size_of::<GdmoAction>(),
                )
            };
            writer.write_all(bytes)?;
        }

        Ok(writer)
    }

    fn write_replaybot<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        const REPLAYBOT_MAGIC: [u8; 4] = *b"RPLY";
        writer.write_all(&REPLAYBOT_MAGIC)?;
        writer.write_u8(2)?; // version
        writer.write_u8(1)?; // frame replay
        writer.write_f32::<LittleEndian>(self.fps as f32)?;

        for action in &self.actions {
            writer.write_u32::<LittleEndian>(action.frame)?;
            let state = (action.down as u8) | ((action.player == Player::Two) as u8) << 1;
            writer.write_u8(state)?;
        }

        Ok(writer)
    }

    fn write_rush<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        writer.write_i16::<LittleEndian>(self.fps as i16)?;

        for action in &self.actions {
            writer.write_i32::<LittleEndian>(action.frame as i32)?;
            let state = (action.down as u8) | ((action.player == Player::Two) as u8) << 1;
            writer.write_u8(state)?;
        }

        Ok(writer)
    }

    fn write_kdbot<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        writer.write_f32::<LittleEndian>(self.fps as f32)?;

        for action in &self.actions {
            writer.write_i32::<LittleEndian>(action.frame as i32)?;
            writer.write_u8(if action.down { 1 } else { 0 })?;
            writer.write_u8(if action.player == Player::Two { 1 } else { 0 })?;
        }

        Ok(writer)
    }

    fn write_plaintext<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        writeln!(writer, "{}", self.fps as u32)?;

        for action in &self.actions {
            let pbutton = 1i32; // jump
            let p2 = if action.player == Player::Two { 0 } else { 1 };
            writeln!(
                writer,
                "{} {} {} {}",
                action.frame,
                if action.down { 1 } else { 0 },
                pbutton,
                p2
            )?;
        }

        Ok(writer)
    }

    fn write_re<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        use std::mem::size_of;

        writer.write_f32::<LittleEndian>(self.fps as f32)?;

        let mut frame_data_list: Vec<FrameData> = Vec::new();
        for action in &self.actions {
            let player2 = action.player == Player::Two;
            if let Some(ext) = self.get_extended(action.frame, player2) {
                frame_data_list.push(FrameData {
                    frame: action.frame,
                    x: ext.x,
                    y: ext.y,
                    rot: ext.rot,
                    y_accel: ext.y_accel as f64,
                    player2,
                });
            }
        }

        writer.write_u32::<LittleEndian>(frame_data_list.len() as u32)?;
        writer.write_u32::<LittleEndian>(self.actions.len() as u32)?;

        for fd in &frame_data_list {
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    fd as *const FrameData as *const u8,
                    size_of::<FrameData>(),
                )
            };
            writer.write_all(bytes)?;
        }

        #[repr(C)]
        struct ActionDataNew {
            frame: u32,
            hold: bool,
            button: i32,
            player2: bool,
        }

        for action in &self.actions {
            let ad = ActionDataNew {
                frame: action.frame,
                hold: action.down,
                button: 1,
                player2: action.player == Player::Two,
            };
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    &ad as *const ActionDataNew as *const u8,
                    size_of::<ActionDataNew>(),
                )
            };
            writer.write_all(bytes)?;
        }

        Ok(writer)
    }

    fn write_ddhor<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        const DDHOR_MAGIC: &[u8; 4] = b"DDHR";
        writer.write_all(DDHOR_MAGIC)?;
        writer.write_i16::<LittleEndian>(self.fps as i16)?;

        let p1_actions: Vec<_> = self
            .actions
            .iter()
            .filter(|a| a.player == Player::One)
            .collect();
        let p2_actions: Vec<_> = self
            .actions
            .iter()
            .filter(|a| a.player == Player::Two)
            .collect();

        writer.write_i32::<LittleEndian>(p1_actions.len() as i32)?;
        writer.write_i32::<LittleEndian>(p2_actions.len() as i32)?;

        for action in p1_actions.iter().chain(p2_actions.iter()) {
            writer.write_f32::<LittleEndian>(action.frame as f32)?;
            writer.write_u8(if action.down { 0 } else { 1 })?;
        }

        Ok(writer)
    }

    fn write_xbot<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        writeln!(writer, "{}", self.fps as u32)?;
        writeln!(writer, "frames")?;

        for action in &self.actions {
            let state = (action.down as u8) + ((action.player == Player::Two) as u8) * 2;
            writeln!(writer, "{} {}", state, action.frame)?;
        }

        Ok(writer)
    }

    fn write_ybot2<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        use ybot_fmt::*;

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut m = ybot_fmt::Macro::create(&mut cursor)?;
            m.set(Meta::DATE, chrono::Utc::now().timestamp())?;
            m.set(Meta::PRESSES, self.actions.len() as u64)?;
            m.set(
                Meta::FRAMES,
                self.actions.last().map(|a| a.frame).unwrap_or(0) as u64 + 1,
            )?;
            m.set(Meta::FPS, self.fps as f32)?;
            m.set(Meta::TOTAL_PRESSES, self.actions.len() as u64)?;

            let mut prev_frame = 0u32;
            for action in &self.actions {
                let delta = (action.frame - prev_frame) as u64;
                let timed_action = TimedAction {
                    delta,
                    action: Action::Button(
                        action.player == Player::One,
                        action.down,
                        PlayerButton::Jump,
                    ),
                };
                m.add(timed_action)?;
                prev_frame = action.frame;
            }
        }

        writer.write_all(cursor.into_inner().as_slice())?;
        Ok(writer)
    }

    fn write_xdbot<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        writeln!(writer, "{}", self.fps as u32)?;

        for action in &self.actions {
            let ext = self.get_extended(action.frame, action.player == Player::Two);
            let (x, y) = ext.map(|e| (e.x, e.y)).unwrap_or((0.0, 0.0));

            writeln!(
                writer,
                "{}|{}|{}|{}|0|{}|{}|0|0|0|0|0|0|0|0|0",
                action.frame,
                if action.down { 1 } else { 0 },
                1,
                if action.player == Player::One { 1 } else { 0 },
                x,
                y
            )?;
        }

        Ok(writer)
    }

    fn write_gdr<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        let mut inputs = Vec::new();

        for action in &self.actions {
            let ext = self.get_extended(action.frame, action.player == Player::Two);
            let (x_pos, y_pos, y_vel, rotation) = ext
                .map(|e| (e.x, e.y, e.y_accel as f32, e.rot))
                .unwrap_or((0.0, 0.0, 0.0, 0.0));

            inputs.push(gdr::Input {
                player2: action.player == Player::Two,
                button: 1,
                down: action.down,
                correction: gdr::Correction {
                    node_x_pos: x_pos,
                    node_y_pos: y_pos,
                    player2: action.player == Player::Two,
                    rotation,
                    rotation_rate: 0.0,
                    time: action.time as f32,
                    x_pos,
                    x_vel: 0.0,
                    y_pos,
                    y_vel,
                },
                frame: action.frame,
            });
        }

        let replay = gdr::Replay {
            author: String::new(),
            description: String::new(),
            duration: self.duration as f32,
            game_version: 0.0,
            version: 1.0,
            framerate: self.fps as f32,
            seed: 0,
            coins: 0,
            ldm: false,
            bot: gdr::BotInfo::default(),
            level: gdr::LevelInfo::default(),
            inputs,
        };

        //if let Ok(data) = rmp_serde::to_vec(&replay) {
        //    writer.write_all(&data)?;
        //} else {
        //    serde_json::to_writer(&mut writer, &replay)?;
        //}
        serde_json::to_writer(&mut writer, &replay)?;

        Ok(writer)
    }

    fn write_qbot<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        use dlhn::Serializer;

        #[derive(Serialize)]

        enum PlayerButton {
            Jump = 1,
            //Left = 2,
            //Right = 3,
        }

        #[derive(Serialize)]
        #[allow(dead_code)]
        enum Action {
            Button {
                #[serde(rename = "is_p2")]
                is_p2: bool,
                push: bool,
                button: PlayerButton,
            },
            FPS(f32),
        }

        #[derive(Serialize)]
        struct Position {
            x: f32,
            y: f32,
            rotate: f32,
        }

        #[derive(Serialize)]
        struct Click {
            frame: u32,
            time: f64,
            action: Action,
            #[serde(rename = "x_vel")]
            x_vel: f64,
            y_vel: f64,
            position: Option<Position>,
        }

        #[derive(Serialize)]
        struct Replay {
            #[serde(rename = "initial_fps")]
            initial_fps: f32,
            fps: f32,
            index: usize,
            clicks: Vec<Click>,
        }

        let clicks: Vec<Click> = self
            .actions
            .iter()
            .map(|action| {
                let ext = self.get_extended(action.frame, action.player == Player::Two);
                let position = ext.map(|e| Position {
                    x: e.x,
                    y: e.y,
                    rotate: e.rot,
                });
                let y_vel = ext.map(|e| e.y_accel as f64).unwrap_or(0.0);

                Click {
                    frame: action.frame,
                    time: action.time,
                    action: Action::Button {
                        is_p2: action.player == Player::Two,
                        push: action.down,
                        button: PlayerButton::Jump,
                    },
                    x_vel: 0.0,
                    y_vel,
                    position,
                }
            })
            .collect();

        let replay = Replay {
            initial_fps: self.fps as f32,
            fps: self.fps as f32,
            index: 0,
            clicks,
        };

        let mut serializer = Serializer::new(&mut writer);
        replay
            .serialize(&mut serializer)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(writer)
    }

    fn write_rbot<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        writer.write_u32::<LittleEndian>(self.fps as u32)?;
        writer.write_u32::<LittleEndian>(self.actions.len() as u32)?;

        for action in &self.actions {
            writer.write_u32::<LittleEndian>(action.frame)?;
            writer.write_u8(if action.down { 1 } else { 0 })?;
            writer.write_u8(if action.player == Player::One { 1 } else { 0 })?;
        }

        Ok(writer)
    }

    fn write_zephyrus<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        writer.write_u16::<LittleEndian>(0x525a)?; // magic
        writer.write_u8(2)?; // version
        writer.write_u32::<LittleEndian>(self.fps as u32)?;
        writer.write_u32::<LittleEndian>(self.actions.len() as u32)?;
        writer.write_u32::<LittleEndian>(0)?; // num frame fixes

        // Write actions
        for action in &self.actions {
            let flags = ((action.player == Player::Two) as u8) << 7
                | ((action.down as u8) << 6)
                | (1i32 << 4) as u8; // button = jump

            writer.write_u32::<LittleEndian>(action.frame)?;
            writer.write_u8(flags)?;
        }

        Ok(writer)
    }

    fn write_re2<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        use std::mem::size_of;

        const RE2_MAGIC: [u8; 3] = *b"RE2";
        writer.write_all(&RE2_MAGIC)?;
        writer.write_u32::<LittleEndian>(self.actions.len() as u32)?;

        #[repr(C)]
        struct FrameData {
            frame: u32,
            hold: bool,
            button: i32,
            player2: bool,
        }

        for action in &self.actions {
            let fd = FrameData {
                frame: action.frame,
                hold: action.down,
                button: 1,
                player2: action.player == Player::Two,
            };
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    &fd as *const FrameData as *const u8,
                    size_of::<FrameData>(),
                )
            };
            writer.write_all(bytes)?;
        }

        Ok(writer)
    }

    fn write_re3<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        use std::mem::size_of;

        writer.write_f32::<LittleEndian>(self.fps as f32)?;

        // Separate p1 and p2 actions
        let p1_actions: Vec<_> = self
            .actions
            .iter()
            .filter(|a| a.player == Player::One)
            .collect();
        let p2_actions: Vec<_> = self
            .actions
            .iter()
            .filter(|a| a.player == Player::Two)
            .collect();

        // Build frame data maps
        let mut p1_frame_data: IndexMap<u32, FrameDataRe3> = IndexMap::new();
        let mut p2_frame_data: IndexMap<u32, FrameDataRe3> = IndexMap::new();

        for action in &self.actions {
            let ext = self.get_extended(action.frame, action.player == Player::Two);
            let fd = FrameDataRe3 {
                frame: action.frame,
                x: ext.map(|e| e.x).unwrap_or(0.0),
                y: ext.map(|e| e.y).unwrap_or(0.0),
                rot: ext.map(|e| e.rot).unwrap_or(0.0),
                y_accel: ext.map(|e| e.y_accel as f64).unwrap_or(0.0),
                player2: action.player == Player::Two,
            };

            if action.player == Player::One {
                p1_frame_data.insert(action.frame, fd);
            } else {
                p2_frame_data.insert(action.frame, fd);
            }
        }

        writer.write_u32::<LittleEndian>(p1_frame_data.len() as u32)?;
        writer.write_u32::<LittleEndian>(p2_frame_data.len() as u32)?;
        writer.write_u32::<LittleEndian>(p1_actions.len() as u32)?;
        writer.write_u32::<LittleEndian>(p2_actions.len() as u32)?;

        // Write frame data
        for (_, fd) in &p1_frame_data {
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    fd as *const FrameDataRe3 as *const u8,
                    size_of::<FrameDataRe3>(),
                )
            };
            writer.write_all(bytes)?;
        }
        for (_, fd) in &p2_frame_data {
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    fd as *const FrameDataRe3 as *const u8,
                    size_of::<FrameDataRe3>(),
                )
            };
            writer.write_all(bytes)?;
        }

        // Write action data
        #[repr(C)]
        struct ActionDataRe3 {
            frame: u32,
            down: bool,
            button: i32,
            player1: bool,
        }

        for action in p1_actions {
            let ad = ActionDataRe3 {
                frame: action.frame,
                down: action.down,
                button: 1,
                player1: true,
            };
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    &ad as *const ActionDataRe3 as *const u8,
                    size_of::<ActionDataRe3>(),
                )
            };
            writer.write_all(bytes)?;
        }
        for action in p2_actions {
            let ad = ActionDataRe3 {
                frame: action.frame,
                down: action.down,
                button: 1,
                player1: false,
            };
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    &ad as *const ActionDataRe3 as *const u8,
                    size_of::<ActionDataRe3>(),
                )
            };
            writer.write_all(bytes)?;
        }

        Ok(writer)
    }

    fn write_gdr2<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        use gdr2::Replay as Gdr2Replay;
        use gdr2::{Bot, Level};

        let mut inputs = Vec::new();

        for action in &self.actions {
            let ext = self.get_extended(action.frame, action.player == Player::Two);
            let physics = ext.map(|e| gdr2::PhysicsData {
                x_position: e.x,
                y_position: e.y,
                rotation: e.rot,
                x_velocity: 0.0,
                y_velocity: e.y_accel as f64,
            });

            inputs.push(gdr2::Input {
                frame: action.frame as u64,
                button: 1,
                player2: action.player == Player::Two,
                down: action.down,
                physics,
            });
        }

        let replay = Gdr2Replay {
            author: String::new(),
            description: String::new(),
            duration: self.duration as f32,
            game_version: 0,
            framerate: self.fps,
            seed: 0,
            coins: 0,
            ldm: false,
            platformer: false,
            bot_info: Bot::default(),
            level_info: Level::default(),
            inputs,
            deaths: Vec::new(),
        };

        let data = replay.export_data()?;
        writer.write_all(&data)?;
        Ok(writer)
    }

    fn write_slc<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        writer.write_f64::<LittleEndian>(self.fps)?;
        writer.write_u32::<LittleEndian>(self.actions.len() as u32)?;

        for action in &self.actions {
            let player2 = action.player == Player::Two;
            let down = action.down;
            let button = 1i32; // jump

            let packed = (action.frame << 4)
                | ((button as u32) << 1)
                | ((player2 as u32) << 3)
                | (down as u32);
            writer.write_u32::<LittleEndian>(packed)?;
        }

        Ok(writer)
    }

    fn write_slc2<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        use slc_oxide::input::PlayerInput;

        let mut replay = Replay::<()>::new(self.fps, ());

        for action in &self.actions {
            replay.add_input(
                action.frame as u64,
                InputData::Player(PlayerInput {
                    button: 1,
                    hold: action.down,
                    player_2: action.player == Player::Two,
                }),
            );
        }

        replay
            .write(&mut writer)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(writer)
    }

    fn write_slc3<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        use slc_oxide::input::PlayerInput;

        let mut replay = Replay::<()>::new(self.fps, ());

        for action in &self.actions {
            replay.add_input(
                action.frame as u64,
                InputData::Player(PlayerInput {
                    button: 1,
                    hold: action.down,
                    player_2: action.player == Player::Two,
                }),
            );
        }

        replay
            .write_v3(&mut writer)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(writer)
    }

    fn write_uvbot<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        writer.write_all(b"UVBOT")?;
        writer.write_u8(2)?; // version
        writer.write_f32::<LittleEndian>(self.fps as f32)?;

        #[derive(Default)]
        struct FrameData {
            p1_physics: Option<UvBotPhysics>,
            p2_physics: Option<UvBotPhysics>,
        }

        let mut frame_data: IndexMap<u64, FrameData> = IndexMap::new();

        for action in &self.actions {
            let frame = action.frame as u64;
            let ext = self.get_extended(action.frame, action.player == Player::Two);
            if let Some(e) = ext {
                let physics = UvBotPhysics {
                    x: e.x,
                    y: e.y,
                    rotation: e.rot,
                    y_velocity: e.y_accel as f64,
                };

                let entry = frame_data.entry(frame).or_default();
                if action.player == Player::One {
                    entry.p1_physics = Some(physics);
                } else {
                    entry.p2_physics = Some(physics);
                }
            }
        }

        let input_count = self.actions.len() as i32;
        let mut p1_phys_count = 0i32;
        let mut p2_phys_count = 0i32;

        for (_, fd) in &frame_data {
            if fd.p1_physics.is_some() {
                p1_phys_count += 1;
            }
            if fd.p2_physics.is_some() {
                p2_phys_count += 1;
            }
        }

        writer.write_i32::<LittleEndian>(input_count)?;
        writer.write_i32::<LittleEndian>(p1_phys_count)?;
        writer.write_i32::<LittleEndian>(p2_phys_count)?;

        for action in &self.actions {
            let frame = action.frame as u64;
            let flags =
                (action.down as u8) | ((1u8) << 1) | ((action.player == Player::Two) as u8) << 2;
            writer.write_u64::<LittleEndian>(frame)?;
            writer.write_u8(flags)?;
        }

        for (frame, fd) in &frame_data {
            if let Some(ref phys) = fd.p1_physics {
                writer.write_u64::<LittleEndian>(*frame)?;
                writer.write_f32::<LittleEndian>(phys.x)?;
                writer.write_f32::<LittleEndian>(phys.y)?;
                writer.write_f32::<LittleEndian>(phys.rotation)?;
                writer.write_f64::<LittleEndian>(phys.y_velocity)?;
            }
        }

        for (frame, fd) in &frame_data {
            if let Some(ref phys) = fd.p2_physics {
                writer.write_u64::<LittleEndian>(*frame)?;
                writer.write_f32::<LittleEndian>(phys.x)?;
                writer.write_f32::<LittleEndian>(phys.y)?;
                writer.write_f32::<LittleEndian>(phys.rotation)?;
                writer.write_f64::<LittleEndian>(phys.y_velocity)?;
            }
        }

        writer.write_all(b"TOBVU")?;

        Ok(writer)
    }

    fn write_tcm<W: Write + Seek>(&self, mut writer: W) -> Result<W> {
        use tcm::input::{Input, InputCommand, PlayerButton, VanillaInput};
        use tcm::meta::MetaV2;
        use tcm::replay::Replay;

        let inputs: Vec<InputCommand> = self
            .actions
            .iter()
            .map(|action| InputCommand {
                frame: action.frame as tcm::Frame,
                input: Input::Vanilla(VanillaInput {
                    button: PlayerButton::Jump,
                    push: action.down,
                    player2: action.player == Player::One,
                }),
            })
            .collect();

        let meta = MetaV2::new(self.fps as f32, 0, None);
        let replay = Replay::new(meta, inputs);
        replay.serialize(&mut writer)?;
        Ok(writer)
    }
}

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
struct FrameDataRe3 {
    frame: u32,
    x: f32,
    y: f32,
    rot: f32,
    y_accel: f64,
    player2: bool,
}

struct UvBotPhysics {
    x: f32,
    y: f32,
    rotation: f32,
    y_velocity: f64,
}

#[cfg(test)]
mod tests {
    use crate::*;
    use std::io::Cursor;

    fn create_test_replay() -> Replay {
        let mut replay = Replay::build();
        replay.fps = 240.0;
        replay.extended_data = true;

        for i in 0..50 {
            let frame = i * 240;
            let time = i as f64;

            if i % 2 == 0 {
                replay.actions.push(Action::new(
                    time,
                    Player::One,
                    Click::Regular(ClickType::Click),
                    0.0,
                    frame,
                ));
                replay.extended.push(ExtendedAction {
                    player2: false,
                    down: true,
                    frame,
                    x: 100.0 + i as f32,
                    y: 200.0 + i as f32,
                    y_accel: 0.5 + i as f32,
                    rot: 90.0 + i as f32,
                    fps_change: None,
                });
            } else {
                replay.actions.push(Action::new(
                    time,
                    Player::One,
                    Click::Regular(ClickType::Release),
                    0.0,
                    frame,
                ));
                replay.extended.push(ExtendedAction {
                    player2: false,
                    down: false,
                    frame,
                    x: 100.0 + i as f32,
                    y: 200.0 + i as f32,
                    y_accel: 0.5 + i as f32,
                    rot: 90.0 + i as f32,
                    fps_change: None,
                });
            }
        }

        for i in 0..25 {
            let frame = i * 480;
            let time = (i * 2) as f64;

            if i % 2 == 0 {
                replay.actions.push(Action::new(
                    time,
                    Player::Two,
                    Click::Regular(ClickType::Click),
                    0.0,
                    frame,
                ));
                replay.extended.push(ExtendedAction {
                    player2: true,
                    down: true,
                    frame,
                    x: 300.0 + i as f32,
                    y: 400.0 + i as f32,
                    y_accel: 1.0 + i as f32,
                    rot: 180.0 + i as f32,
                    fps_change: None,
                });
            } else {
                replay.actions.push(Action::new(
                    time,
                    Player::Two,
                    Click::Regular(ClickType::Release),
                    0.0,
                    frame,
                ));
                replay.extended.push(ExtendedAction {
                    player2: true,
                    down: false,
                    frame,
                    x: 300.0 + i as f32,
                    y: 400.0 + i as f32,
                    y_accel: 1.0 + i as f32,
                    rot: 180.0 + i as f32,
                    fps_change: None,
                });
            }
        }

        replay.sort_actions();
        replay.duration = replay.actions.last().map(|a| a.time).unwrap_or(0.0);
        replay
    }

    fn test_roundtrip(typ: ReplayType, _ext: &str) {
        let original = create_test_replay();
        let writer = original.to_writer();

        let mut buffer = Cursor::new(Vec::new());
        writer
            .write(typ, &mut buffer)
            .expect(&format!("Failed to write {:?}", typ));

        let data = buffer.into_inner();
        println!("{:?} ({} bytes)", typ, data.len());

        let parsed = Replay::build()
            .with_extended(true)
            .parse(typ, Cursor::new(&data))
            .expect(&format!("Failed to parse {:?}", typ));

        assert!(
            !parsed.actions.is_empty(),
            "No actions parsed for {:?}",
            typ
        );

        assert!(
            (parsed.fps - original.fps).abs() < 1.0,
            "FPS mismatch for {:?}: expected {}, got {}",
            typ,
            original.fps,
            parsed.fps
        );

        assert_eq!(
            original.actions.len(),
            parsed.actions.len(),
            "Action count mismatch for {:?}: expected {}, got {}",
            typ,
            original.actions.len(),
            parsed.actions.len()
        );

        let mut orig_sorted: Vec<_> = original.actions.iter().collect();
        let mut parsed_sorted: Vec<_> = parsed.actions.iter().collect();
        orig_sorted.sort_by_key(|a| (a.frame, a.player));
        parsed_sorted.sort_by_key(|a| (a.frame, a.player));

        for (i, (orig, parsed_action)) in orig_sorted.iter().zip(parsed_sorted.iter()).enumerate() {
            if orig.frame != parsed_action.frame
                && (orig.frame as f32 - parsed_action.frame as f32).abs() >= 1.0
            {
                panic!(
                    "Frame mismatch for {:?} at index {}: expected {}, got {}",
                    typ, i, orig.frame, parsed_action.frame
                );
            }
            if orig.player != parsed_action.player {
                panic!(
                    "Player mismatch for {:?} at index {} frame {}: expected {:?}, got {:?}",
                    typ, i, orig.frame, orig.player, parsed_action.player
                );
            }
            if orig.click.is_click() != parsed_action.click.is_click() {
                panic!(
                    "Click state mismatch for {:?} at index {} frame {}",
                    typ, i, orig.frame
                );
            }
        }
    }

    fn test_roundtrip_no_fps(typ: ReplayType, _ext: &str) {
        let original = create_test_replay();
        let writer = original.to_writer();

        let mut buffer = Cursor::new(Vec::new());
        writer
            .write(typ, &mut buffer)
            .expect(&format!("Failed to write {:?}", typ));

        let data = buffer.into_inner();
        println!("{:?} ({} bytes)", typ, data.len());

        let parsed = Replay::build()
            .with_extended(true)
            .with_override_fps(Some(original.fps))
            .parse(typ, Cursor::new(&data))
            .expect(&format!("Failed to parse {:?}", typ));

        assert!(
            !parsed.actions.is_empty(),
            "No actions parsed for {:?}",
            typ
        );

        assert_eq!(
            original.actions.len(),
            parsed.actions.len(),
            "Action count mismatch for {:?}: expected {}, got {}",
            typ,
            original.actions.len(),
            parsed.actions.len()
        );

        let mut orig_sorted: Vec<_> = original.actions.iter().collect();
        let mut parsed_sorted: Vec<_> = parsed.actions.iter().collect();
        orig_sorted.sort_by_key(|a| (a.frame, a.player));
        parsed_sorted.sort_by_key(|a| (a.frame, a.player));

        for (i, (orig, parsed_action)) in orig_sorted.iter().zip(parsed_sorted.iter()).enumerate() {
            if orig.frame != parsed_action.frame
                && (orig.frame as f32 - parsed_action.frame as f32).abs() >= 1.0
            {
                panic!(
                    "Frame mismatch for {:?} at index {}: expected {}, got {}",
                    typ, i, orig.frame, parsed_action.frame
                );
            }
            if orig.player != parsed_action.player {
                panic!(
                    "Player mismatch for {:?} at index {} frame {}: expected {:?}, got {:?}",
                    typ, i, orig.frame, orig.player, parsed_action.player
                );
            }
            if orig.click.is_click() != parsed_action.click.is_click() {
                panic!(
                    "Click state mismatch for {:?} at index {} frame {}",
                    typ, i, orig.frame
                );
            }
        }
    }

    #[test]
    fn test_tasbot_json() {
        test_roundtrip(ReplayType::TasBot, "json");
    }

    #[test]
    fn test_mhr_json() {
        test_roundtrip(ReplayType::Mhr, "mhr.json");
    }

    #[test]
    fn test_zbot() {
        test_roundtrip(ReplayType::Zbot, "zbf");
    }

    #[test]
    fn test_ybotf() {
        test_roundtrip(ReplayType::Ybotf, "ybf");
    }

    #[test]
    fn test_echo() {
        test_roundtrip(ReplayType::Echo, "echo");
    }

    #[test]
    fn test_replaybot() {
        test_roundtrip(ReplayType::ReplayBot, "replaybot");
    }

    #[test]
    fn test_rush() {
        test_roundtrip(ReplayType::Rush, "rsh");
    }

    #[test]
    fn test_plaintext() {
        test_roundtrip(ReplayType::Txt, "txt");
    }

    #[test]
    fn test_xbot() {
        test_roundtrip(ReplayType::Xbot, "xbot");
    }

    #[test]
    fn test_ybot2() {
        test_roundtrip(ReplayType::Ybot2, "ybot");
    }

    #[test]
    fn test_xdbot() {
        test_roundtrip(ReplayType::XdBot, "xd");
    }

    #[test]
    fn test_gdr() {
        test_roundtrip(ReplayType::Gdr, "gdr");
    }

    #[test]
    fn test_rbot() {
        test_roundtrip(ReplayType::Rbot, "rbot");
    }

    #[test]
    fn test_zephyrus() {
        test_roundtrip(ReplayType::Zephyrus, "zr");
    }

    #[test]
    fn test_re2() {
        test_roundtrip(ReplayType::ReplayEngine2, "re2");
    }

    #[test]
    fn test_gdr2() {
        test_roundtrip(ReplayType::Gdr2, "gdr2");
    }

    #[test]
    fn test_silicate() {
        test_roundtrip(ReplayType::Silicate, "slc");
    }

    #[test]
    fn test_silicate2() {
        test_roundtrip(ReplayType::Silicate2, "slc2");
    }

    #[test]
    fn test_silicate3() {
        test_roundtrip(ReplayType::Silicate3, "slc3");
    }

    #[test]
    fn test_gdmo() {
        test_roundtrip(ReplayType::Gdmo, "macro");
    }

    #[test]
    fn test_empty_replay() {
        let original = Replay::build();
        let writer = original.to_writer();

        let mut buffer = Cursor::new(Vec::new());
        writer
            .write(ReplayType::TasBot, &mut buffer)
            .expect("Failed to write empty replay");

        let data = buffer.into_inner();
        let parsed = Replay::build()
            .parse(ReplayType::TasBot, Cursor::new(&data))
            .expect("Failed to parse empty replay");

        assert_eq!(parsed.actions.len(), 0);
    }

    #[test]
    fn test_single_action() {
        let mut replay = Replay::build();
        replay.fps = 240.0;
        replay.extended_data = true;

        replay.actions.push(Action::new(
            0.0,
            Player::One,
            Click::Regular(ClickType::Click),
            0.0,
            0,
        ));
        replay.extended.push(ExtendedAction {
            player2: false,
            down: true,
            frame: 0,
            x: 100.0,
            y: 200.0,
            y_accel: 0.5,
            rot: 90.0,
            fps_change: None,
        });

        let writer = replay.to_writer();

        let mut buffer = Cursor::new(Vec::new());
        writer
            .write(ReplayType::TasBot, &mut buffer)
            .expect("Failed to write single action");

        let data = buffer.into_inner();
        let parsed = Replay::build()
            .parse(ReplayType::TasBot, Cursor::new(&data))
            .expect("Failed to parse single action");

        assert_eq!(parsed.actions.len(), 1);
    }

    #[test]
    fn test_mhrbin() {
        test_roundtrip(ReplayType::MhrBin, "mhr");
    }

    #[test]
    fn test_amethyst() {
        test_roundtrip_no_fps(ReplayType::Amethyst, "thyst");
    }

    #[test]
    fn test_kdbot() {
        test_roundtrip(ReplayType::Kdbot, "kd");
    }

    #[test]
    fn test_ddhor() {
        test_roundtrip(ReplayType::Ddhor, "ddhor");
    }

    #[test]
    fn test_re() {
        test_roundtrip(ReplayType::ReplayEngine, "re");
    }

    #[test]
    fn test_qbot() {
        test_roundtrip(ReplayType::Qbot, "qb");
    }

    #[test]
    fn test_re3() {
        test_roundtrip(ReplayType::ReplayEngine3, "re3");
    }

    #[test]
    fn test_uvbot() {
        test_roundtrip(ReplayType::UvBot, "uv");
    }

    #[test]
    fn test_tcm() {
        test_roundtrip(ReplayType::TcBot, "tcm");
    }
}
