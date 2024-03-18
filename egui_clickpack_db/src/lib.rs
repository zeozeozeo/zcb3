use egui_extras::{Column, TableBuilder};
use fuzzy_matcher::FuzzyMatcher;
use indexmap::IndexMap;
use std::{
    io::Cursor,
    path::PathBuf,
    sync::{Arc, RwLock},
};

const DATABASE_URL: &str = "https://raw.githubusercontent.com/zeozeozeo/clickpack-db/main/db.json";

type RequestFn = dyn Fn(&str) -> Result<Vec<u8>, String> + Sync;
type PickFolderFn = dyn Fn() -> Option<PathBuf> + Sync;

#[derive(Clone, Default)]
enum DownloadStatus {
    #[default]
    NotDownloaded,
    Downloading,
    Downloaded {
        path: PathBuf,
        do_select: bool,
    },
    Error(String),
}

#[derive(serde::Deserialize, Default)]
pub struct Database {
    pub updated_at_unix: i64,
    #[serde(rename = "clickpacks")]
    pub entries: IndexMap<String, Entry>,
}

#[derive(serde::Deserialize, Clone)]
pub struct Entry {
    // size: usize,
    // uncompressed_size: usize,
    has_noise: bool,
    url: String,
    #[serde(skip_deserializing)]
    dwn_status: DownloadStatus,
}

#[derive(Default, Clone)]
enum Status {
    #[default]
    NotLoaded,
    Loading,
    Error(String),
    Loaded {
        did_filter: bool,
    },
}

#[derive(Default)]
pub struct ClickpackDb {
    status: Arc<RwLock<Status>>,
    pub db: Arc<RwLock<Database>>,
    filtered_entries: IndexMap<String, Entry>,
    search_query: String,
    pending_update: Arc<RwLock<IndexMap<String, Entry>>>,
    /// If [`Some`], this clickpack should be selected and the viewport should be closed.
    pub select_clickpack: Option<PathBuf>,
}

pub fn cleanup() {
    log::info!("cleaning up temp directories...");
    let mut temp_dir = std::env::temp_dir();
    if temp_dir.try_exists().unwrap_or(false) {
        temp_dir.push("zcb-clickpackdb");
        if temp_dir.try_exists().unwrap_or(false) {
            let _ = std::fs::remove_dir_all(temp_dir)
                .map_err(|e| log::error!("remove_dir_all failed: {e}"));
        }
    };
}

impl ClickpackDb {
    fn load_database(
        status: Arc<RwLock<Status>>,
        db: Arc<RwLock<Database>>,
        req_fn: &'static RequestFn,
    ) {
        log::info!("loading database from {DATABASE_URL}");
        std::thread::spawn(move || match req_fn(DATABASE_URL) {
            Ok(body) => {
                *db.write().unwrap() = match serde_json::from_slice(&body) {
                    Ok(entries) => entries,
                    Err(e) => {
                        log::error!("failed to parse database: {e}");
                        *status.write().unwrap() = Status::Error(e.to_string());
                        return;
                    }
                };
                log::info!("loaded {} entries", db.read().unwrap().entries.len());
                *status.write().unwrap() = Status::Loaded { did_filter: false };
            }
            Err(e) => {
                log::error!("failed to GET database: {e}");
                *status.write().unwrap() = Status::Error(e.to_string());
            }
        });
    }

    fn update_filtered_entries(&mut self) {
        self.filtered_entries = self.db.read().unwrap().entries.clone();
        if !self.search_query.is_empty() {
            let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();

            self.filtered_entries.sort_by_cached_key(|k, _| {
                std::cmp::Reverse(matcher.fuzzy_match(k, &self.search_query).unwrap_or(0))
            });
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
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        req_fn: &'static RequestFn,
        pick_folder: &'static PickFolderFn,
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
                    *self.status.write().unwrap() = Status::Loaded { did_filter: true };
                }
            }
        }
        self.update_pending_update();
        ui.add_enabled_ui(
            !matches!(status, Status::NotLoaded | Status::Loading),
            |ui| {
                self.show_table(ui, req_fn, pick_folder);
            },
        );
    }

    fn download_entry(
        &mut self,
        mut entry: Entry,
        name: String,
        req_fn: &'static RequestFn,
        mut path: PathBuf,
        do_select: bool,
    ) {
        log::info!("downloading entry \"{name}\" to path {path:?}");
        let pending_update = self.pending_update.clone();
        path.push(&name);
        std::thread::spawn(move || {
            match req_fn(&entry.url) {
                Ok(body) => {
                    log::debug!("body length: {} bytes, extracting zip", body.len());
                    if let Err(e) = zip_extract::extract(Cursor::new(body), &path, true) {
                        log::error!("failed to extract zip to {path:?}: {e}");
                        entry.dwn_status = DownloadStatus::Error(e.to_string());
                    } else {
                        log::info!("successfully extracted zip to {path:?}");
                        entry.dwn_status = DownloadStatus::Downloaded { path, do_select };
                    }
                }
                Err(e) => {
                    entry.dwn_status = DownloadStatus::Error(e);
                }
            }
            pending_update.write().unwrap().insert(name, entry);
        });
    }

    fn show_table(
        &mut self,
        ui: &mut egui::Ui,
        req_fn: &'static RequestFn,
        pick_folder: &'static PickFolderFn,
    ) {
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);

        TableBuilder::new(ui)
            .column(Column::exact(200.0))
            .column(Column::remainder())
            .striped(true)
            .header(30.0, |mut header| {
                header.col(|ui| {
                    // ui.heading("Name");
                    ui.add_space(1.0);
                    let nr_clickpacks = self.db.read().unwrap().entries.len();
                    let textedit = egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text(format!("🔎 Search in {nr_clickpacks} clickpacks"));
                    if ui.add(textedit).changed() {
                        self.update_filtered_entries();
                    }
                });
                header.col(|ui| {
                    ui.with_layout(
                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            ui.heading("Manage");
                        },
                    );
                });
            })
            .body(|body| {
                body.rows(text_height * 1.5, self.filtered_entries.len(), |mut row| {
                    let row_index = row.index();
                    let entry = self.filtered_entries.get_index(row_index).unwrap();
                    let name = entry.0.clone();
                    let entry = entry.1.clone();
                    row.col(|ui| {
                        ui.horizontal(|ui| {
                            ui.style_mut().spacing.item_spacing.x = 5.0;
                            ui.add(egui::Label::new(name.replace('_', " ")).wrap(true));
                            ui.style_mut().spacing.item_spacing.x = 5.0;
                            if entry.has_noise {
                                ui.colored_label(egui::Color32::KHAKI, "🎧")
                                    .on_hover_text("This clickpack has a noise file")
                                    .on_hover_cursor(egui::CursorIcon::Default);
                            }
                            if matches!(entry.dwn_status, DownloadStatus::Downloaded { .. }) {
                                ui.colored_label(egui::Color32::LIGHT_GREEN, "✅")
                                    .on_hover_text("Downloaded")
                                    .on_hover_cursor(egui::CursorIcon::Default);
                            }
                        });
                    });
                    row.col(|ui| self.manage_row(ui, entry, name, req_fn, row_index, pick_folder));
                });
            });
    }

    fn manage_row(
        &mut self,
        ui: &mut egui::Ui,
        entry: Entry,
        name: String,
        req_fn: &'static RequestFn,
        row_index: usize,
        pick_folder: &'static PickFolderFn,
    ) {
        let set_status = |filtered_entries: &mut IndexMap<String, Entry>,
                          status: DownloadStatus| {
            filtered_entries
                .get_index_mut(row_index)
                .unwrap()
                .1
                .dwn_status = status;
        };

        ui.horizontal_centered(|ui| {
            match entry.dwn_status {
                DownloadStatus::NotDownloaded => {
                    ui.style_mut().spacing.item_spacing.x = 5.0;
                    if ui
                        .button("Download")
                        .on_hover_text("Download this clickpack into a new folder")
                        .clicked()
                    {
                        if let Some(path) = pick_folder() {
                            set_status(&mut self.filtered_entries, DownloadStatus::Downloading);
                            self.download_entry(entry.clone(), name.clone(), req_fn, path, false);
                        }
                    }
                    if ui
                        .button("Select")
                        .on_hover_text("Download and use this clickpack")
                        .clicked()
                    {
                        set_status(&mut self.filtered_entries, DownloadStatus::Downloading);

                        // create temp dir
                        let mut path = std::env::temp_dir();
                        let mut new_name = name.clone();
                        path.push("zcb-clickpackdb");
                        path.push(&new_name);
                        while path.try_exists().unwrap_or(false) {
                            path.pop();
                            new_name += "_";
                            path.push(&new_name);
                        }

                        let _ = std::fs::create_dir_all(&path)
                            .map_err(|e| log::error!("create_dir_all failed: {e}"));

                        self.download_entry(entry, name, req_fn, path, true);
                    }
                }
                DownloadStatus::Downloading => {
                    ui.add(egui::Spinner::new());
                    ui.label("Downloading…");
                }
                DownloadStatus::Downloaded {
                    ref path,
                    do_select,
                } => {
                    ui.style_mut().spacing.item_spacing.x = 5.0;
                    if ui.button("Open folder").clicked() {
                        if let Err(e) = open::that(path) {
                            log::error!("failed to open folder {path:?}: {e}");
                        }
                    }
                    if ui.button("Select").clicked() || do_select {
                        if do_select {
                            set_status(
                                &mut self.filtered_entries,
                                DownloadStatus::Downloaded {
                                    path: path.clone(),
                                    do_select: false,
                                },
                            );
                        }
                        self.select_clickpack = Some(path.clone());
                    }
                }
                DownloadStatus::Error(ref e) => {
                    ui.colored_label(egui::Color32::RED, format!("Error: {e}"));
                }
            }
        });
    }
}
