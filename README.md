# [ZCB 3](https://zeozeozeo.github.io/zcb3) | [🚀 Launch](https://zeozeozeo.github.io/zcb3)

Free and easy to use Geometry Dash clickbot.

[🚀 Use in the browser, without downloading](https://zeozeozeo.github.io/zcb3)

![ZCB 3](https://github.com/zeozeozeo/zcb3/blob/main/screenshots/0.png?raw=true)

![Volume expressions](https://github.com/zeozeozeo/zcb3/raw/main/screenshots/1.png?raw=true)

![ClickpackDB](https://github.com/zeozeozeo/zcb3/raw/main/screenshots/2.png?raw=true)

## [Join the discord server (click me)](https://discord.gg/b4kBQyXYZT)

ZCB is also available on web (both desktop & mobile)! [click me](https://zeozeozeo.github.io/zcb3)

## ZCB Live

ZCB also has an in-game version that can be downloaded from the Geode Index: https://geode-sdk.org/mods/zeozeozeo.zcblive

![ZCB Live](/screenshots/live.png)

## Supported replay formats

- Mega Hack Replay JSON (.mhr.json)
- Mega Hack Replay Binary (.mhr)
- TASbot Replay (.json)
- zBot Frame Replay (.zbf)
- OmegaBot 2 Replay (.replay)
- OmegaBot 3 Replay (.replay)
- yBot Frame (no extension by default, rename to .ybf)
- yBot 2 (.ybot)
- Echo (.echo, new binary format, new json format and old json format)
- Amethyst Replay (.thyst)
- osu! replay (.osr)
- GDMO Replay (.macro)
- 2.2 GDMO Replay (.macro, old non-Geode version)
- ReplayBot Replay (.replay)
- KD-BOT Replay (.kd)
- Rush Replay (.rsh)
- Plaintext (.txt)
- GDH Plaintext (.txt)
- DDHOR Replay (.ddhor, old frame format)
- xBot Frame (.xbot)
- [xdBot (.xd)](https://geode-sdk.org/mods/zilko.xdbot/), old and new formats
- GDReplayFormat (.gdr, used in Geode GDMegaOverlay and 2.2 MH Replay)
- qBot (.qb)
- RBot (.rbot, old and new formats)
- Zephyrus (.zr, used in OpenHack)
- ReplayEngine 1 Replay (.re, old and new formats)
- ReplayEngine 2 Replay (.re2)
- ReplayEngine 3 Replay (.re3)
- Silicate (.slc)
- Silicate 2 (.slc)
- Silicate 3 (.slc)
- GDReplayFormat 2 (.gdr2)
- [uvBot (.uv)](https://github.com/thisisignitedoreo/uvbot), thanks @thisisignitedoreo
- TCBot (.tcm)

\+ a replay converter from any format to any other supported format (currently only in the [web version](https://zeozeozeo.github.io/zcb3))

Suggest more formats in the [Discord server](https://discord.gg/b4kBQyXYZT)

## [ClickpackDB](https://zeozeozeo.github.io/clickpack-db)

A collection of 700+ clickpacks sourced from the [ZCB Discord](https://discord.com/invite/b4kBQyXYZT), accessible from within ZCB.

![ClickpackDB window](https://github.com/zeozeozeo/zcb3/raw/main/screenshots/3.png?raw=true)

- To access ClickpackDB, navigate to:

  <kbd>Clickpack</kbd> > <kbd>Open ClickpackDB…</kbd>

- Download clickpacks into a specified folder by clicking <kbd>Download</kbd> next to a clickpack
- Download and select clickpacks by clicking <kbd>Select</kbd>
- Use the searchbar and the <kbd>Tags…</kbd> combobox to filter clickpacks
- Hover on icons next to clickpack names to see their meaning

Or use the web version: [click me](https://zeozeozeo.github.io/clickpack-db)

## Clickpack format

ZCB supports AAC, ADPCM, AIFF, ALAC, CAF, FLAC, MKV, MP1, MP2, MP3, MP4, OGG, Vorbis, WAV, and WebM audio files. (thanks to [Symphonia](https://github.com/pdeljanov/Symphonia))

### Clickpack folder

A clickpack can have `player1`, `player2`, `left1`, `right1`, `left2` and `right2` folders (the last 4 corresponding to platformer left/right directions), which can have `hardclicks`, `hardreleases`, `clicks`, `releases`, `softclicks`, `softreleases`, `microclicks` and `microreleases` folders inside of them (which may have audio files in them). All of the folders are optional, and you don't have to record clicks for both players (but clicks will sound more realistic if you do).

### Noise files

The `noise.*` file can also be named `whitenoise.*` and it can be also be located in the `player1` or `player2` folder. The clickbot prefers the root clickpack directory rather than player1/player2 folders to search for this file.

### Commandline arguments

To see commandline arguments in your terminal, run `zcb --help`

If you run without any arguments, the GUI will start.

## Building

To build, clone the repository (or download zip and extract) and run `cargo build` for debug builds and `cargo build --release` for release builds.

Use `trunk build --release` for web builds.

## License

Public domain (The Unlicense)
