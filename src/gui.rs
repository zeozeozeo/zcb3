use crate::{built_info, Bot, Macro, MacroType};
use anyhow::{Context, Result};
use eframe::{
    egui::{self, RichText},
    IconData,
};
use egui_modal::{Icon, Modal};
use image::io::Reader as ImageReader;
use rfd::FileDialog;
use serde_json::Value;
use std::{io::Cursor, time::Instant};
use std::{io::Read, path::PathBuf};

pub fn run_gui() -> Result<(), eframe::Error> {
    let img = ImageReader::new(Cursor::new(include_bytes!("../icon.ico")))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap();

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(420.0, 390.0)),
        icon_data: Some(IconData {
            rgba: img.to_rgba8().to_vec(),
            width: img.width(),
            height: img.height(),
        }),
        ..Default::default()
    };
    eframe::run_native(
        "ZCB",
        options,
        Box::new(|_cc| {
            // egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::<App>::default()
        }),
    )
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
enum Stage {
    #[default]
    SelectReplay,
    SelectClickpack,
    Render,
}

impl Stage {
    fn previous(self) -> Self {
        match self {
            Stage::SelectClickpack => Stage::SelectReplay,
            Stage::Render => Stage::SelectClickpack,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
struct App {
    stage: Stage,
    replay: Macro,
    soft_threshold: f32,
    bot: Bot,
    output: Option<PathBuf>,
    volume_var: f32,
    noise: bool,
    normalize: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            stage: Stage::default(),
            replay: Macro::default(),
            soft_threshold: 0.15,
            bot: Bot::default(),
            output: None,
            volume_var: 0.20,
            noise: false,
            normalize: false,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            let mut dialog = Modal::new(ctx, "update_dialog");

            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if self.stage != Stage::SelectReplay {
                    if ui
                        .button("Back")
                        .on_hover_text("Go back to the previous stage")
                        .clicked()
                    {
                        self.stage = self.stage.previous();
                    }
                }
                if ui
                    .button("Check for updates")
                    .on_hover_text("Check if your ZCB version is up-to-date")
                    .clicked()
                {
                    self.do_update_check(&dialog);
                }
                ui.hyperlink_to("Join the discord server", "https://discord.gg/b4kBQyXYZT");
            });

            dialog.show_dialog();
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.stage {
                Stage::SelectReplay => self.show_replay_stage(ctx, ui),
                Stage::SelectClickpack => self.show_select_clickpack_stage(ctx, ui),
                Stage::Render => self.show_render_stage(ctx, ui),
            };
        });
    }
}

fn get_latest_tag() -> Result<usize> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Ubuntu Chromium/37.0.2062.94 Chrome/37.0.2062.94 Safari/537.36")
        .build()?;
    let resp = client
        .get("https://api.github.com/repos/zeozeozeo/zcb3/tags")
        .send()?
        .text()?;

    log::debug!("response text: '{resp}'");
    let v: Value = serde_json::from_str(&resp)?;
    let tags = v.as_array().context("not an array")?;
    let latest_tag = tags.get(0).context("couldn't latest tags")?;
    let name = latest_tag.get("name").context("couldn't get tag name")?;

    Ok(name
        .as_str()
        .context("tag name is not a string")?
        .replace(".", "")
        .parse()?)
}

fn get_current_tag() -> usize {
    built_info::PKG_VERSION.replace(".", "").parse().unwrap()
}

impl App {
    fn do_update_check(&mut self, dialog: &Modal) {
        let latest_tag = get_latest_tag();
        let current_tag = get_current_tag();

        if let Ok(tag) = latest_tag {
            log::info!("latest tag: {tag}, current tag {current_tag}");
            if tag > current_tag {
                dialog.open_dialog(
                    Some("New version available"), // title
                    Some(format!(
                        "A new version of ZCB is available (latest: {tag}, this: {current_tag}).\nDownload the new version on the GitHub page or in the Discord server."
                    )), // body
                    Some(Icon::Info),              // icon
                );
            } else {
                dialog.open_dialog(
                    Some("You are up-to-date!"),                                 // title
                    Some(format!("You are running the latest version of ZCB.\nYou can always download new versions on GitHub or on the Discord server.")), // body
                    Some(Icon::Success),                                         // icon
                );
            }
        } else if let Err(e) = latest_tag {
            log::error!("failed to check for updates: {e}");
            dialog.open_dialog(
                Some("Failed to check for updates"), // title
                Some(e),                             // body
                Some(Icon::Error),                   // icon
            );
            return;
        }
    }

    fn show_replay_stage(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("1. Select replay file");

        let mut dialog = Modal::new(ctx, "replay_stage_dialog");

        ui.horizontal(|ui| {
            ui.add(
                egui::Slider::new(&mut self.soft_threshold, -50.0..=50.0)
                    .text("Softclick threshold (seconds)"),
            );
            ui.add_enabled_ui(false, |ui| {
                ui.label("(?)").on_disabled_hover_text(
                    "Time between previous action and soft click in seconds",
                )
            });
        });

        ui.horizontal(|ui| {
            if ui.button("Select replay").clicked() {
                // FIXME: for some reason when selecting files there's a ~2 second freeze in debug mode
                if let Some(file) = FileDialog::new()
                    .add_filter("Replay file", &["json"])
                    .pick_file()
                {
                    log::info!("selected replay file: {file:?}");

                    let filename = file.file_name().unwrap().to_str().unwrap();

                    // read replay file
                    let mut f = std::fs::File::open(file.clone()).unwrap();
                    let mut data = String::new();
                    f.read_to_string(&mut data).unwrap();

                    let replay_type = MacroType::guess_format(&data, filename);
                    if let Ok(replay_type) = replay_type {
                        let replay = Macro::parse(replay_type, &data, 0.15);
                        if let Ok(replay) = replay {
                            self.replay = replay;
                            self.stage = Stage::SelectClickpack;
                        } else if let Err(e) = replay {
                            dialog.open_dialog(
                                Some("Failed to parse replay file"),             // title
                                Some(&format!("{e}. Is the format supported?")), // body
                                Some(Icon::Error),                               // icon
                            );
                        }
                    } else if let Err(e) = replay_type {
                        dialog.open_dialog(
                            Some("Failed to guess replay format"),           // title
                            Some(&format!("{e}. Is the format supported?")), // body
                            Some(Icon::Error),                               // icon
                        );
                    }
                } else {
                    dialog.open_dialog(
                        Some("No file was selected"), // title
                        Some("Please select a file"), // body
                        Some(Icon::Error),            // icon
                    )
                }
            }

            if self.replay.actions.len() > 0 {
                ui.label(format!(
                    "Number of actions in macro: {}",
                    self.replay.actions.len()
                ));
            }
        });

        ui.separator();
        ui.collapsing("Supported file formats", |ui| {
            ui.label(RichText::new("• Mega Hack Replay").strong());
            ui.label(RichText::new("• TASBOT Replay").strong());
            ui.label("...more coming in the next version");
        });

        // show dialog if there is one
        dialog.show_dialog();
    }

    fn show_select_clickpack_stage(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("2. Select clickpack");

        let mut dialog = Modal::new(ctx, "clickpack_stage_dialog");

        if ui.button("Select clickpack").clicked() {
            if let Some(dir) = FileDialog::new().pick_folder() {
                log::info!("selected clickpack folder: {dir:?}");
                self.bot = Bot::new(dir);
                self.stage = Stage::Render;
            } else {
                dialog.open_dialog(
                    Some("No directory was selected"), // title
                    Some("Please select a directory"), // body
                    Some(Icon::Error),                 // icon
                )
            }
        }

        ui.separator();
        ui.collapsing("Info", |ui| {
            ui.label("The clickpack should either have player1 and/or player2 folders inside it, or just audio files.");
            ui.label("Optionally you can put a noise.* or whitenoise.* file inside the clickpack folder to have an option to overlay background noise.");
            ui.label("All audio files will be resampled to 48kHz.");
            ui.label("Loading the clickpack may take a while, please be patient.");
        });
        ui.collapsing("Supported audio formats", |ui| {
            ui.label("AAC, ADPCM, ALAC, FLAC, MKV, MP1, MP2, MP3, MP4, OGG, Vorbis, WAV, and WebM audio files.");
        });

        dialog.show_dialog();
    }

    fn show_render_stage(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("3. Render");

        let mut dialog = Modal::new(ctx, "render_stage_dialog");

        ui.horizontal(|ui| {
            if ui.button("Select output file").clicked() {
                if let Some(path) = FileDialog::new()
                    .add_filter("Supported audio files", &["wav"])
                    .save_file()
                {
                    log::info!("selected output file: {path:?}");
                    self.output = Some(path);
                } else {
                    dialog.open_dialog(
                        Some("No output file was selected"),  // title
                        Some("Please select an output file"), // body
                        Some(Icon::Error),                    // icon
                    );
                }
            }
            if let Some(output) = &self.output {
                ui.label(format!(
                    "Selected output file: {}",
                    output.file_name().unwrap().to_str().unwrap()
                ));
            }
        });

        ui.separator();

        // volume variation slider
        ui.horizontal(|ui| {
            ui.add(
                egui::Slider::new(&mut self.volume_var, -50.0..=50.0).text("Volume variation"),
            );
            ui.add_enabled_ui(false, |ui| {
                ui.label("(?)").on_disabled_hover_text(
                    "Maximum volume variation (+/-) of each click (generated randomly).\n0 = no volume variation.",
                )
            });
        });

        // overlay noise checkbox
        ui.add_enabled_ui(self.bot.has_noise(), |ui| {
            ui.checkbox(&mut self.noise, "Overlay noise")
                .on_disabled_hover_text("Your clickpack doesn't have a noise file.")
                .on_hover_text("Overlays the noise file that's in the clickpack directory.");
        });
        ui.checkbox(&mut self.normalize, "Normalize audio")
            .on_hover_text(
                "Whether to normalize the output audio\n(make all samples to be in range of 0-1)",
            );

        ui.separator();

        ui.add_enabled_ui(self.output.is_some(), |ui| {
            if ui
                .button("Render!")
                .on_disabled_hover_text("Please select an output file.")
                .on_hover_text("Start rendering the macro.\nThis might take some time!")
                .clicked()
            {
                log::info!("rendering macro, this might take some time!");

                let start = Instant::now();
                let mut segment =
                    self.bot
                        .render_macro(self.replay.clone(), self.noise, self.volume_var);
                if self.normalize {
                    segment.normalize();
                }
                let end = start.elapsed();

                let f = std::fs::File::create(self.output.as_ref().unwrap());

                if let Ok(f) = f {
                    if let Err(e) = segment.export_wav(f) {
                        dialog.open_dialog(
                            Some("Failed to export .wav file"),           // title
                            Some(format!("{e}. Is the file writeable?")), // body
                            Some(Icon::Error),                            // icon
                        );
                    } else {
                        dialog.open_dialog(
                            Some("Done!"), // title
                            Some(format!(
                                "Successfully exported '{}' in {end:?} (~{} actions/second)",
                                self.output // lol
                                    .clone()
                                    .unwrap()
                                    .file_name()
                                    .unwrap()
                                    .to_str()
                                    .unwrap(),
                                self.replay.actions.len() as f32 / end.as_secs_f32(),
                            )), // body
                            Some(Icon::Success), // icon
                        );
                    }
                } else if let Err(e) = f {
                    dialog.open_dialog(
                        Some("Failed to open output file"),                 // title
                        Some(format!("{e}. Try running with administrator permissions or select a different output file")), // body
                        Some(Icon::Error),                                  // icon
                    );
                }
            }
        });

        dialog.show_dialog();
    }
}
