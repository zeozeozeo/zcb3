use std::fs;
use std::path::Path;

mod binary;
mod error;
mod physics;
mod tests;

pub use binary::{BinaryReader, BinaryWriter};
pub use error::{Error, Result};
use physics::PhysicsData;

const GDR_MAGIC: &[u8; 3] = b"GDR";
const GDR_VERSION: i32 = 2;

/// Information about the bot that recorded the replay
#[derive(Debug, Clone, PartialEq)]
pub struct Bot {
    pub name: String,
    pub version: i32,
}

impl Default for Bot {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: 1,
        }
    }
}

/// Information about the level that the replay was recorded on
#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    pub id: u32,
    pub name: String,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
        }
    }
}

/// Information about a single input in a replay
#[derive(Debug, Clone, PartialEq)]
pub struct Input {
    /// Frame that the input was recorded on
    pub frame: u64,
    /// Button that was pressed. 1 = Jump, 2 = Left, 3 = Right
    pub button: u8,
    /// Whether this input was for player 2
    pub player2: bool,
    /// Whether the button was pressed or released
    pub down: bool,
    pub physics: Option<PhysicsData>,
}

impl Input {
    pub fn new(frame: u64, button: u8, player2: bool, down: bool) -> Self {
        Self {
            frame,
            button,
            player2,
            down,
            physics: None,
        }
    }

    pub fn with_physics(
        frame: u64,
        button: u8,
        player2: bool,
        down: bool,
        physics: PhysicsData,
    ) -> Self {
        Self {
            frame,
            button,
            player2,
            down,
            physics: Some(physics),
        }
    }

    fn read_extension(&mut self, reader: &mut BinaryReader, extension_tag: &str) -> Result<()> {
        if extension_tag == "Phys" {
            self.physics = Some(PhysicsData {
                x_position: reader.read_f32()?,
                y_position: reader.read_f32()?,
                rotation: reader.read_f32()?,
                x_velocity: reader.read_f64()?,
                y_velocity: reader.read_f64()?,
            });
        }
        Ok(())
    }

    fn write_extension(&self, writer: &mut BinaryWriter, extension_tag: &str) {
        if extension_tag == "Phys" {
            if let Some(physics) = &self.physics {
                writer.write_f32(physics.x_position);
                writer.write_f32(physics.y_position);
                writer.write_f32(physics.rotation);
                writer.write_f64(physics.x_velocity);
                writer.write_f64(physics.y_velocity);
            }
        }
    }
}

/// A GD replay containing metadata and inputs
#[derive(Debug, Default, Clone)]
pub struct Replay {
    pub author: String,
    pub description: String,
    pub duration: f32,
    pub game_version: i32,
    pub framerate: f64,
    pub seed: i32,
    pub coins: i32,
    pub ldm: bool,
    pub platformer: bool,
    pub bot_info: Bot,
    pub level_info: Level,
    pub inputs: Vec<Input>,
    pub deaths: Vec<u64>,
}

impl Replay {
    pub fn new() -> Self {
        Self {
            framerate: 240.0,
            ..Default::default()
        }
    }

    /// Sort the inputs by frame number
    pub fn sort_inputs(&mut self) {
        self.inputs.sort_by_key(|input| input.frame);
    }

    /// Export the replay to a byte vector
    pub fn export_data(&self) -> Result<Vec<u8>> {
        let mut writer = BinaryWriter::new();

        // Write header
        writer.write_bytes(GDR_MAGIC);
        writer.write_varint(GDR_VERSION);

        // Write input tag
        let has_physics = self.inputs.iter().any(|input| input.physics.is_some());
        writer.write_string(if has_physics { "Phys" } else { "" });

        // Write metadata
        writer.write_string(&self.author);
        writer.write_string(&self.description);
        writer.write_f32(self.duration);
        writer.write_varint(self.game_version);
        writer.write_f64(self.framerate);
        writer.write_varint(self.seed);
        writer.write_varint(self.coins);
        writer.write_bool(self.ldm);
        writer.write_bool(self.platformer);
        writer.write_string(&self.bot_info.name);
        writer.write_varint(self.bot_info.version);
        writer.write_varint(self.level_info.id as i32);
        writer.write_string(&self.level_info.name);

        // Write empty extension section
        writer.write_varint(0);

        // Write deaths
        writer.write_varint(self.deaths.len() as i32);
        let mut prev = 0;
        for &death in &self.deaths {
            writer.write_varint((death - prev) as i32);
            prev = death;
        }

        // Count player 1 inputs
        let p1_inputs = self.inputs.iter().filter(|input| !input.player2).count();

        // Write total inputs and p1 input count
        writer.write_varint(self.inputs.len() as i32);
        writer.write_varint(p1_inputs as i32);

        let mut prev = 0;
        for input in &self.inputs {
            if input.player2 {
                continue;
            }

            let delta = input.frame - prev;
            let packed = if self.platformer {
                (delta << 3) | ((input.button as u64) << 1) | (input.down as u64)
            } else {
                (delta << 1) | (input.down as u64)
            };
            writer.write_varint(packed as i32);

            // Write physics extension if present
            if has_physics {
                let mut ext_writer = BinaryWriter::new();
                input.write_extension(&mut ext_writer, "Phys");
                writer.write_varint(ext_writer.data().len() as i32);
                writer.write_bytes(&ext_writer.data());
            }

            prev = input.frame;
        }

        // Write player 2 inputs
        let mut prev = 0;
        for input in &self.inputs {
            if !input.player2 {
                continue;
            }

            let delta = input.frame - prev;
            let packed = if self.platformer {
                (delta << 3) | ((input.button as u64) << 1) | (input.down as u64)
            } else {
                (delta << 1) | (input.down as u64)
            };
            writer.write_varint(packed as i32);

            // Write physics extension if present
            if has_physics {
                let mut ext_writer = BinaryWriter::new();
                input.write_extension(&mut ext_writer, "Phys");
                writer.write_varint(ext_writer.data().len() as i32);
                writer.write_bytes(&ext_writer.data());
            }

            prev = input.frame;
        }

        Ok(writer.into_vec())
    }

    /// Export the replay to a file
    pub fn export_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let data = self.export_data()?;
        fs::write(path, data).map_err(Error::Io)
    }

    /// Import a replay from bytes
    pub fn import_data(data: &[u8]) -> Result<Self> {
        let mut reader = BinaryReader::new(data);
        let mut replay = Replay::new();

        // Read and verify magic
        let magic = reader.read_bytes(3)?;
        if magic != GDR_MAGIC {
            return Err(Error::InvalidMagic);
        }

        // Read version and input tag
        let version = reader.read_varint()?;
        if version != GDR_VERSION {
            return Err(Error::UnsupportedVersion(version));
        }

        let input_tag = reader.read_string()?;
        let has_extension = !input_tag.is_empty();

        // Read metadata
        replay.author = reader.read_string()?;
        replay.description = reader.read_string()?;
        replay.duration = reader.read_f32()?;
        replay.game_version = reader.read_varint()?;
        replay.framerate = reader.read_f64()?;
        replay.seed = reader.read_varint()?;
        replay.coins = reader.read_varint()?;
        replay.ldm = reader.read_bool()?;
        replay.platformer = reader.read_bool()?;
        replay.bot_info.name = reader.read_string()?;
        replay.bot_info.version = reader.read_varint()?;
        replay.level_info.id = reader.read_varint()? as u32;
        replay.level_info.name = reader.read_string()?;

        // Skip extension data
        let ext_size = reader.read_varint()? as usize;
        reader.skip(ext_size)?;

        // Read deaths
        let death_count = reader.read_varint()? as usize;
        let mut prev = 0;
        for _ in 0..death_count {
            let delta = reader.read_varint()? as u64;
            prev += delta;
            replay.deaths.push(prev);
        }

        // Read inputs
        let total_inputs = reader.read_varint()? as usize;
        let p1_inputs = reader.read_varint()? as usize;

        // Read player 1 inputs
        let mut prev = 0;
        for _ in 0..p1_inputs {
            let packed = reader.read_varint()? as u64;
            let mut input = if replay.platformer {
                Input::new(
                    prev + (packed >> 3),
                    ((packed >> 1) & 3) as u8,
                    false,
                    (packed & 1) != 0,
                )
            } else {
                Input::new(prev + (packed >> 1), 1, false, (packed & 1) != 0)
            };

            if has_extension {
                let ext_size = reader.read_varint()? as usize;
                if ext_size > 0 {
                    let ext_data = reader.peek(ext_size).ok_or(Error::UnexpectedEof)?;
                    let mut ext_reader = BinaryReader::new(ext_data);
                    input.read_extension(&mut ext_reader, &input_tag)?;
                    reader.skip(ext_size)?;
                }
            }

            prev = input.frame;
            replay.inputs.push(input);
        }

        // Read player 2 inputs
        let mut prev = 0;
        for _ in p1_inputs..total_inputs {
            let packed = reader.read_varint()? as u64;
            let mut input = if replay.platformer {
                Input::new(
                    prev + (packed >> 3),
                    ((packed >> 1) & 3) as u8,
                    true,
                    (packed & 1) != 0,
                )
            } else {
                Input::new(prev + (packed >> 1), 1, true, (packed & 1) != 0)
            };

            if has_extension {
                let ext_size = reader.read_varint()? as usize;
                if ext_size > 0 {
                    let ext_data = reader.peek(ext_size).ok_or(Error::UnexpectedEof)?;
                    let mut ext_reader = BinaryReader::new(ext_data);
                    input.read_extension(&mut ext_reader, &input_tag)?;
                    reader.skip(ext_size)?;
                }
            }

            prev = input.frame;
            replay.inputs.push(input);
        }

        replay.sort_inputs();
        Ok(replay)
    }

    /// Import a replay from a file
    pub fn import_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let data = fs::read(path).map_err(Error::Io)?;
        Self::import_data(&data)
    }
}
