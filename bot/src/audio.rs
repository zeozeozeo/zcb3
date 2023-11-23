use anyhow::{Context, Result};
use rand::seq::SliceRandom;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
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

/// Interpolation methods that can be selected. For asynchronous interpolation where the
/// ratio between input and output sample rates can be any number, it's not possible to
/// pre-calculate all the needed interpolation filters.
/// Instead they have to be computed as needed, which becomes impractical since the
/// sincs are very expensive to generate in terms of cpu time.
/// It's more efficient to combine the sinc filters with some other interpolation technique.
/// Then, sinc filters are used to provide a fixed number of interpolated points between input samples,
/// and then, the new value is calculated by interpolation between those points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, Copy)]
pub enum InterpolationType {
    /// For cubic interpolation, the four nearest intermediate points are calculated
    /// using sinc interpolation.
    /// Then, a cubic polynomial is fitted to these points, and is used to calculate the new sample value.
    /// The computation time is approximately twice as long as that of linear interpolation,
    /// but it requires much fewer intermediate points for a good result.
    Cubic,
    /// For quadratic interpolation, the three nearest intermediate points are calculated
    /// using sinc interpolation.
    /// Then, a quadratic polynomial is fitted to these points, and is used to calculate the new sample value.
    /// The computation time lies approximately halfway between that of linear and quadratic interpolation.
    Quadratic,
    /// For linear interpolation, the new sample value is calculated by linear interpolation
    /// between the two nearest points.
    /// This requires two intermediate points to be calculated using sinc interpolation,
    /// and the output is obtained by taking a weighted average of these two points.
    /// This is relatively fast, but needs a large number of intermediate points to
    /// push the resampling artefacts below the noise floor.
    #[default]
    Linear,
    /// The Nearest mode doesn't do any interpolation, but simply picks the nearest intermediate point.
    /// This is useful when the nearest point is actually the correct one, for example when upsampling by a factor 2,
    /// like 48kHz->96kHz.
    /// Then, when setting the oversampling_factor to 2 and using Nearest mode,
    /// no unnecessary computations are performed and the result is equivalent to that of synchronous resampling.
    /// This also works for other ratios that can be expressed by a fraction. For 44.1kHz -> 48 kHz,
    /// setting oversampling_factor to 160 gives the desired result (since 48kHz = 160/147 * 44.1kHz).
    Nearest,
}

impl ToString for InterpolationType {
    fn to_string(&self) -> String {
        format!("{self:?}")
    }
}

impl From<InterpolationType> for rubato::SincInterpolationType {
    fn from(val: InterpolationType) -> Self {
        match val {
            InterpolationType::Cubic => rubato::SincInterpolationType::Cubic,
            InterpolationType::Quadratic => rubato::SincInterpolationType::Quadratic,
            InterpolationType::Linear => rubato::SincInterpolationType::Linear,
            InterpolationType::Nearest => rubato::SincInterpolationType::Nearest,
        }
    }
}

/// Different window functions that can be used to window the sinc function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, Copy)]
pub enum WindowFunction {
    /// Blackman. Intermediate rolloff and intermediate attenuation.
    Blackman,
    /// Squared Blackman. Slower rolloff but better attenuation than Blackman.
    Blackman2,
    /// Blackman-Harris. Slow rolloff but good attenuation.
    BlackmanHarris,
    /// Squared Blackman-Harris. Slower rolloff but better attenuation than Blackman-Harris.
    #[default]
    BlackmanHarris2,
    /// Hann. Fast rolloff but not very high attenuation.
    Hann,
    /// Squared Hann. Slower rolloff and higher attenuation than simple Hann.
    Hann2,
}

impl ToString for WindowFunction {
    fn to_string(&self) -> String {
        format!("{self:?}")
    }
}

impl From<WindowFunction> for rubato::WindowFunction {
    fn from(val: WindowFunction) -> Self {
        match val {
            WindowFunction::Blackman => rubato::WindowFunction::Blackman,
            WindowFunction::Blackman2 => rubato::WindowFunction::Blackman2,
            WindowFunction::BlackmanHarris => rubato::WindowFunction::BlackmanHarris,
            WindowFunction::BlackmanHarris2 => rubato::WindowFunction::BlackmanHarris2,
            WindowFunction::Hann => rubato::WindowFunction::Hann,
            WindowFunction::Hann2 => rubato::WindowFunction::Hann2,
        }
    }
}

/// A struct holding the parameters for sinc interpolation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterpolationParams {
    /// Length of the windowed sinc interpolation filter.
    /// Higher values can allow a higher cut-off frequency leading to less high frequency roll-off
    /// at the expense of higher cpu usage. 256 is a good starting point.
    /// The value will be rounded up to the nearest multiple of 8.
    pub sinc_len: usize,
    /// Relative cutoff frequency of the sinc interpolation filter
    /// (relative to the lowest one of fs_in/2 or fs_out/2). Start at 0.95, and increase if needed.
    pub f_cutoff: f32,
    /// The number of intermediate points to use for interpolation.
    /// Higher values use more memory for storing the sinc filters.
    /// Only the points actually needed are calculated during processing
    /// so a larger number does not directly lead to higher cpu usage.
    /// A lower value helps in keeping the sincs in the cpu cache. Start at 128.
    pub oversampling_factor: usize,
    /// Interpolation type, see `SincInterpolationType`
    pub interpolation: InterpolationType,
    /// Window function to use.
    pub window: WindowFunction,
}

impl Default for InterpolationParams {
    fn default() -> Self {
        InterpolationParams {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: InterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        }
    }
}

impl From<InterpolationParams> for rubato::SincInterpolationParameters {
    fn from(val: InterpolationParams) -> Self {
        rubato::SincInterpolationParameters {
            sinc_len: val.sinc_len,
            f_cutoff: val.f_cutoff,
            oversampling_factor: val.oversampling_factor,
            interpolation: val.interpolation.into(),
            window: val.window.into(),
        }
    }
}

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
    pub fn resample(&mut self, rate: u32, params: &InterpolationParams) -> &mut Self {
        if self.sample_rate == rate {
            return self;
        }
        log::debug!(
            "sinc resampling audiosegment, {} => {rate}...",
            self.sample_rate
        );
        let start = Instant::now();
        use rubato::{Resampler, SincFixedIn};
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
                params.clone().into(),
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
    pub fn make_pitch_table(
        &mut self,
        from: f32,
        to: f32,
        step: f32,
        params: &InterpolationParams,
    ) {
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
                seg.resample((self.sample_rate as f32 * cur) as u32, params);
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

    pub fn remove_silence_before(&mut self, threshold: f32) {
        let mut idx = 0;
        for (i, v) in self.data.chunks(2).enumerate() {
            let avg = (v[0] + v[1]) / 2.; // avg of l and r channels
            if avg.abs() > threshold {
                idx = i * 2;
                break;
            }
        }

        // remove all values upto index
        self.data.drain(..idx);
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
        #[allow(clippy::approx_constant)]
        let sample = segment.time_to_sample(3.14);
        assert!(segment.data.get(sample).is_some());
    }
}
