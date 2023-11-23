use crate::{AudioSegment, ClickType, ExtendedAction, InterpolationParams, Player, Replay};
use anyhow::Result;
use fasteval2::Compiler;
use rand::{seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Default)]
pub struct PlayerClicks {
    pub hardclicks: Vec<AudioSegment>,
    pub hardreleases: Vec<AudioSegment>,
    pub clicks: Vec<AudioSegment>,
    pub releases: Vec<AudioSegment>,
    pub softclicks: Vec<AudioSegment>,
    pub softreleases: Vec<AudioSegment>,
    pub microclicks: Vec<AudioSegment>,
    pub microreleases: Vec<AudioSegment>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Pitch {
    pub from: f32,
    pub to: f32,
    pub step: f32,
}

impl Default for Pitch {
    fn default() -> Self {
        Self {
            from: 0.95,
            to: 1.05,
            step: 0.001,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Timings {
    pub hard: f32,
    pub regular: f32,
    pub soft: f32,
}

impl Default for Timings {
    fn default() -> Self {
        Self {
            hard: 2.0,
            regular: 0.15,
            soft: 0.025,
            // lower = microclicks
        }
    }
}

/// Defines the variable that the volume expression should affect.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Default)]
pub enum ExprVariable {
    #[default]
    None,
    Variation,
    Value,
    TimeOffset,
}

impl ToString for ExprVariable {
    fn to_string(&self) -> String {
        match self {
            Self::None => "None".to_string(),
            Self::Variation => "Volume variation".to_string(),
            Self::Value => "Volume value".to_string(),
            Self::TimeOffset => "Time offset".to_string(),
        }
    }
}

impl ExprVariable {
    pub const fn is_volume_change(self) -> bool {
        matches!(self, Self::Variation | Self::Value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct VolumeSettings {
    pub enabled: bool,
    pub spam_time: f32,
    pub spam_vol_offset_factor: f32,
    pub max_spam_vol_offset: f32,
    pub change_releases_volume: bool,
    pub global_volume: f32,
    pub volume_var: f32,
}

impl Default for VolumeSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            spam_time: 0.3,
            spam_vol_offset_factor: 0.9,
            max_spam_vol_offset: 0.3,
            change_releases_volume: false,
            global_volume: 1.0,
            volume_var: 0.2,
        }
    }
}

fn read_clicks_in_directory(
    dir: PathBuf,
    pitch: Pitch,
    sample_rate: u32,
    params: &InterpolationParams,
) -> Vec<AudioSegment> {
    log::debug!(
        "loading clicks from directory {}",
        dir.to_str().unwrap_or("")
    );

    let mut segments = Vec::new();
    let Ok(dir) = dir.read_dir() else {
        log::warn!("can't find directory {dir:?}, skipping");
        return vec![];
    };

    for entry in dir {
        let path = entry.unwrap().path();
        if path.is_file() {
            let Some(f) = std::fs::File::open(path.clone()).ok() else {
                log::error!("failed to open file '{path:?}'");
                continue;
            };
            let Ok(mut segment) = AudioSegment::from_media_source(Box::new(f)) else {
                log::error!("failed to decode file '{path:?}'");
                continue;
            };

            segment.resample(sample_rate, params);
            segment.make_pitch_table(pitch.from, pitch.to, pitch.step, params);
            segments.push(segment);
        }
    }
    segments
}

impl PlayerClicks {
    pub fn from_path(
        mut path: PathBuf,
        pitch: Pitch,
        sample_rate: u32,
        params: &InterpolationParams,
    ) -> Self {
        let mut player = PlayerClicks::default();

        for (dir, clicks) in [
            ("hardclicks", &mut player.hardclicks),
            ("hardreleases", &mut player.hardreleases),
            ("clicks", &mut player.clicks),
            ("releases", &mut player.releases),
            ("softclicks", &mut player.softclicks),
            ("softreleases", &mut player.softreleases),
            ("microclicks", &mut player.microclicks),
            ("microreleases", &mut player.microreleases),
        ] {
            path.push(dir);
            *clicks = read_clicks_in_directory(path.clone(), pitch, sample_rate, params);
            path.pop();
        }

        if !player.has_clicks() {
            log::warn!("no clicks found, assuming there's no subdirectories");
            player.clicks = read_clicks_in_directory(path.clone(), pitch, sample_rate, params);
        }

        player
    }

    #[inline]
    pub fn has_clicks(&self) -> bool {
        [
            &self.hardclicks,
            &self.hardreleases,
            &self.clicks,
            &self.releases,
            &self.softclicks,
            &self.softreleases,
            &self.microclicks,
            &self.microreleases,
        ]
        .iter()
        .any(|c| !c.is_empty())
    }

    /// Choose a random click based on a click type.
    pub fn random_click(&self, click_type: ClickType) -> Option<&AudioSegment> {
        let preferred = click_type.preferred();
        for typ in preferred {
            use ClickType::*;

            let click = match typ {
                HardClick => self.hardclicks.choose(&mut rand::thread_rng()),
                HardRelease => self.hardreleases.choose(&mut rand::thread_rng()),
                Click => self.clicks.choose(&mut rand::thread_rng()),
                Release => self.releases.choose(&mut rand::thread_rng()),
                SoftClick => self.softclicks.choose(&mut rand::thread_rng()),
                SoftRelease => self.softreleases.choose(&mut rand::thread_rng()),
                MicroClick => self.microclicks.choose(&mut rand::thread_rng()),
                MicroRelease => self.microreleases.choose(&mut rand::thread_rng()),
                _ => continue,
            };
            if let Some(click) = click {
                return Some(click);
            }
        }
        None
    }

    /// Finds the longest click amongst all clicks.
    pub fn longest_click(&self) -> f32 {
        let mut max = 0.0f32;
        for segments in [
            &self.hardclicks,
            &self.hardreleases,
            &self.clicks,
            &self.releases,
            &self.softclicks,
            &self.softreleases,
            &self.microclicks,
            &self.microreleases,
        ] {
            for segment in segments {
                max = max.max(segment.duration().as_secs_f32());
            }
        }
        max
    }

    #[inline]
    pub fn num_sounds(&self) -> usize {
        [
            &self.hardclicks,
            &self.hardreleases,
            &self.clicks,
            &self.releases,
            &self.softclicks,
            &self.softreleases,
            &self.microclicks,
            &self.microreleases,
        ]
        .iter()
        .map(|c| c.len())
        .sum()
    }
}

#[derive(Debug, Default)]
pub struct Bot {
    /// Clicks/releases for player 1 and player 2.
    pub player: (PlayerClicks, PlayerClicks),
    /// The longest sound (in seconds, not counting the noise sound).
    pub longest_click: f32,
    /// Noise audio file. Will be resampled to `sample_rate`.
    pub noise: Option<AudioSegment>,
    /// Output sample rate. Clicks will be sinc-resampled to this rate.
    pub sample_rate: u32,
    /// Expression evaluator namespace. Updated with default variables every action.
    pub ns: BTreeMap<String, f64>,
    slab: fasteval2::Slab,
    pub compiled_expr: fasteval2::Instruction,
}

impl Bot {
    #[inline]
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            ..Default::default()
        }
    }

    #[inline]
    pub const fn has_noise(&self) -> bool {
        self.noise.is_some()
    }

    pub fn load_clickpack(
        &mut self,
        clickpack_dir: &Path,
        pitch: Pitch,
        params: &InterpolationParams,
    ) {
        assert!(self.sample_rate > 0);
        let mut player1_path = clickpack_dir.to_path_buf();
        player1_path.push("player1");
        let mut player2_path = clickpack_dir.to_path_buf();
        player2_path.push("player2");

        // check if the clickpack has player1/player2 folders
        if !player1_path.exists() && !player2_path.exists() {
            log::warn!("clickpack directory doesn't have player1/player2 folders");
            let clicks = PlayerClicks::from_path(
                clickpack_dir.to_path_buf(),
                pitch,
                self.sample_rate,
                params,
            );
            self.player = (clicks.clone(), clicks);
            self.load_noise(clickpack_dir, params); // try to load noise
            return;
        }

        // load clicks from player1 and player2 folders
        self.player = (
            PlayerClicks::from_path(player1_path.clone(), pitch, self.sample_rate, params),
            PlayerClicks::from_path(player2_path.clone(), pitch, self.sample_rate, params),
        );

        // find longest click (will be used to ensure that the end doesn't get cut off)
        self.longest_click = self
            .player
            .0
            .longest_click()
            .max(self.player.1.longest_click());
        log::debug!("longest click: {:?}", self.longest_click);

        // search for noise file, prefer root clickpack dir
        self.load_noise(&player1_path, params);
        self.load_noise(&player2_path, params);
        self.load_noise(clickpack_dir, params);
    }

    fn load_noise(&mut self, dir: &Path, params: &InterpolationParams) {
        let Ok(dir) = dir.read_dir() else {
            return;
        };
        for entry in dir {
            let path = entry.unwrap().path();
            let filename = path.file_name().unwrap().to_str().unwrap();
            // if it's a noise* or whitenoise* file we should try to load it
            if path.is_file()
                && (filename.starts_with("noise") || filename.starts_with("whitenoise"))
            {
                log::info!("found noise file {path:?}");
                let f = std::fs::File::open(path.clone()).unwrap();
                self.noise = if let Ok(mut noise) = AudioSegment::from_media_source(Box::new(f)) {
                    noise.resample(self.sample_rate, params);
                    Some(noise)
                } else {
                    None
                };
            }
        }
    }

    fn get_random_click(&mut self, player: Player, click: ClickType) -> &AudioSegment {
        // try to get a random click/release from the player clicks
        // if it doesn't exist for the wanted player, use the other one (guaranteed to have atleast
        // one click)
        let p1 = &self.player.0;
        let p2 = &self.player.1;
        match player {
            Player::One => {
                if let Some(click) = p1.random_click(click) {
                    click
                } else {
                    return p2.random_click(click).unwrap(); // use p2 clicks
                }
            }
            Player::Two => {
                if let Some(click) = p2.random_click(click) {
                    click
                } else {
                    return p1.random_click(click).unwrap(); // use p1 clicks
                }
            }
        }
    }

    pub fn compile_expression(&mut self, expr: &str) -> Result<()> {
        let parser = fasteval2::Parser::new();
        // a [`fasteval2::Slab`] can't be cloned, so we wrap it in a refcell
        self.slab = fasteval2::Slab::new();
        self.ns = BTreeMap::new();

        // try to compile expr
        self.compiled_expr = parser
            .parse(expr, &mut self.slab.ps)?
            .from(&self.slab.ps)
            .compile(&self.slab.ps, &mut self.slab.cs, &mut self.ns);
        Ok(())
    }

    /// Updates the volume variation expressions' namespace.
    pub fn update_namespace(&mut self, a: &ExtendedAction, total_frames: u32, fps: f64) {
        self.ns.insert("frame".to_string(), a.frame as _);
        self.ns.insert("fps".to_string(), fps);
        self.ns.insert("time".to_string(), a.frame as f64 / fps);
        self.ns.insert("x".to_string(), a.x as _);
        self.ns.insert("y".to_string(), a.y as _);
        self.ns
            .insert("p".to_string(), a.frame as f64 / total_frames as f64);
        self.ns.insert("player2".to_string(), a.player2 as u8 as _);
        self.ns.insert("rot".to_string(), a.rot as _);
        self.ns.insert("accel".to_string(), a.y_accel as _);
        self.ns.insert("down".to_string(), a.down as u8 as _);
        self.ns.insert("frames".to_string(), total_frames as _);
        self.ns
            .insert("level_time".to_string(), total_frames as f64 / fps);
        self.ns
            .insert("rand".to_string(), rand::thread_rng().gen_range(0.0..=1.0));
    }

    pub fn eval_expr(&mut self) -> Result<f64> {
        use fasteval2::Evaler;
        let val = self.compiled_expr.eval(&self.slab, &mut self.ns)?;
        Ok(val)
    }

    /// Returns the minimum and maximum values for the volume expression.
    pub fn expr_range(&mut self, replay: &Replay) -> (f64, f64) {
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        for action in &replay.extended {
            self.update_namespace(action, replay.last_frame(), replay.fps.into());
            let val = self.eval_expr().unwrap_or(0.);
            min = min.min(val);
            max = max.max(val);
        }
        (min, max)
    }

    pub fn render_replay(
        &mut self,
        replay: &Replay,
        noise: bool,
        normalize: bool,
        expr_var: ExprVariable,
        enable_pitch: bool,
    ) -> AudioSegment {
        log::info!(
            "starting render, {} actions, noise: {noise}",
            replay.actions.len()
        );

        let longest_time_offset = if expr_var == ExprVariable::TimeOffset {
            self.expr_range(replay).1 as f32
        } else {
            0.
        };

        let mut segment = AudioSegment::silent(
            self.sample_rate,
            replay.duration + self.longest_click + longest_time_offset,
        );
        let start = Instant::now();

        for action in &replay.actions {
            // calculate the volume from the expression if needed
            let (expr_vol, time_offset) = if expr_var != ExprVariable::None {
                // get extended action
                // FIXME: this is very wasteful, currently we binary search the entire
                //        actions array each time
                let extended = replay
                    .extended
                    .binary_search_by(|a| a.frame.cmp(&action.frame))
                    .unwrap_or(usize::MAX);
                let extended = replay
                    .extended
                    .get(extended)
                    .copied()
                    .unwrap_or(ExtendedAction::default());

                // compute expression
                self.update_namespace(&extended, replay.last_frame(), replay.fps.into());
                let value = self.eval_expr().unwrap_or(0.) as f32;

                match expr_var {
                    ExprVariable::Value => (value, 0.),
                    ExprVariable::Variation => (rand::thread_rng().gen_range(0.0..=value), 0.),
                    ExprVariable::TimeOffset => (0., value),
                    _ => unreachable!(),
                }
            } else {
                (0., 0.)
            };

            let mut click = self.get_random_click(action.player, action.click);
            if enable_pitch {
                click = click.random_pitch(); // if no pitch table is generated, returns self
            }

            // overlay
            segment.overlay_at_vol(
                action.time + time_offset,
                click,
                1.0 + action.vol_offset + expr_vol,
            );
        }

        if noise && self.has_noise() {
            let mut noise_duration = Duration::from_secs(0);
            let noise_segment = self.noise.as_ref().unwrap();

            while noise_duration < segment.duration() {
                segment.overlay_at(noise_duration.as_secs_f32(), noise_segment);
                noise_duration += noise_segment.duration();
            }
        }

        if normalize {
            segment.normalize();
        }

        log::info!("rendered in {:?}", start.elapsed());
        segment
    }

    #[inline]
    pub fn has_clicks(&self) -> bool {
        self.player.0.has_clicks() || self.player.1.has_clicks()
    }
}
