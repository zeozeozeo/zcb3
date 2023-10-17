extern crate winres;

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("src/assets/icon.ico");
        res.compile().unwrap();
    }

    built::write_built_file().expect("failed to acquire build-time information");
}
