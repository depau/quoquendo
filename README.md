# quoquendo

The Loquendo 7 (LTTS7) text-to-speech engine rebuilt from its own code, for
its original voices: Roberto (the Italian station voice), Allison, Bernard,
Katrin and the rest. No Loquendo code runs; the voices' files are read as
data. How each stage works, and where every number comes from, is in
[MECHANICS.md](MECHANICS.md).

## Getting a voice

The voices' installers (`<Voice> - <Language>.exe`, e.g. `Raffaele - Italian.exe`
for Roberto) are all it needs; quoquendo reads them itself (nothing is run,
no other program is needed):

```sh
quoquendo --install "Raffaele - Italian.exe"     # or a folder of installers
```

This keeps only the voice's files in `~/.local/share/quoquendo/LTTS7`
(`%APPDATA%` on Windows). An existing LTTS7 installation is found too.

## The app

```sh
cargo run --release                               # the window
quoquendo --say "Attenzione! Allontanarsi dalla linea gialla." --voice Roberto --out out.wav
quoquendo --selftest
```

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

`Voice::from_bytes(VoiceFiles { vde, sde, bin, library, dbl, gpr })` loads a
voice from its files' contents, without touching a filesystem (a server
holding them in memory, a browser), and `VoiceFiles::from_installer(&exe_bytes)`
takes them straight out of the installer. `voice.transcribe(text)` gives the
pronunciation the engine will use, in the voices' own notation.

## Other platforms

The library is pure Rust working on bytes (its one dependency, `lzma-rs`,
reads the installers), so it should build for the browser
(`wasm32-unknown-unknown`) as it is. The app uses eframe, which also targets
the web; its file and audio parts would need web counterparts.

## How close it is

Measured against the original engine run under Wine (`oracle/`, a developer
tool only, not needed to use quoquendo), on the benchmark sentences in
`oracle/bench-*.txt`, without any dictionary made by running the original:

| voice | language | sounds identical | half-sounds picked identically | samples within ±2 |
|---|---|---|---|---|
| Roberto | Italian | 96.6% | 99.3% | 77.9% |
| Allison | US English | 97.7% | 98.6% | 73.9% |
| Bernard | French | 97.9% | 99.8% | 98.5% |
| Jorge | Spanish | 99.8% | 98.9% | 88.9% |
| Katrin | German | 100% (97.6% on unseen sentences) | 98.2% | 71.0% |

Replaying the original engine's own choice of pieces, the audio decoder
matches its output at 89–105 dB.

See MECHANICS.md §4–§6 and §9.

## Voices, names and licences

No voice data, Loquendo library or dictionary is included here or needed to
build: quoquendo reads the files of voices you already have, from their
installers or an existing installation. Loquendo and the voice names belong
to their owners; this project is not affiliated with or endorsed by them.
The Speex decoder in `crates/quoquendo/src/speex` is a Rust port of
Xiph.Org's, under its BSD licence (`crates/quoquendo/LICENSE-speex`).
