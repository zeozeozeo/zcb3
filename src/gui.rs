use crate::built_info;
#[allow(unused_imports)]
use anyhow::{Context as _, Result};
use bot::{
    Action, Bot, ChangeVolumeFor, ClickpackConversionSettings, ExprVariable, ExtendedAction, Pitch,
    RemoveSilenceFrom, Replay, ReplayType, Timings, VolumeSettings,
};
use eframe::{
    egui::{self, DragValue, Key, RichText},
    emath,
    epaint::Color32,
};
use egui_clickpack_db::ClickpackDb;
use egui_modal::{Icon, Modal};
use egui_plot::PlotPoint;
use poll_promise::Promise;
#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;
#[cfg(target_arch = "wasm32")]
use rfd::FileHandle;
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use serde_json::Value;
use std::{
    cell::RefCell,
    io::Write,
    ops::RangeInclusive,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;

#[cfg(not(target_arch = "wasm32"))]
use std::io::BufWriter;

#[cfg(target_arch = "wasm32")]
use std::io::Read;

const MAX_PLOT_POINTS: usize = 4096;

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone)]
struct SendWrapper<T>(T);
#[cfg(target_arch = "wasm32")]
unsafe impl<T> Send for SendWrapper<T> {}
#[cfg(target_arch = "wasm32")]
unsafe impl<T> Sync for SendWrapper<T> {}
#[cfg(target_arch = "wasm32")]
impl<T> std::ops::Deref for SendWrapper<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileDialogType {
    SaveConfig,
    LoadConfig,
    SelectReplay,
    SelectClickpack,
    SelectClickpackConvert,
    SelectOutput,
    ExportMidi,
}

#[derive(Debug, Clone)]
enum FileDialogResult {
    #[cfg(not(target_arch = "wasm32"))]
    File(PathBuf),
    #[cfg(target_arch = "wasm32")]
    Data(String, Vec<u8>), // filename, bytes
    #[cfg(target_arch = "wasm32")]
    DataList(Vec<(String, Vec<u8>)>),
    #[cfg(target_arch = "wasm32")]
    Handle(SendWrapper<FileHandle>),
    #[cfg(not(target_arch = "wasm32"))]
    Folder(PathBuf),
    Cancelled,
}

enum RenderResult {
    Success(bot::AudioSegment, Duration, PathBuf, bot::Bot),
    Error(String, bot::Bot),
    ReplayLoaded(Replay, bot::Bot),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderStage {
    LoadingClickpack,
    RenderingReplay,
    #[cfg(not(target_arch = "wasm32"))]
    ConvertingClickpack,
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_promise<T>(future: impl std::future::Future<Output = T> + Send + 'static) -> Promise<T>
where
    T: Send + 'static,
{
    Promise::spawn_async(future)
}

#[cfg(target_arch = "wasm32")]
fn spawn_promise<T>(future: impl std::future::Future<Output = T> + 'static) -> Promise<T>
where
    T: Send + 'static,
{
    let (sender, promise) = Promise::new();
    wasm_bindgen_futures::spawn_local(async move {
        let result = future.await;
        sender.send(result);
    });
    promise
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq)]
pub enum RenderPostprocessType {
    /// Save the audio file as-is.
    #[default]
    None,
    /// Normalize the audio file.
    Normalize,
    /// Clamp samples to `[-1.0, 1.0]`.
    Clamp,
}

fn get_version() -> String {
    built_info::PKG_VERSION.to_string()
}

fn f32_one() -> f32 {
    1.0
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
struct Config {
    #[serde(default = "get_version")]
    version: String,
    noise: bool,
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
    discard_deaths: bool,
    swap_players: bool,
    plot_data_aspect: f32,
    #[serde(default = "ClickpackConversionSettings::default")]
    conversion_settings: ClickpackConversionSettings,
    #[serde(default = "bool::default")]
    cut_sounds: bool,
    #[serde(default = "f32_one")]
    noise_volume: f32,
    postprocess_type: RenderPostprocessType,
}

impl Config {
    #[cfg(not(target_arch = "wasm32"))]
    fn save(&self, path: &PathBuf) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load(&mut self, path: &PathBuf) -> Result<()> {
        let f = std::fs::File::open(path)?;
        *self = serde_json::from_reader(f)?;
        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    fn load_from_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        *self = serde_json::from_slice(bytes)?;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    fn save_to_bytes(&self) -> Result<Vec<u8>> {
        let json = serde_json::to_string_pretty(self)?;
        Ok(json.into_bytes())
    }

    fn replay_changed(&self, other: &Self) -> bool {
        self.timings != other.timings
            || self.vol_settings != other.vol_settings
            || self.sort_actions != other.sort_actions
            || self.discard_deaths != other.discard_deaths
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: get_version(),
            noise: false,
            pitch_enabled: true,
            pitch: Pitch::default(),
            timings: Timings::default(),
            vol_settings: VolumeSettings::default(),
            litematic_export_releases: false,
            midi_key: 60, // C4
            sample_rate: 48000,
            expr_text: String::new(),
            expr_variable: ExprVariable::Variation { negative: true },
            sort_actions: true,
            plot_data_aspect: 20.0,
            conversion_settings: ClickpackConversionSettings::default(),
            cut_sounds: false,
            noise_volume: 1.0,
            discard_deaths: true,
            swap_players: false,
            postprocess_type: RenderPostprocessType::default(),
        }
    }
}

struct App {
    conf: Config,
    stage: Stage,
    replay: Replay,
    bot: RefCell<Bot>,
    output: Option<PathBuf>,
    #[cfg(target_arch = "wasm32")]
    output_handle: Option<SendWrapper<FileHandle>>,
    // autocutter: AutoCutter,
    last_chars: [Key; 9],
    char_idx: u8,
    expr_error: String,
    plot_points: Rc<Vec<PlotPoint>>,
    #[cfg(not(target_arch = "wasm32"))]
    update_to_tag: Option<Rc<String>>,
    update_expr: bool,
    clickpack_path: Option<PathBuf>,
    conf_after_replay_selected: Option<Config>,
    replay_path: Option<PathBuf>,
    clickpack_has_noise: bool,
    #[cfg(target_arch = "wasm32")]
    clickpack_files: Option<Vec<(String, Vec<u8>)>>,
    expr_variable_variation_negative: bool,
    override_fps_enabled: bool,
    override_fps: f64,
    clickpack_db: ClickpackDb,
    show_clickpack_db: bool,
    clickpack_db_title: String,
    show_suitable_step_notice: bool,
    pending_file_dialog: Option<(FileDialogType, Promise<FileDialogResult>)>,
    render_progress: f32,
    render_stage: Option<RenderStage>,
    render_promise: Option<Promise<RenderResult>>,
    progress_receiver: Option<std::sync::mpsc::Receiver<f32>>,
    #[cfg(target_arch = "wasm32")]
    error_dialog: egui_modal::Modal,
}

impl Default for App {
    fn default() -> Self {
        Self {
            conf: Config::default(),
            #[cfg(target_arch = "wasm32")]
            error_dialog: egui_modal::Modal::new(&egui::Context::default(), "error_dialog"),
            stage: Stage::default(),
            replay: Replay::default(),
            bot: RefCell::new(Bot::default()),
            output: None,
            // autocutter: AutoCutter::default(),
            last_chars: [Key::A; 9],
            char_idx: 0,
            expr_error: String::new(),
            plot_points: Rc::new(vec![]),
            #[cfg(not(target_arch = "wasm32"))]
            update_to_tag: None,
            update_expr: false,
            clickpack_path: None,
            conf_after_replay_selected: None,
            replay_path: None,
            clickpack_has_noise: false,
            expr_variable_variation_negative: true,
            override_fps_enabled: false,
            override_fps: 0.0,
            clickpack_db: ClickpackDb::default(),
            show_clickpack_db: false,
            clickpack_db_title: String::new(),
            show_suitable_step_notice: false,
            pending_file_dialog: None,
            render_progress: 0.0,
            render_stage: None,
            render_promise: None,
            progress_receiver: None,
            #[cfg(target_arch = "wasm32")]
            output_handle: None,
            #[cfg(target_arch = "wasm32")]
            clickpack_files: None,
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

/// Returns whether changed
fn drag_value<Num: emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut Num,
    text: impl Into<String>,
    clamp_range: RangeInclusive<Num>,
    help: &str,
) -> bool {
    let mut changed = false;
    help_text(ui, help, |ui| {
        let resp = ui.add(DragValue::new(value).range(clamp_range.clone()).speed(0.01));
        let dragged = resp.dragged();
        changed = resp.changed();
        ui.label(
            if dragged && (clamp_range.start() == value || clamp_range.end() == value) {
                RichText::new(text).color(Color32::LIGHT_RED)
            } else {
                RichText::new(text)
            },
        );
    });
    changed
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(rx) = &self.progress_receiver {
            while let Ok(progress) = rx.try_recv() {
                self.render_progress = progress;
            }
        }
        // poll pending file dialogs
        let mut dialog = Modal::new(ctx, "file_dialog_modal");
        self.poll_file_dialog(ctx, &dialog);
        self.poll_render_result(ctx, &dialog);
        dialog.show_dialog();

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
                    #[cfg(not(target_arch = "wasm32"))]
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
                            self.load_config();
                        }
                        ui.style_mut().spacing.item_spacing.x = 5.;
                        if ui
                            .button("Save")
                            .on_hover_text("Save the current configuration")
                            .clicked()
                        {
                            self.save_config();
                        }
                    });
                });
            });

            dialog.show_dialog();
            update_dialog.show_dialog();
            modal.show_dialog();

            #[cfg(not(target_arch = "wasm32"))]
            self.show_update_check_modal(&modal);
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
                #[cfg(not(target_arch = "wasm32"))]
                {
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
                #[cfg(target_arch = "wasm32")]
                {
                    self.clickpack_db_title = "ClickpackDB".to_string();
                }
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
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
            #[cfg(target_arch = "wasm32")]
            {
                self.show_clickpack_db(ctx);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Ubuntu Chromium/37.0.2062.94 Chrome/37.0.2062.94 Safari/537.36";

#[cfg(not(target_arch = "wasm32"))]
fn ureq_agent() -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .user_agent(USER_AGENT)
        .build();
    ureq::Agent::new_with_config(config)
}

#[cfg(not(target_arch = "wasm32"))]
fn ureq_fn(url: &str, post: bool) -> Result<Vec<u8>, String> {
    log::debug!("request url: '{url}', POST={post}");
    if post {
        return ureq_agent()
            .post(url)
            .send_empty()
            .map_err(|e| e.to_string())?
            .body_mut()
            .read_to_vec()
            .map_err(|_| "failed to read body".to_string());
    }
    ureq_agent()
        .get(url)
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_to_vec()
        .map_err(|_| "failed to read body".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn get_latest_tag() -> Result<String> {
    let body = ureq_agent()
        .get("https://api.github.com/repos/zeozeozeo/zcb3/tags")
        .call()?
        .body_mut()
        .read_to_string()?;

    log::debug!("response text: '{body}'");
    let v: Value = serde_json::from_str(&body)?;
    let tags = v.as_array().context("not an array")?;
    let latest_tag = tags.first().context("couldn't latest tags")?;
    let name = latest_tag.get("name").context("couldn't get tag name")?;
    let tagname = name.as_str().context("tag name is not a string")?;

    Ok(tagname.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn is_older_version(current: &str, latest: &str) -> bool {
    current
        .split('.')
        .map(|s| s.parse::<u32>().unwrap_or(0))
        .zip(latest.split('.').map(|s| s.parse::<u32>().unwrap_or(0)))
        .any(|(c, l)| c < l)
}

#[cfg(not(target_arch = "wasm32"))]
fn capitalize_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

impl App {
    /// Start an async file dialog
    fn start_file_dialog(&mut self, dialog_type: FileDialogType) {
        if self.pending_file_dialog.is_some() {
            return; // already have a pending dialog
        }

        let promise = spawn_promise(async move {
            match dialog_type {
                FileDialogType::SaveConfig => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if let Some(path) = FileDialog::new()
                            .add_filter("Config file", &["json"])
                            .save_file()
                        {
                            return FileDialogResult::File(path);
                        }
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(handle) = rfd::AsyncFileDialog::new()
                            .add_filter("Config file", &["json"])
                            .save_file()
                            .await
                        {
                            return FileDialogResult::Handle(SendWrapper(handle));
                        }
                    }
                    FileDialogResult::Cancelled
                }
                FileDialogType::LoadConfig => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if let Some(path) = FileDialog::new()
                            .add_filter("Config file", &["json"])
                            .pick_file()
                        {
                            return FileDialogResult::File(path);
                        }
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(handle) = rfd::AsyncFileDialog::new()
                            .add_filter("Config file", &["json"])
                            .pick_file()
                            .await
                        {
                            let data = handle.read().await;
                            return FileDialogResult::Data(handle.file_name(), data);
                        }
                    }
                    FileDialogResult::Cancelled
                }
                FileDialogType::SelectReplay => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if let Some(path) = FileDialog::new()
                            .add_filter("Replay file", bot::Replay::SUPPORTED_EXTENSIONS)
                            .pick_file()
                        {
                            return FileDialogResult::File(path);
                        }
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(handle) = rfd::AsyncFileDialog::new()
                            .add_filter("Replay file", bot::Replay::SUPPORTED_EXTENSIONS)
                            .pick_file()
                            .await
                        {
                            let data = handle.read().await;
                            return FileDialogResult::Data(handle.file_name(), data);
                        }
                    }
                    FileDialogResult::Cancelled
                }
                FileDialogType::SelectClickpack | FileDialogType::SelectClickpackConvert => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if let Some(path) = FileDialog::new().pick_folder() {
                            return FileDialogResult::Folder(path);
                        }
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(handles) = rfd::AsyncFileDialog::new()
                            .add_filter("Supported archive types", &["zip"])
                            .pick_files()
                            .await
                        {
                            let mut list = Vec::new();
                            if handles.len() == 1 && handles[0].file_name().ends_with(".zip") {
                                let handle = &handles[0];
                                let data = handle.read().await;
                                if let Ok(list) = Self::unzip_clickpack(&data) {
                                    return FileDialogResult::DataList(list);
                                }
                            } else {
                                for handle in handles {
                                    let data = handle.read().await;
                                    list.push((handle.file_name(), data));
                                }
                            }
                            return FileDialogResult::DataList(list);
                        }
                    }
                    FileDialogResult::Cancelled
                }
                FileDialogType::SelectOutput => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if let Some(path) = FileDialog::new()
                            .add_filter("Supported audio files", &["wav"])
                            .save_file()
                        {
                            return FileDialogResult::File(path);
                        }
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(handle) = rfd::AsyncFileDialog::new()
                            .add_filter("Supported audio files", &["wav"])
                            .save_file()
                            .await
                        {
                            return FileDialogResult::Handle(SendWrapper(handle));
                        }
                    }
                    FileDialogResult::Cancelled
                }
                FileDialogType::ExportMidi => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if let Some(path) = FileDialog::new()
                            .add_filter("MIDI file", &["mid"])
                            .save_file()
                        {
                            return FileDialogResult::File(path);
                        }
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(handle) = rfd::AsyncFileDialog::new()
                            .add_filter("MIDI file", &["mid"])
                            .save_file()
                            .await
                        {
                            return FileDialogResult::Handle(SendWrapper(handle));
                        }
                    }
                    FileDialogResult::Cancelled
                }
            }
        });

        self.pending_file_dialog = Some((dialog_type, promise));
    }

    /// Poll pending file dialog and handle result
    fn poll_file_dialog(&mut self, ctx: &egui::Context, dialog: &Modal) {
        if let Some((dialog_type, promise)) = &self.pending_file_dialog {
            if let Some(result) = promise.ready() {
                let result = result.clone();
                let dialog_type = *dialog_type;
                self.pending_file_dialog = None;

                match dialog_type {
                    FileDialogType::SaveConfig => match result {
                        #[cfg(not(target_arch = "wasm32"))]
                        FileDialogResult::File(path) =>
                        {
                            #[cfg(not(target_arch = "wasm32"))]
                            if let Err(e) = self.conf.save(&path) {
                                dialog
                                    .dialog()
                                    .with_title("Failed to save config")
                                    .with_body(e)
                                    .with_icon(Icon::Error)
                                    .open();
                            }
                        }
                        #[cfg(target_arch = "wasm32")]
                        FileDialogResult::Handle(handle) => {
                            let handle = handle.0;
                            if let Ok(data) = self.conf.save_to_bytes() {
                                wasm_bindgen_futures::spawn_local(async move {
                                    let _ = handle.write(&data).await;
                                });
                            }
                        }
                        _ => {
                            dialog
                                .dialog()
                                .with_title("No file was selected")
                                .with_body("Please select a file")
                                .with_icon(Icon::Error)
                                .open();
                        }
                    },
                    FileDialogType::LoadConfig => {
                        let load_res = match result {
                            #[cfg(not(target_arch = "wasm32"))]
                            FileDialogResult::File(path) => self.conf.load(&path),
                            #[cfg(target_arch = "wasm32")]
                            FileDialogResult::Data(_, bytes) => self.conf.load_from_bytes(&bytes),
                            _ => Err(anyhow::anyhow!("No file selected")),
                        };

                        if let Err(e) = load_res {
                            dialog
                                .dialog()
                                .with_title("Failed to load config")
                                .with_body(e)
                                .with_icon(Icon::Error)
                                .open();
                        } else {
                            self.update_expr = true;
                            // reload replay if it was loaded
                            if let Some(replay_path) = &self.replay_path.clone() {
                                let _ = self
                                    .load_replay(dialog, replay_path)
                                    .map_err(|e| log::error!("failed to reload replay: {e}"));
                            }
                        }
                    }
                    FileDialogType::SelectReplay => match result {
                        #[cfg(not(target_arch = "wasm32"))]
                        FileDialogResult::File(path) => {
                            self.replay_path = Some(path.clone());
                            if self.load_replay(dialog, &path).is_ok() {
                                self.stage = Stage::SelectClickpack;
                            }
                        }
                        #[cfg(target_arch = "wasm32")]
                        FileDialogResult::Data(name, bytes) => {
                            self.replay_path = None;
                            self.start_load_replay_from_bytes(name, bytes);
                            self.stage = Stage::SelectClickpack;
                        }
                        _ => {
                            dialog
                                .dialog()
                                .with_title("No file was selected")
                                .with_body("Please select a file")
                                .with_icon(Icon::Error)
                                .open();
                        }
                    },
                    FileDialogType::SelectClickpack => match result {
                        #[cfg(not(target_arch = "wasm32"))]
                        FileDialogResult::Folder(path) => {
                            self.select_clickpack(&path);
                            self.stage = if self.replay.has_actions() {
                                Stage::Render
                            } else {
                                Stage::SelectReplay
                            };
                        }
                        #[cfg(target_arch = "wasm32")]
                        FileDialogResult::DataList(files) => {
                            self.select_clickpack_from_bytes(&files);
                            self.stage = if self.replay.has_actions() {
                                Stage::Render
                            } else {
                                Stage::SelectReplay
                            };
                        }
                        _ => {
                            dialog
                                .dialog()
                                .with_title(if cfg!(target_arch = "wasm32") {
                                    "No file was selected"
                                } else {
                                    "No directory was selected"
                                })
                                .with_body(if cfg!(target_arch = "wasm32") {
                                    "Please select a file"
                                } else {
                                    "Please select a directory"
                                })
                                .with_icon(Icon::Error)
                                .open();
                        }
                    },
                    FileDialogType::SelectClickpackConvert => match result {
                        #[cfg(not(target_arch = "wasm32"))]
                        FileDialogResult::Folder(path) => {
                            self.convert_clickpack(&path, dialog);
                        }
                        #[cfg(target_arch = "wasm32")]
                        FileDialogResult::DataList(files) => {
                            self.convert_clickpack_from_bytes(&files, dialog);
                        }
                        _ => {
                            dialog
                                .dialog()
                                .with_title("No directory was selected")
                                .with_body("Please select a directory")
                                .with_icon(Icon::Error)
                                .open();
                        }
                    },
                    FileDialogType::SelectOutput => match result {
                        #[cfg(not(target_arch = "wasm32"))]
                        FileDialogResult::File(path) => {
                            log::info!("selected output file: {path:?}");
                            self.output = Some(path);
                        }
                        #[cfg(target_arch = "wasm32")]
                        FileDialogResult::Handle(handle) => {
                            log::info!("selected output file (handle): {}", handle.0.file_name());
                            self.output = Some(PathBuf::from(handle.0.file_name()));
                            self.output_handle = Some(handle);
                        }
                        _ => {
                            dialog
                                .dialog()
                                .with_title("No output file was selected")
                                .with_body("Please select an output file")
                                .with_icon(Icon::Error)
                                .open();
                        }
                    },
                    FileDialogType::ExportMidi => match result {
                        #[cfg(not(target_arch = "wasm32"))]
                        FileDialogResult::File(path) => {
                            if let Err(e) = self.do_export_midi(&path) {
                                dialog
                                    .dialog()
                                    .with_title("Failed to export MIDI")
                                    .with_body(capitalize_first_letter(&e.to_string()))
                                    .with_icon(Icon::Error)
                                    .open();
                            }
                        }
                        #[cfg(target_arch = "wasm32")]
                        FileDialogResult::Handle(handle) => {
                            if let Ok(data) = self.do_export_midi_bytes() {
                                wasm_bindgen_futures::spawn_local(async move {
                                    let _ = handle.0.write(&data).await;
                                });
                            }
                        }
                        _ => {
                            dialog
                                .dialog()
                                .with_title("No file was selected")
                                .with_body("Please select a file")
                                .with_icon(Icon::Error)
                                .open();
                        }
                    },
                }

                ctx.request_repaint(); // request repaint after handling dialog result
            } else {
                ctx.request_repaint(); // keep repainting while waiting for dialog
            }
        }
    }

    fn start_render(&mut self) {
        if self.render_promise.is_some() {
            return;
        }

        let mut bot = self.bot.replace(bot::Bot::new(self.conf.sample_rate));
        let replay = self.replay.clone();
        let config = self.conf.clone();

        #[cfg(not(target_arch = "wasm32"))]
        let output_path = self.output.clone().expect("no output path");
        #[cfg(target_arch = "wasm32")]
        let output_path = std::path::PathBuf::from("output.wav");

        #[cfg(not(target_arch = "wasm32"))]
        let clickpack_path = self.clickpack_path.clone().expect("no clickpack path");

        #[cfg(target_arch = "wasm32")]
        let clickpack_files = self.clickpack_files.clone().expect("no clickpack files");

        let (tx, rx) = std::sync::mpsc::channel();
        self.progress_receiver = Some(rx);
        self.render_stage = Some(RenderStage::RenderingReplay);

        self.render_promise = Some(spawn_promise(async move {
            let timer = crate::utils::Timer::new();

            // load clickpack
            #[cfg(not(target_arch = "wasm32"))]
            if let Err(e) = bot.load_clickpack(&clickpack_path, config.pitch) {
                return RenderResult::Error(format!("Failed to load clickpack: {e}"), bot);
            }

            #[cfg(target_arch = "wasm32")]
            if let Err(e) = bot.load_clickpack_from_bytes(&clickpack_files, config.pitch) {
                return RenderResult::Error(format!("Failed to load clickpack: {e}"), bot);
            }

            let expr_valid = !config.expr_text.is_empty();
            let expr_var = if expr_valid {
                config.expr_variable
            } else {
                ExprVariable::None
            };

            #[cfg(not(target_arch = "wasm32"))]
            let segment = bot.render_replay(
                &replay,
                config.noise,
                config.noise_volume,
                config.postprocess_type == RenderPostprocessType::Normalize,
                expr_var,
                config.pitch_enabled,
                config.cut_sounds,
                |p| {
                    let _ = tx.send(p);
                },
            );

            #[cfg(target_arch = "wasm32")]
            let segment = bot
                .render_replay_async(
                    &replay,
                    config.noise,
                    config.noise_volume,
                    config.postprocess_type == RenderPostprocessType::Normalize,
                    expr_var,
                    config.pitch_enabled,
                    config.cut_sounds,
                    |p| {
                        let _ = tx.send(p);
                    },
                    || gloo_timers::future::TimeoutFuture::new(0),
                )
                .await;

            RenderResult::Success(segment, timer.elapsed(), output_path, bot)
        }));
    }

    fn poll_render_result(&mut self, ctx: &egui::Context, dialog: &Modal) {
        // poll progress if any
        if let Some(rx) = &self.progress_receiver {
            while let Ok(p) = rx.try_recv() {
                self.render_progress = p;
            }
            ctx.request_repaint();
        }

        // poll promise
        if let Some(promise) = self.render_promise.as_ref() {
            if let Some(_result) = promise.ready() {
                self.render_stage = None;
                self.render_progress = 0.0;
                self.progress_receiver = None;

                let promise_result = self.render_promise.take().unwrap().block_and_take();
                match promise_result {
                    #[allow(unused_variables)] // wasm
                    RenderResult::Success(segment, duration, path, bot) => {
                        self.bot.replace(bot);
                        if segment.frames.is_empty() {
                            // this was a conversion task
                            dialog
                                .dialog()
                                .with_title("Success!")
                                .with_body(format!(
                                    "Successfully converted clickpack in {:?}",
                                    duration
                                ))
                                .with_icon(Icon::Success)
                                .open();
                        } else {
                            // this was a rendering task
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                let f = match std::fs::File::create(&path) {
                                    Ok(f) => f,
                                    Err(e) => {
                                        dialog
                                            .dialog()
                                            .with_title("Failed to create file")
                                            .with_body(e)
                                            .with_icon(Icon::Error)
                                            .open();
                                        return;
                                    }
                                };
                                if let Err(e) = segment.export_wav(
                                    f,
                                    self.conf.postprocess_type == RenderPostprocessType::Clamp,
                                ) {
                                    dialog
                                        .dialog()
                                        .with_title("Failed to export WAV")
                                        .with_body(e)
                                        .with_icon(Icon::Error)
                                        .open();
                                } else {
                                    dialog
                                        .dialog()
                                        .with_title("Success!")
                                        .with_body(format!(
                                            "Successfully rendered and saved to {:?} in {:?}",
                                            path, duration
                                        ))
                                        .with_icon(Icon::Success)
                                        .open();
                                }
                            }
                            #[cfg(target_arch = "wasm32")]
                            {
                                if let Ok(data) = segment.export_wav_bytes(
                                    self.conf.postprocess_type == RenderPostprocessType::Clamp,
                                ) {
                                    wasm_bindgen_futures::spawn_local(async move {
                                        if let Some(handle) = rfd::AsyncFileDialog::new()
                                            .set_file_name("output.wav")
                                            .save_file()
                                            .await
                                        {
                                            let _ = handle.write(&data).await;
                                        }
                                    });
                                    dialog
                                        .dialog()
                                        .with_title("Success!")
                                        .with_body(format!(
                                            "Successfully rendered in {:?}. Please save the file in the dialog.",
                                            duration
                                        ))
                                        .with_icon(Icon::Success)
                                        .open();
                                } else {
                                    dialog
                                        .dialog()
                                        .with_title("Failed to export WAV")
                                        .with_body("Could not encode WAV bytes")
                                        .with_icon(Icon::Error)
                                        .open();
                                }
                            }
                        }
                    }
                    RenderResult::ReplayLoaded(replay, bot) => {
                        self.bot.replace(bot);
                        self.replay = replay;
                        self.update_expr = true;
                        self.conf_after_replay_selected = Some(self.conf.clone());
                    }
                    RenderResult::Error(e, bot) => {
                        self.bot.replace(bot);
                        dialog
                            .dialog()
                            .with_title("Failed")
                            .with_body(e)
                            .with_icon(Icon::Error)
                            .open();
                    }
                }
            }
        }
    }

    fn start_load_replay(&mut self, path: PathBuf) {
        if self.render_promise.is_some() {
            return;
        }

        let bot = self.bot.replace(bot::Bot::new(self.conf.sample_rate));
        let config = self.conf.clone();
        let override_fps_enabled = self.override_fps_enabled;
        let override_fps = self.override_fps;

        self.render_stage = Some(RenderStage::LoadingClickpack);
        self.render_promise = Some(spawn_promise(async move {
            let filename = path.file_name().unwrap_or_default().to_string_lossy();
            let replay_type = match ReplayType::guess_format(&filename) {
                Ok(t) => t,
                Err(e) => {
                    return RenderResult::Error(
                        format!("Failed to guess replay format: {}", e),
                        bot,
                    )
                }
            };

            let f = match std::fs::File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    return RenderResult::Error(format!("Failed to open replay file: {}", e), bot)
                }
            };

            let replay = Replay::build()
                .with_timings(config.timings)
                .with_vol_settings(config.vol_settings)
                .with_extended(true)
                .with_sort_actions(config.sort_actions)
                .with_discard_deaths(config.discard_deaths)
                .with_swap_players(config.swap_players)
                .with_override_fps(if override_fps_enabled {
                    Some(override_fps)
                } else {
                    None
                })
                .parse(replay_type, std::io::BufReader::new(f));

            match replay {
                Ok(r) => RenderResult::ReplayLoaded(r, bot),
                Err(e) => RenderResult::Error(format!("Failed to parse replay file: {}", e), bot),
            }
        }));
    }

    #[cfg(target_arch = "wasm32")]
    fn start_load_replay_from_bytes(&mut self, filename: String, bytes: Vec<u8>) {
        if self.render_promise.is_some() {
            return;
        }

        let bot = self.bot.replace(bot::Bot::new(self.conf.sample_rate));
        let config = self.conf.clone();
        let override_fps_enabled = self.override_fps_enabled;
        let override_fps = self.override_fps;

        self.render_stage = Some(RenderStage::LoadingClickpack);
        self.render_promise = Some(spawn_promise(async move {
            let replay_type = match ReplayType::guess_format(&filename) {
                Ok(t) => t,
                Err(e) => {
                    return RenderResult::Error(
                        format!("Failed to guess replay format: {}", e),
                        bot,
                    )
                }
            };

            let replay = Replay::build()
                .with_timings(config.timings)
                .with_vol_settings(config.vol_settings)
                .with_extended(true)
                .with_sort_actions(config.sort_actions)
                .with_discard_deaths(config.discard_deaths)
                .with_swap_players(config.swap_players)
                .with_override_fps(if override_fps_enabled {
                    Some(override_fps)
                } else {
                    None
                })
                .parse(replay_type, std::io::Cursor::new(bytes));

            match replay {
                Ok(r) => RenderResult::ReplayLoaded(r, bot),
                Err(e) => RenderResult::Error(format!("Failed to parse replay file: {}", e), bot),
            }
        }));
    }

    fn load_replay(&mut self, _dialog: &Modal, path: &Path) -> Result<()> {
        self.start_load_replay(path.to_path_buf());
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn start_convert_clickpack(&mut self, dir: PathBuf) {
        if self.render_promise.is_some() {
            return;
        }

        let mut bot = self.bot.replace(bot::Bot::new(self.conf.sample_rate));
        let config = self.conf.clone();
        let clickpack_path = self.clickpack_path.clone();

        let (_tx, rx) = std::sync::mpsc::channel();
        self.progress_receiver = Some(rx);
        self.render_stage = Some(RenderStage::ConvertingClickpack);

        self.render_promise = Some(spawn_promise(async move {
            let timer = crate::utils::Timer::new();

            // check if clickpack is loaded
            if !bot.has_clicks() {
                if let Some(cp_path) = &clickpack_path {
                    if let Err(e) = bot.load_clickpack(cp_path, bot::Pitch::NO_PITCH) {
                        return RenderResult::Error(
                            format!("Failed to load clickpack: {}", e),
                            bot,
                        );
                    }
                } else {
                    return RenderResult::Error("No clickpack path selected".to_string(), bot);
                }
            }

            if let Err(e) = bot.convert_clickpack(&dir, &config.conversion_settings) {
                RenderResult::Error(format!("Failed to convert clickpack: {}", e), bot)
            } else {
                RenderResult::Success(bot::AudioSegment::default(), timer.elapsed(), dir, bot)
            }
        }));
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn convert_clickpack(&mut self, dir: &Path, _dialog: &Modal) {
        self.start_convert_clickpack(dir.to_path_buf());
    }

    #[cfg(target_arch = "wasm32")]
    fn convert_clickpack_from_bytes(&mut self, _files: &[(String, Vec<u8>)], dialog: &Modal) {
        dialog
            .dialog()
            .with_title("Not supported")
            .with_body("Clickpack conversion is not yet supported on WASM")
            .with_icon(Icon::Error)
            .open();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn show_update_check_modal(&mut self, modal: &Modal) {
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
                if modal.button(ui, "close").clicked() {
                    self.update_to_tag = None;
                }
            });
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
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
    fn export_midi(&mut self) {
        // Check if fps is at most 32767
        if self.replay.fps as u32 > 32767 {
            log::error!("MIDI format only supports up to 32767 PPQN (framerate)");
            return;
        }

        self.start_file_dialog(FileDialogType::ExportMidi);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn do_export_midi(&self, path: &Path) -> Result<()> {
        let f = File::create(path)?;
        self.do_export_midi_to(BufWriter::new(f))
    }

    #[cfg(target_arch = "wasm32")]
    fn do_export_midi_bytes(&self) -> Result<Vec<u8>> {
        let mut data = std::io::Cursor::new(Vec::new());
        self.do_export_midi_to(&mut data)?;
        Ok(data.into_inner())
    }

    fn do_export_midi_to<W: Write>(&self, writer: W) -> Result<()> {
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
        let mut midi_data = writer;
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

    fn save_config(&mut self) {
        self.start_file_dialog(FileDialogType::SaveConfig);
    }

    fn load_config(&mut self) {
        self.start_file_dialog(FileDialogType::LoadConfig);
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
                    self.export_midi();
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

    fn show_replay_stage(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Select replay file");

        let mut dialog = Modal::new(ctx, "replay_stage_dialog");

        ui.collapsing("Timings", |ui| {
            ui.label("Click type timings. The number is the delay between actions (in seconds). \
                    If the delay between the current and previous action is bigger than the specified \
                    timing, the corresponding click type is used.");
            let t = &mut self.conf.timings;

            drag_value(ui, &mut t.hard, "Hard timing",
            t.regular..=f64::INFINITY,
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
                    0.0..=f64::INFINITY,
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
        help_text(
            ui,
            "Whether to start rendering from the first action after the last death\nOnly works with bots that record death actions",
            |ui| {
                ui.checkbox(&mut self.conf.discard_deaths, "Discard deaths");
            },
        );
        help_text(
            ui,
            "Whether to swap player 1 and player 2\nFixes a bug in xdBot which assumes player 2 to be player 1",
            |ui| {
                ui.checkbox(&mut self.conf.swap_players, "Swap players (xdBot)");
            },
        );
        ui.separator();

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.override_fps_enabled, "Override FPS");
            if self.override_fps_enabled {
                drag_value(ui, &mut self.override_fps, "FPS", 0.0..=f64::INFINITY, "");
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
                self.start_file_dialog(FileDialogType::SelectReplay);
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
• DDHOR Replay (.ddhor, old frame format)
• xBot Frame (.xbot)
• xdBot (.xd, old and new formats)
• GDReplayFormat (.gdr, used in Geode GDMegaOverlay and 2.2 MH Replay)
• qBot (.qb)
• RBot (.rbot, old and new formats)
• Zephyrus (.zr, used in OpenHack)
• ReplayEngine 1 Replay (.re, old and new formats)
• ReplayEngine 2 Replay (.re2)
• ReplayEngine 3 Replay (.re3)
• Silicate (.slc)
• Silicate 2 (.slc)
• Silicate 3 (.slc)
• GDReplayFormat 2 (.gdr2)
• uvBot (.uv)
• TCBot (.tcm)",
            );
        });

        // show dialog if there is one
        dialog.show_dialog();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn select_clickpack(&mut self, path: &Path) {
        log::info!("selected clickpack path: {path:?}");
        self.clickpack_has_noise = bot::dir_has_noise(path);
        self.clickpack_path = Some(path.to_path_buf());
        self.bot = RefCell::new(Bot::new(self.conf.sample_rate));
    }

    #[cfg(target_arch = "wasm32")]
    fn select_clickpack_from_bytes(&mut self, files: &[(String, Vec<u8>)]) {
        log::info!("selected clickpack from bytes: {} files", files.len());
        self.clickpack_files = Some(files.to_vec());
        self.clickpack_path = Some(PathBuf::from("web_clickpack"));
        self.bot = RefCell::new(Bot::new(self.conf.sample_rate));
    }

    #[cfg(target_arch = "wasm32")]
    fn unzip_clickpack(data: &[u8]) -> Result<Vec<(String, Vec<u8>)>> {
        let cursor = std::io::Cursor::new(data);
        let mut archive = zip::ZipArchive::new(cursor)?;
        let mut list = Vec::new();
        for i in 0..archive.len() {
            if let Ok(mut file) = archive.by_index(i) {
                if file.is_file() {
                    let mut bytes = Vec::new();
                    if file.read_to_end(&mut bytes).is_ok() {
                        list.push((file.name().to_string(), bytes));
                    }
                }
            }
        }
        Ok(list)
    }

    fn show_select_clickpack_stage(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Select clickpack");

        let _dialog = Modal::new(ctx, "clickpack_stage_dialog");

        ui.horizontal(|ui| {
            if ui.button("Select clickpack").clicked() {
                self.start_file_dialog(FileDialogType::SelectClickpack);
            }
            if ui
                .button(if self.show_clickpack_db {
                    "Close ClickpackDB"
                } else {
                    "Open ClickpackDB…"
                })
                .on_hover_text("Easily download clickpacks from within ZCB")
                .clicked()
            {
                self.show_clickpack_db = !self.show_clickpack_db;
            }
            if let Some(path) = &self.clickpack_path {
                if cfg!(target_arch = "wasm32") {
                    ui.label("Clickpack loaded");
                } else {
                    ui.label(format!("Selected: {}", path.display()));
                }
            } else {
                ui.label("No clickpack selected");
            }
        });
        ui.separator();

        // pitch settings
        ui.collapsing("Pitch variation", |ui| {
            ui.label(
                "Pitch variation can make clicks sound more realistic by \
                    changing their pitch randomly.",
            );
            ui.checkbox(&mut self.conf.pitch_enabled, "Enable pitch variation");
            ui.add_enabled_ui(self.conf.pitch_enabled, |ui| {
                let p = &mut self.conf.pitch;
                let mut changed = false;
                changed |= drag_value(
                    ui,
                    &mut p.from,
                    "Minimum pitch",
                    0.0..=p.to,
                    "Minimum pitch value, 1 means no change",
                );
                changed |= drag_value(
                    ui,
                    &mut p.to,
                    "Maximum pitch",
                    p.from..=f32::INFINITY,
                    "Maximum pitch value, 1 means no change",
                );

                if changed {
                    p.step = p.suitable_step();
                    self.show_suitable_step_notice = true;
                }

                ui.horizontal(|ui| {
                    if drag_value(
                        ui,
                        &mut p.step,
                        "Pitch step",
                        0.0001..=f32::INFINITY,
                        "Step between pitch values. The less = the better & the slower",
                    ) {
                        self.show_suitable_step_notice = false;
                    }
                    if self.show_suitable_step_notice {
                        ui.add_enabled(
                            false,
                            egui::Label::new("(notice: automatically set to a sane value)"),
                        );
                    }
                });
            });
        });

        ui.collapsing("Convert", |ui| {
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
                    self.start_file_dialog(FileDialogType::SelectClickpackConvert);
                }
            });
        });
    }

    fn show_plot(&mut self, ui: &mut egui::Ui) {
        ui.label(
            "Input a mathematical expression to change the volume multiplier \
                depending on some variables.",
        );
        ui.collapsing("Defined variables", |ui| {
            ui.label(
                "• frame: Current frame
• x: Player X position
• y: Player Y position
• p: Percentage in level, 0-1
• player2: 1 if player 2, 0 if player 1
• rot: Player rotation
• accel: Player Y acceleration
• down: Whether the mouse is down, 1 or 0
• fps: The FPS of the replay
• time: Elapsed time in level, in seconds
• frames: Total amount of frames in replay
• level_time: Total time in level, in seconds
• rand: Random value in the range of 0 to 1
• delta: Frame delta between the current and previous action",
            );
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

        let line = Line::new(self.conf.expr_variable.to_string(), plot_points);
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

        ui.label("Select the output audio file. In the dialog, choose a name for the new file and click 'Save'.");

        let mut dialog = Modal::new(ctx, "render_stage_dialog");

        ui.horizontal(|ui| {
            help_text(
                ui,
                if cfg!(target_arch = "wasm32") {
                    "Click 'Render' to generate the audio. You will be prompted to save the file afterwards."
                } else {
                    "Select the output .wav file.\nYou have to click 'Render' to render the output"
                },
                |ui| {
                    if cfg!(not(target_arch = "wasm32")) {
                        if ui.button("Select output file").clicked() {
                            self.start_file_dialog(FileDialogType::SelectOutput);
                        }
                    }
                },
            );
            if cfg!(not(target_arch = "wasm32")) {
                if let Some(output) = &self.output {
                    ui.label(format!(
                        "Selected output file: {}",
                        output
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Unknown")
                    ));
                }
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

            // postprocessing options
            ui.horizontal(|ui| {
                ui.radio_value(
                    &mut self.conf.postprocess_type,
                    RenderPostprocessType::None,
                    "None",
                )
                .on_hover_text("Save the audio as-is, uncapped float range (default)");
                ui.radio_value(
                    &mut self.conf.postprocess_type,
                    RenderPostprocessType::Normalize,
                    "Normalize",
                )
                .on_hover_text("Normalize samples to [-1.0, 1.0]");
                ui.radio_value(
                    &mut self.conf.postprocess_type,
                    RenderPostprocessType::Clamp,
                    "Clamp",
                )
                .on_hover_text("Clamp samples to [-1.0, 1.0] (ACB)");

                help_text(
                    ui,
                    "None = uncapped range (ZCB)\nNormalize = adjust volume of all samples\n\
                    Clamp = clamp samples, do not readjust volume (ACB)",
                    |ui| {
                        ui.label("Postprocessing");
                    },
                );
            });

            // audio framerate inputfield
            ui.horizontal(|ui| {
                if u32_edit_field_min1(ui, &mut self.conf.sample_rate).changed() {
                    // this is bad
                    self.bot.borrow_mut().sample_rate = self.conf.sample_rate;
                }

                help_text(ui, "Audio framerate", |ui| {
                    ui.label("Sample rate");
                });
            });
        });

        ui.collapsing("Advanced", |ui| {
            self.show_plot(ui);
        });

        ui.separator();

        let has_output = self.output.is_some() || cfg!(target_arch = "wasm32");
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
        if let Some(stage) = self.render_stage {
            ui.horizontal(|ui| {
                ui.label(format!("{:?}:", stage));
                ui.add(egui::ProgressBar::new(self.render_progress).show_percentage());
            });
        }

        ui.horizontal(|ui| {
            ui.add_enabled_ui(is_enabled && self.render_stage.is_none(), |ui| {
                let button_text = if self.render_stage.is_some() {
                    "Rendering..."
                } else {
                    "Render!"
                };
                if ui
                    .button(button_text)
                    .on_disabled_hover_text(error_text)
                    .on_hover_text("Start rendering the replay.\nThis might take some time!")
                    .clicked()
                {
                    self.start_render();
                }
            });
            if !is_enabled && self.render_stage.is_none() {
                ui.label(error_text);
            }
        });

        dialog.show_dialog();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn show_clickpack_db(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        self.clickpack_db.show(ui, &ureq_fn, &|| {
            #[cfg(not(target_arch = "wasm32"))]
            {
                FileDialog::new().pick_folder()
            }
            #[cfg(target_arch = "wasm32")]
            {
                None
            }
        });

        if let Some(select_path) = self.clickpack_db.select_clickpack.take() {
            self.select_clickpack(&select_path);
            // self.show_clickpack_db = false; // close this viewport
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn show_clickpack_db(&mut self, ctx: &egui::Context) {
        let mut open = self.show_clickpack_db;
        egui::Window::new("ClickpackDB")
            .open(&mut open)
            .default_size([460.0, 500.0])
            .show(ctx, |ui| {
                // we handles it internally on wasm
                fn dummy_wasm_req(_: &str, _: bool) -> Result<Vec<u8>, String> {
                    Err("unsupported".to_string())
                }
                self.clickpack_db.show(ui, &dummy_wasm_req, &|| None);
            });
        self.show_clickpack_db = open;

        if let Some(select_path) = self.clickpack_db.select_clickpack.take() {
            // find data in DB
            let mut found_data = None;
            {
                let db = self.clickpack_db.db.read().unwrap();
                for entry in db.entries.values() {
                    if let egui_clickpack_db::DownloadStatus::Downloaded { path, data, .. } =
                        &entry.dwn_status
                    {
                        if *path == select_path {
                            found_data = data.clone();
                            break;
                        }
                    }
                }
            }

            if let Some(zip_data) = found_data {
                let load_res = (|| -> Result<()> {
                    let files = Self::unzip_clickpack(&zip_data)?;
                    self.select_clickpack_from_bytes(&files);
                    self.bot
                        .borrow_mut()
                        .load_clickpack_from_bytes(&files, self.conf.pitch)?;
                    Ok(())
                })();

                if let Err(e) = load_res {
                    self.error_dialog
                        .dialog()
                        .with_title("Failed to load clickpack")
                        .with_body(e.to_string())
                        .with_icon(egui_modal::Icon::Error)
                        .open();
                } else {
                    self.clickpack_path = Some(select_path);
                    self.show_clickpack_db = false;
                }
            } else {
                log::error!(
                    "Selected clickpack path {:?} not found in in-memory DB or has no data",
                    select_path
                );
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_gui() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([460.0, 500.0])
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "ZCB3",
        options,
        Box::new(|_cc| {
            egui_extras::install_image_loaders(&_cc.egui_ctx);
            Ok(Box::new(App::default()))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
pub async fn run_gui_wasm(canvas_id: &str) -> Result<(), eframe::Error> {
    use wasm_bindgen::JsCast;
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id(canvas_id).unwrap();
    let canvas = canvas
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| {
            eframe::Error::AppCreation(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed into dyn_into",
            )))
        })?;
    eframe::WebRunner::new()
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(|_cc| {
                egui_extras::install_image_loaders(&_cc.egui_ctx);
                Ok(Box::new(App::default()))
            }),
        )
        .await
        .map_err(|e| {
            eframe::Error::AppCreation(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{:?}", e),
            )))
        })
}
