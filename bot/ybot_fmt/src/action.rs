use varint_rs::{VarintReader, VarintWriter};

use std::io::{Read, Result, Write};

use crate::PlayerButton;

#[derive(Debug, Clone, Copy)]
pub struct TimedAction {
    /// The amount of frames since the previous action.
    pub delta: u64,
    pub action: Action,
}

impl TimedAction {
    pub fn new(delta: u64, action: Action) -> Self {
        Self { delta, action }
    }

    pub fn read(mut r: impl Read) -> Result<Self> {
        let val = r.read_u64_varint()?;
        let flags = val & 0b1111;
        let action = if let Some(x) = action_from_flags(flags as u8) {
            x
        } else {
            let mut buf = [0; 4];
            r.read_exact(&mut buf)?;
            Action::FPS(f32::from_le_bytes(buf))
        };
        let delta = val >> 4;
        Ok(Self { delta, action })
    }

    /// Tries to read a `TimedAction`, returning `None` if EOF is encountered.
    pub fn try_read(r: impl Read) -> Result<Option<Self>> {
        Ok(match Self::read(r) {
            Ok(x) => Some(x),
            Err(e) if e.kind() != std::io::ErrorKind::UnexpectedEof => return Err(e),
            _ => None,
        })
    }

    pub fn write(&self, mut w: impl Write) -> Result<()> {
        let val = self.delta << 4 | action_to_flags(self.action) as u64;
        w.write_u64_varint(val)?;
        if let Action::FPS(fps) = self.action {
            w.write_all(&fps.to_le_bytes())?;
        }
        Ok(())
    }
}

#[inline]
fn action_from_flags(flags: u8) -> Option<Action> {
    let p1 = flags & 1 != 0;
    let push = flags & 2 != 0;
    let button = match flags >> 2 {
        1 => PlayerButton::Jump,
        2 => PlayerButton::Left,
        3 => PlayerButton::Right,
        _ => return None,
    };
    Some(Action::Button(p1, push, button))
}

#[inline]
fn action_to_flags(action: Action) -> u8 {
    match action {
        Action::FPS(_) => 0b1111,
        Action::Button(p1, down, button) => p1 as u8 | ((down as u8) << 1) | ((button as u8) << 2),
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    /// Push/release a button. The first `bool` indicates whether the action is for player 1. The second indicates whether the action is a push.
    Button(bool, bool, crate::PlayerButton),
    /// Change FPS to this value.
    FPS(f32),
}
