use crate::built_info;
use anyhow::{Context, Result};
use bot::{
    Action, Bot, ChangeVolumeFor, ClickpackConversionSettings, ExprVariable, ExtendedAction, Pitch,
    RemoveSilenceFrom, Replay, ReplayType, Timings, VolumeSettings,
};
use eframe::{
    egui::{self, DragValue, IconData, Key, RichText},
    emath,
    epaint::Color32,
};
use egui_clickpack_db::ClickpackDb;
use egui_modal::{Icon, Modal};
use egui_plot::PlotPoint;
use image::ImageReader;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    cell::RefCell,
    fs::File,
    io::{BufWriter, Cursor, Write},
    ops::RangeInclusive,
    path::Path,
    rc::Rc,
    time::{Duration, Instant},
};
use std::{io::BufReader, path::PathBuf};

const MAX_PLOT_POINTS: usize = 4096;

pub fn run_gui() -> Result<(), eframe::Error> {
    let img = ImageReader::new(Cursor::new(include_bytes!("assets/icon.ico")))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([460.0, 440.0])
            .with_icon(IconData {
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
            cc.egui_ctx.style_mut(|s| {
                s.interaction.tooltip_delay = 0.0;
                s.url_in_tooltip = true;
            });
            Ok(Box::<App>::default())
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
            Self::SelectClickpack => Self::SelectReplay,
            Self::Render => Self::SelectClickpack,
            _ => self,
        }
    }
}

fn get_version() -> String {
    built_info::PKG_VERSION.to_string()
}

fn f32_one() -> f32 {
    1.0
}

#[derive(Serialize, Deserialize, Clone)]
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
    midi_key: u8,
    sample_rate: u32,
    expr_text: String,
    expr_variable: ExprVariable,
    sort_actions: bool,
    plot_data_aspect: f32,
    #[serde(default = "ClickpackConversionSettings::default")]
    conversion_settings: ClickpackConversionSettings,
    #[serde(default = "bool::default")]
    cut_sounds: bool,
    #[serde(default = "f32_one")]
    noise_volume: f32,
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

    fn replay_changed(&self, other: &Self) -> bool {
        self.timings != other.timings
            || self.vol_settings != other.vol_settings
            || self.sort_actions != other.sort_actions
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
            midi_key: 60, // C4
            sample_rate: 44100,
            expr_text: String::new(),
            expr_variable: ExprVariable::Variation { negative: true },
            sort_actions: true,
            plot_data_aspect: 20.0,
            conversion_settings: ClickpackConversionSettings::default(),
            cut_sounds: false,
            noise_volume: 1.0,
        }
    }
}

struct App {
    conf: Config,
    stage: Stage,
    replay: Replay,
    bot: RefCell<Bot>,
    output: Option<PathBuf>,
    // autocutter: AutoCutter,
    last_chars: [Key; 9],
    char_idx: u8,
    expr_error: String,
    plot_points: Rc<Vec<PlotPoint>>,
    update_to_tag: Option<Rc<String>>,
    update_expr: bool,
    clickpack_path: Option<PathBuf>,
    conf_after_replay_selected: Option<Config>,
    replay_path: Option<PathBuf>,
    clickpack_num_sounds: Option<usize>,
    clickpack_has_noise: bool,
    expr_variable_variation_negative: bool,
    override_fps_enabled: bool,
    override_fps: f32,
    clickpack_db: ClickpackDb,
    show_clickpack_db: bool,
    clickpack_db_title: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            conf: Config::default(),
            stage: Stage::default(),
            replay: Replay::default(),
            bot: RefCell::new(Bot::default()),
            output: None,
            // autocutter: AutoCutter::default(),
            last_chars: [Key::A; 9],
            char_idx: 0,
            expr_error: String::new(),
            plot_points: Rc::new(vec![]),
            update_to_tag: None,
            update_expr: false,
            clickpack_path: None,
            conf_after_replay_selected: None,
            replay_path: None,
            clickpack_num_sounds: None,
            clickpack_has_noise: false,
            expr_variable_variation_negative: true,
            override_fps_enabled: false,
            override_fps: 0.0,
            clickpack_db: ClickpackDb::default(),
            show_clickpack_db: false,
            clickpack_db_title: String::new(),
        }
    }
}

/// Value is always min clamped with 1.
fn u32_edit_field_min1(ui: &mut egui::Ui, value: &mut u32) -> egui::Response {
    let mut tmp_value = format!("{value}");
    let res = ui.text_edit_singleline(&mut tmp_value);
    if let Ok(result) = tmp_value.parse::<u32>() {
        *value = result.max(1);
    }
    res
}

fn help_text<R>(ui: &mut egui::Ui, help: &str, add_contents: impl FnOnce(&mut egui::Ui) -> R) {
    if help.is_empty() {
        add_contents(ui); // don't show help icon if there's no help text
        return;
    }
    ui.horizontal(|ui| {
        add_contents(ui);
        ui.add_enabled_ui(false, |ui| ui.label("(?)").on_disabled_hover_text(help));
    });
}

fn drag_value<Num: emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut Num,
    text: impl Into<String>,
    clamp_range: RangeInclusive<Num>,
    help: &str,
) {
    help_text(ui, help, |ui| {
        let dragged = ui
            .add(DragValue::new(value).range(clamp_range.clone()).speed(0.01))
            .dragged();
        ui.label(
            if dragged && (clamp_range.start() == value || clamp_range.end() == value) {
                RichText::new(text).color(Color32::LIGHT_RED)
            } else {
                RichText::new(text)
            },
        );
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
                ui.selectable_value(&mut self.stage, Stage::SelectReplay, "Replay");
                ui.selectable_value(&mut self.stage, Stage::SelectClickpack, "Clickpack");
                ui.selectable_value(&mut self.stage, Stage::Render, "Render");
                // ui.selectable_value(&mut self.stage, Stage::AutoCutter, "AutoCutter");
                ui.selectable_value(&mut self.stage, Stage::Donate, "Donate");
                if self.stage == Stage::Secret {
                    ui.selectable_value(&mut self.stage, Stage::Secret, "Secret");
                }
            });
            ui.add_space(2.0);
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            let mut dialog = Modal::new(ctx, "config_dialog");
            let mut update_dialog = Modal::new(ctx, "update_dialog");
            let mut modal = Modal::new(ctx, "update_modal");

            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.style_mut().spacing.item_spacing.x = 5.;
                    if self.stage != self.stage.previous()
                        && ui
                            .button("Back")
                            .on_hover_text("Go back to the previous stage")
                            .clicked()
                    {
                        self.stage = self.stage.previous();
                    }
                    if ui
                        .button("Check for updates")
                        .on_hover_text("Check if your ZCB version is up-to-date")
                        .clicked()
                    {
                        self.do_update_check(&modal, &update_dialog);
                    }

                    ui.hyperlink_to("Join the Discord server", "https://discord.gg/b4kBQyXYZT");

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.style_mut().spacing.item_spacing.x = 5.;
                        if ui
                            .button("Reset")
                            .on_hover_text("Reset the current configuration to defaults")
                            .clicked()
                        {
                            self.conf = Config::default();
                        }
                        ui.style_mut().spacing.item_spacing.x = 5.;
                        if ui
                            .button("Load")
                            .on_hover_text("Load a configuration file")
                            .clicked()
                        {
                            self.load_config(&dialog);

                            // reload replay if it was loaded
                            if let Some(replay_path) = &self.replay_path.clone() {
                                let _ = self
                                    .load_replay(&dialog, replay_path)
                                    .map_err(|e| log::error!("failed to reload replay: {e}"));
                            }
                        }
                        ui.style_mut().spacing.item_spacing.x = 5.;
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
            update_dialog.show_dialog();
            modal.show_dialog();

            self.show_update_check_modal(&modal, &update_dialog, ctx);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
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

        if self.show_clickpack_db {
            if self.clickpack_db_title.is_empty() {
                let updated_at = self.clickpack_db.db.read().unwrap().updated_at_unix;
                if updated_at != 0 {
                    use chrono::{TimeZone, Utc};
                    use timeago::Formatter;
                    let formatter = Formatter::new();
                    let datetime = Utc.timestamp_opt(updated_at, 0).unwrap();
                    let now = Utc::now();
                    self.clickpack_db_title = format!(
                        "ClickpackDB - updated {}, {} clickpacks",
                        formatter.convert_chrono(datetime, now),
                        self.clickpack_db.db.read().unwrap().entries.len()
                    );
                }
            }
            let builder = egui::ViewportBuilder::default()
                .with_title(if self.clickpack_db_title.is_empty() {
                    "ClickpackDB"
                } else {
                    &self.clickpack_db_title
                })
                .with_inner_size([410.0, 510.0])
                .with_resizable(false);
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("immediate_clickpack_db_viewport"),
                builder,
                |ctx, class| {
                    assert!(
                        class == egui::ViewportClass::Immediate,
                        "This egui backend doesn't support multiple viewports",
                    );

                    egui::CentralPanel::default().show(ctx, |ui| {
                        self.show_clickpack_db(ctx, ui);
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        // tell parent viewport that we should not show next frame:
                        self.show_clickpack_db = false;
                    }
                },
            );
        }
    }
}

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Ubuntu Chromium/37.0.2062.94 Chrome/37.0.2062.94 Safari/537.36";

fn ureq_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_read(Duration::from_secs(15))
        .timeout_write(Duration::from_secs(15))
        .user_agent(USER_AGENT)
        .build()
}

fn ureq_get(url: &str) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    ureq_agent()
        .get(url)
        .call()
        .map_err(|e| e.to_string())?
        .into_reader()
        .read_to_end(&mut buf)
        .map_err(|_| "failed to read body".to_string())?;
    Ok(buf)
}

fn get_latest_tag() -> Result<String> {
    let body = ureq_agent()
        .get("https://api.github.com/repos/zeozeozeo/zcb3/tags")
        .call()?
        .into_string()?;

    log::debug!("response text: '{body}'");
    let v: Value = serde_json::from_str(&body)?;
    let tags = v.as_array().context("not an array")?;
    let latest_tag = tags.first().context("couldn't latest tags")?;
    let name = latest_tag.get("name").context("couldn't get tag name")?;
    let tagname = name.as_str().context("tag name is not a string")?;

    Ok(tagname.to_string())
}

fn is_older_version(current: &str, latest: &str) -> bool {
    current
        .split('.')
        .map(|s| s.parse::<u32>().unwrap_or(0))
        .zip(latest.split('.').map(|s| s.parse::<u32>().unwrap_or(0)))
        .any(|(c, l)| c < l)
}

fn update_to_latest(tag: &str) -> Result<()> {
    let body = ureq_agent()
        .get("https://api.github.com/repos/zeozeozeo/zcb3/releases/latest")
        .call()?
        .into_string()?;

    log::debug!("releases response text: '{body}'");
    let v: Value = serde_json::from_str(&body)?;

    let filename = if cfg!(target_os = "windows") {
        "zcb3.exe"
    } else if cfg!(target_os = "macos") {
        "zcb3_macos"
    } else if cfg!(target_os = "linux") {
        "zcb3_linux" // might be any other unix-like OS, but we only support Linux for now
    } else {
        anyhow::bail!("unsupported on this platform");
    };

    // search for the required asset
    let asset_url: Option<&str> = v["assets"]
        .as_array()
        .context("failed to get 'assets' array")?
        .iter()
        .map(|v| v["browser_download_url"].as_str().unwrap_or(""))
        .find(|url| url.contains(filename));

    if let Some(url) = asset_url {
        let mut reader = ureq_agent().get(url).call()?.into_reader();

        // generate random string
        let random_str: String = std::iter::repeat_with(fastrand::alphanumeric)
            .take(8)
            .collect();

        let new_binary = format!(
            "zcb3_update_{tag}_{random_str}{}",
            if cfg!(windows) { ".exe" } else { "" }
        );

        // write the file
        let mut f = std::fs::File::create(&new_binary)?;
        std::io::copy(&mut reader, &mut f)?;

        // replace executable
        self_replace::self_replace(&new_binary)
            .map_err(|e| anyhow::anyhow!("{e}. Use the created executable: {new_binary}"))?;

        if std::path::Path::new(&new_binary).try_exists()? {
            std::fs::remove_file(new_binary)?;
        }
    } else {
        anyhow::bail!("failed to find required asset for tag {tag} (filename: {filename})")
    }

    Ok(())
}

fn capitalize_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

impl App {
    fn show_update_check_modal(&mut self, modal: &Modal, dialog: &Modal, ctx: &egui::Context) {
        let Some(update_to_tag) = self.update_to_tag.clone() else {
            return;
        };
        modal.show(|ui| {
            modal.title(ui, "New version available");
            modal.frame(ui, |ui| {
                modal.body_and_icon(
                    ui,
                    format!(
                        "A new version of ZCB is available (latest: {}, this: {}).\n\
                        Download the new version on the GitHub page, \
                        Discord server or use the auto-updater (note: you might have \
                        to restart ZCB).",
                        update_to_tag,
                        built_info::PKG_VERSION
                    ),
                    Icon::Info,
                );
            });
            modal.buttons(ui, |ui| {
                if modal
                    .button(ui, "auto-update")
                    .on_hover_text(
                        "Automatically update to the newest version.\n\
                                    This might take some time!\n\
                                    You might have to restart ZCB",
                    )
                    .clicked()
                {
                    if let Err(e) = update_to_latest(&update_to_tag) {
                        dialog
                            .dialog()
                            .with_title("Failed to perform auto-update")
                            .with_body(e)
                            .with_icon(Icon::Error)
                            .open();
                    } else {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    self.update_to_tag = None;
                }
                if modal.button(ui, "close").clicked() {
                    self.update_to_tag = None;
                }
            });
        });
    }

    fn do_update_check(&mut self, modal: &Modal, dialog: &Modal) {
        let latest_tag = get_latest_tag();

        if let Ok(latest_tag) = latest_tag {
            log::info!(
                "latest tag: {latest_tag}, current tag {}",
                built_info::PKG_VERSION
            );
            if is_older_version(built_info::PKG_VERSION, &latest_tag) {
                self.update_to_tag = Some(Rc::new(latest_tag));
                modal.open();
            } else {
                let time_traveler = latest_tag != built_info::PKG_VERSION;
                dialog
                    .dialog()
                    .with_title(if time_traveler {
                        "You're a time traveler!"
                    } else {
                        "You are up-to-date!"
                    })
                    .with_body(format!(
                        "You are running {} of ZCB ({}).\n\
                        You can always download new versions on GitHub or on the Discord server.",
                        if time_traveler {
                            "an unreleased version"
                        } else {
                            "the latest version"
                        },
                        get_version(),
                    ))
                    .with_icon(Icon::Success)
                    .open();
            }
        } else if let Err(e) = latest_tag {
            log::error!("failed to check for updates: {e}");
            dialog
                .dialog()
                .with_title("Failed to check for updates")
                .with_body(e)
                .with_icon(Icon::Error)
                .open();
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
            let repeaters = ((ticks / 4.0).round() as usize).max(1);
            let last_ticks = ((ticks % 4.0).round() as usize).clamp(0, 3) as u8 + 1;

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
            Vec3::new(0, 0, 0),
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

    // Function written by forteus19
    // I am not a rust dev so my code is probably trash LOL
    fn export_midi(&self) -> Result<()> {
        // Check if fps is at most 32767
        if self.replay.fps as u32 > 32767 {
            log::error!("MIDI format only supports up to 32767 PPQN (framerate)");
            return Err(anyhow::anyhow!(
                "MIDI format only supports up to 32767 PPQN (framerate)"
            ));
        }

        let Some(path) = FileDialog::new()
            .add_filter("MIDI file", &["mid"])
            .save_file()
        else {
            log::error!("no file was selected");
            return Err(anyhow::anyhow!("no file was selected"));
        };

        // Separate the click types into their own vectors
        let mut separated_actions: [Vec<Action>; 8] = Default::default();
        for action in &self.replay.actions {
            match action.click.click_type() {
                bot::ClickType::HardClick => separated_actions[0].push(*action),
                bot::ClickType::HardRelease => separated_actions[1].push(*action),
                bot::ClickType::Click => separated_actions[2].push(*action),
                bot::ClickType::Release => separated_actions[3].push(*action),
                bot::ClickType::SoftClick => separated_actions[4].push(*action),
                bot::ClickType::SoftRelease => separated_actions[5].push(*action),
                bot::ClickType::MicroClick => separated_actions[6].push(*action),
                bot::ClickType::MicroRelease => separated_actions[7].push(*action),
                _ => (),
            }
        }

        // Create bufwriter for midi data
        // NOTE: all values are big endian!!!
        let mut midi_data = BufWriter::new(File::create(path)?);
        midi_data.write_all(b"MThd")?; // MThd header
        midi_data.write_all(&u32::to_be_bytes(6))?; // MThd length
        midi_data.write_all(&u16::to_be_bytes(1))?; // SMF format
        midi_data.write_all(&u16::to_be_bytes(9))?; // Num tracks
        midi_data.write_all(&u16::to_be_bytes(self.replay.fps as u16))?; // PPQN
        midi_data.flush()?;

        // Create tempo/meta track
        midi_data.write_all(b"MTrk")?; // MTrk header
        midi_data.write_all(&u32::to_be_bytes(11))?; // MTrk length
        midi_data.write_all(&[0x00])?; // 0 delta time
        midi_data.write_all(&[0xFF, 0x51, 0x03])?; // Tempo event
        midi_data.write_all(&[0x0F, 0x42, 0x40])?; // 60 bpm
        midi_data.write_all(&[0x00])?; // 0 delta time
        midi_data.write_all(&[0xFF, 0x2F, 0x00])?; // EOT event
        midi_data.flush()?;

        let key = self.conf.midi_key.min(127);

        for (c, click_vec) in separated_actions.iter().enumerate() {
            // Create a track for each click type;
            // We use a vector instead of writing directly to the file because we don't know
            // what the final size of the track will be
            let mut track_buf: Vec<u8> = Vec::new();

            // Add track name event
            track_buf.push(0x00); // Delta time
            track_buf.extend(&[0xFF, 0x03]); // Track name event
            match c {
                0 => {
                    track_buf.push(10);
                    track_buf.extend(b"Hardclicks");
                }
                1 => {
                    track_buf.push(12);
                    track_buf.extend(b"Hardreleases");
                }
                2 => {
                    track_buf.push(6);
                    track_buf.extend(b"Clicks");
                }
                3 => {
                    track_buf.push(8);
                    track_buf.extend(b"Releases");
                }
                4 => {
                    track_buf.push(10);
                    track_buf.extend(b"Softclicks");
                }
                5 => {
                    track_buf.push(12);
                    track_buf.extend(b"Softreleases");
                }
                6 => {
                    track_buf.push(11);
                    track_buf.extend(b"Microclicks");
                }
                7 => {
                    track_buf.push(13);
                    track_buf.extend(b"Microreleases");
                }
                _ => (),
            }

            // Add program change
            track_buf.push(0x00); // Delta time
            track_buf.extend(&[0b11000000 | (c as u8), c as u8]); // PC event

            let mut i = 0;
            while i < click_vec.len() {
                let delta_time = if i == 0 {
                    click_vec[i].frame
                } else {
                    click_vec[i].frame - click_vec[i - 1].frame - 1
                };

                // Add note-on event
                self.write_vlq(&mut track_buf, delta_time); // Delta time
                track_buf.push(0b10010000 | (c as u8)); // Note-on event
                track_buf.push(key);
                track_buf.push(0x7F); // Velocity 127 (max)
                                      // Add note-off event 1 tick later
                track_buf.push(0x01); // Delta time
                track_buf.push(0b10000000 | (c as u8)); // Note-off event
                track_buf.push(key);
                track_buf.push(0x7F); // Velocity 127 (max)

                i += 1;
            }

            // Add EOT event
            track_buf.push(0x00); // Delta time
            track_buf.extend(&[0xFF, 0x2F, 0x00]); // EOT event

            // Write the buf to the file
            midi_data.write_all(b"MTrk")?; // MTrk header
            midi_data.write_all(&u32::to_be_bytes(track_buf.len() as u32))?; // MTrk size
            midi_data.write_all(&track_buf)?; // MTrk data
        }

        midi_data.flush()?;
        Ok(())
    }

    fn write_vlq(&self, vector: &mut Vec<u8>, value: u32) {
        if value >= (1 << 21) {
            vector.extend(&[
                (value >> 21 & 0x7F) as u8 | 0x80,
                (value >> 14 & 0x7F) as u8 | 0x80,
                (value >> 7 & 0x7F) as u8 | 0x80,
                (value & 0x7F) as u8,
            ]);
        } else if value >= (1 << 14) {
            vector.extend(&[
                (value >> 14 & 0x7F) as u8 | 0x80,
                (value >> 7 & 0x7F) as u8 | 0x80,
                (value & 0x7F) as u8,
            ]);
        } else if value >= (1 << 7) {
            vector.extend(&[(value >> 7 & 0x7F) as u8 | 0x80, (value & 0x7F) as u8]);
        } else {
            vector.push((value & 0x7F) as u8);
        }
    }

    fn save_config(&self, dialog: &Modal) {
        if let Some(file) = FileDialog::new()
            .add_filter("Config file", &["json"])
            .save_file()
        {
            if let Err(e) = self.conf.save(&file) {
                dialog
                    .dialog()
                    .with_title("Failed to save config")
                    .with_body(e)
                    .with_icon(Icon::Error)
                    .open();
            }
        } else {
            dialog
                .dialog()
                .with_title("No file was selected")
                .with_body("Please select a file")
                .with_icon(Icon::Error)
                .open();
        }
    }

    fn load_config(&mut self, dialog: &Modal) {
        if let Some(file) = FileDialog::new()
            .add_filter("Config file", &["json"])
            .pick_file()
        {
            if let Err(e) = self.conf.load(&file) {
                dialog
                    .dialog()
                    .with_title("Failed to load config")
                    .with_body(e)
                    .with_icon(Icon::Error)
                    .open();
            } else {
                self.update_expr = true;
            }
        } else {
            dialog
                .dialog()
                .with_title("No file was selected")
                .with_body("Please select a file")
                .with_icon(Icon::Error)
                .open();
        }
    }

    fn show_secret_stage(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let mut secret_modal = Modal::new(ctx, "secret_stage_dialog");

        // this is so epic
        ui.add_enabled_ui(self.replay.has_actions(), |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("Export replay to .litematic")
                    .on_disabled_hover_text("You have to load a replay first")
                    .clicked()
                {
                    self.export_litematic();
                }
                ui.checkbox(&mut self.conf.litematic_export_releases, "Export releases");
            });

            ui.horizontal(|ui| {
                if ui
                    .button("Export replay to .mid")
                    .on_disabled_hover_text("You have to load a replay first")
                    .clicked()
                {
                    if let Err(e) = self.export_midi() {
                        log::error!("failed to export MIDI: {e}");
                        secret_modal
                            .dialog()
                            .with_title("Failed to export MIDI")
                            .with_body(capitalize_first_letter(&e.to_string()))
                            .with_icon(Icon::Error)
                            .open();
                    }
                }

                ui.add(egui::Slider::new(&mut self.conf.midi_key, 0..=127));

                const NOTES: [&str; 12] = [
                    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
                ];
                let octave = (self.conf.midi_key / 12) as i32 - 1;
                let note = NOTES[(self.conf.midi_key % 12) as usize];

                ui.label(format!("MIDI key ({note}{octave})"));
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

        secret_modal.show_dialog();
    }

    fn show_pwease_donate_stage(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Donations");
        ui.label("ZCB is completely free software, donations are appreciated :3");

        ui.add_space(8.0);

        egui::Grid::new("donate_stage_grid")
            .num_columns(2)
            .min_col_width(16.0)
            .show(ui, |ui| {
                ui.add(
                    egui::Image::new(egui::include_image!("assets/kofi_logo.png")).max_width(20.0),
                );
                ui.hyperlink_to("Donate on Ko-fi", "https://ko-fi.com/zeozeozeo");
                ui.end_row();

                ui.add(
                    egui::Image::new(egui::include_image!("assets/liberapay_logo.png"))
                        .max_width(32.0),
                );
                ui.hyperlink_to("Donate on Liberapay", "https://liberapay.com/zeo");
                ui.end_row();

                ui.add(
                    egui::Image::new(egui::include_image!("assets/donationalerts_logo.png"))
                        .max_width(32.0),
                );
                ui.hyperlink_to(
                    "Donate on DonationAlerts",
                    "https://donationalerts.com/r/zeozeozeo",
                );
                ui.end_row();

                ui.add(
                    egui::Image::new(egui::include_image!("assets/boosty_logo.png"))
                        .max_width(32.0),
                );
                ui.hyperlink_to("Donate on Boosty", "https://boosty.to/zeozeozeo/donate");
                ui.end_row();

                ui.add(
                    egui::Image::new(egui::include_image!("assets/discord_logo.png"))
                        .max_width(16.0),
                );
                ui.hyperlink_to("Join the Discord server", "https://discord.gg/b4kBQyXYZT");
                ui.end_row();

                ui.add(
                    egui::Image::new(egui::include_image!("assets/guilded_logo.png"))
                        .max_width(16.0),
                );
                ui.hyperlink_to("Join the Guilded server", "https://guilded.gg/clickbot");
                ui.end_row();
            });
    }

    fn load_replay(&mut self, dialog: &Modal, file: &Path) -> Result<()> {
        let filename = file.file_name().unwrap().to_str().unwrap();

        // open replay file
        let f = std::fs::File::open(file).unwrap();

        let replay_type = ReplayType::guess_format(filename);

        if let Ok(replay_type) = replay_type {
            // parse replay
            let replay = Replay::build()
                .with_timings(self.conf.timings)
                .with_vol_settings(self.conf.vol_settings)
                .with_extended(true)
                .with_sort_actions(self.conf.sort_actions)
                .with_override_fps(if self.override_fps_enabled {
                    Some(self.override_fps)
                } else {
                    None
                })
                .parse(replay_type, BufReader::new(f));

            if let Ok(replay) = replay {
                self.replay = replay;
                self.update_expr = true;
                self.conf_after_replay_selected = Some(self.conf.clone());
            } else if let Err(e) = replay {
                dialog
                    .dialog()
                    .with_title("Failed to parse replay file")
                    .with_body(format!(
                        "{}. Is the format supported?",
                        capitalize_first_letter(&e.to_string()),
                    ))
                    .with_icon(Icon::Error)
                    .open();
                return Err(e);
            }
        } else if let Err(e) = replay_type {
            dialog
                .dialog()
                .with_title("Failed to guess replay format")
                .with_body(format!("Failed to guess replay format: {e}"))
                .with_icon(Icon::Error)
                .open();
            return Err(e);
        }
        Ok(())
    }

    fn show_replay_stage(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Select replay file");

        let mut dialog = Modal::new(ctx, "replay_stage_dialog");

        ui.collapsing("Timings", |ui| {
            ui.label("Click type timings. The number is the delay between actions (in seconds). \
                    If the delay between the current and previous action is bigger than the specified \
                    timing, the corresponding click type is used.");
            let t = &mut self.conf.timings;

            drag_value(ui, &mut t.hard, "Hard timing",
            t.regular..=f32::INFINITY,
            "Hardclick/hardrelease timing");
            drag_value(ui, &mut t.regular, "Regular timing", t.soft..=t.hard,
            "Click/release timing");
            drag_value(ui, &mut t.soft, "Soft timing", 0.0..=t.regular,
            "Softclick/softrelease timing");
            ui.label(format!("Everything below {}s are microclicks/microreleases.", t.soft));
        });

        ui.collapsing("Volume settings", |ui| {
            ui.label(
                "General volume settings. The volume variation variable \
                defines the range of the random volume offset.",
            );

            let vol = &mut self.conf.vol_settings;
            drag_value(
                ui,
                &mut vol.volume_var,
                "Volume variation",
                0.0..=f32::INFINITY,
                "Maximum volume variation for each action (+/-)",
            );
            drag_value(
                ui,
                &mut vol.global_volume,
                "Global volume",
                0.0..=f32::INFINITY,
                "Constant volume multiplier for all actions",
            );
        });

        ui.collapsing("Spam volume changes", |ui| {
            ui.label(
                "Adjusts the volume of 'spam clicks', which are defined as actions within \
                a maximum time limit, known as the 'spam time'. The volume offset \
                is based on the delta time between actions, multiplied by the spam \
                volume offset factor, and clamped \
                by the maximum spam volume offset. \
                In short, this can be used to lower the volume of clicks in spams.",
            );

            let vol = &mut self.conf.vol_settings;
            ui.checkbox(&mut vol.enabled, "Enable spam volume changes");

            ui.add_enabled_ui(vol.enabled, |ui| {
                help_text(ui, "Apply spam volume changes to releases", |ui| {
                    ui.checkbox(&mut vol.change_releases_volume, "Change releases volume");
                });
                drag_value(
                    ui,
                    &mut vol.spam_time,
                    "Spam time (between actions)",
                    0.0..=f32::INFINITY,
                    "Time between clicks which are considered spam clicks",
                );
                drag_value(
                    ui,
                    &mut vol.spam_vol_offset_factor,
                    "Spam volume offset factor",
                    0.0..=f32::INFINITY,
                    "The value which the volume offset factor is multiplied by",
                );
                drag_value(
                    ui,
                    &mut vol.max_spam_vol_offset,
                    "Maximum spam volume offset",
                    0.0..=f32::INFINITY,
                    "The maximum value of the volume offset factor",
                );
            });
        });

        help_text(ui, "Sort actions by time", |ui| {
            ui.checkbox(&mut self.conf.sort_actions, "Sort actions");
        });
        ui.separator();

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.override_fps_enabled, "Override FPS");
            if self.override_fps_enabled {
                drag_value(ui, &mut self.override_fps, "FPS", 0.0..=f32::INFINITY, "");
            } else {
                self.override_fps = self.replay.fps;
            }
        });

        let num_actions = self.replay.actions.len();
        let replay_changed =
            if let Some(conf_after_replay_selected) = &self.conf_after_replay_selected {
                conf_after_replay_selected.replay_changed(&self.conf)
            } else {
                false
            };

        if (self.override_fps_enabled && num_actions > 0 && self.override_fps != self.replay.fps)
            || self.override_fps_enabled != self.replay.override_fps.is_some()
            || replay_changed
        {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Reload replay to apply settings").color(Color32::LIGHT_RED),
                );
                if let Some(replay_path) = &self.replay_path.clone() {
                    if ui.button("Reload").on_hover_text("Reload replay").clicked() {
                        let _ = self
                            .load_replay(&dialog, replay_path)
                            .map_err(|e| log::error!("failed to reload replay: {e}"));
                    }
                }
            });
        }

        ui.horizontal(|ui| {
            if ui.button("Select replay").clicked() {
                // FIXME: for some reason when selecting files there's a ~2 second freeze in debug mode
                if let Some(file) = FileDialog::new()
                    .add_filter("Replay file", Replay::SUPPORTED_EXTENSIONS)
                    .pick_file()
                {
                    self.replay_path = Some(file.clone());
                    if self.load_replay(&dialog, &file).is_ok() {
                        self.stage = Stage::SelectClickpack;
                    }
                } else {
                    dialog
                        .dialog()
                        .with_title("No file was selected")
                        .with_body("Please select a file")
                        .with_icon(Icon::Error)
                        .open();
                }
            }

            let num_extended = self.replay.extended.len();
            if num_actions > 0 {
                ui.label(format!(
                    "Number of actions in replay: {num_actions} actions, {num_extended} physics"
                ));
            }
        });
        if num_actions > 0 {
            ui.label(format!("Replay FPS: {:.2}", self.replay.fps));
        }

        ui.collapsing("Supported file formats", |ui| {
            ui.label(
                "• Mega Hack Replay JSON (.mhr.json)
• Mega Hack Replay Binary (.mhr)
• TASbot Replay (.json)
• zBot Frame Replay (.zbf)
• OmegaBot 2 Replay (.replay)
• OmegaBot 3 Replay (.replay)
• yBot Frame (no extension by default, rename to .ybf)
• yBot 2 (.ybot)
• Echo (.echo, new binary format, new json format and old json format)
• Amethyst Replay (.thyst)
• osu! replay (.osr)
• GDMO Replay (.macro)
• 2.2 GDMO Replay (.macro, old non-Geode version)
• ReplayBot Replay (.replay)
• KD-BOT Replay (.kd)
• Rush Replay (.rsh)
• Plaintext (.txt)
• GDH Plaintext (.txt)
• ReplayEngine Replay (.re, old and new formats)
• DDHOR Replay (.ddhor, old frame format)
• xBot Frame (.xbot)
• xdBot (.xd, old and new formats)
• GDReplayFormat (.gdr, used in Geode GDMegaOverlay and 2.2 MH Replay)
• qBot (.qb)
• RBot (.rbot, old and new formats)
• Zephyrus (.zr, used in OpenHack)",
            );
        });

        // show dialog if there is one
        dialog.show_dialog();
    }

    fn load_clickpack_no_pitch(&self, dialog: &Modal, bot: &mut Bot) {
        if let Err(e) = bot.load_clickpack(
            &self.clickpack_path.clone().unwrap(),
            Pitch::NO_PITCH, // don't generate pitch table
        ) {
            dialog
                .dialog()
                .with_title("Failed to load clickpack")
                .with_body(e)
                .with_icon(Icon::Error)
                .open();
        }
    }

    fn select_clickpack(&mut self, path: &Path) {
        log::info!("selected clickpack path: {path:?}");
        self.clickpack_has_noise = bot::dir_has_noise(path);
        self.clickpack_path = Some(path.to_path_buf());
        self.bot = RefCell::new(Bot::new(self.conf.sample_rate));
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
                drag_value(
                    ui,
                    &mut p.from,
                    "Minimum pitch",
                    0.0..=p.to,
                    "Minimum pitch value, 1 means no change",
                );
                drag_value(
                    ui,
                    &mut p.to,
                    "Maximum pitch",
                    p.from..=f32::INFINITY,
                    "Maximum pitch value, 1 means no change",
                );
                drag_value(
                    ui,
                    &mut p.step,
                    "Pitch step",
                    0.0001..=f32::INFINITY,
                    "Step between pitch values. The less = the better & the slower",
                );
            });
        });

        let is_convert_tab_open = ui
            .collapsing("Convert", |ui| {
                let conv_settings = &mut self.conf.conversion_settings;

                ui.label("Clickpack conversion. Can be used to modify sounds in batch.");
                ui.separator();

                drag_value(
                    ui,
                    &mut conv_settings.volume,
                    "Volume multiplier",
                    0.0..=f32::INFINITY,
                    "Change the volume of each audio file",
                );

                if conv_settings.volume != 1. {
                    help_text(ui, "Only change volume for this click type", |ui| {
                        egui::ComboBox::from_label("Change volume for")
                            .selected_text(conv_settings.change_volume_for.to_string())
                            .show_ui(ui, |ui| {
                                use ChangeVolumeFor::*;
                                for typ in [All, Clicks, Releases] {
                                    ui.selectable_value(
                                        &mut conv_settings.change_volume_for,
                                        typ,
                                        typ.to_string(),
                                    );
                                }
                            });
                    });
                }

                help_text(ui, "Reverse all audio files", |ui| {
                    ui.checkbox(&mut conv_settings.reverse, "Reverse audio")
                });

                help_text(ui, "Rename all audio files to 1.wav, 2.wav, etc.", |ui| {
                    ui.checkbox(&mut conv_settings.rename_files, "Rename files")
                });

                help_text(
                    ui,
                    "Remove silence from beginning or end of all audio files",
                    |ui| {
                        egui::ComboBox::from_label("Remove silence")
                            .selected_text(conv_settings.remove_silence.to_string())
                            .show_ui(ui, |ui| {
                                use RemoveSilenceFrom::*;
                                for typ in [None, Start, End] {
                                    ui.selectable_value(
                                        &mut conv_settings.remove_silence,
                                        typ,
                                        typ.to_string(),
                                    );
                                }
                            });
                    },
                );

                if conv_settings.remove_silence != RemoveSilenceFrom::None {
                    help_text(
                        ui,
                        "The volume value at which the sound should start (beta)",
                        |ui| {
                            ui.horizontal(|ui| {
                                ui.add(egui::Slider::new(
                                    &mut conv_settings.silence_threshold,
                                    0.0..=1.0,
                                ));
                                ui.label("Silence threshold (volume)");
                                if ui
                                    .button("Reset")
                                    .on_hover_text("Reset the silence threshold")
                                    .clicked()
                                {
                                    conv_settings.silence_threshold = 0.05;
                                }
                            });
                        },
                    );
                }
                ui.horizontal(|ui| {
                    if ui
                        .button("Convert")
                        .on_hover_text(
                            "Convert the clickpack.\n\
                                Note that all files will be exported as .wav",
                        )
                        .clicked()
                    {
                        if let Some(dir) = FileDialog::new().pick_folder() {
                            let start = Instant::now();

                            // check if the clickpack is loaded, load it if not
                            if !self.bot.borrow().has_clicks() {
                                if let Err(e) = self.bot.borrow_mut().load_clickpack(
                                    &self.clickpack_path.clone().unwrap(),
                                    Pitch::NO_PITCH, // don't generate pitch table
                                ) {
                                    dialog
                                        .dialog()
                                        .with_title("Failed to load clickpack")
                                        .with_body(e)
                                        .with_icon(Icon::Error)
                                        .open();
                                }
                            }

                            // convert
                            if let Err(e) = self.bot.borrow().convert_clickpack(&dir, conv_settings)
                            {
                                dialog
                                    .dialog()
                                    .with_title("Failed to convert clickpack")
                                    .with_body(e)
                                    .with_icon(Icon::Error)
                                    .open();
                            } else {
                                dialog
                                    .dialog()
                                    .with_title("Success!")
                                    .with_body(format!(
                                        "Successfully converted clickpack in {:?}.",
                                        start.elapsed()
                                    ))
                                    .with_icon(Icon::Success)
                                    .open();
                            }

                            // finished, unload clickpack
                            *self.bot.borrow_mut() = Bot::new(self.conf.sample_rate);
                        } else {
                            dialog
                                .dialog()
                                .with_title("No directory was selected")
                                .with_body("Please select a directory")
                                .with_icon(Icon::Error)
                                .open();
                        }
                    }
                });
            })
            .fully_open(); // let is_convert_tab_open = ...

        ui.separator();

        ui.horizontal(|ui| {
            ui.style_mut().spacing.item_spacing.x = 5.0;
            if ui.button("Select clickpack").clicked() {
                if let Some(dir) = FileDialog::new().pick_folder() {
                    self.select_clickpack(&dir);
                    if !is_convert_tab_open {
                        self.stage = if self.replay.has_actions() {
                            Stage::Render
                        } else {
                            Stage::SelectReplay
                        };
                    }
                } else {
                    dialog
                        .dialog()
                        .with_title("No directory was selected")
                        .with_body("Please select a directory")
                        .with_icon(Icon::Error)
                        .open();
                }
            }
            if ui
                .button("Open ClickpackDB…")
                .on_hover_text("Easily download clickpacks from within ZCB")
                .clicked()
            {
                self.show_clickpack_db = true;
            }
        });
        if let Some(clickpack_path) = &self.clickpack_path {
            let filename = clickpack_path.file_name().unwrap();

            // clickpack_num_sounds only gets set after rendering where the
            // clickpack gets loaded
            if let Some(num_sounds) = self.clickpack_num_sounds {
                ui.label(format!(
                    "Selected clickpack: {filename:?}, {num_sounds} sounds"
                ));
            } else {
                ui.label(format!("Selected clickpack: {filename:?}"));
            }
        }

        if let Some(clickpack_path) = &self.clickpack_path {
            ui.collapsing("Overview", |ui| {
                let bot = &mut self.bot.borrow_mut();
                let has_clicks = bot.has_clicks();
                ui.label("General overview of how your clickpack is stored internally");
                ui.horizontal(|ui| {
                    ui.label("Path:");
                    let path_str = clickpack_path.to_str().unwrap_or("invalid Path");
                    ui.label(RichText::new(format!(" {} ", path_str.replace('\\', "/"))).code());
                });
                ui.collapsing("Structure", |ui| {
                    if has_clicks {
                        egui::Grid::new("clickpack_structure_grid")
                            .num_columns(2)
                            .spacing([40.0, 4.0])
                            .striped(true)
                            .show(ui, |ui| {
                                for clicks in [
                                    (&bot.clickpack.player1, "player1"),
                                    (&bot.clickpack.player2, "player2"),
                                    (&bot.clickpack.left1, "left1"),
                                    (&bot.clickpack.right1, "right1"),
                                    (&bot.clickpack.left2, "left2"),
                                    (&bot.clickpack.right2, "right2"),
                                ] {
                                    ui.label(clicks.1);
                                    ui.label(format!("{} sounds", clicks.0.num_sounds()));
                                    ui.end_row();
                                }
                            });
                    } else {
                        ui.label("Structure cannot be displayed since the clickpack is not loaded");
                        ui.horizontal(|ui| {
                            ui.label("Load the clickpack to see it:");
                            if ui.button("Load").clicked() {
                                self.load_clickpack_no_pitch(&dialog, bot)
                            }
                        });
                    }
                });
            });
            ui.separator();
        }

        ui.collapsing("Info", |ui| {
            ui.label("The clickpack should either have player1, player2, left1, right1, left2 and right2 \
                    folders inside it, \
                    or just audio files. You can add hardclicks, clicks, softclicks, microclicks, \
                    hardreleases, releases, softreleases and microreleases as directories.");
            ui.label("Optionally you can put a noise.* or whitenoise.* file inside the clickpack \
                    folder to have an option to overlay background noise.");
            ui.label("All audio files will be resampled to the selected sample rate.");
            ui.label("Pitch step is the step between pitch changes in the pitch table. The lower it is, \
                    the more random the pitch is. A pitch value of 1.0 means no pitch.");
        });
        ui.collapsing("Supported audio formats", |ui| {
            ui.label(
                "AAC, ADPCM, AIFF, ALAC, CAF, FLAC, MKV, MP1, MP2, MP3, MP4, OGG, Vorbis, WAV, \
                and WebM audio files.",
            );
        });

        dialog.show_dialog();
    }

    fn render_replay(&mut self, dialog: &Modal) {
        let Some(clickpack_path) = &self.clickpack_path else {
            return;
        };

        // load clickpack
        if let Err(e) = self.bot.borrow_mut().load_clickpack(
            clickpack_path,
            if self.conf.pitch_enabled {
                self.conf.pitch
            } else {
                Pitch::NO_PITCH
            },
        ) {
            dialog
                .dialog()
                .with_title("Failed to load clickpack")
                .with_body(e)
                .with_icon(Icon::Error)
                .open();
            return;
        }

        self.clickpack_num_sounds = Some(self.bot.borrow().clickpack.num_sounds());

        let start = Instant::now();
        let segment = self.bot.borrow_mut().render_replay(
            &self.replay,
            self.conf.noise,
            self.conf.noise_volume,
            self.conf.normalize,
            if !self.conf.expr_text.is_empty() && self.expr_error.is_empty() {
                self.conf.expr_variable
            } else {
                ExprVariable::None
            },
            self.conf.pitch_enabled,
            self.conf.cut_sounds,
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
                dialog
                    .dialog()
                    .with_title("Failed to write output file!")
                    .with_body(format!(
                        "{e}. Try running the program as administrator \
                        or selecting a different directory."
                    ))
                    .with_icon(Icon::Error)
                    .open();
            }
        } else if let Err(e) = f {
            dialog
                .dialog()
                .with_title("Failed to open output file!")
                .with_body(format!(
                    "{e}. Try running the program as administrator \
                    or selecting a different directory."
                ))
                .with_icon(Icon::Error)
                .open();
        }

        let num_actions = self.replay.actions.len();
        let filename = output.file_name().unwrap().to_str().unwrap();

        dialog
            .dialog()
            .with_title("Done!")
            .with_body(format!(
                "Successfully exported '{filename}' in {end:?} (~{} actions/second)",
                num_actions as f32 / end.as_secs_f32()
            ))
            .with_icon(Icon::Success)
            .open();
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
            ui.label("• delta: Frame delta between the current and previous action");
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

        let mut expr_changed = false;

        ui.horizontal(|ui| {
            ui.label("y =");

            // save current expression if the new expression on this frame is invalid
            let prev_expr = self.conf.expr_text.clone();

            if ui.text_edit_singleline(&mut self.conf.expr_text).changed() || self.update_expr {
                expr_changed = true;
                self.update_expr = false;

                // recompile expression, check for compile errors
                let mut bot = self.bot.borrow_mut();
                if let Err(e) = bot.compile_expression(&self.conf.expr_text) {
                    self.expr_error = e.to_string();
                } else {
                    self.expr_error.clear(); // clear errors

                    // update namespace so we can check for undefined variables
                    bot.update_namespace(
                        &ExtendedAction::default(),
                        0,
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
            ui.add_space(4.0);
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
                    ExprVariable::Variation {
                        negative: self.expr_variable_variation_negative,
                    },
                    "Variation",
                )
                .on_hover_text("Changes the bounds of the random volume offset");
                ui.radio_value(&mut self.conf.expr_variable, ExprVariable::Value, "Value")
                    .on_hover_text("Changes the volume value (addition)");
                ui.radio_value(
                    &mut self.conf.expr_variable,
                    ExprVariable::TimeOffset,
                    "Time offset",
                )
                .on_hover_text("Offsets the time of the action");
            });
        });
        if let ExprVariable::Variation { negative } = &mut self.conf.expr_variable {
            help_text(ui, "Extend the variation range to negative numbers", |ui| {
                ui.checkbox(negative, "Negate expression");
                self.expr_variable_variation_negative = *negative;
            });
        }

        // plot data aspect
        ui.horizontal(|ui| {
            drag_value(
                ui,
                &mut self.conf.plot_data_aspect,
                "Data aspect",
                0.001..=f32::INFINITY,
                "",
            );
            if ui.button("Reset").clicked() {
                self.conf.plot_data_aspect = 20.0;
            }
        });

        let plot_points = if expr_changed {
            let prev_frame = RefCell::new(0);

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
                        *prev_frame.borrow(),
                        self.replay.last_frame(),
                        self.replay.fps as _,
                    );
                    *prev_frame.borrow_mut() = action.frame;

                    // compute the expression for this action
                    let value = self.bot.borrow_mut().eval_expr().unwrap_or(0.);
                    (t, value)
                },
                0.0..num_actions as f64,
                num_actions.min(MAX_PLOT_POINTS),
            );
            self.plot_points = points.points().to_vec().into(); // save in cache
            points
        } else {
            // PlotPoints can either be an Owned(Vec<PlotPoint>) or a Generator(ExplicitGenerator),
            // so we have to do this hack in order to not clone all plot points each frame
            let plot_points = self.plot_points.clone();
            PlotPoints::from_explicit_callback(
                move |t| {
                    plot_points
                        .get(t as usize)
                        .unwrap_or(&PlotPoint::new(0.0, 0.0))
                        .y
                },
                0.0..self.plot_points.len().saturating_sub(1) as f64,
                self.plot_points.len(),
            )
        };

        let line = Line::new(plot_points).name(self.conf.expr_variable.to_string());
        ui.add_space(4.0);

        ui.add_enabled_ui(self.expr_error.is_empty() && num_actions > 0, |ui| {
            let plot = Plot::new("volume_multiplier_plot")
                .legend(Legend::default())
                .data_aspect(self.conf.plot_data_aspect)
                .y_axis_min_width(4.0);
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
                            dialog
                                .dialog()
                                .with_title("No output file was selected")
                                .with_body("Please select an output file")
                                .with_icon(Icon::Error)
                                .open();
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
            if !self.clickpack_has_noise {
                self.conf.noise = false;
            }

            // overlay noise checkbox
            ui.add_enabled_ui(self.clickpack_has_noise, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.conf.noise, "Overlay noise")
                        .on_disabled_hover_text("Your clickpack doesn't have a noise file")
                        .on_hover_text("Overlays the noise file that's in the clickpack directory");
                    drag_value(
                        ui,
                        &mut self.conf.noise_volume,
                        "Noise volume",
                        0.0..=f32::INFINITY,
                        "Noise volume multiplier",
                    );
                });
            });

            help_text(
                ui,
                "Cut overlapping click sounds, changes the sound significantly in spams",
                |ui| ui.checkbox(&mut self.conf.cut_sounds, "Cut sounds"),
            );

            // normalize audio checkbox
            ui.checkbox(&mut self.conf.normalize, "Normalize audio")
                .on_hover_text(
                "Whether to normalize the output audio\n(make all samples to be in range of 0-1)",
            );

            // audio framerate inputfield
            ui.horizontal(|ui| {
                u32_edit_field_min1(ui, &mut self.conf.sample_rate);
                help_text(
                    ui,
                    "Audio framerate.\nDon't touch this if you don't know what you're doing",
                    |ui| {
                        ui.label("Sample rate");
                    },
                );
            });
        });

        ui.collapsing("Advanced", |ui| {
            self.show_plot(ui);
        });

        ui.separator();

        let has_output = self.output.is_some();
        let has_clicks = self.clickpack_path.is_some();
        let has_actions = self.replay.has_actions();
        let is_enabled = has_output && has_clicks && has_actions;
        let error_text = if !has_output {
            "Please select an output file"
        } else if !has_clicks {
            "Please select a clickpack"
        } else {
            "Please load a replay"
        };
        ui.horizontal(|ui| {
            ui.add_enabled_ui(is_enabled, |ui| {
                if ui
                    .button("Render!")
                    .on_disabled_hover_text(error_text)
                    .on_hover_text("Start rendering the replay.\nThis might take some time!")
                    .clicked()
                {
                    self.render_replay(&dialog); // TODO: run this on a separate thread
                }
            });
            if !is_enabled {
                ui.label(error_text);
            }
        });

        dialog.show_dialog();
    }

    fn show_clickpack_db(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        self.clickpack_db
            .show(ui, &ureq_get, &|| FileDialog::new().pick_folder());

        if let Some(select_path) = self.clickpack_db.select_clickpack.take() {
            self.select_clickpack(&select_path);
            // self.show_clickpack_db = false; // close this viewport
        }
    }
}
