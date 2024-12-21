use crate::{f32_range, AudioSegment, Click, ClickType, ExtendedAction, Player, Replay};
use anyhow::Result;
use fasteval2::Compiler;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut, Index, IndexMut},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Default)]
pub struct AudioFile {
    pub segment: AudioSegment,
    pub filename: String,
}

impl AudioFile {
    pub const fn new(segment: AudioSegment, filename: String) -> Self {
        Self { segment, filename }
    }
}

impl Deref for AudioFile {
    type Target = AudioSegment;

    fn deref(&self) -> &Self::Target {
        &self.segment
    }
}

impl DerefMut for AudioFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.segment
    }
}

#[derive(Clone, Default)]
pub struct PlayerClicks {
    pub hardclicks: Vec<AudioFile>,
    pub hardreleases: Vec<AudioFile>,
    pub clicks: Vec<AudioFile>,
    pub releases: Vec<AudioFile>,
    pub softclicks: Vec<AudioFile>,
    pub softreleases: Vec<AudioFile>,
    pub microclicks: Vec<AudioFile>,
    pub microreleases: Vec<AudioFile>,
}

impl Index<usize> for PlayerClicks {
    type Output = Vec<AudioFile>;
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.hardclicks,
            1 => &self.hardreleases,
            2 => &self.clicks,
            3 => &self.releases,
            4 => &self.softclicks,
            5 => &self.softreleases,
            6 => &self.microclicks,
            7 => &self.microreleases,
            _ => panic!("invalid index"),
        }
    }
}

// if `path` only has a single subdirectory, returns that subdirectory
fn fix_root_subdir(dir: &Path) -> PathBuf {
    if dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            let entries: Vec<_> = entries.collect();
            if entries.len() == 1 {
                if let Ok(entry) = entries[0].as_ref() {
                    let path = entry.path();
                    if path.is_dir() {
                        return path;
                    }
                }
            }
        }
    }
    dir.to_path_buf()
}

#[cfg(not(target_arch = "wasm32"))]
fn unzip_to_temp_dir(path: &Path) -> Result<PathBuf> {
    fn random_dirname() -> String {
        return format!(
            "zcb-unzipped-{}",
            std::iter::repeat_with(fastrand::alphanumeric)
                .take(16)
                .collect::<String>()
        );
    }
    let mut dir = std::env::temp_dir().join(random_dirname());
    while dir.exists() {
        dir.pop();
        dir.push(random_dirname());
    }

    std::fs::create_dir_all(&dir)?;

    let f = std::fs::File::open(path)?;
    zip_extract::extract(f, &dir, true)?;
    Ok(dir)
}

impl PlayerClicks {
    // parses folders like "softclicks", "soft_clicks", "soft click", "microblablablarelease"
    fn recognize_dir_and_load_files(&mut self, path: &Path, pitch: Pitch, sample_rate: u32) {
        log::debug!("trying to match directory {path:?}");
        if path.is_file() {
            log::debug!("skipping matching file {path:?}");
            return;
        }
        let filename: String = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .chars()
            .filter(|c| c.is_alphabetic())
            .flat_map(|c| c.to_lowercase())
            .collect();
        let patterns = [
            (["hardclick", "hardclicks"], &mut self.hardclicks),
            (["hardrelease", "hardreleases"], &mut self.hardreleases),
            (["click", "clicks"], &mut self.clicks),
            (["release", "releases"], &mut self.releases),
            (["softclick", "softclicks"], &mut self.softclicks),
            (["softrelease", "softreleases"], &mut self.softreleases),
            (["microclick", "microclicks"], &mut self.microclicks),
            (["microrelease", "microreleases"], &mut self.microreleases),
        ];
        let mut matched_any = false;
        for (pats, clicks) in patterns {
            if pats.iter().any(|pat| *pat == filename) {
                log::debug!("directory {path:?} matched patterns {pats:?}");
                matched_any = true;
                clicks.extend(read_clicks_in_directory(path, pitch, sample_rate));
            }
        }
        if !matched_any {
            log::warn!("directory {path:?} did not match any pattern");
        }
    }

    pub fn from_path(path: &Path, pitch: Pitch, sample_rate: u32) -> Self {
        let mut player = PlayerClicks::default();

        #[allow(unused_mut)]
        let mut path = fix_root_subdir(path);

        #[cfg(not(target_arch = "wasm32"))]
        if path.is_file() {
            // try to unzip
            match unzip_to_temp_dir(&path) {
                Ok(p) => {
                    path = p;
                }
                Err(e) => {
                    log::error!("failed to unzip {path:?}: {e}");
                }
            }
        }

        let Ok(dir) = path
            .read_dir()
            .map_err(|e| log::warn!("failed to read directory {path:?}: {e}"))
        else {
            return player;
        };

        for entry in dir {
            if let Ok(entry) = entry {
                let entry_path = fix_root_subdir(&entry.path());
                if entry_path.is_dir() {
                    player.recognize_dir_and_load_files(&entry_path, pitch, sample_rate);
                } else {
                    log::debug!("skipping file {entry_path:?}");
                }
            }
        }

        if !player.has_clicks() {
            log::warn!("no clicks found, assuming there's no subdirectories");
            player
                .clicks
                .extend(read_clicks_in_directory(&path, pitch, sample_rate));
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
        macro_rules! rand_click {
            ($arr:expr) => {{
                let len = $arr.len();
                if len == 0 {
                    continue;
                }
                $arr.get(fastrand::usize(..len))
            }};
        }

        let preferred = click_type.preferred();
        for typ in preferred {
            use ClickType::*;

            let click = match typ {
                HardClick => rand_click!(self.hardclicks),
                HardRelease => rand_click!(self.hardreleases),
                Click => rand_click!(self.clicks),
                Release => rand_click!(self.releases),
                SoftClick => rand_click!(self.softclicks),
                SoftRelease => rand_click!(self.softreleases),
                MicroClick => rand_click!(self.microclicks),
                MicroRelease => rand_click!(self.microreleases),
                None => continue,
            };
            if let Some(click) = click {
                return Some(click);
            }
        }
        None
    }

    /// Finds the longest click amongst all clicks.
    pub fn longest_click(&self) -> f64 {
        let mut max = 0.0f64;
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
                max = max.max(segment.duration().as_secs_f64());
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

    pub fn extend_with(&mut self, other: &PlayerClicks) {
        self.hardclicks
            .extend_from_slice(other.hardclicks.as_slice());
        self.hardreleases
            .extend_from_slice(other.hardreleases.as_slice());
        self.clicks.extend_from_slice(other.clicks.as_slice());
        self.releases.extend_from_slice(other.releases.as_slice());
        self.softclicks
            .extend_from_slice(other.softclicks.as_slice());
        self.softreleases
            .extend_from_slice(other.softreleases.as_slice());
        self.microclicks
            .extend_from_slice(other.microclicks.as_slice());
        self.microreleases
            .extend_from_slice(other.microreleases.as_slice());
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Pitch {
    pub from: f32,
    pub to: f32,
    pub step: f32,
}

impl Pitch {
    pub const NO_PITCH: Pitch = Pitch {
        from: 1.0,
        to: 1.0,
        step: 0.0,
    };
}

impl Default for Pitch {
    fn default() -> Self {
        Self {
            from: 0.98,
            to: 1.02,
            step: 0.0005,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Timings {
    pub hard: f64,
    pub regular: f64,
    pub soft: f64,
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

// used for serde's dumb `default` field
#[inline]
const fn true_value() -> bool {
    true
}

/// Defines the variable that the volume expression should affect.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Default)]
pub enum ExprVariable {
    #[default]
    None,
    Variation {
        #[serde(default = "true_value")]
        negative: bool,
    },
    Value,
    TimeOffset,
}

impl std::fmt::Display for ExprVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Variation { .. } => write!(f, "Volume variation"),
            Self::Value => write!(f, "Volume value"),
            Self::TimeOffset => write!(f, "Time offset"),
        }
    }
}

impl ExprVariable {
    pub const fn is_volume_change(self) -> bool {
        matches!(self, Self::Variation { .. } | Self::Value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
pub enum RemoveSilenceFrom {
    #[default]
    None,
    Start,
    End,
}

impl std::fmt::Display for RemoveSilenceFrom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
pub enum ChangeVolumeFor {
    #[default]
    All,
    Clicks,
    Releases,
}

impl std::fmt::Display for ChangeVolumeFor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ClickpackConversionSettings {
    /// Volume multiplier.
    pub volume: f32,
    /// Whether to change volume only for releases.
    #[serde(default = "ChangeVolumeFor::default")]
    pub change_volume_for: ChangeVolumeFor,
    /// Whether to reverse all audio files.
    pub reverse: bool,
    pub remove_silence: RemoveSilenceFrom,
    pub silence_threshold: f32,
    /// Whether to rename files to '1.wav', '2.wav', etc.
    #[serde(default = "bool::default")]
    pub rename_files: bool,
}

impl Default for ClickpackConversionSettings {
    fn default() -> Self {
        Self {
            volume: 1.,
            change_volume_for: ChangeVolumeFor::All,
            reverse: false,
            remove_silence: RemoveSilenceFrom::None,
            silence_threshold: 0.05,
            rename_files: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct VolumeSettings {
    pub enabled: bool,
    pub spam_time: f64,
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

fn read_clicks_in_directory(dir: &Path, pitch: Pitch, sample_rate: u32) -> Vec<AudioFile> {
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
            let Some(f) = std::fs::File::open(&path).ok() else {
                log::error!("failed to open file '{path:?}'");
                continue;
            };
            log::info!("decoding file {path:?}");
            let Ok(mut segment) = AudioSegment::from_media_source(Box::new(f)) else {
                log::error!("failed to decode file '{path:?}'");
                continue;
            };

            let filename = path.file_name().unwrap().to_str().unwrap().to_string();

            segment.resample(sample_rate);
            segment.make_pitch_table(pitch.from, pitch.to, pitch.step);
            segments.push(AudioFile::new(segment, filename));
        }
    }
    segments
}

#[derive(Default)]
pub struct Clickpack {
    pub player1: PlayerClicks,
    pub player2: PlayerClicks,
    pub left1: PlayerClicks,
    pub right1: PlayerClicks,
    pub left2: PlayerClicks,
    pub right2: PlayerClicks,
}

impl Index<usize> for Clickpack {
    type Output = PlayerClicks;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.player1,
            1 => &self.player2,
            2 => &self.left1,
            3 => &self.right1,
            4 => &self.left2,
            5 => &self.right2,
            _ => panic!("invalid index"),
        }
    }
}

impl IndexMut<usize> for Clickpack {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.player1,
            1 => &mut self.player2,
            2 => &mut self.left1,
            3 => &mut self.right1,
            4 => &mut self.left2,
            5 => &mut self.right2,
            _ => panic!("invalid index"),
        }
    }
}

impl Clickpack {
    fn has_clicks(&self) -> bool {
        self.player1.num_sounds() != 0
            || self.player2.num_sounds() != 0
            || self.left1.num_sounds() != 0
            || self.right1.num_sounds() != 0
            || self.left2.num_sounds() != 0
            || self.right2.num_sounds() != 0
    }

    fn longest_click(&self) -> f64 {
        let mut longest = 0.0f64;
        for click in [
            &self.player1,
            &self.player2,
            &self.left1,
            &self.right1,
            &self.left2,
            &self.right2,
        ] {
            longest = longest.max(click.longest_click());
        }
        longest
    }

    pub fn num_sounds(&self) -> usize {
        self.player1.num_sounds()
            + self.player2.num_sounds()
            + self.left1.num_sounds()
            + self.right1.num_sounds()
            + self.left2.num_sounds()
            + self.right2.num_sounds()
    }
}

const CLICKPACK_DIRNAMES: [&str; 6] = ["player1", "player2", "left1", "left2", "right1", "right2"];

#[derive(Default)]
pub struct Bot {
    /// Clicks/releases for player 1 and player 2.
    pub clickpack: Clickpack,
    /// The longest sound (in seconds, not counting the noise sound).
    pub longest_click: f64,
    /// Noise audio file. Will be resampled to `sample_rate`.
    pub noise: Option<AudioSegment>,
    /// Output sample rate. Clicks will be sinc-resampled to this rate.
    pub sample_rate: u32,
    /// Expression evaluator namespace. Updated with default variables every action.
    pub ns: BTreeMap<String, f64>,
    slab: fasteval2::Slab,
    pub compiled_expr: fasteval2::Instruction,
}

pub fn find_noise_file(dir: &Path) -> Option<PathBuf> {
    let Ok(dir) = dir.read_dir() else {
        return None;
    };
    for entry in dir {
        let path = entry.ok()?.path();
        let filename = path.file_name()?.to_str()?;
        // if it's a noise*, etc file we should try to load it
        let lower_filename = filename.to_lowercase();
        if path.is_file()
            && (lower_filename.starts_with("noise")
                || lower_filename.starts_with("whitenoise")
                || lower_filename.starts_with("pcnoise")
                || lower_filename.starts_with("background"))
        {
            return Some(path);
        }
    }
    None
}

pub fn dir_has_noise(dir: &Path) -> bool {
    if find_noise_file(dir).is_some() {
        return true;
    }
    for dirname in CLICKPACK_DIRNAMES {
        let mut path = dir.to_path_buf();
        path.push(dirname);

        if find_noise_file(&path).is_some() {
            return true;
        }
    }
    false
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

    pub fn load_clickpack(&mut self, clickpack_dir: &Path, pitch: Pitch) -> Result<()> {
        assert!(self.sample_rate > 0);
        let clickpack_dir = fix_root_subdir(clickpack_dir);

        for (i, dir) in CLICKPACK_DIRNAMES.iter().enumerate() {
            let mut path = clickpack_dir.to_path_buf();
            path.push(dir);
            self.clickpack[i] = PlayerClicks::from_path(&path, pitch, self.sample_rate);

            // try to load noise from the sound directories
            if !self.has_noise() {
                self.load_noise(&path);
            }
        }

        if !self.has_clicks() {
            log::warn!("folders {CLICKPACK_DIRNAMES:?} were not found in the clickpack, assuming there is only one player");
            self.clickpack[0] = PlayerClicks::from_path(&clickpack_dir, pitch, self.sample_rate);
        }

        // find longest click (will be used to ensure that the end doesn't get cut off)
        self.longest_click = self.clickpack.longest_click();
        log::debug!("longest click: {}s", self.longest_click);

        // try to load noise from the root clickpack dir
        if !self.has_noise() {
            self.load_noise(&clickpack_dir);
        }

        if self.has_clicks() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "no clicks found in clickpack, did you select the correct folder?"
            ))
        }
    }

    fn load_noise(&mut self, dir: &Path) {
        let Some(path) = find_noise_file(dir) else {
            return;
        };
        let Ok(f) = std::fs::File::open(path) else {
            return;
        };
        self.noise = if let Ok(mut noise) = AudioSegment::from_media_source(Box::new(f)) {
            noise.resample(self.sample_rate);
            Some(noise)
        } else {
            None
        };
    }

    fn get_random_click(&mut self, player: Player, click: Click) -> &AudioSegment {
        // try to get a random click/release from the player clicks
        // if it doesn't exist for the wanted player, use the other one (guaranteed to have atleast
        // one click)
        let p1 = &self.clickpack.player1;
        let p2 = &self.clickpack.player2;
        let l1 = &self.clickpack.left1;
        let r1 = &self.clickpack.right1;
        let l2 = &self.clickpack.left2;
        let r2 = &self.clickpack.right2;

        // :tired_face:
        macro_rules! random_click_ord {
            ($typ:ident, $one:ident, $two:ident, $three:ident, $four:ident, $five:ident, $six: ident) => {
                $one.random_click($typ).unwrap_or_else(|| {
                    $two.random_click($typ).unwrap_or_else(|| {
                        $three.random_click($typ).unwrap_or_else(|| {
                            $four.random_click($typ).unwrap_or_else(|| {
                                $five
                                    .random_click($typ)
                                    .unwrap_or_else(|| $six.random_click($typ).unwrap())
                            })
                        })
                    })
                })
            };
        }
        match click {
            Click::Regular(typ) => {
                if player == Player::One {
                    random_click_ord!(typ, p1, p2, l1, r1, l2, r2)
                } else {
                    random_click_ord!(typ, p2, p1, l2, r2, l1, r1)
                }
            }
            Click::Left(typ) => {
                if player == Player::One {
                    random_click_ord!(typ, l1, r1, p1, l2, r2, p2)
                } else {
                    random_click_ord!(typ, l2, r2, p2, l1, r1, p1)
                }
            }
            Click::Right(typ) => {
                if player == Player::One {
                    random_click_ord!(typ, r1, l1, p1, r2, l2, p2)
                } else {
                    random_click_ord!(typ, r2, l2, p2, r1, l1, p1)
                }
            }
        }
    }

    pub fn compile_expression(&mut self, expr: &str) -> Result<()> {
        let parser = fasteval2::Parser::new();
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
    pub fn update_namespace(
        &mut self,
        a: &ExtendedAction,
        prev_frame: u32,
        total_frames: u32,
        fps: f64,
    ) {
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
        self.ns.insert("rand".to_string(), fastrand::f64());
        self.ns
            .insert("delta".to_string(), (a.frame - prev_frame) as f64);
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
        let mut prev_frame = 0u32;
        for action in &replay.extended {
            self.update_namespace(action, prev_frame, replay.last_frame(), replay.fps.into());
            prev_frame = action.frame;

            let val = self.eval_expr().unwrap_or(0.);
            min = min.min(val);
            max = max.max(val);
        }
        (min, max)
    }

    #[allow(clippy::too_many_arguments)] // TODO
    pub fn render_replay(
        &mut self,
        replay: &Replay,
        noise: bool,
        noise_volume: f32,
        normalize: bool,
        expr_var: ExprVariable,
        enable_pitch: bool,
        cut_sounds: bool,
    ) -> AudioSegment {
        log::info!(
            "starting render, {} actions, noise: {noise}",
            replay.actions.len()
        );

        let longest_time_offset = if expr_var == ExprVariable::TimeOffset {
            self.expr_range(replay).1
        } else {
            0.0
        };

        let mut segment = AudioSegment::silent(
            self.sample_rate,
            replay.duration + self.longest_click + longest_time_offset,
        );
        let start = Instant::now();
        let mut prev_frame = 0u32;

        for (i, action) in replay.actions.iter().enumerate() {
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
                self.update_namespace(
                    &extended,
                    prev_frame,
                    replay.last_frame(),
                    replay.fps.into(),
                );
                prev_frame = extended.frame;

                let value = self.eval_expr().unwrap_or(0.0) as f32;
                match expr_var {
                    ExprVariable::Value => (value, 0.0),
                    ExprVariable::Variation { negative } => {
                        if value == 0.0 {
                            (0.0, 0.0)
                        } else if negative {
                            (f32_range((-value).min(value)..=value.max(-value)), 0.0)
                        } else {
                            (f32_range(value.min(0.0)..=value.max(0.0)), 0.0)
                        }
                    }
                    ExprVariable::TimeOffset => (0.0, value),
                    _ => unreachable!(),
                }
            } else {
                (0.0, 0.0)
            };

            let mut click = self.get_random_click(action.player, action.click);
            if enable_pitch {
                click = click.random_pitch(); // if no pitch table is generated, returns self
            }

            let mut until_next = f64::INFINITY;
            if cut_sounds {
                // find the time until the next action, so we know when to cut
                // off this sound
                for next in replay.actions.iter().skip(i + 1) {
                    if action.player == next.player && next.click.is_click() {
                        until_next = next.time - action.time;
                        break;
                    }
                }
            }

            // overlay
            segment.overlay_at_vol(
                action.time + time_offset as f64,
                click,
                1.0 + action.vol_offset + expr_vol,
                until_next,
            );
        }

        if noise && self.has_noise() {
            let mut noise_duration = Duration::from_secs(0);
            let noise_segment = self.noise.as_ref().unwrap();

            while noise_duration < segment.duration() {
                segment.overlay_at_vol(
                    noise_duration.as_secs_f64(),
                    noise_segment,
                    noise_volume,
                    f64::INFINITY, // don't cut off
                );
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
        self.clickpack.has_clicks()
    }

    pub fn convert_clickpack(
        &self,
        output_path: &Path,
        settings: &ClickpackConversionSettings,
    ) -> Result<()> {
        if !self.has_clicks() {
            anyhow::bail!("no clickpack is loaded");
        }

        // create output directory
        log::debug!("creating output directory for converted files: {output_path:?}");
        let mut path = PathBuf::from(output_path);
        std::fs::create_dir_all(&path)?;

        let convert_player = |player: &PlayerClicks, path: &Path| -> Result<()> {
            let mut player_path = path.to_path_buf();
            for (dir, clicks, is_clicks) in [
                ("hardclicks", &player.hardclicks, true),
                ("hardreleases", &player.hardreleases, false),
                ("clicks", &player.clicks, true),
                ("releases", &player.releases, false),
                ("softclicks", &player.softclicks, true),
                ("softreleases", &player.softreleases, false),
                ("microclicks", &player.microclicks, true),
                ("microreleases", &player.microreleases, false),
            ] {
                // check if we have any clicks in this click type
                if clicks.is_empty() {
                    continue;
                }

                player_path.push(dir);
                log::debug!("creating dir {player_path:?}");
                std::fs::create_dir_all(&player_path)?;

                for (i, click) in clicks.iter().enumerate() {
                    // apply settings
                    let mut click = click.clone();

                    // change volume
                    let change_volume = match settings.change_volume_for {
                        ChangeVolumeFor::All => true,
                        ChangeVolumeFor::Clicks => is_clicks,
                        ChangeVolumeFor::Releases => !is_clicks,
                    };
                    if change_volume && settings.volume != 1. {
                        click.set_volume(settings.volume);
                    }

                    // reverse
                    if settings.reverse {
                        click.reverse();
                    }

                    // remove silence
                    if settings.silence_threshold != 0. {
                        match settings.remove_silence {
                            RemoveSilenceFrom::Start => {
                                click.remove_silence_from_start(settings.silence_threshold)
                            }
                            RemoveSilenceFrom::End => {
                                click.remove_silence_from_end(settings.silence_threshold)
                            }
                            _ => {}
                        }
                    }

                    // create click file
                    if settings.rename_files {
                        player_path.push(format!("{}.wav", i + 1));
                    } else {
                        player_path.push(format!(
                            "{}.wav",
                            if let Some(stem) = Path::new(&click.filename).file_stem() {
                                stem.to_string_lossy().to_string()
                            } else {
                                click.filename.clone()
                            }
                        ));
                    }
                    log::debug!("creating file {player_path:?}");
                    let f = std::fs::File::create(&player_path)?;

                    // export wave file
                    log::debug!("exporting wav file to {player_path:?}");
                    click.export_wav(f)?;
                    player_path.pop();
                }
                player_path.pop();
            }

            Ok(())
        };

        // convert each sound folder
        for (i, dirname) in CLICKPACK_DIRNAMES.iter().enumerate() {
            let sounds = &self.clickpack[i];
            if !sounds.has_clicks() {
                continue;
            }
            path.push(dirname);
            std::fs::create_dir_all(&path)?;
            convert_player(sounds, &path)?;
            path.pop();
        }

        Ok(())
    }
}
