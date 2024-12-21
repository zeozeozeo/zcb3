# ZCB 3

Free and easy to use Geometry Dash clickbot.

![ZCB 3](https://github.com/zeozeozeo/zcb3/blob/master/screenshots/0.png?raw=true)

![Volume expressions](https://github.com/zeozeozeo/zcb3/raw/master/screenshots/1.png?raw=true)

![ClickpackDB](https://github.com/zeozeozeo/zcb3/raw/master/screenshots/2.png?raw=true)

# [Join the discord server (click me)](https://discord.gg/b4kBQyXYZT)

or the Guilded server: https://guilded.gg/clickbot

![](https://www.guilded.gg/canvas_index.html?route=%2Fcanvas%2Fembed%2Fteamcard%2Fjb721qzR&size=large)

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

# GUI note

Note that your GPU must support OpenGL 3.3 in order to run the GUI natively. If that is not the case, you are currently forced to use the web version. (TODO: a software rendering backend would be nice)

# Commandline arguments

To see commandline arguments in your terminal, run `zcb3 --help`

If you run without any arguments, the GUI will start.

```
Run without any arguments to launch GUI.

Usage: zcb3.exe [OPTIONS] --replay <REPLAY> --clicks <CLICKS>

Options:
      --replay <REPLAY>
          Path to replay file
      --clicks <CLICKS>
          Path to clickpack folder
      --noise
          Whether to overlay the noise.* file in the clickpack directory
      --noise-volume <NOISE_VOLUME>
          Noise volume multiplier [default: 1]
  -o, --output <OUTPUT>
          Path to output file [default: output.wav]
      --normalize
          Whether to normalize the output audio (make all samples to be in range of 0-1)
      --pitch-enabled
          Whether pitch variation is enabled
      --pitch-from <PITCH_FROM>
          Minimum pitch value [default: 0.98]
      --pitch-to <PITCH_TO>
          Maximum pitch value [default: 1.02]
      --pitch-step <PITCH_STEP>
          Pitch table step [default: 0.0005]
      --hard-timing <HARD_TIMING>
          Hard click timing [default: 2]
      --regular-timing <REGULAR_TIMING>
          Regular click timing [default: 0.15]
      --soft-timing <SOFT_TIMING>
          Soft click timing (anything below is microclicks) [default: 0.025]
      --vol-enabled
          Enable spam volume changes
      --spam-time <SPAM_TIME>
          Time between actions where clicks are considered spam clicks [default: 0.3]
      --spam-vol-offset-factor <SPAM_VOL_OFFSET_FACTOR>
          The spam volume offset is multiplied by this value [default: 0.9]
      --max-spam-vol-offset <MAX_SPAM_VOL_OFFSET>
          The spam volume offset is clamped by this value [default: 0.3]
      --change-releases-volume
          Enable changing volume of release sounds
      --global-volume <GLOBAL_VOLUME>
          Global clickbot volume factor [default: 1]
      --volume-var <VOLUME_VAR>
          Random variation in volume (+/-) for each click [default: 0.2]
      --sample-rate <SAMPLE_RATE>
          Audio framerate [default: 44100]
      --sort-actions
          Sort actions by time / frame
      --volume-expr <VOLUME_EXPR>
          Volume expression [default: ]
      --expr-variable <EXPR_VARIABLE>
          The variable that the expression should affect [default: None] [possible values: none, variation, value, time-offset]
      --expr-negative
          Extend the variation range to negative numbers. Only works for variation
      --cut-sounds
          Cut overlapping sounds. Changes the sound significantly in spams
  -h, --help
          Print help
  -V, --version
          Print version
```

# TODO

* Translate to other languages
* Progress bar for rendering audio and loading clickpacks

# Building

To build, clone the repository (or download zip and extract) and run `cargo build` for debug builds and `cargo build --release` for release builds. Note that release builds take a lot of time to build, because they use LTO and they strip debug symbols.

### Building for WebAssembly

ZCB can be compiled for WASM and be deployed on a web page.

1. Install the `wasm32-unknown-unknown` target with `rustup target add wasm32-unknown-unknown`
2. Install [Trunk](https://trunkrs.dev/) with `cargo install trunk`. Note: if it fails to find OpenSSL, run `cargo install cargo-binstall` and then `cargo binstall trunk` as a temporary workaround.

Now, if you want to develop locally in a web browser with auto-reloading:

1. Run `trunk serve` to serve on `http://127.0.0.1:8080`. Trunk will rebuild the project automatically.
2. Open `http://127.0.0.1:8080/index.html#dev` in your browser. The `#dev` suffix will prevent the service worker from caching our app. It is needed for ZCB to work offline, kind of like PWA.

Or, if you want to deploy:

1. Run `trunk build --release`
2. There should now be a `dist` directory with the static HTML website.

# Donations 

ZCB is completely free software. Donations are welcome! :D

By donating you'll get a custom role on the Discord server (dm me) and early access to some of my future mods.

* [Ko-fi](https://ko-fi.com/zeozeozeo)
* [Liberapay](https://liberapay.com/zeo)
* [DonationAlerts](https://donationalerts.com/r/zeozeozeo)
* [Boosty](https://boosty.to/zeozeozeo/donate)

# License

Boost Software License - Version 1.0 - August 17th, 2003
