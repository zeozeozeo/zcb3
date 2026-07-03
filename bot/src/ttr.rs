use crate::{ButtonKind, Player};
use anyhow::{Context, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

const LEGACY_MAGIC: [u8; 4] = *b"TTR\0";
const TTR2_MAGIC: [u8; 4] = *b"TTR2";
const TTR3_MAGIC: [u8; 4] = *b"TTR3";

const TTR_FLAG_ACCURACY_CBS: u32 = 1 << 0;
const TTR_FLAG_TWO_PLAYER: u32 = 1 << 3;
const TTR_FLAG_ACCURACY_CBF: u32 = 1 << 5;
const TTR_FLAG_ACCURACY_SUBSTEP: u32 = 1 << 6;
const TTR_FLAG_EXACT_CBS_TIMING: u32 = 1 << 7;
const TTR_FLAG_PERSISTENCE: u32 = 1 << 8;

const TTR3_FLAG_TWO_PLAYER: u32 = 1 << 3;
const TTR3_FLAG_HAS_PERSISTENCE: u32 = 1 << 7;
const TTR3_FLAG_COMPRESSED: u32 = 1 << 10;

const TTR3_SECTION_INPUTS: u8 = 1;
const TTR3_SECTION_PERSISTENCE: u8 = 5;
const TTR3_NATIVE_SOURCE_FORMAT_ID: u64 = 0x0000_0000_FFFF_0003;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastyFormat {
    Legacy,
    Ttr2,
    Ttr3,
}

#[derive(Debug, Clone, Copy)]
pub struct ToastyAction {
    pub time: f64,
    pub frame: u32,
    pub player: Player,
    pub button: ButtonKind,
    pub down: bool,
}

#[derive(Debug, Default)]
pub struct ToastyReplay {
    pub fps: f64,
    pub duration: f64,
    pub actions: Vec<ToastyAction>,
}

#[derive(Clone, Copy)]
struct TtrInput {
    tick: i32,
    action_type: u8,
    flags: u8,
    step_offset: f32,
    cbs_time_offset: f64,
    time_seconds: f64,
}

impl TtrInput {
    fn player(self) -> Player {
        if self.flags & 0x01 != 0 {
            Player::Two
        } else {
            Player::One
        }
    }

    fn down(self) -> bool {
        self.flags & 0x02 != 0
    }

    fn button(self) -> ButtonKind {
        match self.action_type {
            2 => ButtonKind::Left,
            3 => ButtonKind::Right,
            _ => ButtonKind::Jump,
        }
    }

    fn resolved_time(self, fps: f64) -> f64 {
        if self.time_seconds.is_finite() && self.time_seconds >= 0.0 {
            self.time_seconds
        } else {
            let mut time = self.tick.max(0) as f64 / fps;
            if self.cbs_time_offset.is_finite() && self.cbs_time_offset >= 0.0 {
                time += self.cbs_time_offset;
            } else if self.step_offset.is_finite() && self.step_offset > 0.0 {
                time += self.step_offset as f64 / fps;
            }
            time
        }
    }
}

#[derive(Clone, Copy)]
struct SharedMetadata {
    fps: f64,
    duration: f64,
}

#[derive(Clone, Copy)]
struct Ttr3Section {
    kind: u8,
    offset: u64,
    size: u64,
}

#[derive(Clone, Copy)]
struct Ttr3Attempt {
    death_time_seconds: f64,
}

fn write_string(buf: &mut Vec<u8>, value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    let len = bytes.len().min(u16::MAX as usize) as u16;
    buf.write_u16::<LittleEndian>(len)?;
    buf.extend_from_slice(&bytes[..len as usize]);
    Ok(())
}

fn read_string<R: Read>(reader: &mut R) -> Result<String> {
    let len = reader.read_u16::<LittleEndian>()? as usize;
    let mut buf = vec![0; len];
    reader.read_exact(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn write_var_u32<W: Write>(writer: &mut W, mut value: u32) -> Result<()> {
    while value >= 0x80 {
        writer.write_u8((value as u8 & 0x7f) | 0x80)?;
        value >>= 7;
    }
    writer.write_u8(value as u8)?;
    Ok(())
}

fn read_var_u32<R: Read>(reader: &mut R) -> Result<u32> {
    let mut value = 0u32;
    let mut shift = 0u32;
    loop {
        let byte = reader.read_u8()?;
        value |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
        shift += 7;
        if shift >= 35 {
            anyhow::bail!("Toasty varint is too long");
        }
    }
}

fn button_code(button: ButtonKind) -> u8 {
    match button {
        ButtonKind::Jump => 1,
        ButtonKind::Left => 2,
        ButtonKind::Right => 3,
    }
}

fn read_shared_metadata<R: Read + Seek>(
    reader: &mut R,
    header_size: u64,
) -> Result<SharedMetadata> {
    let _author = read_string(reader)?;
    let _name = read_string(reader)?;
    let _level_name = read_string(reader)?;
    let _level_id = reader.read_i32::<LittleEndian>()?;
    let fps = reader.read_f64::<LittleEndian>()?;
    let duration = reader.read_f64::<LittleEndian>()?;
    let _game_version = reader.read_u32::<LittleEndian>()?;
    let _start_pos_x = reader.read_f32::<LittleEndian>()?;
    let _start_pos_y = reader.read_f32::<LittleEndian>()?;
    let _rng_seed = reader.read_u32::<LittleEndian>()?;
    let _record_timestamp = reader.read_i64::<LittleEndian>()?;

    let pos = reader.stream_position()?;
    if pos < header_size {
        reader.seek(SeekFrom::Start(header_size))?;
    }

    Ok(SharedMetadata { fps, duration })
}

fn build_action(input: TtrInput, fps: f64) -> Result<ToastyAction> {
    let frame = u32::try_from(input.tick.max(0)).context("Toasty tick exceeds u32")?;
    Ok(ToastyAction {
        time: input.resolved_time(fps),
        frame,
        player: input.player(),
        button: input.button(),
        down: input.down(),
    })
}

fn skip_bytes<R: Read>(reader: &mut R, len: usize) -> Result<()> {
    let mut limited = reader.take(len as u64);
    std::io::copy(&mut limited, &mut std::io::sink())?;
    Ok(())
}

fn skip_legacy_player_snapshot_v1<R: Read>(reader: &mut R) -> Result<()> {
    skip_bytes(reader, 8 * 5 + 4 + 1)
}

fn skip_legacy_player_snapshot_v2<R: Read>(reader: &mut R) -> Result<()> {
    skip_bytes(reader, 4 * 2 + 8 * 3 + 4 + 1)
}

fn skip_anchor_player<R: Read>(reader: &mut R, include_extended: bool) -> Result<()> {
    // x, y, rotation, gravity as f32 + 3 f64 velocities + 2 flag bytes = 42 bytes
    skip_bytes(reader, 42)?;
    if include_extended {
        skip_bytes(reader, 116)?;
    }
    Ok(())
}

fn skip_legacy_anchor<R: Read>(
    reader: &mut R,
    version: u16,
    has_extended_anchors: bool,
) -> Result<()> {
    if version >= 3 {
        let flags = reader.read_u8()?;
        let has_player2 = flags & 0x01 != 0;
        skip_anchor_player(reader, has_extended_anchors)?;
        let _ = reader.read_u8()?;
        if has_player2 {
            skip_anchor_player(reader, has_extended_anchors)?;
            let _ = reader.read_u8()?;
        }
        if flags & 0x02 != 0 {
            skip_bytes(reader, 8)?;
        }
    } else {
        let flags = reader.read_u8()?;
        let has_player2 = flags & 0x01 != 0;
        skip_legacy_player_snapshot_v2(reader)?;
        if has_player2 {
            skip_legacy_player_snapshot_v2(reader)?;
        }
    }
    Ok(())
}

fn skip_ttr2_anchor<R: Read>(reader: &mut R) -> Result<()> {
    let flags = reader.read_u8()?;
    let has_player2 = flags & 0x01 != 0;
    skip_anchor_player(reader, true)?;
    let _ = reader.read_u8()?;
    if has_player2 {
        skip_anchor_player(reader, true)?;
        let _ = reader.read_u8()?;
    }
    if flags & 0x02 != 0 {
        skip_bytes(reader, 8)?;
    }
    Ok(())
}

fn skip_ttr3_player_state<R: Read>(reader: &mut R) -> Result<()> {
    skip_bytes(reader, 42)?;
    skip_bytes(reader, 116)?;
    Ok(())
}

fn skip_ttr3_anchor<R: Read>(reader: &mut R) -> Result<()> {
    let _time = reader.read_f64::<LittleEndian>()?;
    let flags = reader.read_u8()?;
    let has_player2 = flags & 0x01 != 0;
    let has_rng = flags & 0x02 != 0;
    skip_bytes(reader, 3)?;
    skip_ttr3_player_state(reader)?;
    if has_player2 {
        skip_ttr3_player_state(reader)?;
    }
    if has_rng {
        skip_bytes(reader, 8)?;
    }
    skip_bytes(reader, 8)?;
    Ok(())
}

fn skip_ttr3_anchors<R: Read>(reader: &mut R) -> Result<()> {
    let count = reader.read_u64::<LittleEndian>()?;
    for _ in 0..count {
        skip_ttr3_anchor(reader)?;
    }
    Ok(())
}

fn parse_legacy_payload(
    payload: &[u8],
    fps: f64,
    version: u16,
    flags: u32,
) -> Result<Vec<ToastyAction>> {
    let mut cursor = Cursor::new(payload);
    let has_timed_offsets =
        flags & (TTR_FLAG_ACCURACY_CBS | TTR_FLAG_ACCURACY_CBF | TTR_FLAG_ACCURACY_SUBSTEP) != 0;
    let has_exact_cbs_timing = version >= 6 && flags & TTR_FLAG_EXACT_CBS_TIMING != 0;
    let has_extended_anchors = version >= 6;

    let input_count = cursor.read_u32::<LittleEndian>()?;
    let mut actions = Vec::with_capacity(input_count as usize);

    if version == 1 {
        for _ in 0..input_count {
            let input = TtrInput {
                tick: cursor.read_i32::<LittleEndian>()?,
                action_type: cursor.read_u8()?,
                flags: cursor.read_u8()?,
                step_offset: cursor.read_f32::<LittleEndian>()?,
                cbs_time_offset: -1.0,
                time_seconds: -1.0,
            };
            actions.push(build_action(input, fps)?);
        }

        let anchor_count = cursor.read_u32::<LittleEndian>()?;
        for _ in 0..anchor_count {
            let _tick = cursor.read_i32::<LittleEndian>()?;
            skip_legacy_player_snapshot_v1(&mut cursor)?;
            skip_legacy_player_snapshot_v1(&mut cursor)?;
        }
    } else {
        let mut previous_tick = 0i32;
        for _ in 0..input_count {
            previous_tick += i32::try_from(read_var_u32(&mut cursor)?)
                .context("Toasty delta tick exceeds i32")?;
            let input = TtrInput {
                tick: previous_tick,
                action_type: cursor.read_u8()?,
                flags: cursor.read_u8()?,
                step_offset: if has_timed_offsets {
                    cursor.read_f32::<LittleEndian>()?
                } else {
                    0.0
                },
                cbs_time_offset: if has_exact_cbs_timing {
                    cursor.read_f32::<LittleEndian>()? as f64
                } else {
                    -1.0
                },
                time_seconds: -1.0,
            };
            actions.push(build_action(input, fps)?);
        }

        let anchor_count = cursor.read_u32::<LittleEndian>()?;
        for _ in 0..anchor_count {
            let _delta_tick = read_var_u32(&mut cursor)?;
            skip_legacy_anchor(&mut cursor, version, has_extended_anchors)?;
        }
    }

    if cursor.position() < payload.len() as u64 {
        let checkpoint_count = cursor.read_u32::<LittleEndian>()?;
        for _ in 0..checkpoint_count {
            skip_bytes(&mut cursor, 4 + 8 + 4)?;
        }
    }

    Ok(actions)
}

fn parse_ttr2_inputs<R: Read>(
    reader: &mut R,
    fps: f64,
    has_cbs_timing: bool,
) -> Result<Vec<ToastyAction>> {
    let input_count = reader.read_u32::<LittleEndian>()?;
    let mut actions = Vec::with_capacity(input_count as usize);
    let mut previous_tick = 0i32;
    for _ in 0..input_count {
        previous_tick +=
            i32::try_from(read_var_u32(reader)?).context("ToastyReplay2 delta tick exceeds i32")?;
        let cbs_time_offset = if has_cbs_timing {
            reader.read_f64::<LittleEndian>()?
        } else {
            -1.0
        };
        let input = TtrInput {
            tick: previous_tick,
            action_type: reader.read_u8()?,
            flags: reader.read_u8()?,
            step_offset: if cbs_time_offset.is_finite() && cbs_time_offset >= 0.0 {
                (cbs_time_offset * fps) as f32
            } else {
                0.0
            },
            cbs_time_offset,
            time_seconds: -1.0,
        };
        actions.push(build_action(input, fps)?);
    }
    Ok(actions)
}

fn parse_ttr2_attempt<R: Read>(
    reader: &mut R,
    fps: f64,
    has_cbs_timing: bool,
    base_tick: i32,
    base_time: f64,
) -> Result<(Vec<ToastyAction>, i32, f64)> {
    let death_tick = reader.read_i32::<LittleEndian>()?;
    let _death_player2 = reader.read_u8()?;

    let inputs = parse_ttr2_inputs(reader, fps, has_cbs_timing)?;

    let anchor_count = reader.read_u32::<LittleEndian>()?;
    for _ in 0..anchor_count {
        let _delta_tick = read_var_u32(reader)?;
        skip_ttr2_anchor(reader)?;
    }

    let mut actions = Vec::with_capacity(inputs.len());
    for action in inputs {
        let frame = u32::try_from((base_tick + action.frame as i32).max(0))
            .context("ToastyReplay2 persistence frame exceeds u32")?;
        actions.push(ToastyAction {
            time: base_time + action.time,
            frame,
            ..action
        });
    }

    let next_base_tick = base_tick + death_tick.max(1);
    let next_base_time = base_time + death_tick.max(1) as f64 / fps;
    Ok((actions, next_base_tick, next_base_time))
}

pub fn parse<R: Read + Seek>(mut reader: R) -> Result<ToastyReplay> {
    let mut magic = [0; 4];
    reader.read_exact(&mut magic)?;
    reader.seek(SeekFrom::Start(0))?;

    match magic {
        LEGACY_MAGIC => parse_legacy(reader),
        TTR2_MAGIC => parse_ttr2(reader),
        TTR3_MAGIC => parse_ttr3(reader),
        _ => anyhow::bail!("invalid ToastyReplay magic"),
    }
}

fn parse_legacy<R: Read + Seek>(mut reader: R) -> Result<ToastyReplay> {
    let mut magic = [0; 4];
    reader.read_exact(&mut magic)?;
    if magic != LEGACY_MAGIC {
        anyhow::bail!("invalid ttr magic");
    }

    let version = reader.read_u16::<LittleEndian>()?;
    let flags = reader.read_u32::<LittleEndian>()?;
    let header_size = u64::from(reader.read_u32::<LittleEndian>()?);
    let meta = read_shared_metadata(&mut reader, header_size)?;
    let fps = if meta.fps > 0.0 { meta.fps } else { 240.0 };

    let actions = if version == 1 {
        let mut payload = Vec::new();
        reader.read_to_end(&mut payload)?;
        parse_legacy_payload(&payload, fps, version, flags)?
    } else {
        let uncompressed_size = reader.read_u32::<LittleEndian>()?;
        let mut decoder = ZlibDecoder::new(reader);
        let mut payload = Vec::new();
        decoder.read_to_end(&mut payload)?;
        if payload.len() != uncompressed_size as usize {
            log::warn!(
                "ttr payload size mismatch: header={} decoded={}",
                uncompressed_size,
                payload.len()
            );
        }
        parse_legacy_payload(&payload, fps, version, flags)?
    };

    Ok(ToastyReplay {
        fps,
        duration: meta.duration,
        actions,
    })
}

fn parse_ttr2<R: Read + Seek>(mut reader: R) -> Result<ToastyReplay> {
    let mut magic = [0; 4];
    reader.read_exact(&mut magic)?;
    if magic != TTR2_MAGIC {
        anyhow::bail!("invalid ttr2 magic");
    }

    let version = reader.read_u16::<LittleEndian>()?;
    if version > 2 {
        anyhow::bail!("unsupported ttr2 version {version}");
    }
    let flags = reader.read_u32::<LittleEndian>()?;
    let header_size = u64::from(reader.read_u32::<LittleEndian>()?);
    let meta = read_shared_metadata(&mut reader, header_size)?;
    let fps = if meta.fps > 0.0 { meta.fps } else { 240.0 };

    let uncompressed_size = reader.read_u32::<LittleEndian>()?;
    let mut decoder = ZlibDecoder::new(reader);
    let mut payload = Vec::new();
    decoder.read_to_end(&mut payload)?;
    if payload.len() != uncompressed_size as usize {
        log::warn!(
            "ttr2 payload size mismatch: header={} decoded={}",
            uncompressed_size,
            payload.len()
        );
    }

    let mut cursor = Cursor::new(payload.as_slice());
    let has_cbs_timing = flags & (TTR_FLAG_ACCURACY_CBS | TTR_FLAG_ACCURACY_CBF) != 0;

    let mut actions = Vec::new();
    let mut base_tick = 0i32;
    let mut base_time = 0.0f64;

    if cursor.position() < payload.len() as u64 {
        let attempt_count_probe_pos = cursor.position();
        let mut main_inputs = parse_ttr2_inputs(&mut cursor, fps, has_cbs_timing)?;

        let anchor_count = cursor.read_u32::<LittleEndian>()?;
        for _ in 0..anchor_count {
            let _delta_tick = read_var_u32(&mut cursor)?;
            skip_ttr2_anchor(&mut cursor)?;
        }

        if cursor.position() < payload.len() as u64 {
            let checkpoint_count = cursor.read_u32::<LittleEndian>()?;
            for _ in 0..checkpoint_count {
                skip_bytes(&mut cursor, 4 + 8 + 4)?;
            }
        }

        if cursor.position() < payload.len() as u64 {
            let attempt_count = cursor.read_u32::<LittleEndian>()?;
            for _ in 0..attempt_count {
                let (attempt_actions, next_tick, next_time) =
                    parse_ttr2_attempt(&mut cursor, fps, has_cbs_timing, base_tick, base_time)?;
                actions.extend(attempt_actions);
                base_tick = next_tick;
                base_time = next_time;
            }
        } else if flags & TTR_FLAG_PERSISTENCE != 0 {
            anyhow::bail!("ttr2 persistence flag set without persistence payload");
        }

        for action in &mut main_inputs {
            action.frame = u32::try_from((base_tick + action.frame as i32).max(0))
                .context("ttr2 main frame exceeds u32")?;
            action.time += base_time;
        }
        actions.extend(main_inputs);

        let _ = attempt_count_probe_pos;
    }

    Ok(ToastyReplay {
        fps,
        duration: meta.duration,
        actions,
    })
}

fn parse_ttr3_inputs<R: Read>(reader: &mut R, fps: f64) -> Result<Vec<ToastyAction>> {
    let count = reader.read_u64::<LittleEndian>()?;
    let mut actions = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let time_seconds = reader.read_f64::<LittleEndian>()?;
        let action_type = reader.read_u8()?;
        let flags = reader.read_u8()?;
        let _reserved = reader.read_u16::<LittleEndian>()?;
        let tick = (time_seconds * fps).floor().clamp(0.0, u32::MAX as f64) as i32;
        let input = TtrInput {
            tick,
            action_type,
            flags,
            step_offset: 0.0,
            cbs_time_offset: -1.0,
            time_seconds,
        };
        actions.push(build_action(input, fps)?);
    }
    Ok(actions)
}

fn parse_ttr3_persistence<R: Read>(reader: &mut R, fps: f64) -> Result<Vec<ToastyAction>> {
    let count = reader.read_u64::<LittleEndian>()?;
    let mut all_actions = Vec::new();
    let mut base_tick = 0i32;
    let mut base_time = 0.0f64;

    for _ in 0..count {
        let attempt = Ttr3Attempt {
            death_time_seconds: reader.read_f64::<LittleEndian>()?,
        };
        let _death_player2 = reader.read_u8()?;
        skip_bytes(reader, 7)?;

        let mut attempt_actions = parse_ttr3_inputs(reader, fps)?;
        skip_ttr3_anchors(reader)?;

        for action in &mut attempt_actions {
            action.frame = u32::try_from((base_tick + action.frame as i32).max(0))
                .context("ttr3 persistence frame exceeds u32")?;
            action.time += base_time;
        }
        all_actions.extend(attempt_actions);

        base_tick = (attempt.death_time_seconds * fps)
            .floor()
            .clamp(1.0, i32::MAX as f64) as i32
            + base_tick;
        base_time += attempt.death_time_seconds.max(0.0);
    }

    Ok(all_actions)
}

fn parse_ttr3<R: Read + Seek>(mut reader: R) -> Result<ToastyReplay> {
    let mut data = Vec::new();
    reader.read_to_end(&mut data)?;
    let mut cursor = Cursor::new(&data);

    let mut magic = [0; 4];
    cursor.read_exact(&mut magic)?;
    if magic != TTR3_MAGIC {
        anyhow::bail!("invalid ttr3 magic");
    }

    let version = cursor.read_u16::<LittleEndian>()?;
    let reserved = cursor.read_u16::<LittleEndian>()?;
    let flags = cursor.read_u32::<LittleEndian>()?;
    let header_len = cursor.read_u32::<LittleEndian>()? as usize;
    if version == 0 {
        anyhow::bail!("unsupported ttr3 version 0");
    }
    if reserved != 0 {
        anyhow::bail!("invalid ttr3 header");
    }

    let _source_format_id = cursor.read_u64::<LittleEndian>()?;
    let _game_version = cursor.read_u64::<LittleEndian>()?;
    let _level_id = cursor.read_i32::<LittleEndian>()?;
    let _level_name = read_string(&mut cursor)?;
    let _author = read_string(&mut cursor)?;
    let fps = cursor.read_f64::<LittleEndian>()?;
    let _start_pos_x = cursor.read_f32::<LittleEndian>()?;
    let _start_pos_y = cursor.read_f32::<LittleEndian>()?;
    let _record_timestamp = cursor.read_i64::<LittleEndian>()?;
    let _rng_seed = cursor.read_u32::<LittleEndian>()?;
    if cursor.position() < header_len as u64 {
        let _accuracy_mode = cursor.read_u8()?;
    }
    cursor.seek(SeekFrom::Start(header_len as u64))?;

    let section_count = cursor.read_u16::<LittleEndian>()?;
    let mut sections = Vec::with_capacity(section_count as usize);
    for _ in 0..section_count {
        let kind = cursor.read_u8()?;
        let _r0 = cursor.read_u8()?;
        let _r1 = cursor.read_u8()?;
        let _r2 = cursor.read_u8()?;
        sections.push(Ttr3Section {
            kind,
            offset: cursor.read_u64::<LittleEndian>()?,
            size: cursor.read_u64::<LittleEndian>()?,
        });
    }

    let payload_start = cursor.position() as usize;
    let payload_bytes = &data[payload_start..];
    let decoded_payload = if flags & TTR3_FLAG_COMPRESSED != 0 {
        let expected_size = sections
            .iter()
            .map(|section| section.offset + section.size)
            .max()
            .unwrap_or(0);
        let mut decoder = ZlibDecoder::new(payload_bytes);
        let mut out = Vec::with_capacity(expected_size as usize);
        decoder.read_to_end(&mut out)?;
        out
    } else {
        payload_bytes.to_vec()
    };

    let fps = if fps > 0.0 { fps } else { 240.0 };
    let mut actions = Vec::new();
    let mut duration = 0.0f64;

    for section in sections {
        let start = usize::try_from(section.offset).context("ttr3 section offset exceeds usize")?;
        let end = usize::try_from(section.offset + section.size)
            .context("ttr3 section size exceeds usize")?;
        if end > decoded_payload.len() || start > end {
            anyhow::bail!("ttr3 section range is out of bounds");
        }
        let mut section_reader = Cursor::new(&decoded_payload[start..end]);
        match section.kind {
            TTR3_SECTION_INPUTS => {
                let main_actions = parse_ttr3_inputs(&mut section_reader, fps)?;
                if let Some(last) = main_actions.last() {
                    duration = duration.max(last.time);
                }
                actions.extend(main_actions);
            }
            TTR3_SECTION_PERSISTENCE => {
                let persistent_actions = parse_ttr3_persistence(&mut section_reader, fps)?;
                if let Some(last) = persistent_actions.last() {
                    duration = duration.max(last.time);
                }
                actions = persistent_actions
                    .into_iter()
                    .chain(actions.into_iter())
                    .collect();
            }
            _ => {}
        }
    }

    if flags & TTR3_FLAG_HAS_PERSISTENCE != 0
        && !actions
            .iter()
            .any(|action| matches!(action.player, Player::One | Player::Two))
    {
        log::debug!("parsed ttr3 persistence flag without additional actions");
    }

    Ok(ToastyReplay {
        fps,
        duration,
        actions,
    })
}

fn write_common_header(
    buf: &mut Vec<u8>,
    magic: [u8; 4],
    version: u16,
    flags: u32,
    fps: f64,
    duration: f64,
) -> Result<()> {
    buf.extend_from_slice(&magic);
    buf.write_u16::<LittleEndian>(version)?;
    buf.write_u32::<LittleEndian>(flags)?;
    let header_size_pos = buf.len();
    buf.write_u32::<LittleEndian>(0)?;
    write_string(buf, "")?;
    write_string(buf, "")?;
    write_string(buf, "")?;
    buf.write_i32::<LittleEndian>(0)?;
    buf.write_f64::<LittleEndian>(fps)?;
    buf.write_f64::<LittleEndian>(duration)?;
    buf.write_u32::<LittleEndian>(0)?;
    buf.write_f32::<LittleEndian>(0.0)?;
    buf.write_f32::<LittleEndian>(0.0)?;
    buf.write_u32::<LittleEndian>(0)?;
    buf.write_i64::<LittleEndian>(0)?;
    let header_size = buf.len() as u32;
    buf[header_size_pos..header_size_pos + 4].copy_from_slice(&header_size.to_le_bytes());
    Ok(())
}

fn write_compressed_payload<W: Write>(writer: &mut W, payload: &[u8]) -> Result<()> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(payload)?;
    let compressed = encoder.finish()?;
    writer.write_u32::<LittleEndian>(payload.len() as u32)?;
    writer.write_all(&compressed)?;
    Ok(())
}

fn sorted_actions(actions: &[ToastyAction]) -> Vec<ToastyAction> {
    let mut actions = actions.to_vec();
    actions.sort_by_key(|action| (action.frame, action.player, action.button, action.down));
    actions
}

pub fn write<W: Write + Seek>(
    mut writer: W,
    format: ToastyFormat,
    fps: f64,
    duration: f64,
    actions: &[ToastyAction],
) -> Result<W> {
    match format {
        ToastyFormat::Legacy => write_legacy(&mut writer, fps, duration, actions)?,
        ToastyFormat::Ttr2 => write_ttr2(&mut writer, fps, duration, actions)?,
        ToastyFormat::Ttr3 => write_ttr3(&mut writer, fps, duration, actions)?,
    }
    Ok(writer)
}

fn write_legacy<W: Write>(
    writer: &mut W,
    fps: f64,
    duration: f64,
    actions: &[ToastyAction],
) -> Result<()> {
    let has_two_player = actions.iter().any(|action| action.player == Player::Two);
    let mut header = Vec::new();
    write_common_header(
        &mut header,
        LEGACY_MAGIC,
        6,
        if has_two_player {
            TTR_FLAG_TWO_PLAYER
        } else {
            0
        },
        fps,
        duration,
    )?;

    let mut payload = Vec::new();
    let actions = sorted_actions(actions);
    payload.write_u32::<LittleEndian>(actions.len() as u32)?;
    let mut previous_tick = 0u32;
    for action in actions {
        write_var_u32(&mut payload, action.frame.saturating_sub(previous_tick))?;
        previous_tick = action.frame;
        payload.write_u8(button_code(action.button))?;
        let mut flags = 0u8;
        if action.player == Player::Two {
            flags |= 0x01;
        }
        if action.down {
            flags |= 0x02;
        }
        payload.write_u8(flags)?;
    }
    payload.write_u32::<LittleEndian>(0)?;
    payload.write_u32::<LittleEndian>(0)?;

    writer.write_all(&header)?;
    write_compressed_payload(writer, &payload)?;
    Ok(())
}

fn write_ttr2<W: Write>(
    writer: &mut W,
    fps: f64,
    duration: f64,
    actions: &[ToastyAction],
) -> Result<()> {
    let has_two_player = actions.iter().any(|action| action.player == Player::Two);
    let mut header = Vec::new();
    write_common_header(
        &mut header,
        TTR2_MAGIC,
        2,
        if has_two_player {
            TTR_FLAG_TWO_PLAYER
        } else {
            0
        },
        fps,
        duration,
    )?;

    let mut payload = Vec::new();
    let actions = sorted_actions(actions);
    payload.write_u32::<LittleEndian>(actions.len() as u32)?;
    let mut previous_tick = 0u32;
    for action in actions {
        write_var_u32(&mut payload, action.frame.saturating_sub(previous_tick))?;
        previous_tick = action.frame;
        payload.write_u8(button_code(action.button))?;
        let mut flags = 0u8;
        if action.player == Player::Two {
            flags |= 0x01;
        }
        if action.down {
            flags |= 0x02;
        }
        payload.write_u8(flags)?;
    }
    payload.write_u32::<LittleEndian>(0)?;
    payload.write_u32::<LittleEndian>(0)?;

    writer.write_all(&header)?;
    write_compressed_payload(writer, &payload)?;
    Ok(())
}

fn push_u64_le(buf: &mut Vec<u8>, value: u64) {
    buf.extend_from_slice(&value.to_le_bytes());
}

fn write_ttr3<W: Write>(
    writer: &mut W,
    fps: f64,
    _duration: f64,
    actions: &[ToastyAction],
) -> Result<()> {
    let has_two_player = actions.iter().any(|action| action.player == Player::Two);
    let flags = (if has_two_player {
        TTR3_FLAG_TWO_PLAYER
    } else {
        0
    }) | TTR3_FLAG_COMPRESSED;

    let mut output = Vec::new();
    output.extend_from_slice(&TTR3_MAGIC);
    output.write_u16::<LittleEndian>(1)?;
    output.write_u16::<LittleEndian>(0)?;
    output.write_u32::<LittleEndian>(flags)?;
    let header_len_pos = output.len();
    output.write_u32::<LittleEndian>(0)?;
    output.write_u64::<LittleEndian>(TTR3_NATIVE_SOURCE_FORMAT_ID)?;
    output.write_u64::<LittleEndian>(0)?;
    output.write_i32::<LittleEndian>(0)?;
    write_string(&mut output, "")?;
    write_string(&mut output, "")?;
    output.write_f64::<LittleEndian>(fps)?;
    output.write_f32::<LittleEndian>(0.0)?;
    output.write_f32::<LittleEndian>(0.0)?;
    output.write_i64::<LittleEndian>(0)?;
    output.write_u32::<LittleEndian>(0)?;
    output.write_u8(0)?;
    let header_len = output.len() as u32;
    output[header_len_pos..header_len_pos + 4].copy_from_slice(&header_len.to_le_bytes());

    let mut inputs_section = Vec::new();
    let actions = sorted_actions(actions);
    inputs_section.write_u64::<LittleEndian>(actions.len() as u64)?;
    for action in actions {
        let time = action.time.max(action.frame as f64 / fps);
        inputs_section.write_f64::<LittleEndian>(time)?;
        inputs_section.write_u8(button_code(action.button))?;
        let mut input_flags = 0u8;
        if action.player == Player::Two {
            input_flags |= 0x01;
        }
        if action.down {
            input_flags |= 0x02;
        }
        inputs_section.write_u8(input_flags)?;
        inputs_section.write_u16::<LittleEndian>(0)?;
    }

    output.write_u16::<LittleEndian>(1)?;
    output.write_u8(TTR3_SECTION_INPUTS)?;
    output.write_u8(0)?;
    output.write_u8(0)?;
    output.write_u8(0)?;
    push_u64_le(&mut output, 0);
    push_u64_le(&mut output, inputs_section.len() as u64);

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&inputs_section)?;
    let compressed = encoder.finish()?;
    writer.write_all(&output)?;
    writer.write_all(&compressed)?;
    Ok(())
}
