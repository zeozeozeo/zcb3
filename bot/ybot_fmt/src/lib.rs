use std::io::{Read, Result, Seek, SeekFrom, Write};
use std::marker::PhantomData;
use std::mem::{self, MaybeUninit};

pub use action::*;
mod action;

pub const MAGIC: [u8; 4] = *b"ybot";

const HEADER_LEN: u32 = 16; // magic, version, meta length, blobs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PlayerButton {
    Jump = 1,
    Left = 2,
    Right = 3,
}

pub trait Getter {
    type Output;

    fn get<T: Read + Seek>(self, m: &mut Macro<T>) -> Result<Self::Output>;
}

pub trait Setter {
    type Input;

    fn set<T: Read + Write + Seek>(self, m: &mut Macro<T>, value: Self::Input) -> Result<()>;
}

#[derive(Clone, Copy)]
pub struct Meta<T> {
    offset: u32,
    _ph: PhantomData<T>,
}

impl<T: Copy> Meta<T> {
    const fn new(offset: u32) -> Self {
        Self {
            offset,
            _ph: PhantomData,
        }
    }

    pub fn read(self, r: impl Read + Seek) -> Result<T> {
        Macro::open(r)?.get(self)
    }

    pub fn write(self, r: impl Read + Write + Seek, value: T) -> Result<()> {
        Macro::open(r)?.set(self, value)
    }

    pub fn offset(self) -> u32 {
        self.offset
    }
    pub fn file_offset(self) -> u32 {
        self.offset + HEADER_LEN
    }
}

impl<T: Copy> Getter for Meta<T> {
    type Output = T;
    fn get<I: Read + Seek>(self, m: &mut Macro<I>) -> Result<T> {
        if self.offset() + mem::size_of::<T>() as u32 > m.meta_length {
            let mut u = MaybeUninit::<T>::uninit();
            unsafe {
                u.as_mut_ptr().write_bytes(0xFF, 1);
                return Ok(u.assume_init());
            }
        }
        m.save_pos(|m| {
            m.inner.seek(SeekFrom::Start(self.file_offset() as _))?;
            let mut buf = MaybeUninit::uninit();
            m.inner.read_exact(unsafe {
                std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, mem::size_of::<T>())
            })?;

            Ok(unsafe { buf.assume_init() })
        })
    }
}

impl<T: Copy> Setter for Meta<T> {
    type Input = T;
    fn set<I: Read + Write + Seek>(self, m: &mut Macro<I>, value: T) -> Result<()> {
        let bytes = unsafe {
            std::slice::from_raw_parts(&value as *const T as *const u8, mem::size_of::<T>())
        };
        let needed = (self.offset() + mem::size_of::<T>() as u32).saturating_sub(m.meta_length);
        m.save_pos(|m| {
            if needed > 0 {
                m.inner
                    .seek(SeekFrom::Start(HEADER_LEN as u64 + m.meta_length as u64))?;
                let mut rest = Vec::new();
                m.save_pos(|m| m.inner.read_to_end(&mut rest))?;
                let mut new = vec![0xFF; needed as usize];
                new[(needed as usize - mem::size_of::<T>())..].copy_from_slice(bytes);
                m.inner.write_all(&new)?;
                m.inner.write_all(&rest)?;

                let new_len = m.meta_length + needed;

                // update meta length
                m.inner.seek(SeekFrom::Start(8))?;
                m.inner.write_all(&new_len.to_le_bytes())?;
                m.meta_length = new_len;
            } else {
                m.inner.seek(SeekFrom::Start(self.file_offset() as _))?;
                m.inner.write_all(bytes)?;
            }
            Ok(())
        })?;
        if needed > 0 {
            m.inner.seek(SeekFrom::Current(needed as i64))?;
            m.actions_start += needed;
        }
        Ok(())
    }
}

macro_rules! def_meta {
	(@inner [$offset:expr]) => {};
	(@inner [$offset:expr] $(#[$meta:meta])* $name:ident: $ty:ty, $($rest:tt)*) => {
		impl Meta<$ty> {
			$(#[$meta])*
			pub const $name: Self = Self::new($offset as u32);
		}
		def_meta!(@inner [$offset + mem::size_of::<$ty>()] $($rest)*);
	};
	($($tt:tt)*) => { def_meta!(@inner [0] $($tt)*); };
}

def_meta! {
    /// The UNIX timestamp of when the macro was created.
    DATE: i64,
    /// The amount of presses (i.e. clicks, or left/right inputs in platformer mode) in the level.
    PRESSES: u64,
    /// The amount of frames in the level (one more than the frame of the last action).
    FRAMES: u64,
    /// The FPS of the macro.
    FPS: f32,
    /// The total amount of presses while botting the level.
    TOTAL_PRESSES: u64,
}

#[derive(Clone, Copy)]
pub struct Blob {
    idx: u32,
    default: &'static [u8],
}

impl Getter for Blob {
    type Output = Vec<u8>;

    fn get<T: Read + Seek>(self, m: &mut Macro<T>) -> Result<Self::Output> {
        let mut idx = self.idx;
        if idx >= m.blobs {
            return Ok(self.default.to_vec());
        }
        m.save_pos(|m| {
            m.inner
                .seek(SeekFrom::Start(HEADER_LEN as u64 + m.meta_length as u64))?;
            let mut buf = [0; 4];
            loop {
                m.inner.read_exact(&mut buf)?;
                let len = u32::from_le_bytes(buf);

                if idx == 0 {
                    let mut data = vec![0; len as _];
                    m.inner.read_exact(&mut data)?;
                    return Ok(data);
                }

                m.inner.seek(SeekFrom::Current(len as i64))?;
                idx -= 1;
            }
        })
    }
}

#[derive(Clone, Copy)]
pub struct Text {
    blob: Blob,
}

impl Text {
    pub fn try_get<I: Read + Seek>(
        self,
        m: &mut Macro<I>,
    ) -> Result<std::result::Result<String, std::string::FromUtf8Error>> {
        let bytes = self.blob.get(m)?;
        Ok(String::from_utf8(bytes))
    }
}

impl Getter for Text {
    type Output = String;

    fn get<T: Read + Seek>(self, m: &mut Macro<T>) -> Result<Self::Output> {
        Ok(String::from_utf8_lossy(&self.blob.get(m)?).into_owned())
    }
}

#[derive(Debug)]
pub struct Macro<T> {
    inner: T,
    version: u32,
    meta_length: u32,
    blobs: u32,
    actions_start: u32,
}

impl<T: Read + Seek> Macro<T> {
    pub fn open(mut inner: T) -> Result<Self> {
        let mut buf = [0; 4];

        inner.read_exact(&mut buf)?;
        if buf != MAGIC {
            return Err(std::io::Error::other("invalid magic"));
        }

        inner.read_exact(&mut buf)?;
        let version = u32::from_le_bytes(buf);
        inner.read_exact(&mut buf)?;
        let meta_length = u32::from_le_bytes(buf);
        inner.read_exact(&mut buf)?;
        let blobs = u32::from_le_bytes(buf);

        let mut actions_start = inner.seek(SeekFrom::Current(meta_length as i64))?;
        for _ in 0..blobs {
            inner.read_exact(&mut buf)?;
            let len = u32::from_le_bytes(buf);
            actions_start = inner.seek(SeekFrom::Current(len as i64))?;
        }

        Ok(Self {
            inner,
            version,
            meta_length,
            blobs,
            actions_start: actions_start as u32,
        })
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
    pub fn as_inner(&self) -> &T {
        &self.inner
    }
    pub fn as_inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    fn save_pos<R>(&mut self, f: impl FnOnce(&mut Self) -> Result<R>) -> Result<R> {
        let pos = self.inner.stream_position()?;
        let x = f(self);
        self.inner.seek(SeekFrom::Start(pos))?;
        x
    }
    pub fn get<G: Getter>(&mut self, getter: G) -> Result<G::Output> {
        getter.get(self)
    }

    /// Returns the file offset where the metadata ends and the actions start.
    pub fn actions_start(&self) -> u32 {
        self.actions_start
    }

    /// Returns the next action, or `None` if there are no more actions.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<TimedAction>> {
        TimedAction::try_read(&mut self.inner)
    }

    /// Returns an iterator over the actions in this macro. The iterator simply
    /// calls `self.next()` on each iteration, and only advances the inner
    /// reader each iteration.
    pub fn actions(&mut self) -> Actions<'_, T> {
        Actions { m: self }
    }
}

impl<T: Read + Write + Seek> Macro<T> {
    pub fn create(mut inner: T) -> Result<Self> {
        inner.write_all(&MAGIC)?;
        inner.write_all(&[0, 0, 0, 0])?;
        inner.write_all(&[0, 0, 0, 0])?;
        inner.write_all(&[0, 0, 0, 0])?;
        Ok(Self {
            inner,
            version: 0,
            meta_length: 0,
            blobs: 0,
            actions_start: HEADER_LEN,
        })
    }

    pub fn set<S: Setter>(&mut self, setter: S, value: S::Input) -> Result<()> {
        setter.set(self, value)
    }

    pub fn add(&mut self, action: TimedAction) -> Result<()> {
        action.write(&mut self.inner)
    }
}

#[derive(Debug)]
pub struct Actions<'a, T: Read + Seek> {
    m: &'a mut Macro<T>,
}

impl<T: Read + Seek> Iterator for Actions<'_, T> {
    type Item = Result<TimedAction>;

    fn next(&mut self) -> Option<Result<TimedAction>> {
        self.m.next().transpose()
    }
}
