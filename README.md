# ZCB 3

Free and easy to use Geometry Dash clickbot.

![ZCB 3](https://github.com/zeozeozeo/zcb3/blob/master/screenshots/0.png?raw=true)

![Volume expressions](https://github.com/zeozeozeo/zcb3/raw/master/screenshots/1.png?raw=true)

![ClickpackDB](https://github.com/zeozeozeo/zcb3/raw/master/screenshots/2.png?raw=true)

# [Join the discord server (click me)](https://discord.gg/b4kBQyXYZT)

or the Guilded server: https://guilded.gg/clickbot

# ZCB Live

ZCB also has an in-game version that can be downloaded from the Geode Index: https://geode-sdk.org/mods/zeozeozeo.zcblive

![ZCB Live](/screenshots/live.png)

# Supported replay formats

* Mega Hack Replay JSON (.mhr.json)
* Mega Hack Replay Binary (.mhr)
* TASbot Replay (.json)
* zBot Frame Replay (.zbf)
* OmegaBot 2 Replay (.replay)
* OmegaBot 3 Replay (.replay)
* yBot Frame (no extension by default, rename to .ybf)
* yBot 2 (.ybot)
* Echo (.echo, new binary format, new json format and old json format)
* Amethyst Replay (.thyst)
* osu! replay (.osr)
* GDMO Replay (.macro)
* 2.2 GDMO Replay (.macro, old non-Geode version)
* ReplayBot Replay (.replay)
* KD-BOT Replay (.kd)
* Rush Replay (.rsh)
* Plaintext (.txt)
* GDH Plaintext (.txt)
* ReplayEngine Replay (.re, old and new formats)
* DDHOR Replay (.ddhor, old frame format)
* xBot Frame (.xbot)
* [xdBot (.xd)](https://geode-sdk.org/mods/zilko.xdbot/), old and new formats
* GDReplayFormat (.gdr, used in Geode GDMegaOverlay and 2.2 MH Replay)
* qBot (.qb)
* RBot (.rbot, old and new formats)
* Zephyrus (.zr, used in OpenHack)
* ReplayEngine 2 Replay (.re2)
* Silicate (.slc)

Suggest more formats in the [Discord server](https://discord.gg/b4kBQyXYZT)

# ClickpackDB

A collection of 300+ clickpacks sourced from the [ZCB Discord](https://discord.com/invite/b4kBQyXYZT), accessible from within ZCB.

![ClickpackDB window](https://github.com/zeozeozeo/zcb3/raw/master/screenshots/3.png?raw=true)

* To access ClickpackDB, navigate to:
  
    <kbd>Clickpack</kbd> > <kbd>Open ClickpackDB…</kbd>
* Download clickpacks into a specified folder by clicking <kbd>Download</kbd> next to a clickpack
* Download and select clickpacks by clicking <kbd>Select</kbd>
* Use the searchbar and the <kbd>Tags…</kbd> combobox to filter clickpacks
* Hover on icons next to clickpack names to see their meaning

# Clickpack format

ZCB supports AAC, ADPCM, AIFF, ALAC, CAF, FLAC, MKV, MP1, MP2, MP3, MP4, OGG, Vorbis, WAV, and WebM audio files. (thanks to [Symphonia](https://github.com/pdeljanov/Symphonia))

### Clickpack folder

A clickpack can have `player1`, `player2`, `left1`, `right1`, `left2` and `right2` folders (the last 4 corresponding to platformer left/right directions), which can have `hardclicks`, `hardreleases`, `clicks`, `releases`, `softclicks`, `softreleases`, `microclicks` and `microreleases` folders inside of them (which may have audio files in them). All of the folders are optional, and you don't have to record clicks for both players (but clicks will sound more realistic if you do).

### Noise files

The `noise.*` file can also be named `whitenoise.*` and it can be also be located in the `player1` or `player2` folder. The clickbot prefers the root clickpack directory rather than player1/player2 folders to search for this file.

# Commandline arguments

To see commandline arguments in your terminal, run `zcb --help`

If you run without any arguments, the GUI will start.

# TODO

* Translate to other languages
* Progress bar for rendering audio and loading clickpacks

# Building

To build, clone the repository (or download zip and extract) and run `cargo build` for debug builds and `cargo build --release` for release builds. Note that release builds take a lot of time to build, because they use LTO and they strip debug symbols.

TODO: should we compile with `RUSTFLAGS="--emit=asm"`?

# Donations 

ZCB is completely free software. Donations are welcome! :D

By donating you'll get a custom role on the Discord server (dm me) and early access to some of my future mods.

* [Ko-fi](https://ko-fi.com/zeozeozeo)
* [Liberapay](https://liberapay.com/zeo)
* [DonationAlerts](https://donationalerts.com/r/zeozeozeo)
* [Boosty](https://boosty.to/zeozeozeo/donate)

# License

Boost Software License - Version 1.0 - August 17th, 2003
