use anyhow::{Context, Result};
use rand::seq::SliceRandom;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::io::{BufWriter, Cursor};
use std::time::{Duration, Instant};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

#[inline(always)]
fn time_to_sample(sample_rate: u32, channels: usize, time: f32) -> usize {
    (time * sample_rate as f32) as usize * channels
}

#[derive(Clone, Debug, Default)]
pub struct AudioSegment {
    pub sample_rate: u32,
    /// Interleaved channel data. Always [`AudioSegment::NUM_CHANNELS`] channels.
    pub data: Vec<f32>,
    pub pitch_table: Vec<AudioSegment>,
}

impl AudioSegment {
    pub const NUM_CHANNELS: usize = 2;

    pub fn extend_with(&mut self, data: &[f32], channels: usize) {
        self.data
            .extend_from_slice(&Self::convert_channels(data, channels));
    }

    fn convert_channels(audio: &[f32], channels: usize) -> Vec<f32> {
        match channels.cmp(&Self::NUM_CHANNELS) {
            Ordering::Greater => {
                // remove channels (TODO idk if this is correct)
                let mut new = vec![0.0f32; (audio.len() / channels) * Self::NUM_CHANNELS];
                new.iter_mut().enumerate().for_each(|(i, s)| {
                    for k in 0..channels - 1 {
                        *s = audio[i * k];
                    }
                });
                new
            }
            Ordering::Less => {
                // duplicate channels
                let mut new: Vec<f32> = Vec::with_capacity(audio.len() * Self::NUM_CHANNELS);
                for s in audio {
                    for _ in 0..=Self::NUM_CHANNELS - channels {
                        new.push(*s);
                    }
                }
                new
            }
            Ordering::Equal => audio.to_vec(),
        }
    }

    pub fn from_media_source(media_source: Box<dyn MediaSource>) -> Result<Self> {
        log::info!("decoding media");
        let start = Instant::now();

        // create media source using the boxed reader
        let mss = MediaSourceStream::new(media_source, Default::default());

        // create a hint for the format registry to guess what reader is appropriate
        let hint = Hint::new();

        // use default decoder opts
        let format_opts: FormatOptions = Default::default();
        let metadata_opts: MetadataOptions = Default::default();
        let decoder_opts: DecoderOptions = Default::default();

        // probe media source for a format, get the yielded format reader
        let probed =
            symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;
        let mut format = probed.format;

        // get default track
        let track = format
            .default_track()
            .context("failed to get default track")?;

        // create a new decoder for the track
        let mut decoder =
            symphonia::default::get_codecs().make(&track.codec_params, &decoder_opts)?;

        // store track identifier (will be used to filter packets)
        let track_id = track.id;

        let mut sample_rate = 0u32;
        let mut data: Vec<f32> = vec![];

        loop {
            // get the next packet from the format reader
            let Ok(packet) = format.next_packet() else {
                log::warn!("failed to decode next packet! stopping now");
                break;
            };

            // if the packet does not belong to the selected track, skip it
            if packet.track_id() != track_id {
                continue;
            }

            // decode packet into audio samples, ignore any decode errors
            match decoder.decode(&packet) {
                Ok(audio_buf) => {
                    let spec = *audio_buf.spec();
                    // let dur = (audio_buf.frames() / spec.channels.count()) as u64;
                    sample_rate = spec.rate;

                    // copy audio buf to sample buf
                    let mut sample_buf =
                        SampleBuffer::<f32>::new(audio_buf.capacity() as u64, spec);
                    sample_buf.copy_interleaved_ref(audio_buf); // interleaved!

                    // extend data slice, convert channels to `Self::NUM_CHANNELS`
                    data.extend(Self::convert_channels(
                        sample_buf.samples(),
                        spec.channels.count(),
                    ));
                }
                Err(Error::DecodeError(err)) => log::warn!("decode error: {err}; ignoring"),
                Err(_) => break,
            }
        }

        log::info!("decoded in {:?}", start.elapsed());

        Ok(Self {
            sample_rate,
            data,
            pitch_table: vec![],
        })
    }

    pub fn from_bytes(data: Vec<u8>) -> Result<Self> {
        Self::from_media_source(Box::new(Cursor::new(data)))
    }

    pub fn silent(rate: u32, time: f32) -> Self {
        Self {
            sample_rate: rate,
            data: vec![0.0; time_to_sample(rate, 2, time)],
            pitch_table: vec![],
        }
    }

    pub fn export_wav<W: std::io::Write + std::io::Seek>(&self, writer: W) -> Result<()> {
        let spec = hound::WavSpec {
            channels: Self::NUM_CHANNELS as u16,
            sample_rate: self.sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        log::info!("writing wav file");
        let start = Instant::now();

        // create buffered writer with 64mb buffer size
        let mut wav =
            hound::WavWriter::new(BufWriter::with_capacity(64 * 1024 * 1024, writer), spec)?;
        for sample in &self.data {
            wav.write_sample(*sample)?;
        }
        wav.finalize()?; // flush writer

        log::info!("finished writing wav file in {:?}", start.elapsed());
        Ok(())
    }

    /// Convert time to samples. Clamps maximum to the segment length.
    #[inline(always)]
    fn time_to_sample(&self, time: f32) -> usize {
        time_to_sample(self.sample_rate, Self::NUM_CHANNELS, time)
            .min(self.data.len().saturating_sub(1))
    }

    #[inline]
    pub fn overlay_at(&mut self, time: f32, other: &AudioSegment) {
        assert!(self.sample_rate == other.sample_rate);

        let start = self.time_to_sample(time);
        let end = (start + other.data.len()).min(self.data.len().saturating_sub(1));
        self.data[start..end]
            .par_iter_mut() // run in parallel
            .zip(&other.data)
            .for_each(|(s, o)| *s += o);
    }

    #[inline]
    pub fn overlay_at_vol(&mut self, time: f32, other: &AudioSegment, volume: f32) {
        assert!(self.sample_rate == other.sample_rate);

        let start = self.time_to_sample(time);
        let end = (start + other.data.len()).min(self.data.len().saturating_sub(1));
        self.data[start..end]
            .par_iter_mut() // run in parallel
            .zip(&other.data)
            .for_each(|(s, o)| *s += o * volume);
    }

    /// Returns the duration of the audio segment.
    #[inline]
    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(
            (self.data.len() / Self::NUM_CHANNELS) as f64 / self.sample_rate as f64,
        )
    }

    /// Uses sinc interpolation to resample the audio to the given rate (squared blackman-harris).
    ///
    /// Does not do anything if sample rate is the same.
    pub fn resample(&mut self, rate: u32) -> &mut Self {
        if self.sample_rate == rate {
            return self;
        }
        log::debug!(
            "sinc resampling audiosegment, {} => {rate}...",
            self.sample_rate
        );
        let start = Instant::now();
        use rubato::{
            Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType,
            WindowFunction,
        };
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let resampled = {
            // deinterleave audio & convert to f64
            let mut deinterleaved: Vec<Vec<f64>> = vec![
                Vec::with_capacity(self.data.len() / 2),
                Vec::with_capacity(self.data.len() / 2),
            ];
            for (i, sample) in self.data.iter().enumerate() {
                deinterleaved[i % 2].push(*sample as f64);
            }
            // resample self.sample_rate => rate
            let mut resampler = SincFixedIn::<f64>::new(
                rate as f64 / self.sample_rate as f64,
                2.0,
                params,
                deinterleaved[0].len(),
                2,
            )
            .expect("failed to create resampler");
            resampler
                .process(&deinterleaved, None)
                .expect("failed to resample audio")
        };

        self.data = Vec::with_capacity(resampled[0].len() * 2);

        // interleave audio and convert to f32
        for i in 0..resampled[0].len() {
            for channel_data in resampled.iter().take(2) {
                self.data.push(channel_data[i] as f32);
            }
        }

        log::info!(
            "resampled {} => {rate}; took {:?}",
            self.sample_rate,
            start.elapsed()
        );
        self.sample_rate = rate;
        self
    }

    pub fn normalize(&mut self) {
        let max = self.data.iter().fold(0.0, |a: f32, &b| a.max(b));
        for sample in &mut self.data {
            *sample /= max;
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
        self.data = Vec::new();
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
        self.time_to_sample(time.as_secs_f32())
    }

    #[inline(always)]
    pub fn samples_after_index(&self, idx: usize) -> usize {
        self.data.len() - idx
    }

    pub fn get_preview_data(&self, displayed_samples: usize, time: Duration) -> Vec<f32> {
        let start = self.get_sample_index_which_was_a_duration_ago(time);
        let samples_after = self.samples_after_index(start);
        let mut step = samples_after / displayed_samples;
        // make sure step is aligned to NUM_CHANNELS
        step -= step % Self::NUM_CHANNELS;

        self.data[start..].iter().step_by(step).copied().collect()
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
        let sample = segment.time_to_sample(10.0);
        assert!(segment.data.get(sample).is_some());
        let sample = segment.time_to_sample(0.0);
        assert!(segment.data.get(sample).is_some());
        let sample = segment.time_to_sample(3.14);
        assert!(segment.data.get(sample).is_some());
    }
}
