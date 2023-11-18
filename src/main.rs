mod gui;
use bot::*;

use clap::{Parser, ValueEnum};
use std::{
    io::Read,
    path::{Path, PathBuf},
};

// load i18n macro
use rust_i18n::i18n;
i18n!("locales");

pub mod built_info {
    // the file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[derive(ValueEnum, Debug, Clone)]
enum ArgExprVariable {
    None,
    Variation,
    Value,
    TimeOffset,
}

impl Into<ExprVariable> for ArgExprVariable {
    fn into(self) -> ExprVariable {
        match self {
            Self::None => ExprVariable::None,
            Self::Variation => ExprVariable::Variation,
            Self::Value => ExprVariable::Value,
            Self::TimeOffset => ExprVariable::TimeOffset,
        }
    }
}

impl ToString for ArgExprVariable {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

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
    #[arg(long, short, help = "Path to output file", default_value_t = String::from("output.wav"))]
    output: String,
    #[arg(
        long,
        help = "Whether to normalize the output audio (make all samples to be in range of 0-1)",
        default_value_t = false
    )]
    normalize: bool,

    #[arg(
        long,
        help = "Whether pitch variation is enabled",
        default_value_t = true
    )]
    pitch_enabled: bool,
    #[arg(long, help = "Minimum pitch value", default_value_t = 0.95)]
    pitch_from: f32,
    #[arg(long, help = "Maximum pitch value", default_value_t = 1.05)]
    pitch_to: f32,
    #[arg(long, help = "Pitch table step", default_value_t = 0.001)]
    pitch_step: f32,

    #[arg(long, help = "Hard click timing", default_value_t = 2.0)]
    hard_timing: f32,
    #[arg(long, help = "Regular click timing", default_value_t = 0.15)]
    regular_timing: f32,
    #[arg(
        long,
        help = "Soft click timing (anything below is microclicks)",
        default_value_t = 0.025
    )]
    soft_timing: f32,

    #[arg(long, help = "Enable spam volume changes", default_value_t = true)]
    vol_enabled: bool,
    #[arg(
        long,
        help = "Time between actions where clicks are considered spam clicks",
        default_value_t = 0.3
    )]
    spam_time: f32,
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
    #[arg(long, help = "Audio framerate", default_value_t = 44100)]
    sample_rate: u32,
    #[arg(long, help = "Sort actions by time / frame", default_value_t = true)]
    sort_actions: bool,
    #[arg(long, help = "Volume expression", default_value_t = String::new())]
    volume_expr: String,
    #[arg(long, help = "The variable that the expression should affect", default_value_t = ArgExprVariable::None)]
    expr_variable: ArgExprVariable,
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
    env_logger::init(); // set envvar RUST_LOG=debug to see logs

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
    }
}

/// Run command line interface
fn run_cli(mut args: Args) {
    // read replay
    let mut f = std::fs::File::open(args.replay.clone()).expect("failed to open replay file");
    let mut data = Vec::new();
    f.read_to_end(&mut data)
        .expect("failed to read replay file");

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
        Pitch::default()
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

    // create bot (loads clickpack)
    let mut bot = Bot::new(PathBuf::from(args.clicks), pitch, args.sample_rate)
        .expect("failed to create bot");

    // parse replay
    // let replay = Macro::parse(
    //     MacroType::guess_format(replay_filename).unwrap(),
    //     &replay,
    //     timings,
    //     vol_settings,
    //     false,
    //     args.sort_actions,
    // )
    let format = ReplayType::guess_format(replay_filename).expect("failed to guess format");
    let replay = Replay::build()
        .with_timings(timings)
        .with_vol_settings(vol_settings)
        .with_extended(true)
        .with_sort_actions(args.sort_actions)
        .parse(format, &data)
        .unwrap();

    // try to compile volume expression to check for errors
    if !args.volume_expr.is_empty() {
        bot.compile_expression(&args.volume_expr)
            .expect("failed to compile volume expression");

        // check for undefined vars
        bot.update_namespace(
            &ExtendedAction::default(),
            replay.last_frame(),
            replay.fps as _,
        );
        bot.eval_expr().expect("failed to evaluate expression");
    }

    // render output file
    let segment = bot.render_replay(
        &replay,
        args.noise,
        args.normalize,
        if !args.volume_expr.is_empty() {
            args.expr_variable.into()
        } else {
            ExprVariable::None
        },
        args.pitch_enabled,
    );

    // save
    if args.output.is_empty() {
        log::warn!("output path is empty, defaulting to 'output.wav'");
        args.output = String::from("output.wav"); // can't save to empty path
    } else if !args.output.ends_with(".wav") {
        log::warn!("output path is not a .wav, however the output format is always .wav");
    }

    let f = std::fs::File::create(args.output).unwrap();
    segment.export_wav(f).unwrap();
}
