# quoquendo

The Loquendo 7 (LTTS7) text-to-speech engine rebuilt from its own code, for
its original voices: Roberto (the Italian station voice), Allison, Bernard,
Katrin and the rest. No Loquendo code runs; the voices' files are read as
data. How each stage works, and where every number comes from, is in
[MECHANICS.md](MECHANICS.md).

## Getting a voice

The voices' installers (`<Voice> - <Language>.exe`, e.g. `Raffaele - Italian.exe`
for Roberto) are all it needs. With [innoextract](https://constexpr.org/innoextract/)
installed:

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
holding them in memory, a browser). `voice.transcribe(text)` gives the
pronunciation the engine will use, in the voices' own notation.

## Other platforms

The library has no dependencies beyond a C compiler for the bundled Speex
decoder (`crates/quoquendo/speex`, BSD). For the browser
(`wasm32-unknown-unknown`) what is left is that C code: either compile it
with a WASI/Emscripten sysroot or port the decoder to Rust; everything else
is plain Rust working on bytes. The app uses eframe, which also targets the
web; its file and audio parts would need web counterparts.

## How close it is

Measured against the original engine run under Wine (`oracle/`, a developer
tool only), on benchmark sentences: pronunciation 97–100% of sounds
identical, piece picking 94–98% of half-sounds identical, joining 57–70% of
samples within ±2 of the original's output. See MECHANICS.md §5, §6, §9.
