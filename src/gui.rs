use crate::built_info;
use anyhow::{Context, Result};
use bot::{Bot, Macro, MacroType, Pitch, Timings, VolumeSettings};
use eframe::{
    egui::{self, RichText},
    IconData,
};
use egui_modal::{Icon, Modal};
use image::io::Reader as ImageReader;
use rfd::FileDialog;
use serde_json::Value;
use std::{
    io::Cursor,
    sync::{Arc, Mutex},
    time::Instant,
};
use std::{io::Read, path::PathBuf};

pub fn run_gui() -> Result<(), eframe::Error> {
    let img = ImageReader::new(Cursor::new(include_bytes!("assets/icon.ico")))
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
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
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
    PweaseDonate,
}

impl Stage {
    fn previous(self) -> Self {
        match self {
            Stage::SelectClickpack => Stage::SelectReplay,
            Stage::Render => Stage::SelectClickpack,
            _ => self,
        }
    }
}

#[derive(Debug)]
struct App {
    stage: Stage,
    replay: Arc<Mutex<Macro>>,
    bot: Arc<Mutex<Bot>>,
    output: Option<PathBuf>,
    volume_var: f32,
    noise: bool,
    normalize: bool,
    pitch_enabled: bool,
    pitch: Pitch,
    timings: Timings,
    vol_settings: VolumeSettings,
    render_progress: Arc<Mutex<usize>>,
    /// Title and body of any rendering error (if any). The first
    /// `bool` determines whether it's an error or success.
    render_msg: Arc<Mutex<Option<(bool, String, String)>>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            stage: Stage::default(),
            replay: Arc::new(Mutex::new(Macro::default())),
            bot: Arc::new(Mutex::new(Bot::default())),
            output: None,
            volume_var: 0.20,
            noise: false,
            normalize: false,
            pitch_enabled: true,
            pitch: Pitch {
                from: 0.90,
                to: 1.1,
                step: 0.005,
            },
            timings: Timings {
                hard: 2.0,
                regular: 0.15,
                soft: 0.025,
            },
            vol_settings: VolumeSettings::default(),
            render_progress: Arc::new(Mutex::new(0)),
            render_msg: Arc::new(Mutex::new(None)),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.stage, Stage::SelectReplay, "Replay");
                ui.selectable_value(&mut self.stage, Stage::SelectClickpack, "Clickpack");
                ui.selectable_value(&mut self.stage, Stage::Render, "Render");
                ui.selectable_value(&mut self.stage, Stage::PweaseDonate, "Donate");
            });
            ui.add_space(2.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.stage {
                    Stage::SelectReplay => self.show_replay_stage(ctx, ui),
                    Stage::SelectClickpack => self.show_select_clickpack_stage(ctx, ui),
                    Stage::Render => self.show_render_stage(ctx, ui),
                    Stage::PweaseDonate => self.show_pwease_donate_stage(ctx, ui),
                };
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            let mut dialog = Modal::new(ctx, "update_dialog");

            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if self.stage != self.stage.previous() {
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

    fn show_pwease_donate_stage(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Donations");
        ui.label("If you like what I do, please consider supporting me. ZCB is completely free software :)");
        ui.label("By donating you'll get a custom role on the Discord server and early access to some of my future mods.");

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.add(egui::Image::new(egui::include_image!("assets/kofi_logo.png")).max_width(32.0));
            ui.hyperlink_to("Donate to me on Ko-fi", "https://ko-fi.com/zeozeozeo");
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::Image::new(egui::include_image!("assets/liberapay_logo.png")).max_width(32.0),
            );
            ui.hyperlink_to("Donate to me on Liberapay", "https://liberapay.com/zeo");
        });
    }

    fn show_replay_stage(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Select replay file");

        let mut dialog = Modal::new(ctx, "replay_stage_dialog");

        let t = &mut self.timings;
        ui.add(egui::Slider::new(&mut t.hard, t.regular..=30.0).text("Hard timing"));
        ui.add(egui::Slider::new(&mut t.regular, t.soft..=t.hard).text("Regular timing"));
        ui.add(egui::Slider::new(&mut t.soft, 0.0..=t.regular).text("Soft timing"));

        ui.separator();

        let vol = &mut self.vol_settings;
        ui.add(egui::Slider::new(&mut vol.volume_var, 0.0..=1.0).text("Volume variation"));
        ui.add(egui::Slider::new(&mut vol.global_volume, 0.0..=20.0).text("Global volume"));

        ui.separator();

        ui.checkbox(&mut vol.enabled, "Enable spam volume changes");

        ui.add_enabled_ui(vol.enabled, |ui| {
            ui.checkbox(&mut vol.change_releases_volume, "Change releases volume");
            ui.add(
                egui::Slider::new(&mut vol.spam_time, 0.0..=1.0)
                    .text("Spam time (between actions)"),
            );
            ui.add(
                egui::Slider::new(&mut vol.spam_vol_offset_factor, 0.0..=30.0)
                    .text("Spam volume offset factor"),
            );
            ui.add(
                egui::Slider::new(&mut vol.max_spam_vol_offset, 0.0..=30.0)
                    .text("Maximum spam volume offset"),
            );
        });

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("Select replay").clicked() {
                // FIXME: for some reason when selecting files there's a ~2 second freeze in debug mode
                if let Some(file) = FileDialog::new()
                    .add_filter(
                        "Replay file",
                        &["json", "mhr.json", "mhr", "zbf", "replay", "ybf", "echo"],
                    )
                    .pick_file()
                {
                    log::info!("selected replay file: {file:?}");

                    let filename = file.file_name().unwrap().to_str().unwrap();

                    // read replay file
                    let mut f = std::fs::File::open(file.clone()).unwrap();
                    let mut data = Vec::new();
                    f.read_to_end(&mut data).unwrap();

                    let replay_type = MacroType::guess_format(&data, filename);

                    if let Ok(replay_type) = replay_type {
                        let replay =
                            Macro::parse(replay_type, &data, self.timings, self.vol_settings);
                        if let Ok(replay) = replay {
                            self.replay = Arc::new(Mutex::new(replay));
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

            let num_actions = self.replay.lock().unwrap().actions.len();
            if num_actions > 0 {
                ui.label(format!("Number of actions in macro: {}", num_actions));
            }
        });

        ui.collapsing("Supported file formats", |ui| {
            ui.label(RichText::new("• Mega Hack Replay JSON (.mhr.json)").strong());
            ui.label(RichText::new("• Mega Hack Replay Binary (.mhr)").strong());
            ui.label(RichText::new("• TASBOT Replay (.json)").strong());
            ui.label(RichText::new("• Zbot Replay (.zbf)").strong());
            ui.label(RichText::new("• OmegaBot 2 Replay (.replay)").strong());
            ui.label(
                RichText::new("• Ybot Frame (no extension by default, rename to .ybf)").strong(),
            );
            ui.label(RichText::new("• Echo Binary (.echo)").strong());
            ui.label("Suggest more macro formats in the Discord server");
        });

        // show dialog if there is one
        dialog.show_dialog();
    }

    fn show_select_clickpack_stage(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Select clickpack");

        let mut dialog = Modal::new(ctx, "clickpack_stage_dialog");

        // pitch settings
        ui.checkbox(&mut self.pitch_enabled, "Pitch variation");
        ui.add_enabled_ui(self.pitch_enabled, |ui| {
            let p = &mut self.pitch;
            ui.add(egui::Slider::new(&mut p.from, 0.0..=p.to).text("Minimum pitch"));
            ui.add(egui::Slider::new(&mut p.to, p.from..=50.0).text("Maxiumum pitch"));
            ui.add(egui::Slider::new(&mut p.step, 0.0001..=1.0).text("Pitch step"));
        });
        ui.separator();

        if ui.button("Select clickpack").clicked() {
            if let Some(dir) = FileDialog::new().pick_folder() {
                log::info!("selected clickpack folder: {dir:?}");

                let bot = if self.pitch_enabled {
                    Bot::new(dir, self.pitch)
                } else {
                    Bot::new(dir, Pitch::default())
                };

                if let Ok(bot) = bot {
                    self.bot = Arc::new(Mutex::new(bot));
                    self.stage = Stage::Render;
                } else if let Err(e) = bot {
                    dialog.open_dialog(
                        Some("Failed to load clickpack"), // title
                        Some(e),                          // body
                        Some(Icon::Error),                // icon
                    )
                }
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
            ui.label("Pitch step is the step between pitch changes in the pitch table. The lower it is, the more random the pitch is. Pitch 1.0 = no pitch.");
        });
        ui.collapsing("Supported audio formats", |ui| {
            ui.label("AAC, ADPCM, ALAC, FLAC, MKV, MP1, MP2, MP3, MP4, OGG, Vorbis, WAV, and WebM audio files.");
        });

        dialog.show_dialog();
    }

    fn render_macro(
        bot: Arc<Mutex<Bot>>,
        replay: Arc<Mutex<Macro>>,
        noise: bool,
        normalize: bool,
        output: PathBuf,
        render_progress: Arc<Mutex<usize>>,
        render_msg: Arc<Mutex<Option<(bool, String, String)>>>,
    ) {
        rayon::spawn(move || {
            let start = Instant::now();
            let segment = bot.lock().unwrap().render_macro(
                &replay.lock().unwrap(),
                noise,
                normalize,
                Some(render_progress),
            );
            let end = start.elapsed();
            log::info!("rendered in {end:?}");

            let f = std::fs::File::create(output.clone());

            if let Ok(f) = f {
                if let Err(e) = segment.export_wav(f) {
                    *render_msg.lock().unwrap() = Some((
                        true,
                        "Failed to write output file!".to_string(),
                        format!("{e}. Try running the program as administrator."),
                    ));
                }
            } else if let Err(e) = f {
                *render_msg.lock().unwrap() = Some((
                    true,
                    "Failed to open output file!".to_string(),
                    format!("{e}. Try running the program as administrator."),
                ));
            }

            let num_actions = replay.lock().unwrap().actions.len();
            let filename = output.file_name().unwrap().to_str().unwrap();

            *render_msg.lock().unwrap() = Some((
                false,
                "Done!".to_string(),
                format!(
                    "Successfully exported '{filename}' in {end:?} (~{} actions/second)",
                    num_actions as f32 / end.as_secs_f32()
                ),
            ));
        });
    }

    fn show_render_stage(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Render");

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
        ui.add_enabled_ui(self.bot.lock().unwrap().has_noise(), |ui| {
            ui.checkbox(&mut self.noise, "Overlay noise")
                .on_disabled_hover_text("Your clickpack doesn't have a noise file.")
                .on_hover_text("Overlays the noise file that's in the clickpack directory.");
        });
        ui.checkbox(&mut self.normalize, "Normalize audio")
            .on_hover_text(
                "Whether to normalize the output audio\n(make all samples to be in range of 0-1)",
            );

        ui.separator();

        let render_msg = self.render_msg.lock().unwrap().clone();

        ui.add_enabled_ui(self.output.is_some() && render_msg.is_none(), |ui| {
            if ui
                .button("Render!")
                .on_disabled_hover_text("Please select an output file.")
                .on_hover_text("Start rendering the macro.\nThis might take some time!")
                .clicked()
            {
                // start render task (everything is wrapped in an Arc<Mutex<>>)
                // FIXME: for some reason this still freezes
                Self::render_macro(
                    self.bot.clone(),
                    self.replay.clone(),
                    self.noise,
                    self.normalize,
                    self.output.clone().unwrap(),
                    self.render_progress.clone(),
                    self.render_msg.clone(),
                );
            }
        });

        if render_msg.is_some() {
            let error = render_msg.as_ref().unwrap().0;
            let title = render_msg.as_ref().unwrap().1.clone();
            let body = render_msg.as_ref().unwrap().2.clone();

            dialog.open_dialog(
                Some(title),
                Some(body),
                Some(if error { Icon::Error } else { Icon::Success }),
            );

            // we displayed the message, clear it
            *self.render_msg.lock().unwrap() = None;
        }

        dialog.show_dialog();
    }
}
