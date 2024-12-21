mod gui;
mod utils;

#[cfg(not(target_arch = "wasm32"))]
mod cli;

pub mod built_info {
    // the file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[cfg(windows)]
fn hide_console_window() {
    // note that this does not hide the console window when running from a batch file
    let debug_exists = std::path::Path::new("zcb3.debug")
        .try_exists()
        .unwrap_or(false);
    if !debug_exists {
        unsafe { winapi::um::wincon::FreeConsole() };
    }
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init(); // set envvar RUST_LOG=debug to see logs

    // when compiling natively:
    #[cfg(not(target_arch = "wasm32"))]
    {
        if std::env::args().len() > 1 {
            // we have arguments, probably need to run in cli mode
            use clap::Parser;
            let args = cli::Args::parse();
            log::info!("passed args: {args:?} (running in cli mode)");
            cli::run_cli(args);
        } else {
            log::info!("no args, running gui. pass -h or --help to see help");

            // hide console window if running gui
            #[cfg(windows)]
            {
                hide_console_window();
            }

            gui::run_gui().unwrap();
            egui_clickpack_db::cleanup();
        }
    }

    // when compiling to wasm:
    #[cfg(target_arch = "wasm32")]
    gui::run_gui();
}
