# ZCB 3

Free and easy to use Geometry Dash clickbot.

# [Join the discord server (click me)](https://discord.gg/b4kBQyXYZT)

# Supported replay formats

* Mega Hack Replay JSON (.mhr.json)
* Mega Hack Replay Binary (.mhr)
* TASBOT Replay (.json)
* Zbot Replay (.zbf)
* OmegaBot 2 Replay (.replay)
* Ybot Frame (no extension by default, rename to .ybf)
* Echo Binary (.echo)
* Amethyst Replay (.thyst)

Suggest more formats in the [Discord server](https://discord.gg/b4kBQyXYZT)

# Clickpack format

ZCB supports AAC, ADPCM, ALAC, FLAC, MKV, MP1, MP2, MP3, MP4, OGG, Vorbis, WAV, and WebM audio files.

### Clickpacks have to be arranged like this (for two players):

```
.
└── clickpack/
    ├── player1/
    │   ├── 1.mp3 (those can be named anyhow you like)
    │   ├── 2.mp3
    │   └── ...
    ├── player2/
    │   ├── 1.mp3
    │   └── ...
    └── noise.mp3 (optional noise file)
```

### ...or like this (for one player)

(notice no separate player1 and player2 folders)

```
.
└── clickpack/
    ├── 1.mp3 (those can be named anyhow you like)
    ├── 2.mp3
    ├── ...
    └── noise.mp3 (optional noise file)
```

### Note

The `noise.*` file can also be named `whitenoise.*` and it can be also be located in the `player1` or `player2` folder. The clickbot prefers the root clickpack directory rather than player1/player2 folders to search for this file.

# Commandline arguments

To see commandline arguments in your terminal, run `zcb --help`

If you run without any arguments, the GUI will start.

# TODO

* Translate to other languages
* Progress bar for rendering audio and loading clickpacks

# Donations 

ZCB is completely free software. Donations are welcome! :D

By donating you'll get a custom role on the Discord server (dm me) and early access to some of my future mods.

* [Ko-fi](https://ko-fi.com/zeozeozeo)
* [Liberapay](https://liberapay.com/zeo)
  