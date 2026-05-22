use bot::{
    Action, AudioFile, AudioSegment, Bot, Click, ClickType, Frame, Player, Replay, Timings,
    VolumeSettings,
};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

const SAMPLE_RATE: u32 = 44_100;

fn click_segment() -> AudioSegment {
    let frames = (0..256)
        .map(|i| {
            let gain = 1.0 - (i as f32 / 256.0);
            Frame::new(0.25 * gain, -0.25 * gain)
        })
        .collect();

    AudioSegment {
        sample_rate: SAMPLE_RATE,
        frames,
        pitch_table: Vec::new(),
    }
}

fn bot_with_click() -> Bot {
    let mut bot = Bot::new(SAMPLE_RATE);
    bot.clickpack
        .player1
        .clicks
        .push(AudioFile::new(click_segment(), "click.wav".to_owned()));
    bot.longest_click = 256.0 / SAMPLE_RATE as f64;
    bot
}

fn small_replay() -> Replay {
    let mut replay = Replay::build()
        .with_timings(Timings::default())
        .with_vol_settings(VolumeSettings::default());
    replay.fps = 240.0;

    for i in 0..256 {
        let frame = i * 8;
        replay.actions.push(Action::new(
            frame as f64 / replay.fps,
            Player::One,
            Click::Regular(if i % 2 == 0 {
                ClickType::Click
            } else {
                ClickType::Release
            }),
            0.0,
            frame,
        ));
    }

    replay.duration = replay.actions.last().map(|a| a.time).unwrap_or_default();
    replay
}

fn dense_replay(clicks: usize) -> Replay {
    let mut replay = Replay::build()
        .with_timings(Timings::default())
        .with_vol_settings(VolumeSettings::default());
    replay.fps = 240.0;
    replay.duration = 1.0;
    replay.actions.reserve(clicks);

    for _ in 0..clicks {
        replay.actions.push(Action::new(
            0.5,
            Player::One,
            Click::Regular(ClickType::MicroClick),
            0.0,
            120,
        ));
    }

    replay
}

fn render_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("render");
    group.sample_size(10);

    let small = small_replay();
    group.bench_function("small_macro_256_actions", |b| {
        b.iter(|| {
            let mut bot = bot_with_click();
            black_box(bot.render_replay(
                black_box(&small),
                false,
                0.0,
                false,
                bot::ExprVariable::None,
                false,
                false,
                |_| {},
            ));
        });
    });

    let dense = dense_replay(1_000_000);
    group.bench_function("dense_macro_1m_same_sample_clicks", |b| {
        b.iter(|| {
            let mut bot = bot_with_click();
            black_box(bot.render_replay(
                black_box(&dense),
                false,
                0.0,
                false,
                bot::ExprVariable::None,
                false,
                false,
                |_| {},
            ));
        });
    });

    let expr_replay = small_replay();
    group.bench_function("small_macro_expression", |b| {
        b.iter(|| {
            let mut bot = bot_with_click();
            bot.compile_expression("sin(p) * 0.1 + rand / 100.0")
                .unwrap();
            black_box(bot.render_replay(
                black_box(&expr_replay),
                false,
                0.0,
                false,
                bot::ExprVariable::Value,
                false,
                false,
                |_| {},
            ));
        });
    });

    group.finish();
}

criterion_group!(benches, render_benches);
criterion_main!(benches);
