//! ToastyReplay Lite (`.ttrl`) replay format, ported from https://github.com/ToastexGD/ToastyReplay-Lite

use crate::{
    ttr::ToastyAction,
    {ButtonKind, Player},
};
use anyhow::{Context, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

const TTRL_MAGIC: [u8; 4] = *b"TTRL";
const TTRL_VERSION: u8 = 1;

const FLAG_FRAME_FIX: u8 = 1;
const FLAG_INPUT_CHANNELS: u8 = 2;
const FLAG_PLATFORMER: u8 = 4;
const FLAG_SEED: u8 = 8;
const FLAG_START_POS: u8 = 16;
const FLAG_CONTROL_FLIP: u8 = 32;
const KNOWN_FLAGS: u8 = FLAG_FRAME_FIX
    | FLAG_INPUT_CHANNELS
    | FLAG_PLATFORMER
    | FLAG_SEED
    | FLAG_START_POS
    | FLAG_CONTROL_FLIP;

const SCHEMA_LEGACY: u8 = 1;
const SCHEMA_POSITION: u8 = 2;
const SCHEMA_FULL: u8 = 3;

pub const DEFAULT_GAME_VERSION: u32 = 22081; // TODO: add ui for this? hopefully toastyreplay doesnt break old macros

const MAX_FILE_SIZE: usize = 64 * 1024 * 1024;
const MAX_INPUT_SIZE: usize = 8 * 1024 * 1024;
const MAX_INPUT_EVENTS: usize = 1024 * 1024;
const MAX_FRAME_FIXES: usize = 1024 * 1024;
const MAX_FRAME_FIX_SIZE: usize = MAX_FILE_SIZE - MAX_INPUT_SIZE;
const MIN_FRAME_FIX_RECORD_SIZE: u64 = 21;
const MIN_FILE_SIZE: usize = 25;

#[derive(Debug, Clone, Copy)]
pub struct TtrlFix {
    pub frame: u32,
    pub player: Player,
    pub x: f32,
    pub y: f32,
    pub rot: f32,
    pub y_vel: f64,
}

#[derive(Debug, Default)]
pub struct TtrlReplay {
    pub fps: f64,
    pub seed: Option<u64>,
    pub game_version: u32,
    pub actions: Vec<ToastyAction>,
    pub fixes: Vec<TtrlFix>,
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn crc32_ieee(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ 0xedb8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

fn read_varint<R: Read>(reader: &mut R) -> Result<u64> {
    let mut value = 0u64;
    for shift in (0..64).step_by(7) {
        let byte = reader.read_u8()?;
        if shift == 63 && (byte & 0xfe) != 0 {
            anyhow::bail!("TTRL varint is too long");
        }
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            if shift != 0 && byte == 0 {
                anyhow::bail!("TTRL varint is not canonical");
            }
            return Ok(value);
        }
    }
    anyhow::bail!("TTRL varint is too long");
}

fn write_varint<W: Write>(writer: &mut W, mut value: u64) -> Result<()> {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            writer.write_u8(byte)?;
            return Ok(());
        }
        writer.write_u8(byte | 0x80)?;
    }
}

fn button_from_code(code: u64) -> Result<ButtonKind> {
    match code {
        2 => Ok(ButtonKind::Left),
        3 => Ok(ButtonKind::Right),
        _ => Ok(ButtonKind::Jump),
    }
}

fn button_code(button: ButtonKind) -> u64 {
    match button {
        ButtonKind::Jump => 1,
        ButtonKind::Left => 2,
        ButtonKind::Right => 3,
    }
}

fn checked_tick(previous: u64, delta: u64, tick_count: u64) -> Result<u64> {
    let tick = previous
        .checked_add(delta)
        .context("TTRL tick overflowed")?;
    if tick >= tick_count {
        anyhow::bail!("TTRL tick {tick} is outside the replay ({tick_count} ticks)");
    }
    Ok(tick)
}

pub fn parse<R: Read>(mut reader: R) -> Result<TtrlReplay> {
    let mut data = Vec::new();
    reader.read_to_end(&mut data)?;
    if data.len() < MIN_FILE_SIZE {
        anyhow::bail!("TTRL file is too small");
    }
    if data.len() > MAX_FILE_SIZE {
        anyhow::bail!("TTRL file is too large");
    }

    let payload_len = data.len() - 4;
    let stored = u32::from_le_bytes(data[payload_len..].try_into().unwrap());
    if crc32_ieee(&data[..payload_len]) != stored {
        anyhow::bail!("TTRL checksum mismatch");
    }

    let mut cursor = std::io::Cursor::new(&data[..payload_len]);
    let mut magic = [0u8; 4];
    cursor.read_exact(&mut magic)?;
    if magic != TTRL_MAGIC {
        anyhow::bail!("not a TTRL replay");
    }
    if cursor.read_u8()? != TTRL_VERSION {
        anyhow::bail!("unsupported TTRL version");
    }
    let flags = cursor.read_u8()?;
    if flags & !KNOWN_FLAGS != 0 {
        anyhow::bail!("TTRL replay uses unsupported features");
    }
    let channels = flags & FLAG_INPUT_CHANNELS != 0;

    let numerator = read_varint(&mut cursor)?;
    let denominator = read_varint(&mut cursor)?;
    if numerator == 0 || denominator == 0 {
        anyhow::bail!("TTRL replay has invalid TPS");
    }
    let divisor = gcd(numerator, denominator);
    if numerator / divisor != numerator || denominator / divisor != denominator {
        anyhow::bail!("TTRL replay has non-normalized TPS");
    }
    let fps = numerator as f64 / denominator as f64;

    let game_version = read_varint(&mut cursor)?;
    if game_version == 0 || game_version > u64::from(u32::MAX) {
        anyhow::bail!("TTRL replay has invalid game version");
    }

    let _level_id = read_varint(&mut cursor)?;
    let _level_revision = read_varint(&mut cursor)?;
    let _fingerprint = cursor.read_u64::<LittleEndian>()?;
    let tick_count = read_varint(&mut cursor)?;

    let seed = if flags & FLAG_SEED != 0 {
        Some(read_varint(&mut cursor)?)
    } else {
        None
    };
    if flags & FLAG_START_POS != 0 {
        let _start_pos = cursor.read_f32::<LittleEndian>()?;
    }
    if flags & FLAG_CONTROL_FLIP != 0 {
        let _control_flip = cursor.read_u8()?;
    }

    let input_size = read_varint(&mut cursor)? as usize;
    if input_size > MAX_INPUT_SIZE {
        anyhow::bail!("TTRL input section is too large");
    }
    let remaining = (payload_len as u64).saturating_sub(cursor.position()) as usize;
    if input_size > remaining {
        anyhow::bail!("TTRL file is truncated");
    }
    let input_end = cursor.position() + input_size as u64;

    let mut replay = TtrlReplay {
        fps,
        seed,
        game_version: game_version as u32,
        ..TtrlReplay::default()
    };

    let mut previous_tick = 0u64;
    while cursor.position() < input_end {
        let event = read_varint(&mut cursor)?;
        let (state, shift) = if channels {
            (event & 0xf, 4)
        } else {
            (event & 1, 1)
        };
        let tick = checked_tick(previous_tick, event >> shift, tick_count)?;

        let (button, player) = if channels {
            let code = (state >> 2) + 1;
            if code > 3 {
                anyhow::bail!("TTRL replay has invalid input button");
            }
            (
                button_from_code(code)?,
                if state & 2 != 0 {
                    Player::Two
                } else {
                    Player::One
                },
            )
        } else {
            (ButtonKind::Jump, Player::One)
        };

        replay.actions.push(ToastyAction {
            time: tick as f64 / fps,
            frame: u32::try_from(tick).context("TTRL tick exceeds u32")?,
            player,
            button,
            down: state & 1 != 0,
        });
        if replay.actions.len() > MAX_INPUT_EVENTS {
            anyhow::bail!("TTRL replay has too many inputs");
        }
        previous_tick = tick;
    }

    if flags & FLAG_FRAME_FIX != 0 {
        let schema = cursor.read_u8()?;
        if schema != SCHEMA_LEGACY && schema != SCHEMA_POSITION && schema != SCHEMA_FULL {
            anyhow::bail!("TTRL replay has unsupported frame fix schema");
        }
        let section_size = read_varint(&mut cursor)? as usize;
        if section_size > MAX_FRAME_FIX_SIZE {
            anyhow::bail!("TTRL frame fix section is too large");
        }
        let remaining = (payload_len as u64).saturating_sub(cursor.position()) as usize;
        if section_size > remaining {
            anyhow::bail!("TTRL file is truncated");
        }
        let section_end = cursor.position() + section_size as u64;

        let count = read_varint(&mut cursor)?;
        if count > MAX_FRAME_FIXES as u64 {
            anyhow::bail!("TTRL replay has too many frame fixes");
        }
        let section_start = section_end - section_size as u64;
        let remaining = section_size as u64 - (cursor.position() - section_start);
        if count > remaining / MIN_FRAME_FIX_RECORD_SIZE {
            anyhow::bail!("TTRL replay has invalid frame fix count");
        }

        let mut previous_tick = 0u64;
        for _ in 0..count {
            let value = read_varint(&mut cursor)?;
            let (player, delta) = if schema == SCHEMA_LEGACY {
                (Player::One, value)
            } else {
                (
                    if value & 1 != 0 {
                        Player::Two
                    } else {
                        Player::One
                    },
                    value >> 1,
                )
            };
            let tick = checked_tick(previous_tick, delta, tick_count)?;
            let x = cursor.read_f32::<LittleEndian>()?;
            let y = cursor.read_f32::<LittleEndian>()?;
            let rot = cursor.read_f32::<LittleEndian>()?;
            let y_vel = cursor.read_f64::<LittleEndian>()?;
            if !x.is_finite() || !y.is_finite() || !rot.is_finite() || !y_vel.is_finite() {
                anyhow::bail!("TTRL replay has invalid frame fix");
            }
            if schema == SCHEMA_FULL {
                let state = read_varint(&mut cursor)?;
                if state > u64::from(u16::MAX) {
                    anyhow::bail!("TTRL replay has invalid frame fix");
                }
                let _vehicle_size = cursor.read_f32::<LittleEndian>()?;
                let _player_speed = cursor.read_f32::<LittleEndian>()?;
                let _gravity_mod = cursor.read_f32::<LittleEndian>()?;
                if !_vehicle_size.is_finite()
                    || !_player_speed.is_finite()
                    || !_gravity_mod.is_finite()
                {
                    anyhow::bail!("TTRL replay has invalid frame fix");
                }
            }
            replay.fixes.push(TtrlFix {
                frame: u32::try_from(tick).context("TTRL tick exceeds u32")?,
                player,
                x,
                y,
                rot,
                y_vel,
            });
            previous_tick = tick;
        }

        if cursor.position() != section_end {
            anyhow::bail!("TTRL frame fix section has trailing data");
        }
    }

    if cursor.position() != payload_len as u64 {
        anyhow::bail!("TTRL file has trailing data");
    }

    Ok(replay)
}

pub fn write<W: Write>(
    mut writer: W,
    fps: f64,
    seed: Option<u64>,
    game_version: u32,
    actions: &[ToastyAction],
    fixes: &[TtrlFix],
) -> Result<W> {
    if !fps.is_finite() || fps <= 0.0 {
        anyhow::bail!("TTRL requires a positive finite FPS");
    }
    if game_version == 0 {
        anyhow::bail!("TTRL requires a non-zero game version");
    }

    let mut numerator = (fps * 1000.0).round() as u64;
    let mut denominator = 1000u64;
    if numerator == 0 {
        anyhow::bail!("TTRL requires a positive finite FPS");
    }
    let divisor = gcd(numerator, denominator);
    numerator /= divisor;
    denominator /= divisor;

    let channels = actions
        .iter()
        .any(|action| action.button != ButtonKind::Jump || action.player == Player::Two);
    let platformer = actions
        .iter()
        .any(|action| action.button != ButtonKind::Jump);

    let mut actions: Vec<ToastyAction> = actions.to_vec();
    actions.sort_by_key(|action| action.frame);
    let mut fixes: Vec<TtrlFix> = fixes.to_vec();
    fixes.sort_by_key(|fix| (fix.frame, fix.player));

    let max_tick = actions
        .iter()
        .map(|action| u64::from(action.frame))
        .chain(fixes.iter().map(|fix| u64::from(fix.frame)))
        .max()
        .unwrap_or(0);
    let tick_count = max_tick + 1;

    let mut input_bytes = Vec::new();
    let mut previous_tick = 0u64;
    for action in &actions {
        let tick = u64::from(action.frame);
        let delta = tick - previous_tick;
        if channels {
            let state = ((button_code(action.button) - 1) << 2)
                | if action.player == Player::Two { 2 } else { 0 }
                | u64::from(action.down);
            write_varint(&mut input_bytes, (delta << 4) | state)?;
        } else {
            write_varint(&mut input_bytes, (delta << 1) | u64::from(action.down))?;
        }
        previous_tick = tick;
    }

    let mut fix_bytes = Vec::new();
    if !fixes.is_empty() {
        write_varint(&mut fix_bytes, fixes.len() as u64)?;
        let mut previous_tick = 0u64;
        for fix in &fixes {
            if !fix.x.is_finite()
                || !fix.y.is_finite()
                || !fix.rot.is_finite()
                || !fix.y_vel.is_finite()
            {
                anyhow::bail!("TTRL frame fix physics must be finite");
            }
            let tick = u64::from(fix.frame);
            let delta = tick - previous_tick;
            let player = if fix.player == Player::Two { 1 } else { 0 };
            write_varint(&mut fix_bytes, (delta << 1) | player)?;
            fix_bytes.write_f32::<LittleEndian>(fix.x)?;
            fix_bytes.write_f32::<LittleEndian>(fix.y)?;
            fix_bytes.write_f32::<LittleEndian>(fix.rot)?;
            fix_bytes.write_f64::<LittleEndian>(fix.y_vel)?;
            write_varint(&mut fix_bytes, 0)?;
            fix_bytes.write_f32::<LittleEndian>(1.0)?;
            fix_bytes.write_f32::<LittleEndian>(1.0)?;
            fix_bytes.write_f32::<LittleEndian>(1.0)?;
            previous_tick = tick;
        }
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&TTRL_MAGIC);
    bytes.push(TTRL_VERSION);
    let mut flags = 0u8;
    if !fixes.is_empty() {
        flags |= FLAG_FRAME_FIX;
    }
    if channels {
        flags |= FLAG_INPUT_CHANNELS;
    }
    if platformer {
        flags |= FLAG_PLATFORMER;
    }
    if seed.is_some() {
        flags |= FLAG_SEED;
    }
    bytes.push(flags);
    write_varint(&mut bytes, numerator)?;
    write_varint(&mut bytes, denominator)?;
    write_varint(&mut bytes, u64::from(game_version))?;
    write_varint(&mut bytes, 0)?; // level id
    write_varint(&mut bytes, 0)?; // level revision
    bytes.write_u64::<LittleEndian>(0)?; // level fingerprint
    write_varint(&mut bytes, tick_count)?;
    if let Some(seed) = seed {
        write_varint(&mut bytes, seed)?;
    }
    write_varint(&mut bytes, input_bytes.len() as u64)?;
    bytes.extend_from_slice(&input_bytes);

    if !fix_bytes.is_empty() {
        bytes.push(SCHEMA_FULL);
        write_varint(&mut bytes, fix_bytes.len() as u64)?;
        bytes.extend_from_slice(&fix_bytes);
    }

    let checksum = crc32_ieee(&bytes);
    bytes.write_u32::<LittleEndian>(checksum)?;
    writer.write_all(&bytes)?;
    Ok(writer)
}
