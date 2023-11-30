use anyhow::Result;
use kittyaudio::{RecordMixer, Sound};
use std::{
    io::BufWriter,
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, Default)]
pub struct AudioFile {
    pub sound: Sound,
    pub filename: String,
}

impl AudioFile {
    pub const fn new(sound: Sound, filename: String) -> Self {
        Self { sound, filename }
    }

    /// Export the underlying [`Sound`] as a .wav file with [`hound`].
    pub fn export_wav<W>(&self, writer: W) -> Result<()>
    where
        W: std::io::Write + std::io::Seek,
    {
        // create wav writer
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: self.sample_rate(),
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        // 16mb buffer
        let mut wav =
            hound::WavWriter::new(BufWriter::with_capacity(16 * 1024 * 1024, writer), spec)?;

        // create mixer and immediately play the sound in it
        let mut mixer = RecordMixer::new();
        mixer.play(self.sound.clone());

        let sample_rate = self.sound.sample_rate();

        // record each frame
        while !mixer.is_finished() {
            let frame = mixer.next_frame(sample_rate);
            wav.write_sample(frame.left)?;
            wav.write_sample(frame.right)?;
        }

        wav.finalize()?; // flush writer
        Ok(())
    }
}

impl Deref for AudioFile {
    type Target = Sound;

    fn deref(&self) -> &Self::Target {
        &self.sound
    }
}

impl DerefMut for AudioFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sound
    }
}
