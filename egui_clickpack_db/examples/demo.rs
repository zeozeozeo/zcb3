use std::path::PathBuf;

use eframe::egui;
use egui_clickpack_db::ClickpackDb;

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "ClickpackDB",
        options,
        Box::new(|cc| Ok(Box::new(Example::new(cc)))),
    )
    .unwrap();
}

struct Example {
    clickpack_db: ClickpackDb,
}

impl Example {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            clickpack_db: ClickpackDb::default(),
        }
    }
}

const DUMMY_DB: &str = r#"{
    "updated_at_iso": "2025-05-16T11:00:22.442168+00:00",
    "updated_at_unix": 1747393222,
    "clickpacks": {
        "!dummy": {
            "size": 1,
            "uncompressed_size": 1,
            "has_noise": false,
            "url": "https://github.com/zeozeozeo/clickpack-db/raw/main/out/dummy.zip",
            "checksum": "62c4fdecd69d23ca70726869cf16a677"
        },
        "0Pacitys_Clicks_V2": {
            "size": 101489,
            "uncompressed_size": 99771,
            "has_noise": false,
            "url": "https://github.com/zeozeozeo/clickpack-db/raw/main/out/0Pacitys_Clicks_V2.zip",
            "checksum": "62c4fdecd69d23ca70726869cf16a677"
        }
    },
    "version": 46,
    "hiatus": "https://hiatus.zeo.lol"
}"#;

fn req_fn(url: &str, post: bool) -> Result<Vec<u8>, String> {
    println!("requesting {url}, post: {post}");
    if url == "https://raw.githubusercontent.com/zeozeozeo/clickpack-db/main/db.json" {
        Ok(DUMMY_DB.as_bytes().to_vec())
    } else if url == "https://hiatus.zeo.lol/downloads/all" {
        Ok(r#"{"0Pacitys_Clicks_V2":6969,"!dummy":69}"#.as_bytes().to_vec())
    } else {
        Err("404".to_string())
    }
}

fn dummy_pick_folder() -> Option<PathBuf> {
    Some(PathBuf::from("/tmp"))
}

impl eframe::App for Example {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.clickpack_db.show(ui, &req_fn, &dummy_pick_folder);
        });
    }
}
