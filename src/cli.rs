use clap::{Parser, ValueEnum};

#[derive(ValueEnum, Debug, Clone)]
enum ArgExprVariable {
    None,
    Variation,
    Value,
    TimeOffset,
}

impl std::fmt::Display for ArgExprVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Run without any arguments to launch GUI.", long_about = None)]
pub(crate) struct Args {
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
    #[arg(long, help = "Audio framerate", default_value_t = 44100)]
    sample_rate: u32,
    #[arg(long, help = "Sort actions by time / frame", default_value_t = true)]
    sort_actions: bool,
    #[arg(long, help = "Volume expression", default_value_t = String::new())]
    volume_expr: String,
    #[arg(long, help = "The variable that the expression should affect", default_value_t = ArgExprVariable::None)]
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
}

/// Run command line interface
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn run_cli(mut args: Args) {
    use bot::*;
    // open replay

    use std::{
        io::BufReader,
        path::{Path, PathBuf},
    };
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
        args.normalize,
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
