#![doc = include_str!("../README.md")]

use egui::Color32;
use egui_extras::{Column, TableBuilder};
use fuzzy_matcher::FuzzyMatcher;
use humansize::{format_size, DECIMAL};
use indexmap::IndexMap;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};
#[cfg(not(target_arch = "wasm32"))]
use zip::read::root_dir_common_filter;

#[cfg(not(target_arch = "wasm32"))]
use std::io::Cursor;

const DATABASE_URL: &str = "https://raw.githubusercontent.com/zeozeozeo/clickpack-db/main/db.json";

#[cfg(not(feature = "live"))]
const TEMP_DIRNAME: &str = "zcb-clickpackdb";

// url, is_post
type RequestFn = dyn Fn(&str, bool) -> Result<Vec<u8>, String> + Sync;

#[cfg(not(feature = "live"))]
type PickFolderFn = dyn Fn() -> Option<PathBuf> + Sync;

#[derive(Clone, Default, Debug)]
pub enum DownloadStatus {
    #[default]
    NotDownloaded,
    Downloading,
    Downloaded {
        path: PathBuf,
        do_select: bool,
        #[cfg(target_arch = "wasm32")]
        data: Option<Vec<u8>>,
    },
    Error(String),
}

#[derive(serde::Deserialize, Default)]
pub struct Database {
    pub updated_at_unix: i64,
    #[serde(rename = "clickpacks")]
    pub entries: IndexMap<String, Entry>,
    /// Hiatus URL, usually "https://hiatus.zeo.lol"
    pub hiatus: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct Entry {
    pub size: usize,
    pub uncompressed_size: usize,
    pub has_noise: bool,
    pub url: String,
    #[serde(skip)]
    pub dwn_status: DownloadStatus,
    #[serde(skip)]
    pub downloads: u32,
    // this is a String so we don't have to call to_string each time we draw the table
    #[serde(skip)]
    pub downloads_str: String,
}

#[derive(Default, Clone)]
pub enum Status {
    #[default]
    NotLoaded,
    Loading,
    Error(String),
    Loaded {
        did_filter: bool,
    },
}

#[derive(Default)]
struct Tags {
    noise: bool,
    downloaded: bool,
}

impl Tags {
    #[inline]
    const fn has_any(&self) -> bool {
        self.noise || self.downloaded
    }
}

#[derive(Default)]
pub struct ClickpackDb {
    pub status: Arc<RwLock<Status>>,
    pub db: Arc<RwLock<Database>>,
    filtered_entries: IndexMap<String, Entry>,
    search_query: String,
    pending_update: Arc<RwLock<IndexMap<String, Entry>>>,
    /// If [`Some`], this clickpack should be selected and the viewport should be closed.
    pub select_clickpack: Option<PathBuf>,
    tags: Tags,
    pending_clickpack_delete: Vec<PathBuf>,
    #[cfg(feature = "live")]
    pub has_refreshed: bool,
}

#[cfg(all(not(feature = "live"), not(target_arch = "wasm32")))]
pub fn cleanup() {
    log::info!("cleaning up temp directories...");
    let mut temp_dir = std::env::temp_dir();
    if temp_dir.try_exists().unwrap_or(false) {
        temp_dir.push(TEMP_DIRNAME);
        if temp_dir.try_exists().unwrap_or(false) {
            let _ = std::fs::remove_dir_all(temp_dir)
                .map_err(|e| log::error!("remove_dir_all failed: {e}"));
        }
    };
}

fn tag_text(ui: &mut egui::Ui, color: Color32, emote: &str, text: &str) -> egui::WidgetText {
    use egui::text::{LayoutJob, TextFormat};
    let mut job = LayoutJob::default();
    let default_color = if ui.visuals().dark_mode {
        Color32::LIGHT_GRAY
    } else {
        Color32::DARK_GRAY
    };
    job.append(
        emote,
        0.0,
        TextFormat {
            color,
            ..Default::default()
        },
    );
    job.append(
        text,
        0.0,
        TextFormat {
            color: default_color,
            ..Default::default()
        },
    );
    job.into()
}

impl ClickpackDb {
    fn load_database(
        status: Arc<RwLock<Status>>,
        db: Arc<RwLock<Database>>,
        req_fn: &'static RequestFn,
    ) {
        log::info!("loading database from {DATABASE_URL}");
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || match req_fn(DATABASE_URL, false) {
            Ok(body) => {
                Self::handle_db_response(db, status, body, req_fn);
            }
            Err(e) => {
                log::error!("failed to GET database: {e}");
                *status.write().unwrap() = Status::Error(e.to_string());
            }
        });

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            match reqwest::get(DATABASE_URL).await {
                Ok(resp) => match resp.bytes().await {
                    Ok(body) => {
                        Self::handle_db_response(db, status, body.to_vec(), req_fn);
                    }
                    Err(e) => {
                        log::error!("failed to get bytes from database response: {e}");
                        *status.write().unwrap() = Status::Error(e.to_string());
                    }
                },
                Err(e) => {
                    log::error!("failed to GET database: {e}");
                    *status.write().unwrap() = Status::Error(e.to_string());
                }
            }
        });
    }

    fn handle_db_response(
        db: Arc<RwLock<Database>>,
        status: Arc<RwLock<Status>>,
        body: Vec<u8>,
        req_fn: &'static RequestFn,
    ) {
        *db.write().unwrap() = match serde_json::from_slice(&body) {
            Ok(entries) => entries,
            Err(e) => {
                log::error!("failed to parse database: {e}");
                *status.write().unwrap() = Status::Error(e.to_string());
                return;
            }
        };
        let hiatus_url;
        {
            let db_lock = db.read().unwrap();
            hiatus_url = db_lock.hiatus.clone();
            log::info!(
                "loaded {} entries, hiatus url: {}",
                db_lock.entries.len(),
                hiatus_url,
            );
        }
        *status.write().unwrap() = Status::Loaded { did_filter: false };

        // now load downloads from hiatus
        Self::load_hiatus(db, status, hiatus_url, req_fn);
    }

    #[allow(unused_variables)] // wasm
    fn load_hiatus(
        db: Arc<RwLock<Database>>,
        status: Arc<RwLock<Status>>,
        hiatus_url: String,
        req_fn: &'static RequestFn,
    ) {
        let downloads_endpoint = hiatus_url.clone() + "/downloads/all";

        #[cfg(not(target_arch = "wasm32"))]
        match req_fn(&downloads_endpoint, false) {
            Ok(body) => Self::handle_hiatus_response(db, status, body),
            Err(e) => log::error!("failed to GET {downloads_endpoint} (hiatus): {e}"),
        }

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            match reqwest::get(&downloads_endpoint).await {
                Ok(resp) => match resp.bytes().await {
                    Ok(body) => Self::handle_hiatus_response(db, status, body.to_vec()),
                    Err(e) => log::error!("failed to get bytes from hiatus response: {e}"),
                },
                Err(e) => log::error!("failed to GET {downloads_endpoint} (hiatus): {e}"),
            }
        });
    }

    fn handle_hiatus_response(
        db: Arc<RwLock<Database>>,
        status: Arc<RwLock<Status>>,
        body: Vec<u8>,
    ) {
        let downloads: HashMap<String, u32> = match serde_json::from_slice(&body) {
            Ok(entries) => entries,
            Err(e) => {
                log::error!("failed to parse hiatus downloads: {e}");
                return;
            }
        };

        // update entries w/ downloads
        let mut db_lock = db.write().unwrap();
        for (name, downloads) in downloads {
            if downloads == 0 {
                continue; // shouldn't happen
            }
            if let Some(entry) = db_lock.entries.get_mut(&name) {
                entry.downloads = downloads;
                entry.downloads_str = downloads.to_string();
            }
        }

        // reload sorting
        *status.write().unwrap() = Status::Loaded { did_filter: false };
    }

    fn update_filtered_entries(&mut self) {
        self.filtered_entries = self.db.read().unwrap().entries.clone();

        // handle tags
        if self.tags.has_any() {
            self.filtered_entries.retain(|_, v| {
                if self.tags.noise && !v.has_noise {
                    return false;
                }
                if self.tags.downloaded
                    && !matches!(v.dwn_status, DownloadStatus::Downloaded { .. })
                {
                    return false;
                }
                true
            });
        }

        // sort by most downloads
        self.filtered_entries
            .sort_by(|_, v1, _, v2| v2.downloads.cmp(&v1.downloads));

        // fuzzy sort with search query
        if !self.search_query.is_empty() {
            let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
            self.filtered_entries
                .retain(|k, _| matcher.fuzzy_match(k, &self.search_query).is_some());
            self.filtered_entries.sort_by_cached_key(|k, _| {
                std::cmp::Reverse(matcher.fuzzy_match(k, &self.search_query).unwrap_or(0))
            });
        }
    }

    #[cfg(feature = "live")]
    pub fn mark_downloaded(&mut self, name: &str, path: PathBuf, downloaded: bool) {
        let update_status = |status: &mut DownloadStatus| {
            if downloaded {
                *status = DownloadStatus::Downloaded {
                    path: path.clone(),
                    do_select: false,
                    data: None,
                };
            } else {
                *status = DownloadStatus::NotDownloaded;
            }
        };

        if let Some(entry) = self.db.write().unwrap().entries.get_mut(name) {
            update_status(&mut entry.dwn_status);
        }
        if let Some(entry) = self.filtered_entries.get_mut(name) {
            update_status(&mut entry.dwn_status);
        }
    }

    fn update_pending_update(&mut self) {
        let mut is_empty = true;
        for (k, v) in self.pending_update.read().unwrap().iter() {
            is_empty = false;
            self.db
                .write()
                .unwrap()
                .entries
                .insert(k.clone(), v.clone());
            if self.filtered_entries.contains_key(k) {
                self.filtered_entries.insert(k.clone(), v.clone());
            }
        }
        if !is_empty {
            self.pending_update.write().unwrap().clear();
        }
        for path in self.pending_clickpack_delete.drain(..) {
            if let Err(e) = std::fs::remove_dir_all(&path) {
                log::error!("failed to delete clickpack directory {path:?}: {e}");
            }
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        req_fn: &'static RequestFn,
        #[cfg(not(feature = "live"))] pick_folder: &'static PickFolderFn,
    ) {
        let mut status = self.status.read().unwrap().clone();
        match status {
            Status::NotLoaded => {
                (*self.status.write().unwrap(), status) = (Status::Loading, Status::Loading);
                Self::load_database(self.status.clone(), self.db.clone(), req_fn);
            }
            Status::Loading => {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new());
                    ui.label("Loading database…");
                });
            }
            Status::Error(ref e) => {
                ui.colored_label(egui::Color32::RED, format!("Error loading database: {e}"));
            }
            Status::Loaded { did_filter } => {
                if !did_filter {
                    self.update_filtered_entries();
                    #[cfg(feature = "live")]
                    {
                        self.has_refreshed = true;
                    }
                    *self.status.write().unwrap() = Status::Loaded { did_filter: true };
                }
            }
        }
        self.update_pending_update();
        ui.add_enabled_ui(
            !matches!(status, Status::NotLoaded | Status::Loading),
            |ui| {
                #[cfg(not(feature = "live"))]
                self.show_table(ui, req_fn, pick_folder);
                #[cfg(feature = "live")]
                self.show_table(ui, req_fn);
            },
        );
    }

    #[allow(unused_variables)] // wasm
    fn download_entry(
        &mut self,
        mut entry: Entry,
        name: String,
        req_fn: &'static RequestFn,
        path: PathBuf,
        do_select: bool,
        hiatus_url: String,
    ) {
        log::info!("downloading entry \"{name}\" to path {path:?}");
        let pending_update = self.pending_update.clone();

        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            match req_fn(&entry.url, false) {
                Ok(body) => Self::handle_download_entry(
                    entry.clone(),
                    name.clone(),
                    path,
                    do_select,
                    body,
                    pending_update.clone(),
                ),
                Err(e) => {
                    entry.dwn_status = DownloadStatus::Error(e);
                    pending_update.write().unwrap().insert(name.clone(), entry);
                }
            }
            Self::increment_download_counter(hiatus_url, name, req_fn);
        });

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            match reqwest::get(&entry.url).await {
                Ok(resp) => match resp.bytes().await {
                    Ok(body) => Self::handle_download_entry(
                        entry.clone(),
                        name.clone(),
                        path,
                        do_select,
                        body.to_vec(),
                        pending_update.clone(),
                    ),
                    Err(e) => {
                        entry.dwn_status = DownloadStatus::Error(e.to_string());
                        pending_update.write().unwrap().insert(name.clone(), entry);
                    }
                },
                Err(e) => {
                    entry.dwn_status = DownloadStatus::Error(e.to_string());
                    pending_update.write().unwrap().insert(name.clone(), entry);
                }
            }
            // TODO: wasm Self::increment_download_counter(hiatus_url, name, req_fn);
        });
    }

    fn handle_download_entry(
        mut entry: Entry,
        name: String,
        path: PathBuf,
        do_select: bool,
        body: Vec<u8>,
        pending_update: Arc<RwLock<IndexMap<String, Entry>>>,
    ) {
        log::debug!("body length: {} bytes (zip)", body.len());
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut archive = match zip::ZipArchive::new(Cursor::new(&body)) {
                Ok(a) => a,
                Err(e) => {
                    log::error!("failed to read zip: {e}");
                    entry.dwn_status = DownloadStatus::Error(e.to_string());
                    pending_update.write().unwrap().insert(name, entry);
                    return;
                }
            };
            if let Err(e) = archive.extract_unwrapped_root_dir(&path, root_dir_common_filter) {
                log::error!("failed to extract zip to {path:?}: {e}");
                entry.dwn_status = DownloadStatus::Error(e.to_string());
            } else {
                log::info!("successfully extracted zip to {path:?}");
                entry.dwn_status = DownloadStatus::Downloaded { path, do_select };
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            entry.dwn_status = DownloadStatus::Downloaded {
                path,
                do_select,
                data: Some(body),
            };
        }

        pending_update.write().unwrap().insert(name, entry);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn increment_download_counter(hiatus_url: String, name: String, req_fn: &'static RequestFn) {
        let inc_endpoint = hiatus_url + "/inc/" + urlencoding::encode(&name).as_ref();
        match req_fn(&inc_endpoint, true /* POST */) {
            Ok(_) => {
                log::info!("incremented download counter for {name}");
            }
            Err(e) => {
                log::error!("failed to increment download counter for {name}: {e}");
            }
        }
    }

    fn refresh_button(&mut self, ui: &mut egui::Ui) {
        if ui
            .button("🔄 Refresh")
            .on_hover_text("Fetch the database again")
            .clicked()
        {
            *self.status.write().unwrap() = Status::NotLoaded;
        }
    }

    fn show_table(
        &mut self,
        ui: &mut egui::Ui,
        req_fn: &'static RequestFn,
        #[cfg(not(feature = "live"))] pick_folder: &'static PickFolderFn,
    ) {
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);

        TableBuilder::new(ui)
            .column(Column::exact(200.0))
            .column(Column::auto())
            .striped(true)
            .header(30.0, |mut header| {
                header.col(|ui| {
                    // ui.heading("Name");
                    let nr_clickpacks = self.db.read().unwrap().entries.len();
                    ui.horizontal_centered(|ui| {
                        let textedit = egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text(format!("🔎 Search in {nr_clickpacks} clickpacks"));
                        if ui.add(textedit).changed() {
                            self.update_filtered_entries();
                        }
                    });
                });
                header.col(|ui| {
                    ui.horizontal_centered(|ui| {
                        ui.style_mut().spacing.item_spacing.x = 5.0;
                        self.refresh_button(ui);
                        egui::ComboBox::new("manage_tags_combobox", "")
                            .selected_text("Tags…")
                            .show_ui(ui, |ui| {
                                let job = tag_text(ui, Color32::KHAKI, "🎧", " Has noise");
                                if ui.checkbox(&mut self.tags.noise, job).changed() {
                                    self.update_filtered_entries();
                                }
                                let job = tag_text(ui, Color32::LIGHT_GREEN, "✅", " Downloaded");
                                if ui.checkbox(&mut self.tags.downloaded, job).changed() {
                                    self.update_filtered_entries();
                                }
                            })
                    });
                });
            })
            .body(|body| {
                body.rows(text_height * 1.5, self.filtered_entries.len(), |mut row| {
                    let row_index = row.index();
                    let Some(entry) = self.filtered_entries.get_index(row_index) else {
                        return;
                    };
                    let name = entry.0.clone();
                    let entry = entry.1.clone();
                    row.col(|ui| {
                        ui.horizontal(|ui| {
                            ui.style_mut().spacing.item_spacing.x = 5.0;
                            ui.add(egui::Label::new(name.replace('_', " ")).wrap());
                            if entry.downloads != 0 {
                                ui.add_enabled(
                                    false,
                                    egui::Label::new(&entry.downloads_str).wrap(),
                                )
                                .on_disabled_hover_text("Number of downloads");
                            }
                            if entry.has_noise {
                                ui.colored_label(Color32::KHAKI, "🎧")
                                    .on_hover_text("This clickpack has a noise file")
                                    .on_hover_cursor(egui::CursorIcon::Default);
                            }
                            if matches!(entry.dwn_status, DownloadStatus::Downloaded { .. }) {
                                ui.colored_label(Color32::LIGHT_GREEN, "✅")
                                    .on_hover_text("Downloaded")
                                    .on_hover_cursor(egui::CursorIcon::Default);
                            }
                        });
                    });
                    row.col(|ui| {
                        #[cfg(not(feature = "live"))]
                        self.manage_row(ui, entry, name, req_fn, pick_folder);
                        #[cfg(feature = "live")]
                        self.manage_row(ui, entry, name, req_fn);
                    });
                });
            });

        if self.filtered_entries.is_empty() {
            ui.horizontal(|ui| {
                ui.label("Nothing here yet…");
                ui.style_mut().spacing.item_spacing.x = 5.0;
                if ui.button("Clear tags").clicked() {
                    self.tags = Tags::default();
                    self.update_filtered_entries();
                }
                self.refresh_button(ui);
            });
        } else if self.filtered_entries.len() <= 15 {
            ui.label(format!(
                "Showing {} entr{}",
                self.filtered_entries.len(),
                if self.filtered_entries.len() == 1 {
                    "y"
                } else {
                    "ies"
                }
            ));
        }
    }

    #[allow(unused_variables)] // wasm
    fn manage_row(
        &mut self,
        ui: &mut egui::Ui,
        entry: Entry,
        name: String,
        req_fn: &'static RequestFn,
        #[cfg(not(feature = "live"))] pick_folder: &'static PickFolderFn,
    ) {
        macro_rules! set_status {
            ($status:expr) => {
                self.db
                    .write()
                    .unwrap()
                    .entries
                    .get_mut(&name)
                    .unwrap()
                    .dwn_status = $status;
                self.update_filtered_entries();
            };
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(14.0);
            match entry.dwn_status {
                DownloadStatus::NotDownloaded => {
                    #[cfg(all(not(feature = "live"), not(target_arch = "wasm32")))]
                    {
                        ui.style_mut().spacing.item_spacing.x = 5.0;
                        if ui
                            .button("Download")
                            .on_hover_text("Download this clickpack into a new folder")
                            .clicked()
                        {
                            #[cfg(not(target_arch = "wasm32"))]
                            let path = pick_folder();
                            #[cfg(target_arch = "wasm32")]
                            let path = Some(std::path::PathBuf::from("wasm_memory"));

                            if let Some(path) = path {
                                set_status!(DownloadStatus::Downloading);
                                let hiatus_url = self.db.read().unwrap().hiatus.clone();
                                self.download_entry(
                                    entry.clone(),
                                    name.clone(),
                                    req_fn,
                                    path,
                                    false,
                                    hiatus_url,
                                );
                            }
                        }
                    }
                    if ui
                        .button(if cfg!(feature = "live") {
                            "Download"
                        } else {
                            "Select"
                        })
                        .on_hover_text(if cfg!(feature = "live") {
                            "Download this clickpack into .zcb/clickpacks"
                        } else {
                            "Download and use this clickpack"
                        })
                        .clicked()
                    {
                        set_status!(DownloadStatus::Downloading);

                        // create dir
                        let mut new_name = name.clone();
                        #[cfg(not(feature = "live"))]
                        let mut path = {
                            #[cfg(not(target_arch = "wasm32"))]
                            let mut path = std::env::temp_dir();
                            #[cfg(target_arch = "wasm32")]
                            let mut path = PathBuf::from("wasm_memory");

                            path.push(TEMP_DIRNAME);
                            path.push(&new_name);
                            path
                        };
                        #[cfg(feature = "live")]
                        let mut path = {
                            let mut path = PathBuf::from(".zcb").join("clickpacks");
                            path.push(&new_name);
                            path
                        };
                        while path.try_exists().unwrap_or(false) {
                            path.pop();
                            new_name += "_";
                            path.push(&new_name);
                        }

                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = std::fs::create_dir_all(&path)
                            .map_err(|e| log::error!("create_dir_all failed: {e}"));

                        // download clickpack zip & extract it
                        let hiatus_url = self.db.read().unwrap().hiatus.clone();
                        self.download_entry(entry.clone(), name, req_fn, path, true, hiatus_url);
                    }
                }
                DownloadStatus::Downloading => {
                    ui.add(egui::Spinner::new());
                    ui.label("Downloading…");
                }
                DownloadStatus::Downloaded {
                    ref path,
                    do_select,
                    #[cfg(target_arch = "wasm32")]
                    ref data,
                } => {
                    ui.style_mut().spacing.item_spacing.x = 5.0;
                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.button("📂 Open folder").clicked() {
                        if let Err(e) = open::that(path) {
                            log::error!("failed to open folder {path:?}: {e}");
                        }
                    }
                    if ui
                        .button("Select")
                        .on_hover_text("Select this clickpack as the current one")
                        .clicked()
                        || (cfg!(not(feature = "live")) && do_select)
                    {
                        if do_select {
                            set_status!(DownloadStatus::Downloaded {
                                path: path.clone(),
                                do_select: false,
                                #[cfg(target_arch = "wasm32")]
                                data: data.clone(),
                            });
                        }
                        log::info!("selecting clickpack {path:?}");
                        self.select_clickpack = Some(path.clone());
                    }
                    ui.style_mut().spacing.item_spacing.x = 5.0;
                    #[cfg(feature = "live")]
                    if ui
                        .button("Delete")
                        .on_hover_text("Delete this clickpack from .zcb/clickpacks")
                        .clicked()
                    {
                        log::info!("enqueuing clickpack {path:?} for deletion");
                        self.pending_clickpack_delete.push(path.clone());
                        set_status!(DownloadStatus::NotDownloaded);
                    }
                }
                DownloadStatus::Error(ref e) => {
                    ui.colored_label(egui::Color32::RED, format!("Error: {e}"));
                }
            }

            ui.label(format_size(entry.size, DECIMAL))
                .on_hover_text(format!(
                    "Uncompressed size: {}",
                    format_size(entry.uncompressed_size, DECIMAL),
                ));
        });
    }
}
