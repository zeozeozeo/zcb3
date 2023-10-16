use crate::{AudioSegment, ClickType, Macro, Player};
use anyhow::Result;
use rand::{seq::SliceRandom, Rng};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

const SAMPLE_RATE: u32 = 48000;

#[derive(Debug, Clone, Default)]
pub struct PlayerClicks {
    pub clicks: Vec<AudioSegment>,
    pub releases: Vec<AudioSegment>,
    pub softclicks: Vec<AudioSegment>,
    pub softreleases: Vec<AudioSegment>,
}

fn read_clicks_in_directory(
    dir: PathBuf,
    pitch_from: f32,
    pitch_to: f32,
    pitch_step: f32,
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
            segment.resample(SAMPLE_RATE); // make sure samplerate is equal to SAMPLE_RATE
            segment.make_pitch_table(pitch_from, pitch_to, pitch_step);
            segments.push(segment);
        }
    }
    segments
}

impl PlayerClicks {
    pub fn from_path(mut path: PathBuf, pitch_from: f32, pitch_to: f32, pitch_step: f32) -> Self {
        let mut player = PlayerClicks::default();
        let read_in_dir =
            |path: PathBuf| read_clicks_in_directory(path, pitch_from, pitch_to, pitch_step);

        // -.-
        path.push("clicks");
        player.clicks = read_in_dir(path.clone());
        path.pop();
        path.push("releases");
        player.releases = read_in_dir(path.clone());
        path.pop();
        path.push("softclicks");
        player.softclicks = read_in_dir(path.clone());
        path.pop();
        path.push("softreleases");
        player.softreleases = read_in_dir(path);
        player
    }

    /// Choose a random click based on a click type.
    pub fn random_click(&self, click_type: ClickType) -> &AudioSegment {
        match click_type {
            ClickType::Click => self.clicks.choose(&mut rand::thread_rng()).unwrap(),
            ClickType::Release => {
                if self.releases.is_empty() {
                    return self.random_click(ClickType::Click); // no releases, use clicks
                };
                self.releases.choose(&mut rand::thread_rng()).unwrap()
            }
            ClickType::SoftClick => {
                if self.softclicks.is_empty() {
                    return self.random_click(ClickType::Click); // no softclicks, use regular clicks
                };
                self.softclicks.choose(&mut rand::thread_rng()).unwrap()
            }
            ClickType::SoftRelease => {
                if self.softreleases.is_empty() {
                    return self.random_click(ClickType::Release); // no softreleases, use regular releases
                };
                self.softreleases.choose(&mut rand::thread_rng()).unwrap()
            }
            ClickType::None => unreachable!(),
        }
    }

    /// Finds the longest click amongst all clicks.
    pub fn longest_click(&self) -> f32 {
        let mut max = 0.0f32;
        for segments in [
            &self.clicks,
            &self.releases,
            &self.softclicks,
            &self.softreleases,
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
    pub fn new(
        clickpack_dir: PathBuf,
        pitch_from: f32,
        pitch_to: f32,
        pitch_step: f32,
    ) -> Result<Self> {
        let mut bot = Self::default();
        bot.load_clickpack(clickpack_dir, pitch_from, pitch_to, pitch_step);
        if bot.player.0.clicks.is_empty() && bot.player.1.clicks.is_empty() {
            return Err(anyhow::anyhow!(
                "couldn't find any clicks, did you choose the right folder?"
            ));
        }
        Ok(bot)
    }

    pub const fn has_noise(&self) -> bool {
        self.noise.is_some()
    }

    fn load_clickpack(
        &mut self,
        clickpack_dir: PathBuf,
        pitch_from: f32,
        pitch_to: f32,
        pitch_step: f32,
    ) {
        let mut player1_path = clickpack_dir.clone();
        player1_path.push("player1");
        let mut player2_path = clickpack_dir.clone();
        player2_path.push("player2");

        // check if the clickpack has player1/player2 folders
        if !player1_path.exists() && !player2_path.exists() {
            log::warn!("clickpack directory doesn't have player1/player2 folders");
            let clicks =
                PlayerClicks::from_path(clickpack_dir.clone(), pitch_from, pitch_to, pitch_step);
            self.player = (clicks.clone(), clicks);
            self.load_noise(clickpack_dir); // try to load noise
            return;
        }

        // load clicks from player1 and player2 folders
        self.player = (
            PlayerClicks::from_path(player1_path.clone(), pitch_from, pitch_to, pitch_step),
            PlayerClicks::from_path(player2_path.clone(), pitch_from, pitch_to, pitch_step),
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

    fn get_random_click(&self, player: Player, click: ClickType) -> &AudioSegment {
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
        replay: Macro,
        noise: bool,
        volume_var: f32,
        normalize: bool,
    ) -> AudioSegment {
        log::info!(
            "starting render, {} actions, noise: {noise}, volume variation: {volume_var}",
            replay.actions.len()
        );

        let mut segment = AudioSegment::silent(SAMPLE_RATE, replay.duration + self.longest_click);
        let start = Instant::now();
        let variate_volume = volume_var != 0.0;

        for action in replay.actions {
            let click = self
                .get_random_click(action.player, action.click)
                .random_pitch(); // if no pitch table is generated, returns self

            if variate_volume {
                // overlay with volume variation
                segment.overlay_at_vol(
                    action.time,
                    click,
                    1.0 + rand::thread_rng().gen_range(-volume_var..volume_var),
                );
            } else {
                // overlay normally
                segment.overlay_at(action.time, click);
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
}
