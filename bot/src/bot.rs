use crate::{AudioSegment, ClickType, Macro, Player};
use anyhow::Result;
use rand::Rng;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const SAMPLE_RATE: u32 = 48000;

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
    silent_segment: AudioSegment,
    prev_idx: Option<usize>,
    prev_click_typ: Option<ClickType>,
}

impl PlayerClicks {
    #[inline]
    pub fn has_clicks(&self) -> bool {
        for clicks in [
            &self.hardclicks,
            &self.hardreleases,
            &self.clicks,
            &self.releases,
            &self.softclicks,
            &self.softreleases,
            &self.microclicks,
            &self.microreleases,
        ] {
            if !clicks.is_empty() {
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Pitch {
    pub from: f32,
    pub to: f32,
    pub step: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
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

fn read_clicks_in_directory(dir: PathBuf, pitch: Pitch) -> Vec<AudioSegment> {
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
            segment.resample(SAMPLE_RATE); // make sure samplerate is equal to SAMPLE_RATE
            segment.make_pitch_table(pitch.from, pitch.to, pitch.step);
            segments.push(segment);
        }
    }
    segments
}

impl PlayerClicks {
    pub fn from_path(mut path: PathBuf, pitch: Pitch) -> Self {
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
            *clicks = read_clicks_in_directory(path.clone(), pitch);
            path.pop();
        }

        player
    }

    /// Choose a random click based on a click type.
    pub fn random_click(&mut self, click_type: ClickType) -> &AudioSegment {
        // :sob:
        macro_rules! get_click {
            ($clicks:expr, $typ:expr) => {{
                // get click index
                let idx = if $clicks.len() == 1
                    || $typ.is_release()
                    || self.prev_idx.is_none()
                    || self.prev_click_typ != Some($typ)
                {
                    rand::thread_rng().gen_range(0..$clicks.len())
                } else {
                    // previous was a click
                    // generate a random index thats not `prev_idx`
                    let prev_idx = self.prev_idx.unwrap();
                    let mut idx = prev_idx;
                    while idx == prev_idx {
                        idx = rand::thread_rng().gen_range(0..$clicks.len());
                    }
                    idx
                };
                if !$typ.is_release() {
                    self.prev_click_typ = Some($typ);
                }
                self.prev_idx = Some(idx);
                &mut $clicks[idx]
            }};
        }

        let preferred = click_type.preferred();
        for typ in preferred {
            use ClickType::*;

            // this looks unnecessary, but the borrow checker doesn't think the same
            let has_clicks = match typ {
                HardClick => !self.hardclicks.is_empty(),
                HardRelease => !self.hardreleases.is_empty(),
                Click => !self.clicks.is_empty(),
                Release => !self.releases.is_empty(),
                SoftClick => !self.softclicks.is_empty(),
                SoftRelease => !self.softreleases.is_empty(),
                MicroClick => !self.microclicks.is_empty(),
                MicroRelease => !self.microreleases.is_empty(),
                _ => continue,
            };
            if has_clicks {
                return match typ {
                    HardClick => get_click!(self.hardclicks, typ),
                    HardRelease => get_click!(self.hardreleases, typ),
                    Click => get_click!(self.clicks, typ),
                    Release => get_click!(self.releases, typ),
                    SoftClick => get_click!(self.softclicks, typ),
                    SoftRelease => get_click!(self.softreleases, typ),
                    MicroClick => get_click!(self.microclicks, typ),
                    MicroRelease => get_click!(self.microreleases, typ),
                    _ => continue,
                };
            }
        }
        &mut self.silent_segment
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
}

#[derive(Debug, Clone, Default)]
pub struct Bot {
    pub player: (PlayerClicks, PlayerClicks),
    pub longest_click: f32,
    pub noise: Option<AudioSegment>,
}

impl Bot {
    pub fn new(clickpack_dir: PathBuf, pitch: Pitch) -> Result<Self> {
        let mut bot = Self::default();
        bot.load_clickpack(clickpack_dir, pitch);
        if !bot.player.0.has_clicks() && !bot.player.1.has_clicks() {
            return Err(anyhow::anyhow!(
                "couldn't find any sounds, did you choose the right folder?"
            ));
        }
        Ok(bot)
    }

    pub const fn has_noise(&self) -> bool {
        self.noise.is_some()
    }

    fn load_clickpack(&mut self, clickpack_dir: PathBuf, pitch: Pitch) {
        let mut player1_path = clickpack_dir.clone();
        player1_path.push("player1");
        let mut player2_path = clickpack_dir.clone();
        player2_path.push("player2");

        // check if the clickpack has player1/player2 folders
        if !player1_path.exists() && !player2_path.exists() {
            log::warn!("clickpack directory doesn't have player1/player2 folders");
            let clicks = PlayerClicks::from_path(clickpack_dir.clone(), pitch);
            self.player = (clicks.clone(), clicks);
            self.load_noise(clickpack_dir); // try to load noise
            return;
        }

        // load clicks from player1 and player2 folders
        self.player = (
            PlayerClicks::from_path(player1_path.clone(), pitch),
            PlayerClicks::from_path(player2_path.clone(), pitch),
        );

        // find longest click (will be used to ensure that the end doesn't get cut off)
        self.longest_click = self
            .player
            .0
            .longest_click()
            .max(self.player.1.longest_click());
        log::debug!("longest click: {:?}", self.longest_click);

        // search for noise file, prefer root clickpack dir
        self.load_noise(player1_path);
        self.load_noise(player2_path);
        self.load_noise(clickpack_dir);
    }

    fn load_noise(&mut self, dir: PathBuf) {
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
                    noise.resample(SAMPLE_RATE);
                    Some(noise)
                } else {
                    None
                };
            }
        }
    }

    fn get_random_click(&mut self, player: Player, click: ClickType) -> &AudioSegment {
        // try to get a random click from the player clicks
        // if it doesn't exist for the wanted player, use the other one (guaranteed to have atleast
        // one click)
        match player {
            Player::One => {
                if self.player.0.clicks.is_empty() {
                    self.player.1.random_click(click)
                } else {
                    self.player.0.random_click(click)
                }
            }
            Player::Two => {
                if self.player.1.clicks.is_empty() {
                    self.player.0.random_click(click)
                } else {
                    self.player.1.random_click(click)
                }
            }
        }
    }

    /// Always outputs files with sample rate of 48000.
    pub fn render_macro(
        &mut self,
        replay: &Macro,
        noise: bool,
        normalize: bool,
        render_progress: Option<Arc<Mutex<usize>>>,
    ) -> AudioSegment {
        log::info!(
            "starting render, {} actions, noise: {noise}",
            replay.actions.len()
        );

        let mut segment = AudioSegment::silent(SAMPLE_RATE, replay.duration + self.longest_click);
        let start = Instant::now();

        for action in &replay.actions {
            let click = self
                .get_random_click(action.player, action.click)
                .random_pitch(); // if no pitch table is generated, returns self

            // overlay
            segment.overlay_at_vol(action.time, click, 1.0 + action.vol_offset);

            if let Some(progress) = render_progress.as_ref() {
                *progress.lock().unwrap() += 1;
            }
        }

        if noise && self.noise.is_some() {
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
