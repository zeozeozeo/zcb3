use crate::built_info;
use anyhow::{Context, Result};
use bot::{Bot, Macro, MacroType, Pitch, Timings, VolumeSettings};
use eframe::{
    egui::{self, Key},
    IconData,
};
use egui_modal::{Icon, Modal};
use image::io::Reader as ImageReader;
use rfd::FileDialog;
use serde_json::Value;
use std::{io::Cursor, time::Instant};
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
    // AutoCutter,
    PweaseDonate,
    Secret,
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

// #[derive(Debug)]
struct App {
    stage: Stage,
    replay: Macro,
    bot: Bot,
    output: Option<PathBuf>,
    volume_var: f32,
    noise: bool,
    normalize: bool,
    pitch_enabled: bool,
    pitch: Pitch,
    timings: Timings,
    vol_settings: VolumeSettings,
    // autocutter: AutoCutter,
    last_chars: [Key; 9],
    char_idx: u8,
    litematic_export_releases: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            stage: Stage::default(),
            replay: Macro::default(),
            bot: Bot::default(),
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
            // autocutter: AutoCutter::default(),
            last_chars: [Key::A; 9],
            char_idx: 0,
            litematic_export_releases: false,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.input(|i| {
            const BOYKISSER: [Key; 9] = [
                Key::B,
                Key::O,
                Key::Y,
                Key::K,
                Key::I,
                Key::S,
                Key::S,
                Key::E,
                Key::R,
            ];
            for key in BOYKISSER {
                if i.key_pressed(key) {
                    self.last_chars[self.char_idx as usize] = key;
                    self.char_idx += 1;
                    self.char_idx %= BOYKISSER.len() as u8;
                    break;
                }
            }
            if self.last_chars == BOYKISSER {
                self.last_chars = [Key::A; BOYKISSER.len()];
                self.stage = Stage::Secret;
            }
        });

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.stage, Stage::SelectReplay, "Replay");
                ui.selectable_value(&mut self.stage, Stage::SelectClickpack, "Clickpack");
                ui.selectable_value(&mut self.stage, Stage::Render, "Render");
                // ui.selectable_value(&mut self.stage, Stage::AutoCutter, "AutoCutter");
                ui.selectable_value(&mut self.stage, Stage::PweaseDonate, "Donate");
                if self.stage == Stage::Secret {
                    ui.selectable_value(&mut self.stage, Stage::Secret, "Secret");
                }
            });
            ui.add_space(2.0);
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            let mut dialog = Modal::new(ctx, "update_dialog");

            egui::ScrollArea::horizontal().show(ui, |ui| {
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
            });

            dialog.show_dialog();
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.stage {
                    Stage::SelectReplay => self.show_replay_stage(ctx, ui),
                    Stage::SelectClickpack => self.show_select_clickpack_stage(ctx, ui),
                    Stage::Render => self.show_render_stage(ctx, ui),
                    // Stage::AutoCutter => self.autocutter.show_ui(ctx, ui),
                    Stage::PweaseDonate => self.show_pwease_donate_stage(ctx, ui),
                    Stage::Secret => self.show_secret_stage(ctx, ui),
                };
            });
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

    fn export_litematic(&self) {
        use rustmatica::{
            block_state::types::{HorizontalDirection::*, Instrument},
            util::{UVec3, Vec3},
            BlockState, Region,
        };
        let mut blocks: Vec<(BlockState, bool)> = vec![];

        // 1 repeater tick = 2 game ticks, or 0.1 seconds
        let mut prev_time = 0.;
        for action in &self.replay.actions {
            if !self.litematic_export_releases && action.click.is_release() {
                continue;
            }

            let mut delay = (action.time - prev_time) / 1.42; // 142% speed makes it align a bit better
            if self.litematic_export_releases {
                delay /= 1.42;
            }
            prev_time = action.time;

            let ticks = delay / 0.1;
            // repeaters can have 4 ticks max, so we need to duplicate
            // some repeaters if the action delay is more than 0.4s
            let repeaters = ((ticks / 4.).round() as usize).max(1);
            let last_ticks = ((ticks % 4.).round() as usize).clamp(0, 3) as u8 + 1;

            for i in 0..repeaters {
                let block = BlockState::Repeater {
                    delay: if i != repeaters - 1 { 4 } else { last_ticks },
                    facing: West, // points to east
                    locked: false,
                    powered: false,
                };
                blocks.push((block, false));
            }

            // now, we need to add the note block
            blocks.push((
                BlockState::NoteBlock {
                    instrument: if action.click.is_release() {
                        Instrument::Basedrum
                    } else {
                        Instrument::Hat
                    },
                    note: 0,
                    powered: false,
                },
                action.click.is_release(),
            ))
        }

        let mut region = Region::new(
            "omagah".into(),
            UVec3::new(0, 0, 0),
            Vec3::new(blocks.len() as i32, 2, 1),
        );

        for (x, block) in blocks.iter().enumerate() {
            let is_release = block.1;
            region.set_block(
                UVec3::new(x, 0, 0),
                if is_release {
                    BlockState::Stone
                } else {
                    BlockState::Glass
                },
            );
            region.set_block(UVec3::new(x, 1, 0), block.0.clone());
        }

        let litematic = region.as_litematic("Made with ZCB3".into(), "zeozeozeo".into());
        let _ = litematic.write_file("omagah.litematic");
    }

    fn show_secret_stage(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        // this is so epic
        ui.add_enabled_ui(!self.replay.actions.is_empty(), |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("Export replay to .litematic")
                    .on_disabled_hover_text("You have to load a replay first")
                    .clicked()
                {
                    self.export_litematic();
                }
                ui.checkbox(&mut self.litematic_export_releases, "Export releases");
            });
        });
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
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::Image::new(egui::include_image!("assets/donationalerts_logo.png"))
                    .max_width(32.0),
            );
            ui.hyperlink_to(
                "Donate to me on DonationAlerts",
                "https://donationalerts.com/r/zeozeozeo",
            );
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::Image::new(egui::include_image!("assets/boosty_logo.png")).max_width(32.0),
            );
            ui.hyperlink_to(
                "Donate to me on Boosty",
                "https://boosty.to/zeozeozeo/donate",
            );
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
                    .add_filter("Replay file", Macro::SUPPORTED_EXTENSIONS)
                    .pick_file()
                {
                    log::info!("selected replay file: {file:?}");

                    let filename = file.file_name().unwrap().to_str().unwrap();

                    // read replay file
                    let mut f = std::fs::File::open(file.clone()).unwrap();
                    let mut data = Vec::new();
                    f.read_to_end(&mut data).unwrap();

                    let replay_type = MacroType::guess_format(filename);

                    if let Ok(replay_type) = replay_type {
                        let replay =
                            Macro::parse(replay_type, &data, self.timings, self.vol_settings);
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

            let num_actions = self.replay.actions.len();
            if num_actions > 0 {
                ui.label(format!("Number of actions in macro: {}", num_actions));
            }
        });

        ui.collapsing("Supported file formats", |ui| {
            ui.label("• Mega Hack Replay JSON (.mhr.json)");
            ui.label("• Mega Hack Replay Binary (.mhr)");
            ui.label("• TASBOT Replay (.json)");
            ui.label("• Zbot Replay (.zbf)");
            ui.label("• OmegaBot 2 Replay (.replay)");
            ui.label("• Ybot Frame (no extension by default, rename to .ybf)");
            ui.label("• Echo Binary (.echo)");
            ui.label("• Amethyst Replay (.thyst)");
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
                    self.bot = bot;
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

    fn render_replay(&mut self, dialog: &Modal) {
        let start = Instant::now();
        let segment = self
            .bot
            .render_macro(&self.replay, self.noise, self.normalize, None);
        let end = start.elapsed();
        log::info!("rendered in {end:?}");

        let output = self
            .output
            .clone()
            .unwrap_or(PathBuf::from("you_shouldnt_see_this.wav"));
        let f = std::fs::File::create(output.clone());

        if let Ok(f) = f {
            if let Err(e) = segment.export_wav(f) {
                dialog.open_dialog(
                    Some("Failed to write output file!"),
                    Some(format!("{e}. Try running the program as administrator.")),
                    Some(Icon::Error),
                );
            }
        } else if let Err(e) = f {
            dialog.open_dialog(
                Some("Failed to open output file!"),
                Some(format!("{e}. Try running the program as administrator.")),
                Some(Icon::Error),
            );
        }

        let num_actions = self.replay.actions.len();
        let filename = output.file_name().unwrap().to_str().unwrap();

        dialog.open_dialog(
            Some("Done!"),
            Some(format!(
                "Successfully exported '{filename}' in {end:?} (~{} actions/second)",
                num_actions as f32 / end.as_secs_f32()
            )),
            Some(Icon::Success),
        );
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

        let has_output = self.output.is_some();
        let has_clicks = self.bot.has_clicks();
        ui.add_enabled_ui(
            has_output && has_clicks && !self.replay.actions.is_empty(),
            |ui| {
                if ui
                    .button("Render!")
                    .on_disabled_hover_text(if !has_output {
                        "Please select an output file"
                    } else if !has_clicks {
                        "Please select a clickpack"
                    } else {
                        "Please load a replay"
                    })
                    .on_hover_text("Start rendering the replay.\nThis might take some time!")
                    .clicked()
                {
                    // start render task (everything is wrapped in an Arc<Mutex<>>)
                    // FIXME: for some reason this still freezes
                    self.render_replay(&dialog);
                }
            },
        );

        dialog.show_dialog();
    }
}
