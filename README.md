# quoquendo

The Loquendo 7 (LTTS7) text-to-speech engine rebuilt from its own code, for
its original voices: Roberto (the Italian station voice), Allison, Bernard,
Katrin and the rest. No Loquendo code runs; the voices' files are read as
data. How each stage works, and where every number comes from, is in
[MECHANICS.md](MECHANICS.md).

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
cargo run --release -- --dev                      # with the developer tools
quoquendo --say "Attenzione! Allontanarsi dalla linea gialla." --voice Roberto --out out.wav
quoquendo --selftest
```

The window has a Speak tab (with what the engine did, behind a toggle:
sounds, the recorded pieces chosen, each playable on its own, and the
waveform), a Voices tab (the library) and an Explore tab (every recorded take
of every phone, and every recorded sentence). Developer mode, off by default,
adds the voice's details, swappable stages and the comparison with the
original engine.

### In the browser

```sh
trunk serve --release                             # http://127.0.0.1:8080
```

The same app; voices live in the browser's own storage. Browsers can't fetch
the installers from archive.org (it sends no CORS header for `.exe` files), so
downloads need a voice mirror (`QQ_MIRROR_URL` at build time, or Settings);
installers downloaded by hand can always be imported. GitHub Pages builds it
from `main` (`.github/workflows/pages.yml`).

### On Android

`android/` is a system text-to-speech engine: `cd android && ./gradlew
assembleDebug`, install it, pick quoquendo in the system's TTS settings, and
get voices in its Voices screen.

## The library

```toml
[dependencies]
quoquendo = { path = "crates/quoquendo" }
```

```rust
let voice = quoquendo::Voice::find(std::path::Path::new(".../LTTS7/data"), "Roberto")?;
let speech = voice.say("Il treno regionale è in arrivo al binario tre.")?;
// speech.samples: f32 in [-1, 1], speech.rate: 32000
std::fs::write("out.wav", quoquendo::synth::to_wav_bytes(&speech)).unwrap();
```

Without a filesystem (a browser, a server holding files in memory):
`pack::unpack_installer(&exe_bytes)` gives a voice's files as an LTTS7 tree,
`VoiceFiles::from_map(name, &files)` (or `from_layout` over any `read`/`list`)
picks what one voice needs, and `Voice::from_bytes` loads it.
`voice.transcribe(text)` gives the pronunciation the engine will use, in the
voices' own notation.

## How close it is

Measured against the original engine run under Wine (`oracle/`, a developer
tool only, not needed to use quoquendo), on the benchmark sentences in
`oracle/bench-*.txt`, without any dictionary made by running the original:

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
