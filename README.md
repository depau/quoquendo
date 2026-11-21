<h1><picture><source media="(prefers-color-scheme: dark)" srcset="assets/brand/logo.svg"><img src="assets/brand/logo-light.svg" alt="quoquendo" height="72"></picture></h1>

The Loquendo 7 (LTTS7) text-to-speech engine rebuilt from its own code, for
its original voices: Roberto (the Italian station voice), Allison, Bernard,
Katrin and the rest. No Loquendo code runs; the voices' files are read as
data. How each stage works, and where every number comes from, is in
[MECHANICS.md](MECHANICS.md).

## Get the app

- **Browser**: <https://quoquendo.depau.eu>
- **Android**: the [nightly release][NIGHTLY], rebuilt on every push to
  `main`; Obtainium tracks it:

  [![Add to Obtainium](https://raw.githubusercontent.com/NavicApp/Branding/refs/heads/main/assets/obtainium.svg)][ADD_NIGHTLY_TO_OBTAINIUM]
- **Desktop** (Linux, Windows, macOS): build it with Rust (on Linux with the
  ALSA headers, `libasound2-dev` / `alsa-lib`):

  ```sh
  git clone https://github.com/depau/quoquendo && cd quoquendo
  cargo run --release
  ```

[NIGHTLY]: https://github.com/depau/quoquendo/releases/tag/nightly
[ADD_NIGHTLY_TO_OBTAINIUM]: https://apps.obtainium.imranr.dev/redirect?r=obtainium://app/%7B%22id%22%3A%22eu.depau.quoquendo.nightly%22%2C%22url%22%3A%22https%3A%2F%2Fgithub.com%2Fdepau%2Fquoquendo%22%2C%22author%22%3A%22depau%22%2C%22name%22%3A%22Quoquendo%20Nightly%22%2C%22additionalSettings%22%3A%22%7B%5C%22includePrereleases%5C%22%3Atrue%2C%5C%22fallbackToOlderReleases%5C%22%3Atrue%2C%5C%22verifyLatestTag%5C%22%3Afalse%2C%5C%22sortMethodChoice%5C%22%3A%5C%22date%5C%22%2C%5C%22versionDetection%5C%22%3Afalse%2C%5C%22releaseDateAsVersion%5C%22%3Atrue%2C%5C%22apkFilterRegEx%5C%22%3A%5C%22%5Equoquendo-nightly-.%2A%5C%5C%5C%5C.apk%24%5C%22%7D%22%7D

## Getting a voice

The voices' installers (`<Voice> - <Language>.exe`, e.g. `Raffaele - Italian.exe`
for Roberto) are all it needs; quoquendo reads them itself, Inno Setup and
Wise alike (nothing is run, no other program is needed):

```sh
quoquendo --install "Raffaele - Italian.exe"     # or a folder of installers
quoquendo --unpack <installer|folder> <dir>       # the same into any folder
```

This keeps only the voice's files and its language's in
`~/.local/share/quoquendo/LTTS7` (`%APPDATA%` on Windows). The app's Voices
tab does the same, and downloads any of the pack's 47 installers from
archive.org, or their files from a voice mirror ([mirror/](mirror/README.md)).

Of the 47 voices, Jordi (Catalan), Empar (Valencian), Carmela (Galician) and
LinLin (Chinese) can't be read properly yet: no installer carries their
language module.

## The app

```sh
cargo run --release                               # the window
cargo run --release -- --advanced                 # in Advanced mode
cargo run --release -- --original                 # in Original engine (Wine) mode
quoquendo --say "Attenzione! Allontanarsi dalla linea gialla." --voice Roberto --out out.flac
quoquendo --selftest
```

The window has a Speak tab and a Voices tab. Speak shows what the engine
makes of the text while you type: its sounds (green when the voice recorded
them, red when it never did), the recorded pieces chosen for them, each
playable on its own, and the waveform with the sounds under it. Speak plays
it, lighting the word, the sound and the piece being said. Save writes WAV,
FLAC (the default), MP3 or Opus; each file carries the text with each
word's time: cue markers in WAV, synced lyrics (LRC in `LYRICS`/`USLT`, and
`SYLT` in MP3) in the others. `--say` picks the format by the file's
extension. The Voices tab is the library: Use picks a voice, and Explore
opens every recorded take of every phone, and every recorded sentence.

Two modes, in Settings, are off by default:

- **Advanced**: the voice's details, its inventory and its language data;
  reading the text with another language's rules; pronunciation from a
  command (desktop: any program that prints a transcription); and joining the
  pieces like the original or smoothed.
- **Original engine (Wine)** (desktop): the original Loquendo engine run under
  Wine as a separate program for the pronunciation or the piece picking, its
  audio beside ours on one time axis (each with its own play button;
  switching plays the other from the same moment), and Compare with the
  original, which scores each stage against it. It needs the setup in
  [The original engine](#the-original-engine).

`--screenshot <file.png>` draws the window once everything is ready, saves
it and quits (with `--play-at <seconds>`, showing the speech at that moment
as if playing); add `--advanced` or `--original` for those modes.

### In the browser

```sh
trunk serve --release                             # http://127.0.0.1:8080
```

The same app; voices live in the browser's own storage. Browsers can't fetch
the installers from archive.org (it sends no CORS header for `.exe` files), so
downloads go through the voice mirror: the site the app is served from, or
the one in Settings. Installers downloaded by hand can always be imported.
The app and the voices are served by one Cloudflare Worker over one R2
bucket, deployed from `main` (`.github/workflows/site.yml`; setup in
`mirror/README.md`).

The address bar follows the page, so it can be shared and back/forward work:
`#/speak/Roberto` (with `?text=…` when the text isn't the voice's sample),
`#/voices`, and `#/explore/Roberto` (with `list=phones`, `phone=a`,
`sentence=12`; the recorded sentences are listed unless `list=phones`). A
link to a voice that isn't installed offers to download it.

### On Android

`android/` is a system text-to-speech engine: install it, pick Quoquendo in
the system's TTS settings, and get voices in its Voices screen. Try it shows
the sounds, recorded pieces and waveform of the text as you type, follows
them word by word while it speaks, and saves WAV, FLAC, MP3 or Opus files
with the text and its timing embedded. Apps that read aloud get the word
being spoken too. Settings chooses the voice apps get for each language;
Advanced adds the reading language, the voice's details and how pieces are
joined.

The nightly is `eu.depau.quoquendo.nightly`; Obtainium uses the release date
as its version and offers each new one as an update. To build it yourself:
`cd android && ./gradlew assembleDebug`.

## The original engine

Original engine (Wine) mode, `--compare` and `--compare-all` run the real
Loquendo 7 engine on the same text, under Wine, as a separate program
(`oracle/run.sh`), and read back what each of its stages produced. Nothing
of it is loaded into quoquendo, and nothing outside its own Wine prefix is
touched. It works on Linux, from a checkout of this repository (the app finds
`oracle/` next to the current folder or the program).

1. Install Wine (with 32-bit support), `xvfb-run` and the MinGW-w64 cross
   compiler (`i686-w64-mingw32-gcc`).
2. Build the program that drives the engine:

   ```sh
   (cd oracle && i686-w64-mingw32-gcc -O1 -o loqref.exe loqref.c)
   ```
3. Put the voices' installers in one folder and point `LOQ_INSTALLERS` at it.
   `Raffaele - Italian.exe` must be among them: the engine itself is
   installed from it (silently, on a virtual display); every other voice is
   only its files, unpacked over that installation.
4. Make a licence with [loquendo-keygen](https://github.com/depau/loquendo-keygen)
   for one of this computer's network cards and the Wine user `wineuser`, as
   `license-wineuser.reg` in the repository root (`LOQ_LICENCE=<file>` for
   another one):

   ```sh
   loqkeygen --mac <this computer's MAC, e.g. 52:54:00:12:34:56> --user wineuser -o license-wineuser.reg
   ```

   The licences the installers write make every voice say only that the
   licence isn't valid for this system.
5. Run the app in that mode, or the comparisons:

   ```sh
   LOQ_INSTALLERS=~/Downloads/Loquendo cargo run --release -- --original
   LOQ_INSTALLERS=~/Downloads/Loquendo quoquendo --compare "Il treno è in arrivo."
   ```

The first use of a voice installs it into `oracle/prefix` (`LOQ_PREFIX` for
another place), which takes a minute; the licence is imported again whenever
the file changes. Every result is cached in `oracle/cache`, so each text
runs the engine once. To drive the engine by hand:
`oracle/run.sh <voice> <utf-8 text file> <output prefix>` writes its
transcription (`.fon`), its events (`.ev`) and its audio (`.wav`); see
MECHANICS.md §8.

## The library

```toml
[dependencies]
quoquendo = { path = "crates/quoquendo" }
```

```rust
let voice = quoquendo::Voice::find(std::path::Path::new(".../LTTS7/data"), "Roberto")?;
let speech = voice.say("Il treno regionale è in arrivo al binario tre.")?;
// speech.samples: f32 in [-1, 1], speech.rate: 32000; speech.timeline: when
// each sound, pause and recorded piece is heard
let flac = quoquendo::export::encode(&speech, quoquendo::export::Format::Flac, None)?;
std::fs::write("out.flac", flac).unwrap();
```

`voice.utterance(text)` also gives where each word sits in the text, for
`export::Tagged` (the text embedded with each word's time) or for following
along while it plays.

Without a filesystem (a browser, a server holding files in memory):
`pack::unpack_installer(&exe_bytes)` gives a voice's files as an LTTS7 tree,
`VoiceFiles::from_map(name, &files)` (or `from_layout` over any `read`/`list`)
picks what one voice needs, and `Voice::from_bytes` loads it.
`voice.transcribe(text)` gives the pronunciation the engine will use, in the
voices' own notation.

## How close it is

Measured against the original engine run under Wine ([The original
engine](#the-original-engine), not needed to use quoquendo), on the benchmark
sentences in `oracle/bench-*.txt`, without any dictionary made by running the
original:

| voice | language | sounds identical | half-sounds picked identically | output samples identical |
|---|---|---|---|---|
| Roberto, Giulia, Luca, Matteo, Paola | Italian | 100% | 100% | 100% |
| Allison, Dave, Steven, Susan | US English | 100% | 100% | 100% |
| Kate, Simon, Elizabeth | British English | 100% | 100% | 100% |
| Bernard, Juliette | French | 100% | 100% | 100% |
| Jorge, Carmen, Leonor, Carlos, Diego, … | Spanish | 100% | 100% | 100% |
| Katrin, Stefan | German | 100% | 100% | 100% |
| Afroditi | Greek | 100% | 100% | 100% |
| Amalia, Eusebio, Felipe, Fernanda, Gabriela | Portuguese | 100% | 100% | 100% |
| Krzysztof, Zosia | Polish | 100% | 100% | 100% |
| Annika, Sven | Swedish | 100% | 100% | 100% |
| Willem | Dutch | 100% | 100% | 100% |
| Frida | Danish | 100% | 100% | 100% |
| Dmitri, Olga | Russian | 100% | 100% | 100% |

The pronunciation of these languages is ported from the original's own
language modules and each voice's own phone rules (MECHANICS.md §4b–§4n;
Danish and Russian numbers through the engine's generic number reader, §4n);
the original's choice of pieces, its decoder, its joins and, for voices
recorded at 44.1 kHz (Marcello, Florence, Alan, Grace, Ximena…), its
resampling to 32 kHz are reproduced exactly (§2c, §2d, §5, §6).
Every voice's row, stage by stage, is in
`oracle/out/dashboard.md` (`quoquendo --compare-all`).

See MECHANICS.md §2d, §4–§6 and §9.

## Voices, names and licences

No voice data, Loquendo library or dictionary is included here or needed to
build: quoquendo reads the files of voices you already have, from their
installers or an existing installation. Loquendo and the voice names belong
to their owners; this project is not affiliated with or endorsed by them.
The Speex decoder in `crates/quoquendo/src/speex` is a Rust port of
Xiph.Org's, under its BSD licence (`crates/quoquendo/LICENSE-speex`).
