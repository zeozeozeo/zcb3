use anyhow::Result;
use bot::{Click, ClickType, Replay};
use eframe::egui::{self};
use egui_modal::{Icon, Modal};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::NamedTempFile;

#[derive(Default)]
struct VideoPack {
    hardclicks: Vec<PathBuf>,
    hardreleases: Vec<PathBuf>,
    clicks: Vec<PathBuf>,
    releases: Vec<PathBuf>,
    softclicks: Vec<PathBuf>,
    softreleases: Vec<PathBuf>,
    microclicks: Vec<PathBuf>,
    microreleases: Vec<PathBuf>,
}

// https://stackoverflow.com/a/76820878
fn recurse_files(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut buf = vec![];
    let entries = path.read_dir()?;

    for entry in entries {
        let entry = entry?;
        let meta = entry.metadata()?;

        if meta.is_dir() {
            let mut subdir = recurse_files(&entry.path())?;
            buf.append(&mut subdir);
        }

        if meta.is_file() {
            buf.push(entry.path());
        }
    }

    Ok(buf)
}

impl VideoPack {
    fn load(path: &Path) -> Result<Self> {
        let mut pack = Self::default();
        for entry in path.read_dir()? {
            let path = entry?.path();
            if path.is_dir() {
                pack.load_dir(&path)?;
            }
        }
        if pack.num_videos() == 0 {
            anyhow::bail!("no videos found in videopack, did you select the wrong folder?");
        }
        Ok(pack)
    }

    fn load_dir(&mut self, path: &Path) -> Result<()> {
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
                clicks.append(&mut recurse_files(path)?);
            }
        }

        if !matched_any {
            log::debug!("directory {path:?} did not match any patterns");
        }
        Ok(())
    }

    fn num_videos(&self) -> usize {
        self.hardclicks.len()
            + self.hardreleases.len()
            + self.clicks.len()
            + self.releases.len()
            + self.softclicks.len()
            + self.softreleases.len()
            + self.microclicks.len()
            + self.microreleases.len()
    }

    fn grid_show_files(ui: &mut egui::Ui, clicks: &[PathBuf]) {
        ui.horizontal_wrapped(|ui| {
            for (i, path) in clicks.iter().enumerate() {
                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                ui.code(filename);
                if i < clicks.len() - 1 {
                    ui.label(", ");
                }
            }
        });
    }

    fn show_grid(&self, ui: &mut egui::Ui) {
        egui::Grid::new("videopack_grid")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.label("hardclicks");
                Self::grid_show_files(ui, &self.hardclicks);
                ui.end_row();

                ui.label("hardreleases");
                Self::grid_show_files(ui, &self.hardreleases);
                ui.end_row();

                ui.label("clicks");
                Self::grid_show_files(ui, &self.clicks);
                ui.end_row();

                ui.label("releases");
                Self::grid_show_files(ui, &self.releases);
                ui.end_row();

                ui.label("softclicks");
                Self::grid_show_files(ui, &self.softclicks);
                ui.end_row();

                ui.label("softreleases");
                Self::grid_show_files(ui, &self.softreleases);
                ui.end_row();

                ui.label("microclicks");
                Self::grid_show_files(ui, &self.microclicks);
                ui.end_row();

                ui.label("microreleases");
                Self::grid_show_files(ui, &self.microreleases);
                ui.end_row();
            });
    }

    fn file_for_click(&self, click: Click) -> Option<PathBuf> {
        macro_rules! rand_click {
            ($arr:expr) => {{
                if $arr.is_empty() {
                    continue;
                }
                $arr.get(fastrand::usize(..$arr.len()))
            }};
        }

        let mut path = None;

        for typ in click.click_type().preferred() {
            let p = match typ {
                ClickType::HardClick => rand_click!(self.hardclicks),
                ClickType::HardRelease => rand_click!(self.hardreleases),
                ClickType::Click => rand_click!(self.clicks),
                ClickType::Release => rand_click!(self.releases),
                ClickType::SoftClick => rand_click!(self.softclicks),
                ClickType::SoftRelease => rand_click!(self.softreleases),
                ClickType::MicroClick => rand_click!(self.microclicks),
                ClickType::MicroRelease => rand_click!(self.microreleases),
                ClickType::None => continue,
            };

            if let Some(p) = p {
                path = Some(p);
                break;
            }
        }

        path.cloned()
    }
}

#[derive(Default)]
pub struct Video {
    pack: Option<VideoPack>,
}

impl Video {
    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, replay: &Replay) {
        let mut modal = Modal::new(ctx, "video_modal");

        ui.heading("Video");
        ui.label(
            "Automatically concatenate video files based on actions in the replay. \
            Can be used to make a fake mouse overlay or not-so-legit handcam footage",
        );
        ui.separator();

        ui.collapsing("Videopack", |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("Like a clickpack, but with video files instead. Allowed folders: ");
                ui.code("hardclicks");
                ui.label(", ");
                ui.code("hardreleases");
                ui.label(", ");
                ui.code("clicks");
                ui.label(", ");
                ui.code("releases");
                ui.label(", ");
                ui.code("softclicks");
                ui.label(", ");
                ui.code("softreleases");
                ui.label(", ");
                ui.code("microclicks");
                ui.label(", ");
                ui.code("microreleases");
            });
            ui.horizontal(|ui| {
                if ui.button("Load").clicked() {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        let pack = VideoPack::load(&dir);
                        if let Ok(pack) = pack {
                            self.pack = Some(pack);
                        } else if let Err(e) = pack {
                            self.pack = None;
                            log::error!("error loading videopack: {e}");
                            modal
                                .dialog()
                                .with_title("Failed to load videopack")
                                .with_body(e)
                                .with_icon(Icon::Error)
                                .open();
                        }
                    }
                }
                if let Some(pack) = &self.pack {
                    let num_videos = pack.num_videos();
                    ui.label(format!(
                        "Loaded {} video{}",
                        num_videos,
                        if num_videos == 1 { "" } else { "s" }
                    ));
                }
            });

            if let Some(pack) = &self.pack {
                pack.show_grid(ui);
            }
        });

        ui.collapsing("Render", |ui| {
            ui.label(
                "Render a video file based on the actions in the replay. \
                Requires FFmpeg to be installed",
            );

            const VIDEO_EXTS: &[&str; 9] = &[
                "mp4", "mkv", "avi", "mov", "webm", "flv", "wmv", "m4v", "3gp",
            ];

            if ui.button("Render").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Video", VIDEO_EXTS)
                    .save_file()
                {
                    if let Err(e) = self.render(replay, &path) {
                        log::error!("{e}");
                        modal
                            .dialog()
                            .with_title("Failed to render video")
                            .with_body(e)
                            .with_icon(Icon::Error)
                            .open();
                    }
                }
            }
        });

        modal.show_dialog();
    }

    fn make_command(
        &self,
        replay: &Replay,
        output: &Path,
        filter_tmpfile: &mut NamedTempFile,
        input_tmpfile: &mut NamedTempFile,
    ) -> Result<Vec<String>> {
        let mut cmd = Vec::new();
        let Some(pack) = &self.pack else {
            anyhow::bail!("no videopack loaded");
        };

        // we'll also build the concat filter argument
        let mut filter_complex = Vec::new();

        for (i, action) in replay.actions.iter().enumerate() {
            if let Some(file) = pack.file_for_click(action.click) {
                // get time between current and next action
                let dur = replay.actions.get(i + 1).map(|a| a.time - action.time);

                // write input file
                writeln!(input_tmpfile, "file '{}'", file.to_string_lossy())?;

                // if this is not the last clip, cut it to the start
                // of the next clip
                if let Some(dur) = dur {
                    writeln!(input_tmpfile, "outpoint {dur}")?;
                }

                filter_complex.push(format!("[{i}:v] [{i}:a]"));
            }
        }

        // finish building the concat filter
        filter_complex.push(format!("concat=n={}:v=1:a=1 [v] [a]", filter_complex.len()));
        log::debug!("filter_complex: {filter_complex:?}");

        // add the input files (temp file with the input commands in this case)
        cmd.push("-i".to_owned());
        cmd.push(input_tmpfile.path().to_string_lossy().into_owned());

        // since the maximum command length is 8191 characters, we'll have
        // to resort to temp files for the filter
        filter_tmpfile.write_all(filter_complex.join(" ").as_bytes())?;

        // add the filter to the command & map arguments
        cmd.push("-filter_complex_script".to_string()); // "_script" to specify a file
        cmd.push(filter_tmpfile.path().to_string_lossy().into_owned());
        cmd.push("-map".to_string());
        cmd.push("[v]".to_string());
        cmd.push("-map".to_string());
        cmd.push("[a]".to_string());

        // add the output file
        cmd.push(output.to_string_lossy().into_owned());
        Ok(cmd)
    }

    fn render(&self, replay: &Replay, output: &Path) -> Result<()> {
        // make temp files
        let mut filter_tmpfile = tempfile::Builder::new().suffix(".txt").tempfile()?;
        let mut input_tmpfile = tempfile::Builder::new().suffix(".txt").tempfile()?;

        // spawn child process
        let cmd = self.make_command(replay, output, &mut filter_tmpfile, &mut input_tmpfile)?;
        log::info!("ffmpeg arguments: {cmd:?}");
        let output = Command::new("ffmpeg").args(cmd).output()?;

        if !output.status.success() {
            anyhow::bail!(
                "failed to render video: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }
}
