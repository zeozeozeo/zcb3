mod gui;
mod utils;

#[cfg(target_arch = "wasm32")]
mod bundler;

#[cfg(not(target_arch = "wasm32"))]
use clap::{Parser, ValueEnum};

#[cfg(all(not(target_env = "musl"), not(target_arch = "wasm32")))]
// Added this cfg for BEMalloc
use malloc_best_effort::BEMalloc;

#[cfg(all(not(target_env = "musl"), not(target_arch = "wasm32")))]
#[global_allocator]
static GLOBAL: BEMalloc = BEMalloc::new(); // tcmalloc on linux/mac, mimalloc on winders

pub mod built_info {
    // the file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(ValueEnum, Debug, Clone)]
enum ArgExprVariable {
    None,
    Variation,
    Value,
    TimeOffset,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(ValueEnum, Debug, Clone, PartialEq)]
pub enum ArgRenderPostprocessType {
    /// Save the audio file as-is (ZCB).
    None,
    /// Normalize the audio file.
    Normalize,
    /// Clamp samples to `[-1.0, 1.0]` (ACB).
    Clamp,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Parser, Debug)]
#[command(author, version, about = "Run without any arguments to launch GUI.", long_about = None)]
struct Args {
    #[arg(long, help = "Path to replay file")]
    replay: String,
    #[arg(long, help = "Path to clickpack folder")]
    clicks: String,
    #[arg(
        long,
        help = "Whether to overlay the noise.* file in the clickpack directory",
        default_value_t = false
    )]
    noise: bool,
    #[arg(long, help = "Noise volume multiplier", default_value_t = 1.0)]
    noise_volume: f32,
    #[arg(long, short, help = "Path to output file", default_value_t = String::from("output.wav"))]
    output: String,
    #[arg(long, help = "Audio postprocessing type", default_value_t = ArgRenderPostprocessType::None, value_enum)]
    postprocess_type: ArgRenderPostprocessType,

    #[arg(
        long,
        help = "Whether pitch variation is enabled",
        default_value_t = true
    )]
    pitch_enabled: bool,

    #[arg(long, help = "Minimum pitch value", default_value_t = 0.98)]
    pitch_from: f32,
    #[arg(long, help = "Maximum pitch value", default_value_t = 1.02)]
    pitch_to: f32,
    #[arg(long, help = "Pitch table step", default_value_t = 0.0005)]
    pitch_step: f32,

    #[arg(long, help = "Hard click timing", default_value_t = 2.0)]
    hard_timing: f64,
    #[arg(long, help = "Regular click timing", default_value_t = 0.15)]
    regular_timing: f64,
    #[arg(
        long,
        help = "Soft click timing (anything below is microclicks)",
        default_value_t = 0.025
    )]
    soft_timing: f64,

    #[arg(long, help = "Enable spam volume changes", default_value_t = true)]
    vol_enabled: bool,
    #[arg(
        long,
        help = "Time between actions where clicks are considered spam clicks",
        default_value_t = 0.3
    )]
    spam_time: f64,
    #[arg(
        long,
        help = "The spam volume offset is multiplied by this value",
        default_value_t = 0.9
    )]
    spam_vol_offset_factor: f32,
    #[arg(
        long,
        help = "The spam volume offset is clamped by this value",
        default_value_t = 0.3
    )]
    max_spam_vol_offset: f32,
    #[arg(
        long,
        help = "Enable changing volume of release sounds",
        default_value_t = false
    )]
    change_releases_volume: bool,
    #[arg(long, help = "Global clickbot volume factor", default_value_t = 1.0)]
    global_volume: f32,

    #[arg(
        long,
        help = "Random variation in volume (+/-) for each click",
        default_value_t = 0.2
    )]
    volume_var: f32,
    #[arg(long, help = "Audio framerate", default_value_t = 48000)]
    sample_rate: u32,
    #[arg(long, help = "Sort actions by time / frame", default_value_t = true)]
    sort_actions: bool,
    #[arg(long, help = "Volume expression", default_value_t = String::new())]
    volume_expr: String,
    #[arg(long, help = "The variable that the expression should affect", default_value_t = ArgExprVariable::None, value_enum)]
    expr_variable: ArgExprVariable,
    #[arg(
        long,
        help = "Extend the variation range to negative numbers. Only works for variation",
        default_value_t = true
    )]
    expr_negative: bool,
    #[arg(
        long,
        help = "Cut overlapping sounds. Changes the sound significantly in spams",
        default_value_t = false
    )]
    cut_sounds: bool,
    #[arg(
        long,
        help = "Whether to start rendering from the first action after the last death. Only applies to GDReplayFormat 2 and Silicate 2",
        default_value_t = true
    )]
    discard_deaths: bool,
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
    #[cfg(all(not(target_env = "musl"), not(target_arch = "wasm32")))]
    {
        BEMalloc::init();
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .init();
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if std::env::args().len() > 1 {
            // we have arguments, probably need to run in cli mode
            let args = Args::parse();
            log::info!("passed args: {args:?} (running in cli mode)");
            run_cli(args);
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

    #[cfg(target_arch = "wasm32")]
    {
        // Set up WASM-specific initialization
        utils::setup_wasm();

        // Run GUI in WASM mode
        wasm_bindgen_futures::spawn_local(async {
            gui::run_gui_wasm("main_canvas")
                .await
                .expect("failed to start eframe");
        });
    }
}

/// Run command line interface
#[cfg(not(target_arch = "wasm32"))]
fn run_cli(mut args: Args) {
    use bot::*;
    use std::io::BufReader;
    use std::path::{Path, PathBuf};

    // open replay
    let f = std::fs::File::open(args.replay.clone()).expect("failed to open replay file");

    let replay_filename = Path::new(&args.replay)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();

    let pitch = if args.pitch_enabled {
        Pitch {
            from: args.pitch_from,
            to: args.pitch_to,
            step: args.pitch_step,
        }
    } else {
        Pitch::NO_PITCH
    };

    let timings = Timings {
        hard: args.hard_timing,
        regular: args.regular_timing,
        soft: args.soft_timing,
    };

    let vol_settings = VolumeSettings {
        enabled: args.vol_enabled,
        spam_time: args.spam_time,
        spam_vol_offset_factor: args.spam_vol_offset_factor,
        max_spam_vol_offset: args.max_spam_vol_offset,
        change_releases_volume: args.change_releases_volume,
        global_volume: args.global_volume,
        volume_var: args.volume_var,
    };

    // create bot and load clickpack
    let mut bot = Bot::new(args.sample_rate);
    bot.load_clickpack(&PathBuf::from(args.clicks), pitch)
        .expect("failed to load clickpack");

    // parse replay
    let format = ReplayType::guess_format(replay_filename).expect("failed to guess format");
    let replay = Replay::build()
        .with_timings(timings)
        .with_vol_settings(vol_settings)
        .with_extended(true)
        .with_sort_actions(args.sort_actions)
        .with_discard_deaths(args.discard_deaths)
        .parse(format, BufReader::new(f))
        .unwrap();

    // try to compile volume expression to check for errors
    if !args.volume_expr.is_empty() {
        bot.compile_expression(&args.volume_expr)
            .expect("failed to compile volume expression");

        // check for undefined vars
        bot.update_namespace(
            &ExtendedAction::default(),
            0,
            replay.last_frame(),
            replay.fps as _,
        );
        bot.eval_expr().expect("failed to evaluate expression");
    }

    // render output file
    let segment = bot.render_replay(
        &replay,
        args.noise,
        args.noise_volume,
        args.postprocess_type == ArgRenderPostprocessType::Normalize,
        if !args.volume_expr.is_empty() {
            match args.expr_variable {
                ArgExprVariable::None => ExprVariable::None,
                ArgExprVariable::Value => ExprVariable::Value,
                ArgExprVariable::TimeOffset => ExprVariable::TimeOffset,
                ArgExprVariable::Variation => ExprVariable::Variation {
                    negative: args.expr_negative,
                },
            }
        } else {
            ExprVariable::None
        },
        args.pitch_enabled,
        args.cut_sounds,
        |p| {
            if ((p * 100.0) as u32).is_multiple_of(10) {
                log::info!("Rendering progress: {}%", (p * 100.0) as u32);
            }
        },
    );

    // save
    if args.output.is_empty() {
        log::warn!("output path is empty, defaulting to 'output.wav'");
        args.output = String::from("output.wav"); // can't save to empty path
    } else if !args.output.ends_with(".wav") {
        log::warn!("output path is not a .wav, however the output format is always .wav");
    }

    let f = std::fs::File::create(args.output).unwrap();
    segment
        .export_wav(f, args.postprocess_type == ArgRenderPostprocessType::Clamp)
        .unwrap();
}
