# quoquendo — mechanics

How this app makes the Loquendo 7 (LTTS7) voices speak again from their
original files, and where every number comes from. `mechanics.json` holds the
same values; anything marked **estimated** is my own choice after looking for
the engine's value and not finding a readable one.

quoquendo itself runs no Loquendo code. The DLLs were read statically (Ghidra,
Deep mode), and the original engine can be run separately under Wine as a
**reference** (§8) to compare every stage against it.

The pipeline uses the original engine's own module names (its module table in
`LoqTTS7.dll` at 0x100b3fe0 reads TOP, LES, FON, CAT, SIG, ACU), and every
stage can be swapped (window: Pronunciation / Piece picking / Joining;
command line: `--fon`, `--cat`, `--join`):

```
text ─FON─► sounds to say ─CAT─► pieces of recordings ─SIG─► audio
      rules | original | command   builtin | original    like the original | smoothed
```

**Layout.** `crates/quoquendo` is the engine as a library (no dependencies
beyond a C compiler for the bundled Speex decoder; no process spawning, no
GUI): `Voice::find(data_dir, "Roberto")` or `Voice::from_bytes(VoiceFiles {…})`
then `voice.say(text)` gives 32 kHz mono samples, `voice.transcribe(text)`
the FON result. The root package is the app (egui window, command line,
installer unpacking, and the Wine reference harness under `oracle/`).

**What a voice needs**, all from its own installer (`<Voice> - <Language>.exe`,
Inno Setup 5.2.3; `quoquendo --install` unpacks it with `innoextract`):
`data/<Voice>.vcf`, `data/voices/<Voice>/` (`.sde` index, `.bin` recordings,
`.vde`), `bin/LTTS7<Voice>.dll` (its phone table, read as data, §5) and
`data/languages/<Language>/` (`<MotherTongue>.dbl` dictionary and, for
English, `.gpr` letter-to-sound model, §4). Nothing of the original is run.
The optional `lexicons/<MotherTongue>.tsv` dictionaries are the only thing
still made by running the original under Wine (§9); without them US English
and Italian stay close (below), French does not.

Stages hand each other plain data (`synth::TargetPhrase`, `synth::Phrase`,
`synth::Speech`). FON output also has a text form, the original engine's own
transcription notation (`i l-tt r `E n o #50 ... @500?`), so **any program**
can be the FON stage: `--fon "<command>"` gets the text on stdin and prints a
transcription.

## 1. The voice is the language

`data/voices/<Voice>/<Voice>.vde` names the voice's `MotherTongue`
(Roberto: Italian, Katrin: German), `Gender`, `BasePitch`, `BaseSpeed`. The
app runs that language's text rules and uses the phone symbols the voice's own
recordings contain (Roberto 60 symbols, Katrin 88). Roberto is selected first
when installed; Italian voices' sample text ends with the station warning
"Attenzione! Allontanarsi dalla linea gialla." as a benchmark.

## 2. The recordings file — Speex ultra-wideband (SIG input)

`LTTS7Codec210.dll` exports one function, `LTTS7Codec210` (0x10004760), a
name→function table at 0x1002e028 (`ELQOpenDecoder` 0x100045e0, `ELQDecode`
0x10004530, ...). Its strings are libspeex's ("ultra-wideband (sub-band
CELP)", "Invalid mode encountered: corrupted stream?"), so the codec is
**Speex**, compiled in from the BSD-licensed 1.2rc2 sources (`speex/`).

File layout (all four Katrin/Roberto files checked):

| part | content | source |
|---|---|---|
| bytes 0–1 | XOR key for the text header = byte0 ^ byte1 | derived: gives `LOQUENDOTTS7` at 2..14 in every file |
| text header | `(id u16le, len u16le, value)` fields; id 1 = header length (Roberto 128) | decoded |
| 28 bytes | big-endian: 44000 bit/s, quality 10, 640 samples/frame, 32000 Hz, 110 bytes/frame, frame count (Roberto 335637), key word | `ELQDecode` reads 0x1c bytes at the header (0x10004530) |
| frames | frame k at header + 28 + 110·k, every byte XOR the key word's low byte (Roberto 0xc9; 0 = none) | 0x10004340, XOR loop 0x10004290 |

Decoder settings, from `ELQOpenDecoder` 0x100045e0 and the reset 0x10004230:
UWB mode at 32000 Hz, enhancement **off**, positions shifted by **509
samples** (decoder delay), and **6 frames** decoded before a seek target, with
a fresh decoder per seek.

Two differences between Loquendo's (older) Speex and Speex 1.2, both found by
comparing against the original's audio and then confirmed in its code:

- The wideband layer hands the band above it **twice** its excitation:
  `SPEEX_GET_INNOV` (decoder ctl 0x1000f0e0, case 0x66) writes
  `e[2i] = 2.0 · exc[i]` (constant 2.0 at 0x10022e58), with alternating signs
  in the folding (-1.0 at 0x100242f8). Speex 1.2 hands 1×, so 8–16 kHz was
  6 dB too quiet. Patched in `speex/libspeex/sb_celp.c`.
- Speex 1.2 high-passes its output; the older decoder doesn't. Turned off
  (`SPEEX_SET_HIGHPASS` 0): below 250 Hz the match went from 10 dB to 40 dB.

Result: replaying the pieces the original engine chose, our decoder matches its
audio **69.5 dB** inside the pieces (median over the 20 benchmark sentences;
half the samples identical, the rest within a few units of 32768).

## 3. The voice index — `<Voice>.sde`

Text with every byte ≥ 0x80 shifted up by 0x80 (line feeds are not). Read as
Latin-1.

```
14,4                         header (meaning unknown; Katrin 10,16)
LTTSComponentVersion=7.0.0
N=4329,89554                 sentences, phones incl. both edge silences (80896 + 2×4329)
U=# s `o n o-i n a v: e r tt i tt a m `e n tt e-u $ `i tt e-dd a l: a-g `a b: j a @
L# !1KD nm #;                first phone, left half: !start length pitch
R# nB #<                     first phone, right half: length pitch
L$ X, #:                     second phone ...
S=I                          (after some sentences) I question, E exclamation
```

- Each `U=` line is **one recorded sentence**: phones separated by spaces,
  **word boundaries by `-`**, `#`/`@` silences at both edges.
- One `L` + one `R` record per phone except the two edges. `L` is the phone's
  left half, `R` its right half: the original engine's units are these
  **half-phones** (§5).
- Numbers are **base 93**, digits `"` (0x22) to `~` (0x7E); `!` marks the
  sentence's absolute start. Times are in **10 µs** steps (samples = ticks ×
  0.32); Roberto's last phone ends 22 ms before the end of its frames (a test
  checks it). The last field is the **pitch of that half in Hz** (measured in
  the audio: 118.5 Hz where the record says 117).
- Roberto's symbols: `T$` /tʃ/, `Dg` /dʒ/, `Ts` /ts/, `Dz` /dz/, `$` /ʃ/, `Gn`,
  `Gl`, `N` (before k/g), `M` (before f/v), `tt`/`dd` plain t/d, `j`, `w`,
  `` `E `` open e, `` `Oa `` open o, `:` long consonant, `` ` `` stress.

## 4. FON — text → sounds

The original's dictionary is not in `Italian.lex` (4 KB, a small add-on list)
but compiled into `LTTS7Italian.dll` (600 KB; only a list of names is readable
there). The builtin rules (`g2p.rs`) are therefore **measured against the
original's own transcriptions** (§9) rather than read from it.

Italian rules: soft `c/g` → `T$`/`Dg` (also `cc/gg` → `T$:`/`Dg:`); doubled
consonants → one long phone (`n:`, `dd:`, `k:`); `cqu` → `k: w`; velar `N` before
hard c/g; `i` before a vowel glides to `j` except after `r`/`l` (original: 8%
and 3% glide there, 64–98% elsewhere); a word-final `-ia` after a consonant is
a full `i` (94–98%); `z` is `Dz` at the start of a word (272 to 4), long `Ts:`
between vowels (2745 to 203), `Ts` after a consonant, `-izz-` → `Dz:`; `s` voiced
between vowels and before voiced consonants; stress and open/closed e/o from
learned ending tables (§9); a written final accent wins (è ò open, é ó
closed). In sentences: articles, prepositions, conjunctions, clitics and `non`
are unstressed; `n` takes the place of the next word's first sound (`un pacco`
u m, `non vieni` n o M); apostrophes join (`l'esame` → one word); numbers are
read per group as the original does (9542 "novemila cinquecento quarantadue",
1001 "milleuno").

**Dictionaries.** When `lexicons/<MotherTongue>.tsv` exists (Italian,
EnglishUs, French), every word in it reads exactly as the original engine
transcribed it on its own; the rules above only handle words it doesn't have.
They are made by `quoquendo --learn-lexicon <word list> --voice <voice>`
(§8, §9) and are not shipped: they come from the original engine.

English in sentences (Allison's transcriptions): function words take weak
forms (`the` Dh HEh, before a vowel Dh i; `to` th HEh; `for` f HR; `our` Aa:
HR); inside a word `s t` is said `s d` (`Boston` b `` `Aa: `` s d HEh n); a final
t/k/p/d before another word's consonant is unreleased (`Hut` `Huk` `Hup`
`Hud`), but `-nd` loses its d before `the` (`mind the` m `` `HAI `` n); t/d
before `you` become `T$`/`Dg` (`connect you` … Huk T$ j u); a final z before s
is s (`please stand`); `to` after a vowel or n and before a vowel is flapped
(`train to arrive` … n HTh u). No linking sounds are added between words.

French in sentences (Bernard's): little words (`le la les de des et à je
nous vous sur sont est`...) lose their stress, `` `OE `` becoming `HEh`;
liaison moves the silent consonant to the next word (`mes amis` m e | z a m
`` `i ``, `nous avons` n u | z a v `` `o) ``); `prochain arrêt` loses its nasal
(p HRF Oa $ `` `E `` | n a HRF `` `E ``); a final consonant + r/l keeps its e
before a consonant (`votre patience` v `` `Oa `` tt HRF HEh); `m'appelle` is the
elided word plus the rest; `est-ce` `` `E `` s.

**The languages' own data** (read from the installer's files, `g2p::load_language_data`):

- `<MotherTongue>.dbl` is a plain **SQLite 3** database (read by `sqlite.rs`,
  a small pure-Rust reader): table `lessico` with `grafemica` (spelling),
  `fonetica` (pronunciation in the voice's notation), `gradoaccentuale`,
  `grammaticale`, `gennum`, `tempomodo`, `abbreviazione`, `altreinfo`,
  `omografotempo`. US English 1 715 words, French 1 308, German 3 577:
  exceptions, homographs, abbreviations (`ELQDB*` in LTTS7Util.dll reads it
  with SQL).
- `<MotherTongue>.gpr` (English) is LTTS7GPR.dll's letter-to-sound model
  ("LoqG2P", `gpr.rs`): per letter an ordered list of context rules (left and
  right context strings, from a string pool) giving an output class; the
  first matching rule wins, else the letter's default class; a class is a
  list of engine feature words (§5), named through the voice's phone table.
  Layout from the loader 0x10001710, matching from 0x10001220 (the early
  exit at 0x1000141f returns the matched rule's class).
- After them the original applies word rules (LTTS7English.dll); measured
  against its transcriptions of 128 374 words, these reproduce **80.5%** of
  words exactly (35% from the model alone): `s` + t/k/p inside a word loses
  aspiration; `t s` → `Ts`; `HEh Hr` → `HR` before a consonant or the end;
  `HEh l` → `Hl` at the end or before z/d; `Hr` → `HR` after `HAI` and
  stressed I/E; dark `Hl` after `i:`/`i`/`HEI` at the end; `th` flapped
  (`HTh`) between a vowel (or n, r) and an unstressed vowel when the vowel
  before is stressed or full, or in the last syllable before i/I/HR/Hl;
  other stops before a consonant (not r, l, j, w) unreleased; `'s` is the
  stem plus I z / s / z, without the stem's flap.

Sounds identical to the original on the benchmarks **without** the Wine-made
dictionaries: Italian 96.6% (rules and learned ending tables), US English
**97.7%** (its own data, above; 70.4% before), French 69.8% (rules only: its
letter-to-sound rules are code, FonTrasParola 0x1000f730 in LTTS7French.dll,
3 570 decompiled lines, not ported yet).

Pauses are the original's defaults: `ShortPauseLength` 50 ms (`-`),
`MediumPauseLength` 120 ms (`,` `;` `:`), `LongPauseLength` 500 ms (`.` `!` `?`
newline) — its transcription confirms `#50` and `@500`.

## 5. CAT — picking pieces (the original's rules, decompiled)

**What the original does** (LoqTTS7.dll): units are half-phones; the event
19 it sends per pick names the sentence and how many half-phones it took. Its
picker is **greedy, left to right**, not a global search:

| step | address | rule |
|---|---|---|
| driver | 0x10047ec0 | for each wanted half-phone: pick, then jump ahead by the count picked; at most 0x800 per phrase |
| per step | 0x10046de0 | ask every database of the voice, **last first** (Gilded phrase lists before the main one); a score ≥ 5 000 000 takes the whole matching unit |
| scoring loop | 0x10046b20 | every recorded instance of the wanted phone; skip sentences on a 30-entry exclusion list; score = context (0x10045ad0), **+1** if it carries straight on from the previous pick; for `D`/`I` units or whole matches × a prosody factor (0x10046670: 10, 5, +2, +5); + pitch/duration closeness (0x100468c0); keep the maximum. At a right half take **2** half-phones when the recording's next phone is the wanted one (so most joins fall mid-phone) |
| context score | 0x10045ad0 | similarity of neighbours outward while ≥ 30: left half right 6/3/1…, left 10/4/2/1…; right half right 10/6/3/1…, left 6/3/1…; plus a 16×16 prosodic-class table (0x10070f28) with the same weights; × a per-phone factor ÷ 10; question units ×1.5 when a question is wanted, else ÷3; a recorded sentence that is the whole phrase with the same tune 10 000 000 (`e`/`F` 5 000 000) |
| prosody closeness | 0x100468c0 | (100 − Δf0)·3·2 when the pitch jump is under 100 Hz, + 100 − (ratio − 100) when durations are within 2× |

**The tables it reads** are built while the voice loads
(CatLoadVoiceReadOnlyData 0x1004bd60) and stored with `ELQSharedResourceAdd`
(LTTS7Util.dll; LoqTTS7's pointer 0x100b7324) under the names
`TabelleFonetiche_<voice>` and `SomiglianzaFonetica_<voice>`; the constructors
registered for them in 0x10043720 are:

| table | constructor | content |
|---|---|---|
| feature word per phone | 0x10050780 | bits: `&3` 1 vowel / 2 consonant / 3 pause; `0x10000` stressed; `0x10` unvoiced; `&0x1e3` manner family (0xc2 glide, 0x1c2 nasal, 0x1e2/0x102/0x142 stops and fricatives…) |
| weight class per phone ("TabelleFonetiche") | 0x100498f0 → 0x100457b0 | 0–10: unvoiced stops 1, fricatives 2–3, nasals 5, liquids 4–6, unstressed vowels 8, stressed vowels 9 |
| similarity matrix N×N ("SomiglianzaFonetica") | 0x10049a70 → 0x100495b0 | **30** for the same phone (and pause–pause), ~17–21 for neighbours (`a`/`` `a ``, `p`/`b`, `f`/`v`), 0 for unrelated |
| prosodic class per phone of a phrase | 0x1004bf00 (default CatVidCalcProsLab; no voice library replaces it) | leading silences 15; up to the first stressed vowel 7, after it 0; from the last stressed vowel 3 (statement ending in `@`) or 1; the phone before the edge and the edge 4 or 2; the first phone 5 (8/9 with no stress, 10/11 alone) |
| class-match score 16×16 | 0x10070f28 | e.g. same class 20–25, 5↔8/9 16, 0↔6 10; class 15 scores 0 |

Recorded sentences are labelled with the same 0x1004bf00 once at load (the
`.sde` loader 0x10049f10, with the sentence's `S=` type, `D` by default) and
the wanted phrase before each pick (0x10049cd0). Before labelling, the last
phone of every word but the last is marked **6** — in a recording, the phone
before each `-` of its `U=` line (`s `` `o `` n o-i n…`), in the wanted phrase
the `w` mark at +0xe of its phone — and the labeller keeps a 6 (12/13 in the
final stretch, 14 as first phone). Without that mark half the context scores
were wrong.

**Where the tables come from.** The voice's own library
`bin/LTTS7<Voice>.dll` (installed with it) holds its phone table: 16-byte
records `{ name pointer, id, byte, base feature word, word }`, ids 0, 1, 2…,
record 0 `{ "", 0, 0, 0xffffffff }`. `phoneset.rs` finds it by that shape
(no fixed offsets; any voice), converts each base word with 0x10050780 and
computes the weight classes and the similarity matrix with ports of
0x100457b0 and 0x100495b0 (helpers 0x10049470 place, 0x10049550 manner,
0x10049500 height). Checked against the tables the original itself builds
(`oracle/loqref.c LOQ_DUMP_TABLES` hooks the `ELQSharedResourceAdd` pointer
at 0x100b7324): identical for Roberto (61 phones), Allison (85) and Bernard
(60), every feature, class and all N×N similarities (`cargo test phoneset`).

**Checked term by term.** `LOQ_TRACE_CAT=1` makes `oracle/loqref.c` redirect
the original's three calls inside 0x10046b20 (to 0x10045ad0 at 0x10046c9d, to
0x10046670 at 0x10046d2f, to 0x100468c0 at 0x10046d65, the last through an
assembly thunk since it takes the position in EDX) and log every candidate's
context score, class part, factor and prosody; `QQ_TRACE_CAT=1` makes
quoquendo log the same. On "Attenzione! Allontanarsi dalla linea gialla."
all 97 322 candidates now score the same in both.

**Exact scoring** (0x10045ad0, 0x10046b20, 0x10046670, 0x100468c0,
0x100464b0), as `synth::select_like_original` now does it:

- context: outward from the candidate, each neighbour adds similarity ×
  weight (left half: right 6, 3, then 1; left 10, 4, 2, then 1; right half:
  right 10, 6, 3, then 1; left 6, 3, then 1) and goes on only while it is the
  same phone (30); the class-match score of the phone itself ×10 and of each
  neighbour × the same weights. The left side is scaled by the phone's weight
  class / 10, and the class part too when the context stops at the first
  neighbour on the far side. A question recording: class part ×1.5 when a
  question is wanted and it matches back to its first phone, else both ÷3.
- the score is (context + 1 if it carries straight on + prosody) × a factor,
  the factor only for `D`/`I` recordings or whole-phrase matches: 10 (5 for a
  right half that doesn't also take the next left half), +2 when the
  recording keeps matching (class score ≥ 10, distance < 1000) up to a phone
  of weight class < 6, +5 when carrying on and the recording keeps matching
  (distance ≤ 1) until a phone of lower weight class.
- prosody (vowels and voiced candidates): the half picked last against the
  half *before* the candidate in its recording: (100 − |Δpitch|)·3·2 under
  100 Hz, + 100 − (ratio − 100) for their lengths (each with the following
  half) within 2×; ÷3 for stops/fricatives, ×3/5 for nasals/liquids at a
  phrase start or next to an unvoiced consonant.
- a right half also takes the next left half when the recording has the
  wanted next phone there and its class score is ≥ 10.

Half-phones from the same recording and place as the original's:

| voice | benchmark | similarity guessed | tables | + word-end marks |
|---|---|---|---|---|
| Roberto | `oracle/bench-it.txt` | 39.6% | 84.8% | **94.5%** |
| Allison | `oracle/bench-en.txt` | 43.6% | 83.0% | **93.7%** |
| Bernard | `oracle/bench-fr.txt` | 43.8% | 80.5% | **97.9%** |

(`--compare` gives the picker the original's own transcription; that string
glues some words together, `h `` `HAI `` HEh v s `` `E ``…, so word starts are
taken from ours where the sounds line up.)

Voices whose library isn't found fall back to the earlier approximation (100
same phone, 50 same without stress/length, no classes).

## 6. SIG — joining (the original's rules, decompiled)

Decoding: each piece plays **160 samples (5 ms) later** than the `.sde` times
say (measured; pieces then match the original's audio at 70–80 dB).

The original handles every half-phone as a unit (`0x1001faa0`), decoded with
**64 samples** of margin on each side (`(rate/10·400)/20000`, 0x1001f580);
half-phones inside one picked run are plain copies (algorithm 0), the last
one of a run is joined to the next run (algorithm 1, `0x1001f870` →
`0x10024660`); pauses are zero-filled units with no lead-in margin and a
4× (256-sample) tail. `synth::render` (`JoinStyle::Original`) does the same:

- **Cross-fade** (any join but voiced–voiced; 0x100244a0): from the previous
  unit's last samples through its tail margin into the next unit's lead-in
  and first samples, `n = tail + lead-in − 1` points (127 between speech),
  Hann window `0.5·(1 − cos(2π(i+1)/(2n+1)))` over 2n points (0x100241e0,
  type 2 at 0x100242ba; previous unit takes the falling half), sum clamped
  and truncated to 16 bits, written into both units; then only the units'
  own samples are played. Into a pause the speech rings out over the
  pause's first 63 samples; out of one it fades in over ~255 samples.
- **Voiced join** (both half-phones voiced, `sig.rs`): pitch marks of each
  half (0x1001e3a0: voiceless every `rate/125`; voiced from `period =
  rate/F0` of the half's record, the best rising zero crossing 0.5–1.5
  periods on, scored by correlation, peak and distance — 75 of 110 traced
  halves get identical marks, most others ±1 sample since our decoding is
  near- not bit-identical); each period with its margins becomes a frame;
  period lengths ease toward the other side when the pitch differs by
  0.5–14 semitones (`x = periods before / all` kept in [0.2, 0.5]; previous
  half `p + (first − last)·x·k/(n−1)`, next half `p + (last − first)·(1−x)·
  (n−1−j)/(n−1)`, truncated; changes of ±1 dropped; undone if they average
  over 10%) — **all 23 traced joins** reproduced exactly; loudness eased with
  a linear gain ramp toward the other side's mean (never below 0.5); frames
  overlap-added back with the same cross-fade over the 64 samples before each
  mark. A one-half unit keeps the marks this gave it for its next join.

Match with the original's audio (its own pieces through our SIG), samples
within ±2 / whole-file SNR:

| voice | hard cuts (before) | + cross-fades, voiced joins, pause units |
|---|---|---|
| Roberto | 26.0% / −1.1 dB | **60.1% / 16.0 dB** |
| Allison | 33.2% / 0.1 dB | **56.8% / 7.7 dB** |
| Bernard | 35.0% / 1.2 dB | **70.1% / 33.0 dB** |

**Smoothed** (my own, `JoinStyle::Smooth`): cuts in mid-phone, slides the new
piece up to ±5 ms to the best waveform match and cross-fades over 5 ms.

## 7. Command line

- `quoquendo --install "<Voice> - <Language>.exe"` (or a folder of
  installers) takes a voice straight from Loquendo's installer (Inno Setup
  5.2.3) into the app's own voice folder, `~/.local/share/quoquendo/LTTS7`
  (`$XDG_DATA_HOME`, `%APPDATA%` on Windows). Only the voice's files are kept:
  `data/<Voice>.vcf`, `data/voices/<Voice>/` (`.sde` index, `.bin`
  recordings, `.vde`, Gilded lists) and `bin/LTTS7<Voice>.dll` (its phone
  table). The installer is only read, with the `innoextract` program; nothing
  is run. Typing an installer's path in the window's folder box does the same.
  Voices from there and from any other installation found are listed together.

- `quoquendo --say "testo" [--voice Roberto] [--out file.wav]
  [--fon rules|original|<command>] [--cat builtin|original] [--join original|smooth]`
- `quoquendo --compare <file|text>` scores every stage against the original
  engine and writes `oracle/out/compare/<n>-original/-replay/-ours.wav`.
- `quoquendo --fon-list <words>` prints our transcription per line.
- `quoquendo --selftest`, `quoquendo --screenshot file.png`.

## 8. The reference: the original engine under Wine

`oracle/run.sh <voice> <text file> <prefix>` installs the voice's installer
(`<Voice> - *.exe`, found by itself) once into a private Wine prefix
(`oracle/prefix`, Wine user `wineuser`) and imports the licence there only
(never into the real system), again whenever the licence file changes. The
licence is the one made for this computer by the loquendo-keygen project
(`license-wineuser.reg`, or `LOQ_LICENCE=<file>`). Without it, run.sh falls
back to the loquendo-tts-server licence from that project's history, bound
to the network card `7A:8B:9C:1D:2E:3F`, and runs the engine in its own
network namespace with only a dummy card of that address (`unshare -rn`: no
root, no network).
`oracle/loqref.c` (built with `i686-w64-mingw32-gcc`) loads `LoqTTS7.dll` and
records:

| file | stage | API |
|---|---|---|
| `.fon` | FON | `ttsPhoneticTranscription(reader, text, out, size, &len)` (stdcall, 0x10055390), `PhoneticCoding=Loquendo` |
| `.ev` event 3 | FON+prosody | each phone said and its duration in ms (`ttsGetPhonemeInfo`), pauses `#`/`@` included |
| `.ev` event 19 | CAT | each piece: sentence id string at +4, half-phone count at +0x18 |
| `.wav` | SIG | `LTTS7AudioFile` device, 32 kHz mono |

Tracing the original from inside (all in `oracle/loqref.c`, by redirecting a
call's rel32 to a logging wrapper): `LOQ_DUMP_TABLES` (the CAT tables as
built), `LOQ_TRACE_CAT` (+`LOQ_TRACE_CLASSES`: every candidate's context,
class part, factor, prosody, the labels), `LOQ_TRACE_JOIN` (every voiced
join's units before/after and every half-phone's pitch marks). quoquendo's
counterparts: `QQ_TRACE_CAT=1`, `QQ_CHECK_MARKS=<trace.join>`,
`QQ_DEBUG_JOIN=1`, `QQ_NO_LEXICON=1` (rules alone).

`LOQ_LINES=1` treats every line as an utterance; `LOQ_FON_ONLY=1` writes
`<prefix>.fonlist` with one transcription per line (87 421 words in 6 s).
Results are cached in `oracle/cache`. In the window, picking "Original engine"
for a stage, or "Compare with the original", runs it in the background.

## 9. Learned from the original

`oracle/out/words/all.txt` (87 421 words from the Hunspell it_IT dictionary)
was transcribed by the original engine. From those transcriptions:

- `src/it-position.tsv`: 630 word endings → which vowel takes the stress,
  counted from the end; **95.8%** right on words held out of the learning.
- `src/it-quality.tsv`: 302 endings of the stressed syllable → open or closed
  e/o; **91.5%** on held-out words.

Whole words identical to the original: 53.8% → **88.5%**. Sentences of the
benchmark (`oracle/bench-it.txt`): 38% → **87.6%** of words identical.

Then the transcriptions themselves became the dictionaries (§4): Italian
87 421 words, US English 128 376 (the SCOWL-based en_US word list), French
635 540 (fr_FR Hunspell, expanded). `--compare` now also counts single sounds.
Sounds identical to the original on the benchmarks:

| voice | benchmark | sounds | words |
|---|---|---|---|
| Roberto | `oracle/bench-it.txt` (20 sentences) | **97.5%** | 92.0% |
| Allison | `oracle/bench-en.txt` (10) | **96.3%** | 70.1% (the original groups words differently) |
| Bernard | `oracle/bench-fr.txt` (10) | **100%** | 88.2% |

## 10. What the engine's own code says (LoqTTS7.dll, decompiled)

The program the voice ships as (`Raffaele - Italian.exe`) is an Inno Setup
5.2.3 installer (strings "Inno Setup Setup Data (5.2.3)" at 0x40b188 used from
`entry`); its code only unpacks files. The speech code is in `LoqTTS7.dll`:

| what | address | rule / value | used in quoquendo |
|---|---|---|---|
| module table | 0x100b3fe0 | TOP, LES, FON, CAT, SIG, ACU | stage names (pipeline.rs) |
| ticks → samples | 0x1001e3a0 | `(t/100000)·rate + ((t%100000)·(rate/10))/10000` | `loquendo::ticks_to_samples` |
| unit fetch | 0x1001f580 | decode from `start − pre` for `len + pre + post`; `post = (rate/10·400)/20000` = **64** samples at 32 kHz, ×4 for algorithms 2/4; `pre = post` unless the unit starts at 0 | documented (only the PSOLA paths overlap these margins) |
| per-unit loop | 0x1001faa0 | decode → pitch marks → byte order → dispatch | — |
| algorithm names | 0x10043ec0 | `COPIA`=0, `CONCATENAZIONE`=1, `SEQUENS`=2, `SEQUENSSOLOTONO`=3, `RIAGGIUSTA`=4, none set = 5 | documented |
| voice setting | 0x10043e60 | reads `Algorithm` from the voice config; Roberto's `.vde`/`.vcf` set none → 5 | — |
| demiphone runs | 0x100472f0, 0x100470f0 | in the default mode consecutive half-phones from the same sentence at contiguous positions form one run, played with algorithm **1** (plain copy); pause entries (types 2/3) go the SEQUENS way (0x10047160, pitch/duration interpolated); at most **0x800 = 2048** half-phones per phrase | hard-cut joins (SIG "like the original"); `synth::MAX_DEMIPHONES_PER_PHRASE` splits longer phrases |
| join dispatch | 0x1001f8e0 (table 0x1001fa88) | algorithms 0/1: copy; 2/4: PSOLA loop 0x1001ef50 → 0x1001f210; 3: "CatAlgo unknown" | Original join = copy |
| pitch marks | 0x1001e3a0 | unvoiced (`'u'`): a mark every `rate/125` samples (256); voiced: period = `rate / F0` from the unit, or found by autocorrelation in [50, 199] samples (default **150**); each mark searched in [0.5, 1.5] periods after the last at a rising zero crossing, scored `(peak·3 − distance) + correlation` (first) or `(peak − distance)·3 + correlation·4` | documented (needed only for speed/pitch changes) |
| PSOLA period | 0x1001df70 | output period k uses source mark `((n−1) + (marks−1)·k·2)/(2n−2)`; target period = source period·100 / pitch-percent, interpolated between the unit's start and end percentages; capped at `rate/40` (800) | documented |
| PSOLA window | 0x1001f210 | windows at 0x1007a810 and 0x1007a328 (the second when the period shrinks) | not yet |
| unit fields | 0x10048930 (debug dump) | ALGO, FONEMA, PHONEMELABEL, CONFINI (boundaries), DURATA, F0 start/end, GUADAGNO (gain) start/end, TIMBRO, TIPOVOC | — |
| output resampler | 0x10028ee0 | resamples when voice and output rates differ by more than 100 Hz | not needed (both 32 kHz) |

So for normal speech the original copies recorded runs untouched, which is why
pieces match at ~70 dB. The few-sample cut shifts are still unexplained by
these functions; they are not caused by PSOLA, which only runs for pauses or
when a speed/pitch change asks for it.

## 11. Not done / unknown

- French (and other rule-coded languages') letter-to-sound rules without
  the Wine-made dictionary: the code is FonTrasParola (LTTS7French.dll
  0x1000f730, 3 570 lines); Italian's exception and ending tables in
  LTTS7Italian.dll's data (`vits$1`, `en$1`… reversed endings with codes)
  are readable but not decoded — our learned tables stand in (96.6%).
- Reading the installers natively (Inno Setup 5.2.3 headers + LZMA) instead
  of calling `innoextract`; needed for a browser build (the library already
  loads everything from bytes, `Voice::from_bytes`).
- Bit-exact Speex decoding (we are within ±1–2 LSB, 70–80 dB): it decides
  the last pitch marks and so the last SIG differences.
- The CAT gap that remains (3–6%), the original's automatic pauses, speed
  and pitch controls, Gilded phrases, the `.sde` header pair (`14,4`/`10,16`).
