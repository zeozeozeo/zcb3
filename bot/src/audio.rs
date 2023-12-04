use anyhow::{Context, Result};
use rand::seq::SliceRandom;
use rayon::prelude::*;
use std::io::{BufWriter, Cursor};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::time::{Duration, Instant};
use symphonia::core::audio::{AudioBuffer, AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::conv::{FromSample, IntoSample};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::sample::Sample;

/// Represents an audio sample. Stores a left and right channel.
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct Frame {
    /// Left channel value. Float.
    pub left: f32,
    /// Right channel value. Float.
    pub right: f32,
}

impl Frame {
    /// A frame with all channels set to 0.0.
    pub const ZERO: Self = Self {
        left: 0.0,
        right: 0.0,
    };

    /// Create a new audio frame from left and right values.
    #[inline]
    pub const fn new(left: f32, right: f32) -> Self {
        Self { left, right }
    }

    /// Create a new audio frame from a single value.
    #[inline]
    pub const fn from_mono(value: f32) -> Self {
        Self::new(value, value)
    }
}

impl From<[f32; 2]> for Frame {
    fn from(lr: [f32; 2]) -> Self {
        Self::new(lr[0], lr[1])
    }
}

impl From<(f32, f32)> for Frame {
    fn from(lr: (f32, f32)) -> Self {
        Self::new(lr.0, lr.1)
    }
}

impl Add for Frame {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.left + rhs.left, self.right + rhs.right)
    }
}

impl AddAssign for Frame {
    fn add_assign(&mut self, rhs: Self) {
        self.left += rhs.left;
        self.right += rhs.right;
    }
}

impl Sub for Frame {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.left - rhs.left, self.right - rhs.right)
    }
}

impl SubAssign for Frame {
    fn sub_assign(&mut self, rhs: Self) {
        self.left -= rhs.left;
        self.right -= rhs.right;
    }
}

impl Mul<f32> for Frame {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.left * rhs, self.right * rhs)
    }
}

impl MulAssign<f32> for Frame {
    fn mul_assign(&mut self, rhs: f32) {
        self.left *= rhs;
        self.right *= rhs;
    }
}

impl Div<f32> for Frame {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.left / rhs, self.right / rhs)
    }
}

impl DivAssign<f32> for Frame {
    fn div_assign(&mut self, rhs: f32) {
        self.left /= rhs;
        self.right /= rhs;
    }
}

impl Neg for Frame {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.left, -self.right)
    }
}

/// p. 43: http://yehar.com/blog/wp-content/uploads/2009/08/deip.pdf
#[inline]
fn interpolate_frame(
    previous: Frame,
    current: Frame,
    next: Frame,
    next_next: Frame,
    fraction: f32,
) -> Frame {
    let c0 = current;
    let c1 = (next - previous) * 0.5;
    let c2 = previous - current * 2.5 + next * 2.0 - next_next * 0.5;
    let c3 = (next_next - previous) * 0.5 + (current - next) * 1.5;
    ((c3 * fraction + c2) * fraction + c1) * fraction + c0
}

#[inline(always)]
fn time_to_frame(sample_rate: u32, time: f32) -> usize {
    (time * sample_rate as f32) as usize
}

#[derive(Clone, Debug, Default)]
pub struct AudioSegment {
    pub sample_rate: u32,
    /// Interleaved channel data. Always [`AudioSegment::NUM_CHANNELS`] channels.
    pub frames: Vec<Frame>,
    pub pitch_table: Vec<AudioSegment>,
}

fn load_frames_from_buffer_ref(buffer: &AudioBufferRef) -> Result<Vec<Frame>> {
    match buffer {
        AudioBufferRef::U8(buffer) => load_frames_from_buffer(buffer),
        AudioBufferRef::U16(buffer) => load_frames_from_buffer(buffer),
        AudioBufferRef::U24(buffer) => load_frames_from_buffer(buffer),
        AudioBufferRef::U32(buffer) => load_frames_from_buffer(buffer),
        AudioBufferRef::S8(buffer) => load_frames_from_buffer(buffer),
        AudioBufferRef::S16(buffer) => load_frames_from_buffer(buffer),
        AudioBufferRef::S24(buffer) => load_frames_from_buffer(buffer),
        AudioBufferRef::S32(buffer) => load_frames_from_buffer(buffer),
        AudioBufferRef::F32(buffer) => load_frames_from_buffer(buffer),
        AudioBufferRef::F64(buffer) => load_frames_from_buffer(buffer),
    }
}

fn load_frames_from_buffer<S: Sample>(buffer: &AudioBuffer<S>) -> Result<Vec<Frame>>
where
    f32: FromSample<S>,
{
    let num_channels = buffer.spec().channels.count();
    match num_channels {
        1 => Ok(buffer
            .chan(0)
            .iter()
            .map(|sample| Frame::from_mono((*sample).into_sample()))
            .collect()),
        2 => Ok(buffer
            .chan(0)
            .iter()
            .zip(buffer.chan(1).iter())
            .map(|(left, right)| Frame::new((*left).into_sample(), (*right).into_sample()))
            .collect()),
        _ => anyhow::bail!("unsupported number of channels {num_channels}, expected 1 or 2"),
    }
}

impl AudioSegment {
    pub fn extend_with(&mut self, data: &[Frame]) {
        self.frames.extend_from_slice(data)
    }

    pub fn from_media_source(media_source: Box<dyn MediaSource>) -> Result<Self> {
        use std::io::ErrorKind::UnexpectedEof;

        // create a media source stream from the provided media source
        let mss = MediaSourceStream::new(media_source, Default::default());

        // create a hint to help the format registry to guess what format
        // the media source is using, we'll let symphonia figure that out for us
        let hint = Hint::new();

        // use default options for reading and encoding
        let format_opts: FormatOptions = Default::default();
        let metadata_opts: MetadataOptions = Default::default();
        let decoder_opts: DecoderOptions = Default::default();

        // probe the media source for a format
        let probed =
            symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;

        let mut format = probed.format;
        let track = format
            .default_track()
            .context("failed to get default track")?;

        // create a decoder for the track
        let mut decoder =
            symphonia::default::get_codecs().make(&track.codec_params, &decoder_opts)?;

        // store the track identifier, we'll use it to filter packets
        let track_id = track.id;

        // get sample rate
        let sample_rate = track
            .codec_params
            .sample_rate
            .context("failed to get sample rate")?;

        let mut frames = Vec::new(); // audio data

        loop {
            // get the next packet from the format reader
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(Error::IoError(e)) => {
                    // if we reached eof, stop decoding
                    if e.kind() == UnexpectedEof {
                        break;
                    }
                    // ...otherwise return KaError
                    return Err(Error::IoError(e).into());
                }
                Err(e) => return Err(e.into()), // not io error
            };

            // if the packet does not belong to the selected track, skip it
            if packet.track_id() != track_id {
                continue;
            }

            // decode packet
            let buffer = decoder.decode(&packet)?;
            frames.append(&mut load_frames_from_buffer_ref(&buffer)?);
        }

        Ok(Self {
            sample_rate,
            frames,
            pitch_table: vec![],
        })
    }

    pub fn from_bytes(data: Vec<u8>) -> Result<Self> {
        Self::from_media_source(Box::new(Cursor::new(data)))
    }

    pub fn silent(rate: u32, time: f32) -> Self {
        Self {
            sample_rate: rate,
            frames: vec![Frame::ZERO; time_to_frame(rate, time)],
            pitch_table: vec![],
        }
    }

    pub fn export_wav<W: std::io::Write + std::io::Seek>(&self, writer: W) -> Result<()> {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: self.sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        log::info!("writing wav file");
        let start = Instant::now();

        // create buffered writer with 16mb buffer size
        let mut wav =
            hound::WavWriter::new(BufWriter::with_capacity(16 * 1024 * 1024, writer), spec)?;
        for frame in &self.frames {
            wav.write_sample(frame.left)?;
            wav.write_sample(frame.right)?;
        }
        wav.finalize()?; // flush writer

        log::info!("finished writing wav file in {:?}", start.elapsed());
        Ok(())
    }

    /// Convert time to samples. Clamps maximum to the segment length.
    #[inline(always)]
    fn time_to_frame(&self, time: f32) -> usize {
        time_to_frame(self.sample_rate, time).min(self.frames.len().saturating_sub(1))
    }

    #[inline]
    pub fn overlay_at(&mut self, time: f32, other: &AudioSegment) {
        assert!(self.sample_rate == other.sample_rate);

        let start = self.time_to_frame(time);
        let end = (start + other.frames.len()).min(self.frames.len().saturating_sub(1));
        self.frames[start..end]
            .par_iter_mut() // run in parallel
            .zip(&other.frames)
            .for_each(|(s, o)| {
                s.left += o.right;
                s.right += o.right;
            });
    }

    #[inline]
    pub fn overlay_at_vol(&mut self, time: f32, other: &AudioSegment, volume: f32) {
        assert!(self.sample_rate == other.sample_rate);

        let start = self.time_to_frame(time);
        let end = (start + other.frames.len()).min(self.frames.len().saturating_sub(1));
        self.frames[start..end]
            .par_iter_mut() // run in parallel
            .zip(&other.frames)
            .for_each(|(s, o)| {
                s.left += o.right * volume;
                s.right += o.right * volume;
            });
    }

    /// Returns the duration of the audio segment.
    #[inline]
    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.frames.len() as f64 / self.sample_rate as f64)
    }

    /// Uses sinc interpolation to resample the audio to the given rate.
    ///
    /// Does not do anything if sample rate is the same.
    pub fn resample(&mut self, rate: u32) -> &mut Self {
        let mut fractional_position = 0.0f64;
        let mut iter = self.frames.iter();
        let mut frames = [Frame::ZERO; 4]; // prev, cur, next, next next
        let push_frame = |frame: Frame, frames: &mut [Frame]| {
            for i in 0..frames.len() - 1 {
                frames[i] = frames[i + 1];
            }
            frames[frames.len() - 1] = frame;
        };

        // fill resampler with 3 frames
        for _ in 0..3 {
            push_frame(iter.next().copied().unwrap_or(Frame::ZERO), &mut frames);
        }

        let mut resampled_frames = Vec::with_capacity(self.frames.len());
        let dt = rate as f64 / self.sample_rate as f64;

        'outer: loop {
            resampled_frames.push(interpolate_frame(
                frames[0],
                frames[1],
                frames[2],
                frames[3],
                fractional_position as f32,
            ));

            fractional_position += dt;
            while fractional_position >= 1.0 {
                fractional_position -= 1.0;
                let Some(frame) = iter.next().copied() else {
                    break 'outer;
                };
                push_frame(frame, &mut frames);
            }
        }

        self.sample_rate = rate;
        self.frames = resampled_frames;
        self
    }

    pub fn normalize(&mut self) {
        let max: Frame = self.frames.iter().fold(Frame::ZERO, |acc, f| acc + *f);
        for frame in &mut self.frames {
            frame.left /= max.left;
            frame.right /= max.right;
        }
    }

    /// Generates a pitch table for an audiosegment (pitch ranges from `from` to `to` with step `step`).
    pub fn make_pitch_table(&mut self, from: f32, to: f32, step: f32) {
        let old_seg = self.clone();
        log::info!(
            "generating pitch table; {from} => {to} (+= {step}, {} computations)",
            ((to - from) / step) as usize
        );

        self.pitch_table = vec![old_seg; ((to - from) / step) as usize];
        self.pitch_table
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, seg)| {
                let cur = from + (i as f32 * step);
                log::debug!("resampling step: {cur}");
                seg.resample((self.sample_rate as f32 * cur) as u32);
                seg.sample_rate = self.sample_rate; // keep same sample rate
            });
    }

    /// Does not clear the pitch table, only clears data
    #[inline]
    pub fn clear(&mut self) {
        self.frames = Vec::new();
    }

    /// Chooses random pitch from the pitch table. If pitch table is not generated,
    /// returns [`self`]    
    #[inline]
    pub fn random_pitch(&self) -> &AudioSegment {
        if self.pitch_table.is_empty() {
            return self;
        }
        self.pitch_table.choose(&mut rand::thread_rng()).unwrap()
    }

    pub fn get_sample_index_which_was_a_duration_ago(&self, ago: Duration) -> usize {
        if self.duration() < ago {
            return 0;
        }
        let time = self.duration() - ago;
        self.time_to_frame(time.as_secs_f32())
    }

    #[inline(always)]
    pub fn samples_after_index(&self, idx: usize) -> usize {
        self.frames.len() - idx
    }

    pub fn remove_silence_from_start(&mut self, threshold: f32) {
        let mut idx = 0;
        for (i, v) in self.frames.iter().enumerate() {
            let avg = (v.left + v.right) / 2.; // avg of l and r channels
            if avg.abs() > threshold {
                idx = i;
                break;
            }
        }

        // remove all values upto index
        self.frames.drain(..idx);
    }

    pub fn remove_silence_from_end(&mut self, threshold: f32) {
        let mut idx = 0;
        for (i, v) in self.frames.iter().rev().enumerate() {
            let avg = (v.left + v.right) / 2.; // avg of l and r channels
            if avg.abs() > threshold {
                idx = i;
                break;
            }
        }

        // remove all values from index
        self.frames.drain((self.frames.len() - idx)..);
    }

    pub fn set_volume(&mut self, volume: f32) -> &mut Self {
        for sample in &mut self.frames {
            *sample *= volume;
        }
        self
    }

    pub fn reverse(&mut self) -> &mut Self {
        self.frames.reverse();
        self
    }

    /*
    pub fn find_peaks(&self, threshold: f32) {
        const CHUNK_SIZE: usize = 44100 / 4; // 11025
        use itertools::Itertools;

        for chunk in self.data.chunks(CHUNK_SIZE) {
            let mut vol = 0.;
            for (l, r) in chunk.iter().tuples() {
                let avg = (l + r) / 2.;
                vol += avg.abs() / (chunk.len() / 2) as f32;
            }
        }
    }
    */
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_to_sample() {
        let segment = AudioSegment::silent(44100, 5.0);

        // make sure indexes are valid
        let sample = segment.time_to_frame(10.0);
        assert!(segment.frames.get(sample).is_some());
        let sample = segment.time_to_frame(0.0);
        assert!(segment.frames.get(sample).is_some());
        #[allow(clippy::approx_constant)]
        let sample = segment.time_to_frame(3.14);
        assert!(segment.frames.get(sample).is_some());
    }
}
