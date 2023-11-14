use crate::built_info;
use anyhow::{Context, Result};
use bot::{Bot, ExprVariable, ExtendedAction, Pitch, Replay, ReplayType, Timings, VolumeSettings};
use eframe::{
    egui::{self, Key, RichText},
    epaint::Color32,
    IconData,
};
use egui_modal::{Icon, Modal};
use egui_plot::PlotPoint;
use image::io::Reader as ImageReader;
use rfd::FileDialog;
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{cell::RefCell, io::Cursor, rc::Rc, time::Instant};
use std::{io::Read, path::PathBuf};

const MAX_PLOT_POINTS: usize = 4096;

pub fn run_gui() -> Result<(), eframe::Error> {
    rust_i18n::set_locale("en");

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
    Donate,
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

fn get_version() -> String {
    built_info::PKG_VERSION.to_string()
}

#[derive(Serialize, Deserialize)]
struct Config {
    #[serde(default = "get_version")]
    version: String,
    noise: bool,
    normalize: bool,
    pitch_enabled: bool,
    pitch: Pitch,
    timings: Timings,
    vol_settings: VolumeSettings,
    litematic_export_releases: bool,
    sample_rate: u32,
    expr_text: String,
    expr_variable: ExprVariable,
    sort_actions: bool,
    plot_data_aspect: f32,
}

impl Config {
    fn save(&self, path: &PathBuf) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    fn load(&mut self, path: &PathBuf) -> Result<()> {
        let f = std::fs::File::open(path)?;
        *self = serde_json::from_reader(f)?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: get_version(),
            noise: false,
            normalize: false,
            pitch_enabled: true,
            pitch: Pitch::default(),
            timings: Timings::default(),
            vol_settings: VolumeSettings::default(),
            litematic_export_releases: false,
            sample_rate: 44100,
            expr_text: String::new(),
            expr_variable: ExprVariable::Variation,
            sort_actions: true,
            plot_data_aspect: 20.0,
        }
    }
}

struct App {
    conf: Config,
    stage: Stage,
    replay: Replay,
    bot: Rc<RefCell<Bot>>,
    output: Option<PathBuf>,
    // autocutter: AutoCutter,
    last_chars: [Key; 9],
    char_idx: u8,
    expr_error: String,
    plot_points: Vec<PlotPoint>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            conf: Config::default(),
            stage: Stage::default(),
            replay: Replay::default(),
            bot: Rc::new(RefCell::new(Bot::default())),
            output: None,
            // autocutter: AutoCutter::default(),
            last_chars: [Key::A; 9],
            char_idx: 0,
            expr_error: String::new(),
            plot_points: vec![],
        }
    }
}

/// Value is always min clamped with 1.
fn u32_edit_field(ui: &mut egui::Ui, value: &mut u32) -> egui::Response {
    let mut tmp_value = format!("{value}");
    let res = ui.text_edit_singleline(&mut tmp_value);
    if let Ok(result) = tmp_value.parse::<u32>() {
        *value = result.max(1);
    }
    res
}

/*
fn i64_edit_field(ui: &mut egui::Ui, value: &mut i64) -> egui::Response {
    let mut tmp_value = format!("{value}");
    let res = ui.text_edit_singleline(&mut tmp_value);
    if let Ok(result) = tmp_value.parse::<i64>() {
        *value = result;
    }
    res
}
*/

fn help_text<R>(ui: &mut egui::Ui, help: &str, add_contents: impl FnOnce(&mut egui::Ui) -> R) {
    ui.horizontal(|ui| {
        add_contents(ui);
        ui.add_enabled_ui(false, |ui| ui.label("(?)").on_disabled_hover_text(help));
    });
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.input(|i| {
            use Key::*;
            const BOYKISSER: [Key; 9] = [B, O, Y, K, I, S, S, E, R];
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
                ui.selectable_value(&mut self.stage, Stage::SelectReplay, t!("topbar.replay"));
                ui.selectable_value(
                    &mut self.stage,
                    Stage::SelectClickpack,
                    t!("topbar.clickpack"),
                );
                ui.selectable_value(&mut self.stage, Stage::Render, t!("topbar.render"));
                // ui.selectable_value(&mut self.stage, Stage::AutoCutter, t!("topbar.autocutter"));
                ui.selectable_value(&mut self.stage, Stage::Donate, t!("topbar.donate"));
                if self.stage == Stage::Secret {
                    ui.selectable_value(&mut self.stage, Stage::Secret, t!("topbar.secret"));
                }
            });
            ui.add_space(2.0);
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            let mut dialog = Modal::new(ctx, "update_and_config_dialog");

            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    if self.stage != self.stage.previous()
                        && ui
                            .button(t!("bottombar.back"))
                            .on_hover_text(t!("bottombar.back_hover"))
                            .clicked()
                    {
                        self.stage = self.stage.previous();
                    }
                    if ui
                        .button(t!("bottombar.check_for_updates"))
                        .on_hover_text(t!("bottombar.check_for_updates"))
                        .clicked()
                    {
                        self.do_update_check(&dialog);
                    }
                    ui.hyperlink_to("Join the Discord server", "https://discord.gg/b4kBQyXYZT");

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button("Load")
                            .on_hover_text("Load a configuration file")
                            .clicked()
                        {
                            self.load_config(&dialog);
                        }
                        if ui
                            .button("Save")
                            .on_hover_text("Save the current configuration")
                            .clicked()
                        {
                            self.save_config(&dialog);
                        }
                    });
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
                    Stage::Donate => self.show_pwease_donate_stage(ctx, ui),
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
        .replace('.', "")
        .parse()?)
}

fn get_current_tag() -> usize {
    built_info::PKG_VERSION.replace('.', "").parse().unwrap()
}

impl App {
    fn do_update_check(&mut self, dialog: &Modal) {
        let latest_tag = get_latest_tag();
        let current_tag = get_current_tag();

        if let Ok(tag) = latest_tag {
            log::info!("latest tag: {tag}, current tag {current_tag}");
            if tag > current_tag {
                dialog.open_dialog(
                    Some(t!("update.new_version_title")),
                    Some(t!(
                        "update.new_version_body",
                        tag = tag,
                        current_tag = current_tag,
                    )),
                    Some(Icon::Info),
                );
            } else {
                dialog.open_dialog(
                    Some(t!("update.up_to_date_title")),
                    Some(t!("update.up_to_date_body")),
                    Some(Icon::Success),
                );
            }
        } else if let Err(e) = latest_tag {
            log::error!("failed to check for updates: {e}");
            dialog.open_dialog(
                Some(t!("update.failed_to_check")),
                Some(e),
                Some(Icon::Error),
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
            if !self.conf.litematic_export_releases && action.click.is_release() {
                continue;
            }

            let mut delay = (action.time - prev_time) / 1.42; // 142% speed makes it align a bit better
            if self.conf.litematic_export_releases {
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

    fn save_config(&self, dialog: &Modal) {
        if let Some(file) = FileDialog::new()
            .add_filter("Config file", &["json"])
            .save_file()
        {
            if let Err(e) = self.conf.save(&file) {
                dialog.open_dialog(Some("Failed to save config"), Some(e), Some(Icon::Error));
            }
        } else {
            dialog.open_dialog(
                Some("No file was selected"),
                Some("Please select a file"),
                Some(Icon::Error),
            );
        }
    }

    fn load_config(&mut self, dialog: &Modal) {
        if let Some(file) = FileDialog::new()
            .add_filter("Config file", &["json"])
            .pick_file()
        {
            if let Err(e) = self.conf.load(&file) {
                dialog.open_dialog(Some("Failed to load config"), Some(e), Some(Icon::Error));
            }
        } else {
            dialog.open_dialog(
                Some("No file was selected"),
                Some("Please select a file"),
                Some(Icon::Error),
            );
        }
    }

    fn show_secret_stage(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        // this is so epic
        ui.add_enabled_ui(!self.replay.actions.is_empty(), |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(t!("secret.export_to_litematic"))
                    .on_disabled_hover_text(t!("secret.export_to_litematic_disabled_hover"))
                    .clicked()
                {
                    self.export_litematic();
                }
                ui.checkbox(
                    &mut self.conf.litematic_export_releases,
                    t!("secret.export_releases"),
                );
            });
        });

        #[cfg(windows)]
        ui.horizontal(|ui| {
            if ui
                .button("Don't hide console on startup")
                .on_hover_text("Run with 'set RUST_LOG=debug' to see debug logs")
                .clicked()
            {
                // on Windows there's a check for this on startup
                let _ = std::fs::File::create("zcb3.debug");
            }
            ui.label("(applied after restart)");
        });
    }

    fn show_pwease_donate_stage(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading(t!("donations.heading"));
        ui.label(t!("donations.consider_supporting"));

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.add(egui::Image::new(egui::include_image!("assets/kofi_logo.png")).max_width(32.0));
            ui.hyperlink_to(t!("donations.kofi"), "https://ko-fi.com/zeozeozeo");
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::Image::new(egui::include_image!("assets/liberapay_logo.png")).max_width(32.0),
            );
            ui.hyperlink_to(t!("donations.liberapay"), "https://liberapay.com/zeo");
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::Image::new(egui::include_image!("assets/donationalerts_logo.png"))
                    .max_width(32.0),
            );
            ui.hyperlink_to(
                t!("donations.donationalerts"),
                "https://donationalerts.com/r/zeozeozeo",
            );
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::Image::new(egui::include_image!("assets/boosty_logo.png")).max_width(32.0),
            );
            ui.hyperlink_to(t!("donations.boosty"), "https://boosty.to/zeozeozeo/donate");
        });
    }

    fn show_replay_stage(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading(t!("replay.heading"));

        let mut dialog = Modal::new(ctx, "replay_stage_dialog");

        ui.collapsing("Timings", |ui| {
            ui.label("Click type timings. The number is the delay between actions (in seconds). \
                    If the delay between the current and previous action is bigger than the specified \
                    timing, the corresponding click type is used.");
            let t = &mut self.conf.timings;
            help_text(ui, "Hardclick/hardrelease timing", |ui| {
                ui.add(egui::Slider::new(&mut t.hard, t.regular..=30.0).text(t!("replay.hard_timing")));
            });
            help_text(ui, "Click/release timing", |ui| {
                ui.add(
                    egui::Slider::new(&mut t.regular, t.soft..=t.hard)
                        .text(t!("replay.regular_timing")),
                );
            });
            help_text(ui, "Softclick/softrelease timing", |ui| {
                ui.add(egui::Slider::new(&mut t.soft, 0.0..=t.regular).text(t!("replay.soft_timing")));
            });
            ui.label(format!("Everything below {}s are microclicks/microreleases.", t.soft));
        });

        ui.collapsing("Volume settings", |ui| {
            ui.label("General volume settings.");

            let vol = &mut self.conf.vol_settings;
            help_text(ui, "Maximum volume variation for each action (+/-)", |ui| {
                ui.add(
                    egui::Slider::new(&mut vol.volume_var, 0.0..=1.0)
                        .text(t!("replay.volume_variation")),
                );
            });
            help_text(ui, "Constant volume multiplier for all actions", |ui| {
                ui.add(
                    egui::Slider::new(&mut vol.global_volume, 0.0..=20.0)
                        .text(t!("replay.global_volume")),
                );
            });
        });

        ui.collapsing("Spam volume changes", |ui| {
            ui.label(
                "This can be used to change the volume of the clicks in spams. \
                    The spam time is the maximum time between actions when they can be \
                    considered 'spam actions'. The spam volume offset factor depends on the delta. \
                    The maximum spam offset factor is the maximum value this factor can be.",
            );

            let vol = &mut self.conf.vol_settings;
            ui.checkbox(&mut vol.enabled, t!("replay.enable_spam_volume_changes"));

            ui.add_enabled_ui(vol.enabled, |ui| {
                help_text(ui, "Apply spam volume changes to releases", |ui| {
                    ui.checkbox(
                        &mut vol.change_releases_volume,
                        t!("replay.change_releases_volume"),
                    );
                });
                help_text(
                    ui,
                    "Time between actions when they can be considered spam",
                    |ui| {
                        ui.add(
                            egui::Slider::new(&mut vol.spam_time, 0.0..=1.0)
                                .text(t!("replay.spam_time")),
                        );
                    },
                );
                help_text(ui, "Volume offset factor for spam actions", |ui| {
                    ui.add(
                        egui::Slider::new(&mut vol.spam_vol_offset_factor, 0.0..=30.0)
                            .text(t!("replay.spam_offset_factor")),
                    );
                });
                help_text(ui, "Maximum value of the volume offset factor", |ui| {
                    ui.add(
                        egui::Slider::new(&mut vol.max_spam_vol_offset, 0.0..=30.0)
                            .text(t!("replay.max_spam_offset")),
                    );
                });
            });
        });

        help_text(ui, "Sort actions by time", |ui| {
            ui.checkbox(&mut self.conf.sort_actions, "Sort actions");
        });
        ui.separator();

        ui.horizontal(|ui| {
            if ui.button(t!("replay.select_replay")).clicked() {
                // FIXME: for some reason when selecting files there's a ~2 second freeze in debug mode
                if let Some(file) = FileDialog::new()
                    .add_filter(
                        t!("replay.replay_file_explorer"),
                        Replay::SUPPORTED_EXTENSIONS,
                    )
                    .pick_file()
                {
                    log::info!("selected replay file: {file:?}");

                    let filename = file.file_name().unwrap().to_str().unwrap();

                    // read replay file
                    let mut f = std::fs::File::open(file.clone()).unwrap();
                    let mut data = Vec::new();
                    f.read_to_end(&mut data).unwrap();

                    let replay_type = ReplayType::guess_format(filename);

                    if let Ok(replay_type) = replay_type {
                        // parse replay
                        let replay = Replay::build()
                            .with_timings(self.conf.timings)
                            .with_vol_settings(self.conf.vol_settings)
                            .with_extended(true)
                            .with_sort_actions(self.conf.sort_actions)
                            .parse(replay_type, &data);

                        if let Ok(replay) = replay {
                            self.replay = replay;
                            self.stage = Stage::SelectClickpack;
                        } else if let Err(e) = replay {
                            dialog.open_dialog(
                                Some(t!("replay.failed_to_parse_title")),
                                Some(t!("replay.failed_to_parse_body", e = e)),
                                Some(Icon::Error),
                            );
                        }
                    } else if let Err(e) = replay_type {
                        dialog.open_dialog(
                            Some(t!("replay.failed_to_guess_title")),
                            Some(t!("replay.failed_to_parse_body", e = e)),
                            Some(Icon::Error),
                        );
                    }
                } else {
                    dialog.open_dialog(
                        Some(t!("replay.no_file_title")),
                        Some(t!("replay.no_file_body")),
                        Some(Icon::Error),
                    )
                }
            }

            let num_actions = self.replay.actions.len();
            let num_extended = self.replay.extended.len();
            if num_actions > 0 {
                ui.label(t!(
                    "replay.num_actions",
                    num_actions = num_actions,
                    num_extended_actions = num_extended
                ));
            }
        });

        ui.collapsing(t!("replay.supported_file_formats_collapsing"), |ui| {
            ui.label(t!("replay.file_format_1"));
            ui.label(t!("replay.file_format_2"));
            ui.label(t!("replay.file_format_3"));
            ui.label(t!("replay.file_format_4"));
            ui.label(t!("replay.file_format_5"));
            ui.label(t!("replay.file_format_6"));
            ui.label(t!("replay.file_format_7"));
            ui.label(t!("replay.file_format_8"));
            ui.label(t!("replay.file_format_9"));
            ui.label(t!("replay.file_format_10"));
            ui.label(t!("replay.file_format_11"));
            ui.label(t!("replay.file_format_12"));
            ui.label(t!("replay.file_format_13"));
            ui.label(t!("replay.file_format_14"));
            ui.label(t!("replay.file_format_15"));
            ui.label(t!("replay.file_format_16"));
            ui.label(t!("replay.file_format_17"));
        });

        // show dialog if there is one
        dialog.show_dialog();
    }

    fn show_select_clickpack_stage(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Select clickpack");

        let mut dialog = Modal::new(ctx, "clickpack_stage_dialog");

        // pitch settings
        ui.collapsing("Pitch variation", |ui| {
            ui.label(
                "Pitch variation can make clicks sound more realistic by \
                    changing their pitch randomly.",
            );
            ui.checkbox(&mut self.conf.pitch_enabled, "Enable pitch variation");
            ui.add_enabled_ui(self.conf.pitch_enabled, |ui| {
                let p = &mut self.conf.pitch;
                help_text(ui, "Minimum pitch value. 1 = no change", |ui| {
                    ui.add(egui::Slider::new(&mut p.from, 0.0..=p.to).text("Minimum pitch"));
                });
                help_text(ui, "Maximum pitch value. 1 = no change", |ui| {
                    ui.add(egui::Slider::new(&mut p.to, p.from..=50.0).text("Maxiumum pitch"));
                });
                help_text(
                    ui,
                    "Step between pitch values. The more = the better & the slower",
                    |ui| {
                        ui.add(egui::Slider::new(&mut p.step, 0.0001..=1.0).text("Pitch step"));
                    },
                );
            });
        });

        // samplerate edit field
        ui.collapsing("Other", |ui| {
            ui.horizontal(|ui| {
                u32_edit_field(ui, &mut self.conf.sample_rate);
                help_text(
                    ui,
                    "Audio framerate.\nDon't touch this if you don't know what you're doing",
                    |ui| {
                        ui.label("Sample rate");
                    },
                );
            });
        });

        ui.separator();

        if ui.button("Select clickpack").clicked() {
            if let Some(dir) = FileDialog::new().pick_folder() {
                log::info!("selected clickpack folder: {dir:?}");

                let bot = if self.conf.pitch_enabled {
                    Bot::new(dir, self.conf.pitch, self.conf.sample_rate)
                } else {
                    Bot::new(dir, Pitch::default(), self.conf.sample_rate)
                };

                if let Ok(bot) = bot {
                    self.bot = Rc::new(RefCell::new(bot));
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

        ui.collapsing("Info", |ui| {
            ui.label("The clickpack should either have player1 and/or player2 folders inside it, \
                    or just audio files. You can add hardclicks, clicks, softclicks, microclicks, \
                    hardreleases, releases, softreleases and microreleases as directories.");
            ui.label("Optionally you can put a noise.* or whitenoise.* file inside the clickpack \
                    folder to have an option to overlay background noise.");
            ui.label("All audio files will be resampled to the selected sample rate.");
            ui.label("Loading the clickpack may take a while, please be patient.");
            ui.label("Pitch step is the step between pitch changes in the pitch table. The lower it is, \
                    the more random the pitch is. Pitch 1.0 = no pitch.");
        });
        ui.collapsing("Supported audio formats", |ui| {
            ui.label("AAC, ADPCM, ALAC, FLAC, MKV, MP1, MP2, MP3, MP4, OGG, Vorbis, WAV, and WebM audio files.");
        });

        dialog.show_dialog();
    }

    fn render_replay(&mut self, dialog: &Modal) {
        let start = Instant::now();
        let segment = self.bot.borrow_mut().render_replay(
            &self.replay,
            self.conf.noise,
            self.conf.normalize,
            if !self.conf.expr_text.is_empty() && self.expr_error.is_empty() {
                self.conf.expr_variable
            } else {
                ExprVariable::None
            },
        );
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

    fn show_plot(&mut self, ui: &mut egui::Ui) {
        ui.label(
            "Input a mathematical expression to change the volume multiplier \
                depending on some variables.",
        );
        ui.collapsing("Defined variables", |ui| {
            ui.label("• frame: Current frame");
            ui.label("• x: Player X position");
            ui.label("• y: Player Y position");
            ui.label("• p: Percentage in level, 0-1");
            ui.label("• player2: 1 if player 2, 0 if player 1");
            ui.label("• rot: Player rotation");
            ui.label("• accel: Player Y acceleration");
            ui.label("• down: Whether the mouse is down, 1 or 0");
            ui.label("• fps: The FPS of the replay");
            ui.label("• time: Elapsed time in level, in seconds");
            ui.label("• frames: Total amount of frames in replay");
            ui.label("• level_time: Total time in level, in seconds");
            ui.label("• rand: Random value in the range of 0 to 1");
            ui.label(
                RichText::new(
                    "NOTE: Some variables may not be set due to different replay formats",
                )
                .color(Color32::YELLOW),
            );
        });
        ui.label("x = action index");
        ui.label("Example expression: sqrt(p) + sin(p) / 10");
        ui.separator();

        let mut expr_updated = false;

        ui.horizontal(|ui| {
            ui.label("y =");

            // save current expression if the new expression on this frame is invalid
            let prev_expr = self.conf.expr_text.clone();

            if ui.text_edit_singleline(&mut self.conf.expr_text).changed() {
                expr_updated = true;

                // recompile expression, check for compile errors
                let mut bot = self.bot.borrow_mut();
                if let Err(e) = bot.compile_expression(&self.conf.expr_text) {
                    self.expr_error = e.to_string();
                } else {
                    self.expr_error.clear(); // clear errors

                    // update namespace so we can check for undefined variables
                    bot.update_namespace(
                        &ExtendedAction::default(),
                        self.replay.last_frame(),
                        self.replay.fps as _,
                    );

                    if let Err(e) = bot.eval_expr() {
                        self.expr_error = e.to_string();
                    }
                }

                // if an error has occured, use the expression from the previous changed() event
                // FIXME: this won't work if the previous event also had an invalid expression
                if !self.expr_error.is_empty() {
                    let _ = bot.compile_expression(&prev_expr);
                }
            }
        });

        // display error message if any
        if !self.expr_error.is_empty() {
            ui.add_space(4.);
            ui.label(
                egui::RichText::new(format!("ERROR: {}", self.expr_error))
                    .strong()
                    .color(Color32::LIGHT_RED),
            );
        }

        // display plot
        use egui_plot::{Legend, Line, Plot, PlotPoints};

        let num_actions = self.replay.extended.len();
        if num_actions == 0 {
            ui.label(
                egui::RichText::new("NOTE: You don't have a replay loaded")
                    .strong()
                    .color(Color32::YELLOW),
            );
        }

        // what variable to change
        help_text(ui, "The variable that the expression should affect", |ui| {
            ui.horizontal(|ui| {
                ui.label("Change:");
                ui.radio_value(
                    &mut self.conf.expr_variable,
                    ExprVariable::Variation,
                    ExprVariable::Variation.to_string(),
                )
                .on_hover_text("Changes the bounds of the random volume offset");
                ui.radio_value(
                    &mut self.conf.expr_variable,
                    ExprVariable::Value,
                    ExprVariable::Value.to_string(),
                )
                .on_hover_text("Changes the volume value (addition)");
                ui.radio_value(
                    &mut self.conf.expr_variable,
                    ExprVariable::TimeOffset,
                    ExprVariable::TimeOffset.to_string(),
                )
                .on_hover_text("Offsets the time of the action");
            });
        });

        // plot data aspect
        ui.horizontal(|ui| {
            ui.add(
                egui::Slider::new(&mut self.conf.plot_data_aspect, 0.001..=30.0)
                    .text("Data aspect"),
            );
            if ui.button("Reset").clicked() {
                self.conf.plot_data_aspect = 20.;
            }
        });

        let plot_points = if expr_updated {
            // compute a brand new set of points
            let points = PlotPoints::from_parametric_callback(
                |t| {
                    if num_actions == 0 {
                        return (0., 0.);
                    }

                    let idx = (t as usize).min(num_actions - 1);
                    let action = self.replay.extended[idx];

                    // update namespace
                    // we can use `self.bot` here because it is an Rc<RefCell<>>
                    self.bot.borrow_mut().update_namespace(
                        &action,
                        self.replay.last_frame(),
                        self.replay.fps as _,
                    );

                    // compute the expression for this action
                    let value = self.bot.borrow_mut().eval_expr().unwrap_or(0.);
                    (t, value)
                },
                0.0..num_actions as f64,
                num_actions.min(MAX_PLOT_POINTS),
            );
            self.plot_points = points.points().to_vec(); // save in cache
            points
        } else {
            // this clone is really expensive, but it is faster than
            // recomputing the entire set of points each frame
            PlotPoints::Owned(self.plot_points.clone())
        };

        let line = Line::new(plot_points).name(self.conf.expr_variable.to_string());
        ui.add_space(4.);

        ui.add_enabled_ui(self.expr_error.is_empty() && num_actions > 0, |ui| {
            let plot = Plot::new("volume_multiplier_plot")
                .legend(Legend::default())
                .data_aspect(self.conf.plot_data_aspect)
                .y_axis_width(4);
            plot.show(ui, |plot_ui| {
                plot_ui.line(line);
            })
            .response
            .on_disabled_hover_text(if num_actions == 0 {
                "Please load a replay"
            } else {
                "The expression is invalid"
            });
        });
    }

    fn show_render_stage(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Render");

        let mut dialog = Modal::new(ctx, "render_stage_dialog");

        ui.horizontal(|ui| {
            help_text(
                ui,
                "Select the output .wav file.\nYou have to click 'Render' to render the output",
                |ui| {
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
                },
            );
            if let Some(output) = &self.output {
                ui.label(format!(
                    "Selected output file: {}",
                    output.file_name().unwrap().to_str().unwrap()
                ));
            }
        });

        ui.separator();

        ui.collapsing("Audio settings", |ui| {
            // make sure we disable noise if the clickpack doesn't have it
            if self.conf.noise && !self.bot.borrow().has_noise() {
                self.conf.noise = false;
            }

            // overlay noise checkbox
            ui.add_enabled_ui(self.bot.borrow().has_noise(), |ui| {
                ui.checkbox(&mut self.conf.noise, "Overlay noise")
                    .on_disabled_hover_text("Your clickpack doesn't have a noise file.")
                    .on_hover_text("Overlays the noise file that's in the clickpack directory.");
            });
            ui.checkbox(&mut self.conf.normalize, "Normalize audio")
                .on_hover_text(
                "Whether to normalize the output audio\n(make all samples to be in range of 0-1)",
            );
        });

        ui.collapsing("Advanced", |ui| {
            self.show_plot(ui);
        });

        ui.separator();

        let has_output = self.output.is_some();
        let has_clicks = self.bot.borrow().has_clicks();
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
                    self.render_replay(&dialog); // TODO: run this on a separate thread
                }
            },
        );

        dialog.show_dialog();
    }
}
