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

**Layout.** `crates/quoquendo` is the engine as a library (pure Rust, its
one dependency `lzma-rs` for the installers; no process spawning, no GUI): `Voice::find(data_dir, "Roberto")` or `Voice::from_bytes(VoiceFiles {…})`
then `voice.say(text)` gives 32 kHz mono samples, `voice.transcribe(text)`
the FON result. The root package is the app (egui window, command line,
installer unpacking, and the Wine reference harness under `oracle/`).

**What a voice needs**, all from its own installer (`<Voice> - <Language>.exe`,
Inno Setup 5.2.3; `quoquendo --install` reads it with `inno.rs`, §7):
`data/<Voice>.vcf`, `data/voices/<Voice>/` (`.sde` index, `.bin` recordings,
`.vde`), `bin/LTTS7<Voice>.dll` (its phone table, read as data, §5) and
`data/languages/<Language>/` (`<MotherTongue>.dbl` dictionary and, for
English, `.gpr` letter-to-sound model, §4). Nothing of the original is run.
The optional `lexicons/<MotherTongue>.tsv` dictionaries are the only thing
still made by running the original under Wine (§9); every language stays
close without them (below).

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
**Speex 1.1** (its messages end in ": corrupted stream?", 1.2's don't), built
with MSVC for 32-bit x87. quoquendo decodes it with `speex/`, a Rust port of
that decoder as the DLL has it (BSD, `LICENSE-speex`), for the one frame
layout the voices use: every frame of all 19 voice files we have (2.6
million frames) is narrowband submode 7 + wideband submode 4 +
ultra-wideband submode 1, 492 + 352 + 36 = 880 bits; other layouts are
refused when the file is opened. Mode tables: narrowband 0x1002393c
(decoder init 0x10008d90, decode 0x10009520, ctl 0x1000a820), wideband
0x10023ab4 and ultra-wideband 0x10023b2c (init 0x1000db20, decode
0x1000e170, ctl 0x1000f0e0); submodes 0x100238f8 (nb 7), 0x10023bb0 (wb 4),
0x10023a70 (uwb 1). Its codebooks are byte for byte Speex's, except that
the pitch gain table keeps 3 values per entry (1.2 pads to 4).

How it differs from Speex 1.2 (each in the DLL's code):

- **No one-subframe delay.** Each narrowband subframe is synthesised at once
  with its own LPCs (`iir_mem2` 0x100030e0); the excitation buffer keeps
  max_pitch + 1 = 145 past samples. Speex 1.2 outputs the excitation one
  subframe late and filters it with the previous LPCs, which is 5 ms (160
  samples at 32 kHz): the "play 160 samples later" correction an earlier
  version needed is gone with it.
- LSPs are interpolated (0x10005520), then held apart by 0.002 (narrowband)
  or 0.05 (sub-bands) in a separate pass (0x10005390); the cosine is
  Speex's polynomial (0x10004af0: x < 1.5707963268 → ((C3 − C4·x²)·x² −
  C2)·x² + C1, else its negative at π − x).
- Pitch (0x10006bb0): three delayed copies of the excitation and
  `exc = 1e-15 + g0·e2 + g1·e1 + g2·e0`, gains `cb·0.015625 + 0.5`.
- The innovation handed up (`SPEEX_GET_INNOV`, ctl 0x1000a820 case 0x66) is
  the first codebook only; the excitation (case 0x65) is the whole frame's.
- Sub-bands: innovation gain `exp(q·0.27027027027027023 − 0.15556)` times
  (1 + RMS of the low band's excitation) over the filter ratio
  `(low π-gain + 0.01)/(0.01 + high π-gain)`; second codebook at 0.4 of it.
  Folding (ultra-wideband) `exp((q − 10)·0.125)` over the filter ratio, ×0.7
  (0x10023ca8 + 0x24), alternating sign. Both bands are rebuilt by two
  `fir_mem_up` filters (0x10003960; h0 at 0x100240a8, h1 = h0 with odd taps
  negated at 0x100241a8) and combined as `2·(y0 − y1)`, held within ±32767
  (0x1000c290); the narrowband output is held the same way.
- The wideband layer hands the band above it **twice** its excitation:
  `SPEEX_GET_INNOV` (ctl 0x1000f0e0, case 0x66) writes `e[2i] = 2.0 ·
  exc[i]`, zero between (constant 2.0 at 0x10022e58).
- No output high-pass. Samples become 16-bit as `floor(x + 0.5)`, clamped
  to 32767/−32768 (0x1000f640).
- **Precision.** Every expression runs on the x87 FPU at double precision
  and is rounded to `float` when assigned (`fstp dword` after each
  statement, e.g. in `iir_mem2` and `fir_mem_up`); the port computes in f64
  and rounds at each assignment, with the DLL's double constants (0.4 and
  1/2.2 are doubles here, not 1.2's floats).

File layout (all four Katrin/Roberto files checked):

| part | content | source |
|---|---|---|
| bytes 0–1 | XOR key for the text header = byte0 ^ byte1 | derived: gives `LOQUENDOTTS7` at 2..14 in every file |
| text header | `(id u16le, len u16le, value)` fields; id 1 = header length (Roberto 128) | decoded |
| 28 bytes | big-endian: 44000 bit/s, quality 10, 640 samples/frame, 32000 Hz, 110 bytes/frame, frame count (Roberto 335637), key word | `ELQDecode` reads 0x1c bytes at the header (0x10004530) |
| frames | frame k at header + 28 + 110·k, every byte XOR the key word's low byte (Roberto 0xc9; 0 = none) | 0x10004340, XOR loop 0x10004290 |

Decoder settings, from `ELQOpenDecoder` 0x100045e0 and the reset 0x10004230:
UWB mode at 32000 Hz, enhancement **off**, positions shifted by **509
samples** (decoder delay), and **6 frames** decoded before a seek target by
a fresh decoder; what counts as a seek, and the rest of what the decoder
keeps between requests, is in §2d.

Result: replaying the requests the original engine made of its decoder, our
decoder returns the same samples **bit for bit** (§2d); a test pins the
output for synthetic frames.

## 2b. The 44 kHz recordings — LTTS7CodecMsx (SIG input)

Eleven voices (Alan, Carmela, Dmitri, Empar, Florence, Grace, Jordi,
Marcello, Olga, Willem, Ximena) ship `<Voice>.44000.LTTS7CodecMsx.58300.10.bin`,
named in the text header's field 5. `LTTS7CodecMsx.dll` exports
`LTTS7CodecMsx` (0x10002a90), the same name→function table as Codec210's at
0x10015ae0 (`ELQOpenDecoder` 0x10002820, `ELQDecode` 0x10002780,
`ELQDecoderFileChanged` 0x10001d40, `ELQCloseDecoder` 0x10002710). It is
Speex ultra-wideband again, but built from a later Speex (1.2's decoder
loops) and changed by Loquendo. `speex/msx.rs` decodes it.

File layout (all 31 CodecMsx files checked):

| part | content | source |
|---|---|---|
| text header | as Codec210's; field 5 `LTTS7CodecMsx` | — |
| 124 bytes | 31 big-endian words: 58300 bit/s, quality 10, 640 samples/frame, 44000 Hz, **106 bytes/frame**, frame count, key word, version 2, offset of the first frame from here, number of codebook sets (Marcello 79), bytes per set of each codebook (1280, 80, 80, 320, 256, 64, 512, 512, 1024), divisors 128 and 128, entry sizes (10, 5, 5, 8, 8, 8, 8, 5, 8) | `ELQDecoderFileChanged` 0x10001d40 |
| one byte per frame | which codebook set the frame uses | read at `ctx[0] + frame` in 0x10001190, 0x100046d0, 0x100048a0, 0x10004a10 |
| codebooks | per set, in this order: narrowband LSP 128×10, 16×5, 16×5; narrowband innovation 64×5; wideband LSP 32×8, 8×8; ultra-wideband LSP 64×8, 64×8; wideband innovation 128×8 (signed bytes) | 0x10001d40 |
| frames | frame k at header + word 8 + 106·k, XOR the key word's low byte | `ELQDecode` → 0x100024d0, XOR 0x10002420 |

How it decodes, against Codec210 (§2):

- **No mode bits.** The submode of each layer follows from the frame size
  (nb_decoder_init 0x10003450, sb_decode 0x10005080): 106 bytes gives
  narrowband submode 8 (table 0x10015df8: 472 bits), wideband 5 (0x10015fb8:
  344 bits), ultra-wideband 2 (0x100160e4: 32 bits).
- **Codebooks from the file**, the set the frame's byte names. LSPs:
  narrowband 7 + 4 + 4 bits (stage one over all ten, the next two over LSPs
  0–4 and 5–9, ×0.0039062 then ×0.0019531; 0x100046d0), wideband 5 + 3
  (0x100048a0), ultra-wideband 6 + 6 (0x10004a10); start values as Speex's
  (0x10004400, 0x10004420, 0x10004440; steps set by 0x100042d0). Innovation
  `shape · sign / 128` (0x10001190, divisors from the header). The pitch gain
  table is Speex's (0x100152e0, 4 values per entry as in 1.2).
- **Speex 1.2's loops** (nb_decode 0x10003800, sb_decode 0x10005080): the
  pitch taps are added one at a time into a zeroed buffer (0x10003120); the
  frame's output is the excitation from one subframe earlier, each subframe
  filtered with the previous subframe's LPCs, and the band π-gains and
  ratios come from those previous LPCs too; the sub-bands play their
  excitation one subframe late the same way.
- Kept from Codec210: excitation `exc32 + innov`, then `+ innov2` (second
  codebook ×**0.454545**, a double at 0x100162e0); wideband gains
  `exp(q·0.27027027027027023 − 0.15556)` and (1 + RMS of the narrowband
  excitation), second codebook ×0.4; folding `exp((q − 10)·0.125)`/ratio ×
  0.7. The wideband hands its innovation up **without** Codec210's factor 2
  (`innov_save[2i] = exc[i]`, 0x100056a0). No output clamp in the
  narrowband; the sub-band combine (0x10004b80) and 16-bit rounding
  (0x10001500) are Codec210's.
- Decoder settings (`ELQOpenDecoder` 0x10002820): enhancement off, delay
  **510** samples and **9** warm-up frames at 44 kHz (509 and 6 at 32 kHz).
  Between requests it keeps what Codec210's keeps (§2d), with one addition:
  a position whose frame is at or past the header's frame count (+0x11f0,
  stored by 0x10001dab) has the delay taken off again (0x100024f1).

Result: replaying the requests the original engine made of its decoder,
ours returns the same samples bit for bit (§2d).

## 2c. Leaving the engine — the output resampler

The engine joins units at the voice's rate (positions from the `.sde` at
44000 Hz too) and converts the stream to the rate asked for, here the
32000 Hz `oracle/loqref.c` sets, when the two differ by more than 100 Hz
(`VoiceResamplerRun` 0x10028ee0). The resampler (0x1001c370, `SrcUD`
0x1001be70, `FilterUD` 0x1001bae0) is Julius O. Smith's `resample`, fixed
point, "large filter", interpolated coefficients:

- filter: one 8192-tap wing, 256 taps per input sample, lowpass 0.90 under
  a Kaiser window β = 9, largest tap 32767 (`Imp` 0x1007ad88, `ImpD`
  0x1007ed88); `codec.rs` designs it and a test checks it equals the DLL's;
- gain `LpScl` = 14746 (0x100a5cd8) × factor + 0.5 = 10724; time step
  `dtb` = 32768/factor + 0.5 = 45056; filter step `dhb` = min(256, 256·factor)
  × 128 + 0.5 = 23831; each product rounded to 14 fraction bits, the sum
  shifted by 2, scaled, rounded by 13 bits and clamped;
- the right wing's phase is `(−time) & 0x7fff` (Smith's original uses
  `(time ^ 0x7fff) + 1`).

`codec::resample` does this over the whole utterance and matches the
original's 32 kHz output, fed the original's own 44 kHz output, sample for
sample except at phone ends: the engine runs the resampler once per phone,
asking for `floor(pending · 32/44) − 16` samples (0x10028c00) and keeping
the input from the next output's position on, so the ~10 samples before
each phone end are computed with whatever stale samples the 4096-sample
input buffer holds past the new data (the first call: zeros). Those few
samples per phone are not reproduced yet; with them our replay of the
original's pieces scores **81.5 dB** inside pieces at 32 kHz on
`oracle/bench-it.txt` (mean of the per-sentence medians, which run 45–91),
against 106.7 dB at 44 kHz.

## 2d. Decoding bit for bit

`oracle/loqref.c` with `LOQ_TRACE_DECODE=1` logs every request LoqTTS7.dll
makes of either codec, with the samples it got back, to `<prefix>.dec`: the
codecs hand the engine their functions through their name tables, and the
`ELQDecode` slot (LTTS7Codec210.dll 0x1002e064, LTTS7CodecMsx.dll
0x10015b1c) is swapped for a wrapper before the voice loads. `ELQDecode`
(0x10004530, 0x10002780) is cdecl `(decoder, position, samples, short *out,
file)`. `quoquendo --check-decode <prefix>.dec --voice <v>` replays the
requests in order through `AudioBank::read` and counts the bit-identical
ones.

**The decoder keeps state between requests** (0x10004340, CodecMsx
0x100024d0). For a position, frame f = (position + delay) / 640:

- f is the frame decoded last, or the one before it (both are kept; their
  numbers start at 0xfffffffe, `ELQOpenDecoder` 0x100046d3 / 0x10002a22):
  its samples are copied;
- f is the frame after the last: the same decoder goes on to decode it, and
  the last becomes the one before (0x100042f0 / 0x10002480);
- anything else: a fresh decoder (0x10004230 / 0x10002390) decodes frames
  max(0, f − warm-up) to f;
- a frame past the end of the file is silence, without running the decoder.

The engine fetches a phone run's units as overlapping requests (each with a
margin either side of the unit, §10 unit fetch), so most requests carry
on with the decoder of the request before. Decoding every request with a
fresh decoder, as quoquendo did before, matched 20–24% of them; with the
rules above, all of them:

| voice | codec | benchmark | requests bit-identical, before | after |
|---|---|---|---|---|
| Roberto | Codec210 | `oracle/bench-it.txt` | 382 of 1930 (19.8%) | **1930 of 1930** |
| Allison | Codec210 | `oracle/bench-en.txt` | 170 of 702 (24.2%) | **702 of 702** |
| Marcello | CodecMsx | `oracle/bench-it.txt` | 412 of 1930 (21.3%) | **1930 of 1930** |

Katrin (`bench-de`, 718 requests), Bernard (`bench-fr`, 576), Jorge
(`bench-es`, 836), Afroditi (`bench-el`, 888) and Grace (CodecMsx,
`bench-en`, 690) are bit-identical too. Each line of a benchmark is its own
utterance (`LOQ_LINES=1`); the decoder carries on across them as well.
`AudioBank::read` is therefore history-dependent, as the original is: a
request gives the original's samples when the requests before it were the
original's too.

**Floating point.** The x87 control word is 0x027f before every request
(precision control 53 bits, exceptions masked), MXCSR 0x1f80 then 0x1fa0. So
every `fadd`/`fmul`/`fdiv` rounds to a double, which with the decoder's
magnitudes is IEEE double arithmetic, and each `fstp dword` is a rounding to
`float`: the port computes in `f64` and rounds each assignment, with no
80-bit arithmetic. The decoders' own code (Codec210 0x10001000–0x1000ffff,
CodecMsx up to 0x10005c00) holds no transcendental instruction; `f2xm1`,
`fyl2x`, `fsin`, `fcos`, `fsqrt` occur only inside the statically linked
MSVC runtime. What the decode path reaches of it (REA `get_call_graph` from
nb_decode 0x10009520 and sb_decode 0x1000e170; CodecMsx 0x10003800 and
0x10005080):

| call | Codec210 | CodecMsx | as run |
|---|---|---|---|
| `_CIexp` | 0x10011aec | 0x10006dac | the SSE2 routine (below) |
| `_CIsqrt` (compute_rms) | 0x100105c0 | 0x10005ed0 | `fsqrt` under the caller's control word: the correctly rounded double square root |
| `floor` (16-bit rounding) | 0x10011c90 | 0x10005da0 | SSE2, exact |
| `_ftol2` | 0x10011050 | 0x10005cf0 | SSE2 `cvttsd2si`, exact |

`_CIexp` takes the runtime's SSE2 routine when its flag (0x100300cc /
0x1001c184, set at load by 0x10015195 / 0x10007fd9 from CPUID and an OS
check) is set, which loqref logs (`S` lines: 1), and every exception is
masked in MXCSR and the control word. The routine (0x100175ee / 0x1000b1ce,
the same code and constants in both DLLs) rounds x·64/ln 2 with the
1.5·2⁵² shifter (0x10026ff0, 0x10027010), takes r = x − n·(ln 2/64) in two
parts (0x10027020, 0x10027030), reads 2^(j/64) as a relative correction and
mantissa bits from a 64-entry table (0x10027060; CodecMsx 0x10017980), and
evaluates r + corr + r⁴(c₀ + c₁r) + r²(c₂ + c₃r) (0x10027040, 0x10027050)
on two SSE lanes before scaling by 2^(j/64 + m). `x87::exp` is that routine
operation by operation. Called under Wine, the DLL's `_CIexp` returns the
same bits as `x87::exp` on 463 627 random arguments up to ±619 and on all 80
the decoders pass (q/3.5, q·0.27027027027027023 − 0.15556, (q − 10)·0.125,
pinned in a test); glibc's `exp` agrees on those 80 but not on 0.47% of the
random ones, and other platforms' libraries need not agree at all.
Arguments beyond ±619, where the routine finishes on the x87 at 64-bit
precision, are not ported; the decoders never pass them.

## 3. The voice index — `<Voice>.sde`

Text with every byte ≥ 0x80 shifted up by 0x80 (line feeds are not). Read as
Latin-1.

```
14,4                         licence bit: byte 14, mask 4 (Katrin 10,16)
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
- The first line is a **licence bit**, not synthesis data. The loader
  0x10049f10 keeps it (up to 79 characters) at database+0x30; a plain-text
  `.sde` gets `332198,21119` (0x10096818). `CatGetVoiceArrayIndexes`
  (0x1004c190) hands it to the licence check 0x10002a00, which splits it at
  the comma: the voice is valid only if licence byte [first + 1] has the bit
  `second` set (Roberto byte 14 mask 4, Katrin 10/16, Allison 7/16, Bernard
  12/1); `332198,21119` skips the check. quoquendo ignores it.
- `^` is phone id 1 in every voice's phone table (base 0x10 for Roberto,
  0x48 for Katrin). Only Roberto's index uses it: 56 of 4 329 recorded
  sentences label a stretch the labeller left unnamed (`b r ^ s`,
  `k w a f: ^ r`), with its own L/R timings. Text never produces it, so it
  only takes part as a context neighbour in CAT, through the similarity
  table (§5).

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
**97.7%** (its own data, above; 70.4% before), French 97.9% (`french.rs`: its
own rules are code, FonTrasParola 0x1000f730 in LTTS7French.dll, 3 570
decompiled lines, not ported; ours were fitted to the original's output:
88.0% of held-out words, 98.4% of their sounds). Spanish (Jorge, `spanish.rs`)
99.8%: written accent or the usual stress rule, i/u gliding, c/z `Th`, ll
`Gl`; b/d/g softened between vowels across words. German (Katrin,
`german.rs`) 100% (97.6% on sentences the rules were not fitted to,
`oracle/bench-de2.txt`): be-/ge-/ver-/er-/ent- prefixes before a root (not
before a bare ending: geben) stay unstressed, a vowel before one consonant,
a stop + r/l, doubled or h is long, -e -en -er -el -eln endings reduce
(`HEh`, `Hn`, `Hm` after b, `AR`, `Hl n`), -ig is `I c`, -ier- takes the
stress, ch `x` after back vowels else `c`, st-/sp- `$`, dt `t`; compounds
split after a known first part (Haupt|bahn|hof, Ab|stand) and the later
parts lose their stress. The dictionary's citation forms are reshaped in
the sentence too (nächste: no aspiration after s; München: `c Hn`). Across
words final b/d/g devoice, a final stop is released into a vowel or h and
unreleased before other consonants (`Hut Hub Huk`), p/t/k aspirate before
a vowel or stop+r (not after s), n before k/g is `N`, a vowel-initial word
inside a phrase starts with `Hk`; little words lose their stress except at
the end of a phrase (observed; not in the module). Left: loanword stress
(Konzert, Büro).

**German, decompiled** (LTTS7German.dll; its one export `German` 0x10007f00
hands the engine a table of named functions at 0x100294a8: LesAccenta
0x100014d0, LesNormalizzaAccento 0x10003410, LesGradiAccentuali 0x10008130,
FonTrasParola 0x10015b50, FonAllofonoLingua 0x10012830, FonGetSamples
0x100071e0). The loader 0x10007720 reads the voice dictionary three ways:
`SELECT grafemica, fonetica FROM lessico` (pronunciations), `SELECT
grafemica, gradoaccentuale, grammaticale, tempomodo, gennum … WHERE both set`
(grammar) and the abbreviations. 0x1000a470 turns `grammaticale` into a
class: PREP/PREPS/PREP_CONG 0x19, PREPA 0x1e, ARTIC 0x18, PRON/PRONP/PRONPS/
PRONR/PRONP_AGG(_PRONPS) 0x20, CONG 0x1f, ECONG/CONGE 0x82, NEGAZ 0x2e,
SIGLAU 0x84, AVV(_CONG) 0x33, AGG(_PRON) 0x29, VERBO 0x2a, VERBOA 0x2b,
VERBOE 0x2c, NONVERBO 0x2f, SOST 0x15. The stress-grade pass 0x10008130
gives a word grade 3 (unstressed) when its class is one of 0x19 0x1e 0x18
0x20 0x21 0x16 0x22 0x1f 0x82 0x2e or its grade is already DEA, and it has
at most 7 letters, except `sie`/`Sie`, `mit`, `nicht` (written `sìe`,
`mìt`, `nìcht` in the module's marked spelling), which keep grade 0;
`es` is unstressed unless punctuation follows. `g2p::german_unstressed`
does this from the dictionary's own columns; the four little words the
dictionary doesn't classify (mein, meine, wegen, für) stand in for the
module's morphological analyser. FonAllofonoLingua (1 964 lines) rewrites
phones by the engine's feature codes (0x19e2 → 0x59f2 before 0x1942…);
not ported, our fitted allophone rules match it on both benchmarks.

Pauses are the original's defaults: `ShortPauseLength` 50 ms (`-`),
`MediumPauseLength` 120 ms (`,` `;` `:`), `LongPauseLength` 500 ms (`.` `!` `?`
newline) — its transcription confirms `#50` and `@500`.

## 4b. Italian FON, ported from LTTS7Italian.dll (`lang/italian.rs`)

With `LTTS7Italian.dll` in the voice's tree (`VoiceFiles.language_library`,
from the `.lde`'s `Library`) the Italian front end is the module itself,
ported: its tables are read from the DLL's bytes at the addresses its code
uses (`lang/common.rs` `Pe`), nothing is copied, nothing runs. The export
`Italian` (0x1000b640) walks a name → function table at **0x10031ff8**
(19 pairs): `GetVersion` 0x1000b550, `ELQGetGraphemes` 0x1000b520,
`ELQGetPhonemes` 0x1000b530, `LesAccenta` 0x10002aa0,
`LesNormalizzaAccento` 0x10002b50, `FonTrasParola` 0x1000f210,
`LesIniNumeri` 0x1000b4d0 (installs the number readers 0x10005680,
0x100049b0, 0x1000a0b0…), `LesAnalizza` 0x1000d890, `LesGetSymbolName`
0x100032a0, `LesGradiAccentuali` 0x1000b6b0, `LesConfiniProsodici`
0x1000c080, `LesPronunciabile` 0x1000b2e0, `FonTrasConfini` 0x1000f0e0,
`LesEspandiAbbreviazioni` 0x10007430, `ELQGetLicenseMessage` 0x1000b5a0,
`FonGetSamples` 0x1000b540, `LesGetAbbreviazioni` 0x1000b610,
`LesGetNumeroAbbreviazioni` 0x1000b600, `LesGetDefaultCurrency` 0x100074b0.
LoqTTS7.dll binds them by name (LES 0x10036140, FON 0x1002d5c0) and runs,
per sentence of 24-byte word records (+4 text, +0xe class, +0xf subclass,
+0x10 information, +0x11 boundary, +0x12 stress grade; ctx+8 the sentence
type: 0 `.`, 3 `?`, 5 `:` `;`, 6 `!`): LesAnalizza, LesAccenta,
LesConfiniProsodici, LesGradiAccentuali; then its phrase loop (0x1002c300)
calls FonTrasParola per word and, through 0x1002d060, FonTrasConfini at
every word start. Ported:

| function | address | what it does | tables |
|---|---|---|---|
| LesAnalizza | 0x1000d890 → 0x1000cdf0, 0x1000d180, 0x1000cb40 | classes: function words, neighbours (articles vs `lo`/`la` pronouns, 0x1000cf20), endings (verb forms, clitics 0x1000c820/0x1000c7b0) | function words 0x1006b3e8 (0x17b × 8 bytes); word forms 0x10066668 (0x675 × 12) |
| LesAccenta | 0x10002aa0 → 0x10002430, 0x10002790, 0x10002990 | marks the stressed vowel (grave, acute for closed e/o; code n = n-th vowel from the end, +30 acute) | homographs 0x1002bb78 (0x3b × 8); stress endings 0x10028078 (0x44a × 12, `{reversed ending, exception list, code}`, searched by 0x100010c0, exceptions 0x10001000); `-ano`/`-ino` lists 0x1002bef0, 0x1002bd50 |
| LesConfiniProsodici | 0x1000c080 → 0x1000baa0, 0x1000b970 | boundaries; a pause (0x0a, 50 ms) between groups when both sides have ≥ 2 words and ≥ 15 / 20 letters | — |
| LesGradiAccentuali | 0x1000b6b0 | short function words inside a phrase unstressed (grade 3) | — |
| FonTrasParola | 0x1000f210 → 0x1000db80, 0x1000dd10 (zz), 0x1000e5c0 (s), 0x1000ebf0 / 0x1000ef50 (i) | letters → phone codes | characters 0x10030ed8 (8 bytes each); codes 0x1006bf4c, 0x1006bee4, 0x1006be7c; whole-word exceptions 0x1006c138 (0x1ea74 bytes, `word=phones`, `ELQSearchCompactString` LTTS7Util.dll 0x100289f0); word lists at 0x1008abac…0x1008bf4c; symbols 0x100318c0 |
| FonTrasConfini | 0x1000f0e0 | across words without a pause: s → z before a voiced consonant, n → m/M/N | — |

Checked stage by stage against the original's own calls: `oracle/loqref.c`
`LOQ_TRACE_LES=1` (§8), every slot of the table at 0x10031ff8 swapped for a logging
thunk (records before and after each function, FonTrasParola's input and
output codes). On the 20 000 held-out words (`oracle/words/it.txt`, every
4th alphabetic word of LibreOffice `it_IT/it_IT.dic`) and the benchmark,
LesAnalizza, LesAccenta, LesConfiniProsodici, LesGradiAccentuali and
FonTrasParola give the original's records and codes for **every** call.

Whole transcriptions (`QQ_NO_LEXICON=1`, against the original's
`LOQ_FON_ONLY` output): benchmark `oracle/bench-it.txt` 96.6% → **100%**
of sounds (20 of 20 sentences identical, 3 before); held-out words 87.1% →
**99.9%** (19 980 of 20 000).

The engine's own tokenizer (LoqTTS7.dll TOP/LES) is approximated
(`Italian::sentences`): lower case, proper nouns (capitalised inside a
sentence: class 0x15), elisions as their own record joined to the next
(boundary 0), numbers read by `g2p::italian_number` (class 0x15,
subclass `z`, the last `y`), punctuation records (class `2`, `|` ends a
sentence, `}` inside). Not ported yet (§11): the engine's spelling of
unpronounceable words (`LesPronunciabile`: `CD` → `T$ i-dd i`), its
number/date/time/currency readers, abbreviations, `Italian.lex`, and
0x10001290 (stress of class `-` participles in sentences).

## 4c. English FON, ported from LTTS7English.dll (`lang/english.rs`)

One DLL serves three variants. Its export `English` (0x100019f0) takes the
first token of the descriptor's `Language` list that is `englishus`,
`englishgb` or `englishau`, and hands the engine that variant's
name → function table: EnglishUs **0x100290d0** (30 pairs), EnglishGb
0x100291c0 and EnglishAu 0x100292a8 (29 pairs each). There is no
LesAccenta. The module reads the language pack's `<MotherTongue>.dbl`
(`SELECT grafemica, fonetica, gradoaccentuale, grammaticale, gennum,
tempomodo FROM lessico`, 0x10001460) and runs its `.gpr` through
LTTS7GPR.dll `ELQGPRRun` (0x100016f0 → 0x10001fc0; the first rule with
context wins, 0x10001220, `gpr.rs`). The database lookup is LTTS7Util
`ELQDBTableFind` (0x10005df0): a binary search with ASCII-only case
folding (`ELQstricmp` 0x100285e0) over the rows in table order, widened to
every equal row. A record's `+0x14` is an `ELQList` of those rows
(`english::Rows`: adding a row makes it current only in an empty list;
deleting the head makes the new head current, 0x10008115).

| function | address (US / GB / AU) | what it does |
|---|---|---|
| LesGestisciMaiuscole | 0x1000c420 | lower case through the character table 0x10027d00 (8 bytes per character: lower case, flags 0x10 vowel, 0x20 upper) |
| LesAnalizza | 0x10011f90 = 0x100119c0 + 0x100103c0 | the rows and the class from `grammaticale` (0x1000f390), SUB from `tempomodo` (0x10010000), INFO from `gennum` (0x1000feb0), GRADE from `gradoaccentuale` (0x100100f0: ACC 0, ACD 2, DEA 3); `'s` looked up on the stem; homographs get `DB_…` classes 0xe8–0xed; words missing from the database get a class from the ending table 0x10040410 (0x270 entries, the same search as Italian); then context rules (`english/disambigua.rs`) |
| LesConfiniProsodici | 0x1000eb20 | a group state machine (0x1000e3c0) over classes and the fixed phrases at 0x1002b4d8/0x1002b528/0x1002b4f4, 0x1002b57c…, 0x1002b58c… (`ELQSearchString` 0x10028ca0); boundaries 5/3/1/9/0x16, 0x0b at `,` `-` `(`, by sentence type at the end; 10 (a pause) when both halves are long (0x1000dce0: ≥ 2 words and > 14 / 17 letters) and no `because…`/`after…` word (0x1002b458, 0x1002b488) precedes |
| LesGradiAccentuali | 0x1000da10 | rows that disagree on the grade: US 0x1000ce10, GB/AU 0x1000c830 (`english/omografi.rs`) pick one and delete the others (0x1000c4a0); otherwise the first row's grade, 0, punctuation 0x0f |
| FonTrasParola | 0x10018140 / 0x100178f0 / 0x10012170 | the first row with `fonetica` (symbols at most 5 characters, 0x1000bf90, through the variant's table 0x10043180 / 0x10042ec0 / 0x10042940), else the `.gpr`; `'s` added (0x10017a90 / 0x100174e0 / 0x10012000); a stress placed when there is none (0x10017fe0 / 0x10017790: after an s+stop cluster, a stop + r/l…, or the first vowel letter's stressed vowel 0x10017d00) or all but the first removed (0x10017b70 / 0x100172a0) |
| FonAllofonoLingua | 0x100182a0 / 0x100122c0 (GB and AU) | phone-list rewriting per phone (`english/allofono_us.rs`, `allofono_gb.rs`): glides between words, unreleased and flapped stops, syllabic n/l/r, dark l, weak forms, linking r (GB/AU). It reads the whole sentence: FON ctx+0x20 is the sentence's record count, the chunk descriptor's +4 its record count and +0x18 its first record |

Checked call by call against the original (oracle/loqref.c with
LOQ_TRACE_LES, LOQ_LES_DLL=LTTS7English.dll, LOQ_LES_TABLE per variant;
the test `lang::english::trace::english_trace` replays each stage on the
original's own input). On 20 000 held-out words per variant
(`oracle/words/en.txt`, `en-gb.txt`, `en-au.txt`: every 2nd/4th/2nd
alphabetic word of LibreOffice `en/en_US.dic`, `en_GB.dic`, `en_AU.dic`)
and on the benchmarks, every stage gives the original's records and codes
for **every** call, all three variants.

Not ported yet (§11): the text hooks the engine's tokenizer calls
(LesPronunciabile 0x10002e90, LesGetSymbolName 0x100021e0 / 0x100021a0,
LesNormalizzaAccento 0x1000c3b0, LesElaboraParola 0x1000a600,
abbreviations 0x100288e0, 0xfc pairs) and the number readers
(LesIniNumeri 0x1000beb0, slot 9 0x1000b540, with LTTS7Util `ELQNum*` in
`lang/elqnum.rs`). The **voice library's own phone stage** comes after
FON (LoqTTS7 0x1002c300 calls the voice's function at +0x18 per phone,
after the voice-phone mapping 0x1002da60 / 0x1003fdc0). It is
`FonVidIngloba`, `FonVidAnalisi2Sintesi`, `FonVidMetaFonema`,
`FonVidAllofonoAnalisi` and `FonVidAllofonoVoce` in LTTS7Allison.dll,
LTTS7Dave.dll, LTTS7Kate.dll, LTTS7Grace.dll, LTTS7Alan.dll… (Jorge,
Katrin and Bernard have all but `FonVidIngloba`; Roberto and Marcello have
none). It changes the phones that are said: Allison says `s d` for the
module's `s t`, `HTh u` for `th u`, and drops the `Hw`/`Hj` glides. The
module's transcription (`ttsPhoneticTranscription`, `.fon`) equals the
port; the phones in the events (`--compare` FON) do not until that stage
is ported.

## 4d. Spanish FON, ported from LTTS7Spanish.dll (`lang/spanish.rs`)

The export `Spanish` (0x1000d060) walks the descriptor's `Language`
names and hands the engine one of six function tables: castilian
0x100298b8, superspanish 0x10029720 (25 slots, no FonAllofonoLingua),
americanspanish 0x100297e8, mexican 0x10029988, chilean 0x10029a58,
argentine 0x10029b28 (26 pairs each). They differ only in
ELQGetPhonemes (phone tables 0x100292a0 / 0x10029288 / 0x10029290 /
0x100292b0 / 0x100292a8 / 0x10029298, `{entries, u16 count}`),
LesGetSymbolName, FonGetSamples and LesGetDefaultCurrency; the FON code
reads the variant number from the engine (`**(ctx+0x44)`: 0 castilian, 1
mexican, 2 argentine, 3 chilean, 4 american, 5 super). `MotherTongue`
SpanishEs/Mx/Ar/Cl/Am selects it. Slots: GetVersion 0x1000cfb0,
ELQGetGraphemes 0x1000c800 (characters 0x10028198, 8 bytes: lower, upper,
flags 0x01 letter, 0x10 vowel, 0x20 upper, 0x80 accented), FonTrasParola
0x1000d510, LesAccenta 0x10001600, LesPronunciabile 0x10003d70,
LesIniNumeri 0x10007ec0, LesAnalizza 0x1000c7d0, LesGetSymbolName
0x10003b60 (Spain) / 0x10003c20, LesGradiAccentuali 0x100081e0,
LesConfiniProsodici 0x1000a7f0, FonTrasConfini 0x100102d0,
FonAllofonoLingua 0x10010820, LesNormalizzaAccento 0x100017d0,
ELQLanguageLoadReadOnlyData 0x1000c990 (the queries below),
LesGetAbbreviazioni 0x1000ce90 / LesGetNumeroAbbreviazioni 0x1000ce40.

| function | address | what it does | data |
|---|---|---|---|
| LesAnalizza | 0x1000c250, 0x1000c4a0, 0x1000ac20 | each word's rows of the lexical database (`SELECT grafemica, fonetica, gradoaccentuale, grammaticale, specgramm, gennum, tempomodo FROM lessico`, `ELQDBTableFind`: ASCII-folded binary search, all equal rows, the record's +0x14 list); class from `grammaticale` (0x1000bb00), subclass from `tempomodo`/`specgramm`, gender/number from `gennum`; unknown words by ending (0x1000b4d0: infinitives, gerunds, verb, noun, adjective endings); nouns' gender by ending; `uno`/`-cientos` agree with the next noun | ending lists 0x100277e8 (0x1d), 0x10027860 (0x20), 0x10027688 (0x58), 0x100278e0 (0x2a), 0x10027988 (0x3d), `ELQSearchString` mode 8 |
| LesAccenta | 0x10001600 → 0x100014c0, 0x100013a0, 0x10001280 | words without a written accent get an acute on the stressed vowel: one vowel itself, else the ending table (the code Italian's port shares, `common::lookup`); `-mente` on its stem and `ménte`; graves become acutes | stress endings 0x100221b0 (0x28c × 12) |
| LesConfiniProsodici | 0x1000a7f0 → 0x1000a010, 0x10008b40, 0x1000a660, 0x100083e0, 0x10009100 | group kinds per word (a state machine over classes), boundaries 5/3/1/0x16, a pause (10) between two groups both longer than 5 words and 24 letters where allowed (not after `que`, before a relative, after a modal verb before a verb), 0x0b at commas, the sentence end by type | locutions 0x10025ffc/0x10026008/0x10026050/0x10026118, interjections 0x10025fd0/0x10025fd4/0x10025fd8; 0x15b modal verb forms from the immediates at 0x1000911b |
| LesGradiAccentuali | 0x100081e0 | punctuation 0x0f; short (≤ 5 letters) function words at boundaries 0 1 2 3 5 6 7 0x16, apostrophes and 0x8f forms unstressed (3), else 0; the word's rows of the other `gradoaccentuale` (DEA = 3) dropped (the list walk stops once its head is deleted) | — |
| FonTrasParola | 0x1000d510 → 0x1000d450, 0x1000d380 | a row with `fonetica` gives the word's phones (symbols split at space/`-`, ≤ 5 characters); else letters in context, per variant (c/z `Th` in Spain, `s` elsewhere; ll `Gl`/`Zg`/`Y`; y `J`/`Zg`; x, h, intervocalic h dropped but in Mexico; an unstressed word loses its first accent) | sets 0x1002a264, 0x1002a27c, 0x1002a238, 0x1002a22c, 0x1002a1fc, 0x1002a290 |
| FonTrasConfini | 0x100102d0 | the incoming word's doubled consonant dropped; across words i/u glides, voicing assimilation, n→m, Chilean/Argentine s voicing; the next word's d/g soften, a lone `y` is `j` | — |
| FonAllofonoLingua | 0x10010820 | per phone of the chunk: z→s, n/l before palatals, r: at a word start and after s n l, b→BH/p, seseo (Th→s, Mexico and America), ll→Y in Mexico, `tt Th` → `Ts` inside a word (but `etcétera`, deleting the entry) | — |
| LesNormalizzaAccento, LesPronunciabile, LesGetSymbolName | 0x100017d0, 0x10003c90, 0x10003690 / 0x10003ba0 | apostrophe accents; pronounceable when it has a vowel; letter names read from the switch tables (index bytes 0x10003a5c, jumps 0x10003934; outside Spain b v w first, 0x10003be8 / 0x10003bd8) | — |

Checked call by call against the original (LES tracer with all 26 slots
and FonAllofonoLingua's phone window): on the 20 000 held-out words of
LibreOffice `es_ES.dic` (Jorge) and `es_MX.dic` (Carlos) LesAnalizza,
LesAccenta, LesConfiniProsodici, LesGradiAccentuali and FonTrasParola give
the original's records and codes for **every** call. Whole
transcriptions (`QQ_NO_LEXICON=1`): held-out words 99.97% (Spain) and
99.96% (America) identical, the rest all-capital acronyms the engine
spells (§11); bench `oracle/bench-es.txt` 100% for Jorge, Carmen and
Leonor (99.8% before); `bench-es-am.txt` 90.0–91.2% (81.4–83% before),
every difference a number: the numbers are read by the module's
LesIniNumeri slot 9 (0x10007490) and its slots 0–6 through LTTS7Util's
ELQNum*, not ported yet (§11).

## 5. CAT — picking pieces (the original's rules, decompiled)

**What the original does** (LoqTTS7.dll): units are half-phones; the event
19 it sends per pick names the sentence and how many half-phones it took. Its
picker is **greedy, left to right**, not a global search:

| step | address | rule |
|---|---|---|
| driver | 0x10047ec0 | for each wanted half-phone: pick, then jump ahead by the count picked; at most 0x800 per phrase |
| per step | 0x10046de0 | ask every database of the voice, **last first** (Gilded phrase lists before the main one); a score ≥ 5 000 000 takes the whole matching unit; a pick ending on a left half takes the recording's right half too when it is the wanted one |
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

**Every database, last first.** A voice's `.vcf` lists `VocalDBName[0..]`:
the main recordings, then (Gilded voices) `<Voice>GildedPhrases` and
`<Voice>GildedParalinguistics`, each an `.sde` (with `A=` text lines and
`S=e`/`f`/`x` tunes) and recordings of their own. 0x10046de0 asks them last
first with one best score across all; once a database leaves the best at
5 000 000 or more (a whole-phrase match, 0x10045ad0 above) the rest are not
asked and the pick takes all of that recording (`2·phones − 4` halves).
`VoiceFiles::from_layout` loads them into `VoiceFiles.extra`;
`VoiceDatabase::add_database` appends their sentences to `units`
(`db_start`, `database_of`, `bank`), every phone's instances listing the
last database's first; a piece event's `<db>,<sentence>` id maps onto that.
Giulia, Luca, Matteo and Paola say "Attenzione!" with a Gilded phrase
(`1,25`): the earlier "piece from sentence 25 not found" was that id read as
a main-database sentence.

**Two formats of voice library.** Older libraries (Roberto, Allison,
Katrin, Bernard, Jorge…) hold their own feature words (`#` 0x18, `@` 0x28,
`^` 0x48), which the engine converts with 0x10050780; the newer ones
(Giulia, Luca, Steven, Carlos, Soledad, Florence, Ximena, Marcello…) hold
the engine's words already (`#` 0x13, `@` 0x17, `^` 0x10). The engine
converts only below a version (CatLoadVoiceReadOnlyData 0x1004bd60,
0x10051210); 0x10050680 tells them apart by the `@` record (0x28 → convert),
which `PhoneTables::from_library` does too. Converting a newer table a second
time scrambled every feature, class and similarity: that was the 8–20% CAT
of Steven, Carlos, Soledad, Florence, Ximena and Marcello, and their poor
replays (every voiced/voiceless decision at the joins wrong). Giulia's
tables now equal the original's dump (`LOQ_DUMP_TABLES`) in every feature,
class and similarity.

Half-phones from the same recording and place as the original's, given its
transcription (`--compare`; the original's pieces located by the ticks of
every half SIG played, `LOQ_TRACE_UNITS`, where the sounds alone were
ambiguous):

| voice | benchmark | similarity guessed | tables | + word-end marks | + every database, both library formats |
|---|---|---|---|---|---|
| Roberto | `oracle/bench-it.txt` | 39.6% | 84.8% | 94.5% | **100%** |
| Allison | `oracle/bench-en.txt` | 43.6% | 83.0% | 93.7% | **100%** |
| Bernard | `oracle/bench-fr.txt` | 43.8% | 80.5% | 97.9% | **100%** |
| Katrin | `oracle/bench-de.txt` | | | 98.2% | **100%** |
| Jorge | `oracle/bench-es.txt` | | | 98.9% | **100%** |

Every other voice with a benchmark: 100% except Afroditi 98.1%, Annika 99.6%,
Dave 99.1%, Dmitri 98.9%, Elizabeth 99.8%, Grace 99.3%, Krzysztof 99.5%,
Olga 99.3%, Simon 98.9%, Stefan 99.7%, Sven 99.2%, Zosia 98.4% (§11).
Giulia and Steven went from 10.8% and 15.6% to 100%.

(`--compare` gives the picker the original's own transcription; that string
glues some words together, `h `` `HAI `` HEh v s `` `E ``…, so word starts are
taken from ours where the sounds line up.)

Voices whose library isn't found fall back to the earlier approximation (100
same phone, 50 same without stress/length, no classes).

## 6. SIG — joining (the original's rules, decompiled)

The original handles **every half-phone as a unit of its own** (per-unit loop
`0x1001faa0`): decoded with **64 samples** of margin on each side
(`(rate/10·400)/20000`, 0x1001f580; no lead-in for a half at the very start
of a recording), in the order SIG plays them, then given its pitch marks
(0x1001e3a0). A half of no length (an `L` record of 0 ticks) is a unit too.
Half-phones that follow each other in one recording make a run (0x100472f0):
inside it they are copied (algorithm 0); the run's last half is joined to the
next unit (algorithm 1, `0x1001f8a3` → `0x10024660`). A pause is **two**
zero-filled half units of half its length each (no lead-in, a 4× = 256-sample
tail); the first is joined to the second, the second to nothing, so the
speech after a pause starts exactly as recorded. Each unit then plays its own
samples (`pre .. pre + len`). `synth::render` (`JoinStyle::Original`) does
all of this unit by unit; `sig::join` is 0x10024660:

- **Cross-fade** (unless both halves are voiced, flag +0x640; 0x100244a0):
  from `a.len − b.pre + 1 + a.pre` (unsigned, at most a's whole buffer) to
  the end of the previous unit's buffer, n points (127 between speech), at
  most 500 and no more than the next unit's buffer; window **type 2** of 2n
  points `0.5 − 0.5·cos(2π(i+1)/(2n+1))` (0x100241e0, case 0x100242ba), the
  previous unit on the falling half; the sum clamped to ±32767/−32768
  (0x100a5c20, 0x100a5c40), truncated (`_ftol2` 0x10069060) and written
  into both units.
- **Voiced join** (0x10024660; both halves' marks, at most 50 periods, two
  or more each side, each frame under 1000 samples; anything refused leaves
  both units as they were, without a cross-fade): every period with the
  unit's margins becomes a frame. Loudness: √Σs²/count per frame (`fsqrt`
  0x10069600, stored as a float); each side's mean summed in floats; when
  the first is at least the second its periods get `max(0.5, float(k·(mB/mA
  − 1)/float(na−1) + 1))`, else the second's `max(0.5, float(j·(1 − r)/
  float(nb−1) + r))`, r = mA/mB — every product and quotient a double, as the
  x87 at precision 53 computes them. Periods: `x = float(float(na)/total)`
  kept in [0.2 (0x100a5cd0), 0.5]; when the mean periods (summed in floats,
  0x10024590) differ by 0.5–14 semitones (`|ln(pB/pA)·12/ln 2|`, 0x100a5b80,
  0x100a5cc0) the first side's become `trunc((first_b − last_a)·x·(k/
  float(na−1)) + p)`, the second's `trunc((last_a − first_b)·float(1−x)·
  ((total−j−1)/float(nb−1)) + p)`, undone when Σ float(|Δ|/p)/total > 0.1
  (0x100a5cb8). Overlap-add (0x10024360): frame 0 copied; each next frame's
  samples × its gain (a double, at most 32767, `modf` rounding half up for
  positives, 0x10059ce0), the output trimmed by the previous frame's tail,
  then laid from `len − pre + d` (d the period change, dropped when within
  ±1 or when it would empty the frame): the overlap cross-faded with the
  same window, truncated **without** a clamp, the rest appended (more than
  500 to overlap: the frame is dropped). A start past `2·(a.len + b.len)`
  abandons the join. The two units get their new lengths and marks
  (`M[i] − M[0]`, `M[na+i] − M[na]`) and their buffers back out of the
  joined one, margins included (a later join reads them).
- `cos` is the runtime's SSE2 routine (0x10069510 → 0x1006b13e, taken with
  SSE2 present and every exception masked): x·32/π rounded, a 64-entry
  table of sin/cos parts (0x100a9f70) and two polynomials in double
  arithmetic; `sig::msvc_cos` is that routine operation by operation. 2π is
  the DLL's `6.28318530717958` (0x100a5be0), 7 ulps under the true one:
  with the true 2π or the platform's `cos`, single samples of cross-fades
  came out 1 off.

Checked with `oracle/loqref.c` `LOQ_TRACE_JOIN=2` (every join's two units
before and after, every sample): given the original's units, `sig::join`
returns the same samples, lengths and marks for 1 080 of the 1 082 joins of
the five benchmarks (both residuals in §11), and `sig::pitch_marks` the same
marks for 422 of 423 voiced halves.

Match with the original's audio, its own pieces (`LOQ_TRACE_UNITS`: every
half SIG played, by its ticks) through our SIG, whole files (hard cuts:
samples within ±2; after: samples identical):

| voice | hard cuts | cross-fades and voiced joins on pieces | per-half units, exact joins |
|---|---|---|---|
| Roberto | 26.0% | 77.8% | **100%** |
| Allison | 33.2% | 74.3% | **100%** |
| Katrin | — | 70.8% | **100%** |
| Bernard | 35.0% | 98.3% | **100%** |
| Jorge | — | 88.8% | **100%** |

and so are all the other 32 kHz voices of the pack with a benchmark (Gilded
databases included, §5): Afroditi, Amalia, Annika, Carlos, Carmen, Dave,
Diego, Elizabeth, Esperanza, Eusebio, Felipe, Fernanda, Francisca, Frida,
Gabriela, Giulia, Krzysztof, Leonor, Luca, Matteo, Paola, Simon, Soledad,
Stefan, Steven, Susan, Sven, Zosia. The 44 kHz CodecMsx voices are not yet
(§2c, §11): Alan 80.2%, Dmitri 78.6%, Florence 93.0%, Grace 79.4%,
Marcello 63.3%, Olga 79.0%, Willem 86.1%, Ximena 82.1%.

**Smoothed** (my own, `JoinStyle::Smooth`): cuts in mid-phone, slides the new
piece up to ±5 ms to the best waveform match and cross-fades over 5 ms.

## 7. Command line

- `quoquendo --install "<Voice> - <Language>.exe"` (or a folder of
  installers) takes a voice straight from Loquendo's installer (Inno Setup
  5.2.3) into the app's own voice folder, `~/.local/share/quoquendo/LTTS7`
  (`$XDG_DATA_HOME`, `%APPDATA%` on Windows). Only the voice's files are kept:
  `data/<Voice>.vcf`, `data/voices/<Voice>/` (`.sde` index, `.bin`
  recordings, `.vde`, Gilded lists) and `bin/LTTS7<Voice>.dll` (its phone
  table). The installer is only read (`inno.rs`, no other program); nothing
  is run. Its layout: the loader's offset table (magic `rDlPtS` CD E6 D7 7B
  0B 2A; header offset at +32, data offset at +36); at the header, the
  64-byte version string then two blocks (CRC, stored size, compressed flag;
  4 KiB pieces each after a CRC; LZMA with a 5-byte properties header and
  no end marker): the setup entries, and the file locations (70 bytes each:
  chunk start +8, offset in the chunk +12, size +20, compressed size +28,
  flags +68, bit 4 = x86 calls filtered, bit 7 = compressed). File entries
  are source, destination, font and six condition strings, two Windows
  versions (20 bytes), then location, attributes, external size,
  permissions, options, type (23 bytes). Chunks start with `zlb` 1A; the
  call filter makes E8/E9 operands with a 00/FF high byte absolute in each
  64 KiB block, undone by subtracting the position after the operand. All
  files of Katrin's and Raffaele's installers come out byte for byte as
  innoextract writes them. `VoiceFiles::from_installer` loads a voice
  straight from the installer's bytes. Typing an installer's path in the window's folder box does the same.
  Voices from there and from any other installation found are listed together.

- `quoquendo --say "testo" [--voice Roberto] [--out file.wav]
  [--fon rules|original|<command>] [--cat builtin|original] [--join original|smooth]`
- `quoquendo --compare <file|text>` scores every stage against the original
  engine and writes `oracle/out/compare/<n>-original/-replay/-ours.wav`.
- `quoquendo --fon-list <words>` prints our transcription per line.
- `quoquendo --check-decode <prefix>.dec [--voice <v>]` replays the
  original's decoder requests (`LOQ_TRACE_DECODE`, §2d) through ours.
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
join's units before/after and every half-phone's pitch marks),
`LOQ_TRACE_DECODE` (every decoder request and its samples, §2d),
`LOQ_TRACE_LES` (`oracle/lestrace.h`: every slot of the language module's
function table swapped for a logging thunk — each LES function's word
records before and after, FonTrasParola's word and codes, FonTrasConfini's
arguments — to `<prefix>.les`; `LOQ_LES_DLL`/`LOQ_LES_TABLE` for another
module than Italian's; `oracle/parse_les.py` turns it into JSON lines,
§4b). quoquendo's counterparts: `QQ_TRACE_CAT=1`,
`QQ_CHECK_MARKS=<trace.join>`, `--check-decode <trace.dec>`,
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
| output resampler | 0x10028ee0 | resamples when voice and output rates differ by more than 100 Hz | ported for the 44 kHz CodecMsx voices (§2c); 32 kHz voices pass through |

So for normal speech the original copies recorded runs untouched, which is why
pieces match at ~70 dB. The few-sample cut shifts are still unexplained by
these functions; they are not caused by PSOLA, which only runs for pauses or
when a speed/pitch change asks for it.

## 11. Not done / unknown

- An exact port of French letter-to-sound (FonTrasParola, LTTS7French.dll
  0x1000f730, 3 570 lines) instead of fitted rules (97.9%); German loanword
  stress (97.6% held out).
- Italian (§4b): the module's LES/FON functions are ported and match the
  original call for call; the 20 held-out words still differing are all
  spelled out by the engine (`AN` `` `a-`E n: e ``, `CD`, `CPU`, `DNA`,
  `b`, `dg`…): LoqTTS7.dll's TOP/LES spelling of words LesPronunciabile
  (0x1000b2e0 → 0x1000b140) rejects is not ported, nor its number, date,
  time, currency and abbreviation readers (LesIniNumeri's 0x10005680…,
  LesEspandiAbbreviazioni), the `Italian.lex` lexicon (`CA FE DB 60`, not
  decrypted yet) and 0x10001290 (stress of class-`-` participles inside
  sentences; alone such words take the noun class and match). The fitted
  rules (`g2p::italian`, `it-position.tsv`, `it-quality.tsv`) remain as
  the front end of Italian voices whose tree lacks LTTS7Italian.dll.
- The 44 kHz voices' output resampler as the engine streams it (§2c): SIG
  hands VoiceResamplerRun (0x10028ee0, called at 0x10015c77) one chunk per
  wanted phone, both its half units as the joins left them (a pause too:
  `LOQ_TRACE_RESAMPLE` on Marcello, every chunk = the two halves' lengths);
  0x10028c00 asks for `floor(pending·32/44)`, minus 16 when above 16, and
  keeps the input not consumed; 0x1001c370 (Smith's `resample`, 4096-sample
  input buffer refilled from Xoff = 55 each call, `Time` set on the first
  call only, readData 0x1001c030, SrcUD 0x1001be70) carries the rest over.
  `codec::StreamResampler` is a first port (render feeds it phone by phone):
  the first chunks of Marcello's first sentence come out identical, then a
  few samples at a chunk end differ and the stream drifts (+8 samples over
  the sentence). Unknown still: how many outputs one pass may produce (the
  value 0x1001c370 truncates on entry, our guess `(4096 − 2·Xoff)·factor`),
  what the end-of-utterance call (0x10028eb4, flag 1) adds, and LpScl (the
  caller truncates `factor·14746`, 0x1001c930). Whole files 63–93%
  identical, as before.
- Two joins of 1 082 traced (Roberto, Bernard) differ in the next unit's
  tail margin only: the joined buffer is shorter than `b.pre + b.len +
  b.post` and the engine copies the rest out of its scratch memory
  (0x100b747c/0x100b74b4, the buffer reused across joins), whose stale
  contents we fill with zeros. Neither reaches the output on the benchmarks
  (replays 100% identical). One voiced half of 423 (Bernard) gets other
  pitch marks than the original's, also without effect there.
- CAT given the original's FON: 100% on 30 voices; Afroditi 98.1%, Annika
  99.6%, Dave 99.1%, Dmitri 98.9%, Elizabeth 99.8%, Grace 99.3%, Krzysztof
  99.5%, Olga 99.3%, Simon 98.9%, Stefan 99.7%, Sven 99.2%, Zosia 98.4% not
  yet diffed term by term (`LOQ_TRACE_CAT` against `QQ_TRACE_CAT`). The
  original's automatic pauses (TOP/LES) are a FON matter: given its FON the
  picker already gets them as pauses. Speed and pitch controls.
- The app's piece player (`src/app.rs`) and the JNI `analyse` read pieces
  from `db.audio`; pieces from a Gilded database need `db.bank(unit)`.
- Four voices have no LTTS7 language module in any installer of the pack:
  Jordi (Catalan), Empar (Valencian), Carmela (Galician), LinLin (Chinese).
  The original refuses them too: with the voice unpacked into a full engine,
  `ttsLoadPersona` fails with 12 "Invalid function or API parameter" (Jordi,
  LinLin). The Loquendo 6 pack's installers carry LTTS6 modules for Catalan
  (`LoqCatalan6.1.dll`, `Catalan6.1.phd`/`.lex`) and Mandarin
  (`LoqChinese6.2.dll`, `Chinese6.2.phd`), a source to port from; none for
  Galician or Valencian was found.
- Spanish numbers (§4d): the engine reads a number token with the module's
  LesIniNumeri slot 9 (0x10007490, `Les::leggi_numero`), which calls back
  its slots 0–6 (word by id 0x10003470, phone numbers 0x10002980, cardinal
  0x10006e10, digit 0x10006dd0, by-digits 0x100029d0, currency
  0x10005f10, decimals 0x10007d00) through LTTS7Util's ELQNum*
  (`lang/elqnum.rs`, `NumSlots`). Not ported: `Spanish::number` reads the
  digits one by one with the slot-0 words, so `bench-es-am.txt`'s two
  number sentences (2475, 15 de marzo de 2024) are its only differences
  (Carlos 91.2%); `oracle/bench-es-text.txt` is not written yet.
- Spanish all-capital acronyms (`ABS`, `CEE`, `PNV`, `UGT`): the original
  spells them letter by letter (`` `a-BH `e-`e s e ``); our tokenizer reads
  them as words. These are the 6–7 held-out words of 20 000 left (99.97%
  Spain, 99.96% America); the module's own LesPronunciabile and
  LesGetSymbolName are ported, the engine's acronym decision is not.
