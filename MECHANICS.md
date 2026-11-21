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

**As the engine streams it.** SIG hands VoiceResamplerRun one chunk per
phone (call at 0x10015c77): both its half units as the joins left them, a
pause too. The first chunk of an utterance makes the state (0x10028ac0;
the module's reset 0x10015c30 drops it after each utterance): factor =
to/from (32000/44000, 0x10028bb3) and Smith's object (0x1001c6d0): the
input history `X` of 0x2800 bytes (5120 samples), zeroed, the factor at
+0x20, 1.0 at +0x30, "first call" at +0x2c, large filter on, linear
interpolation instead when upsampling (+0xe, 0x1001c0e0). Every chunk:

- is appended to the input waiting (`ELQBufferCat`, count at +0x18) and is
  owed `floor(n·factor)` outputs in the output queue (chunk +0xc,
  0x1002904a; queue 0x10023e20, add 0x10023e80);
- 0x10028c00 asks for `floor(pending·factor)` outputs (floor 0x10069b40),
  16 fewer when above 16, from 0x1001c930 → 0x1001c370 with `LpScl` =
  trunc(1.0·14746), `Nmult` 65, `Nwing` 8192; the input consumed leaves
  the buffer (`ELQBufferLeftShift`), the outputs join the queue
  (0x10023ec0), and the queue hands each phone on, first in first out,
  once its owed samples are there (0x10024010, `ELQBufferPop`).

0x1001c370, one call: at most `trunc(4096·factor)` = 2978 outputs per pass
(0x1001c377; 4096.0 at 0x100a5c00), `LpScl` = trunc(14746·factor + 0.5) =
10724 when factor < 1, `Xoff` = trunc((65 + 1)·0.5·max(1, 1/factor) + 10)
= 55 (10.0 at 0x100a5b30); `Time` = `Xoff` << 15 on the first call only
(+0x2c, cleared by 0x1001c930 at 0x1001c9ea). readData (0x1001c030)
reads the input from its start on every call (position +0x38 reset on the
call's first read) into `X` from `Xread` (`Xoff` at first) up to 4096, so
past the new data `X` holds what earlier calls left there. Each pass asks
SrcUD for what is still wanted, or `trunc(4096·factor) − trunc(2·Xoff·
factor)` = 2898 when `trunc(4096·factor) − 2·Xoff·factor` is below that
(0x1001c4d9); SrcUD (0x1001be70) makes exactly that many, whatever input
there is, and reports by how many whole samples `Time` moved, 16-bit. `Time`
goes back by that, `X` moves down by it (`X[i] = X[i + Nx]` for
`Xoff − Nx + 4096` samples) and the next read goes to `4096 − Nx`. The
last sample readData reports only stops a call that has no sample past
`Xoff` (Smith's `last`, kept as the DLL keeps it). Each call's input
therefore runs about 22 samples short of what its last ~16 outputs read:
those are computed with whatever `X` held there.

**The end of a phrase** (module method 0x10015bd0 → 0x10028e40): with input
waiting (else nothing), `ELQBufferEnlarge` (no effect on the samples), one
more call of 0x10028c00 with its flag set, which hands every phone still
owed what there is (`ELQBufferPopForced` at 0x10023f20 takes at most what
is made) and drops what is left (`ELQBufferClean`, 0x10024190); then the
input waiting is dropped. `X` and `Time` carry on into the next phrase. So
each phrase comes out exactly `Σ floor(nᵢ·factor)` samples long whenever
the resampler made that many.

Every floating-point step (factor, products, floors) is a double under the
engine's precision control (53 bits) and every conversion a truncation
(control word | 0xc00, `_ftol2`), so `codec::StreamResampler` computes them
in `f64` with `as` conversions. `oracle/loqref.c` `LOQ_TRACE_RESAMPLE=3`
logs every call with the state before and after (§8);
`codec::tests::replays_the_engines_resampler` (`QQ_RESAMPLE_TRACE=<prefix>.res`)
replays every call from the engine's state and as one stream (Marcello,
three lines of `bench-it`: 184 calls; Florence `bench-fr`: 312; Grace
`bench-en-au`: 437; Dmitri `bench-ru`: 465), all identical, and what the
queue hands on equals the engine's `.wav` on every utterance.
`synth::render` feeds it phone by phone and flushes at
every phrase's end. Replaying the original's own pieces through our SIG,
samples identical:

| voice | before | after |
|---|---|---|
| Marcello (`bench-it`) | 63.4% | **100%** |
| Alan (`bench-en-au`) | 80.2% | **100%** |
| Grace (`bench-en-au`) | 79.4% | **100%** |
| Dmitri (`bench-ru`) | 78.7% | **100%** |
| Olga (`bench-ru`) | 79.1% | **100%** |
| Willem (`bench-nl`) | 86.3% | **100%** |
| Florence (`bench-fr`) | 93.1% | **100%** |
| Ximena (`bench-es-am`) | 82.2% | **100%** |

Where FON and CAT are the original's too (Marcello, Florence, Alan, Grace,
Ximena), `quoquendo --say` writes the original's `.wav` byte for byte on
every line of their benchmarks. `codec::resample` (the whole utterance at
once) remains for smoothed joins and the inside-pieces score.

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

The original's FON is code in each `LTTS7<Language>.dll`; for the ported
languages it is read from there (below and §4b–§4d).

**The languages' own data** (read from the installer's files: the `.dbl`
by `g2p::Frontend::from_files` for unported languages, both by the ported
modules):

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

Italian, English, Spanish, French and German have **no built-in rules**:
their FON is the original's module, ported (§4b–§4d, `lang/`), reading its
tables from the language pack's DLL. The rules fitted to the original's
output that stood in for them (`g2p::italian`/`english`/`french`, the
learned Italian ending tables, `french.rs`, `german.rs`, `spanish.rs` and
their connected-speech passes) are deleted. A voice of these languages
whose tree lacks the module (no `.lde`, or a `Library` DLL that is missing
or isn't the expected one) reports `language_missing` (`Voice::from_bytes`)
instead of speaking with approximate rules. Greek (§4f), Portuguese (§4g),
Dutch (§4i), Swedish (§4j), Danish (§4k), Polish (§4l) and Russian (§4m)
are ported the same way.

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
`es` is unstressed unless punctuation follows. The deleted fitted rules
did this from the dictionary's own columns; the four little words the
dictionary doesn't classify (mein, meine, wegen, für) stood in for the
module's morphological analyser in the deleted fitted rules.
FonAllofonoLingua (1 964 lines) rewrites phones by the engine's feature
codes (0x19e2 → 0x59f2 before 0x1942…): `lang/german/allofono.rs`.

**German numbers** (`lang/german/numeri.rs`). `LesIniNumeri` 0x10007100
installs slot 0 word by id 0x100026a0 (0x53 `{id, char*}` pairs at
0x10027798, `""` 0x10026b34), 1 phone number 0x10002620 (area-code length
0x100025a0: `ELQSearchString` flag 2 over 0xf7 codes at 0x10027318, then
`ELQNumConvertPhoneNumber` steps 0 and 1), 2 cardinal 0x10006590, 3 digit
0x10006550 (`null`/`eins`, else 0x100056a0: words of 1–20 and the tens), 4
by-digits 0x10002670, 5 currency name 0x10005490 (8 × `{code, unit, units,
cent, cents}` at 0x100276f8), 6 decimals 0x10006f40, 7 date extraction
0x100026d0 (two-digit years from 80, from 0 after `.`), 9 the reader
0x10006c20 (slot 8 is empty). Slot 9 makes the reading mode the engine's
kind (5/6 ordinal → 4 with +0x48 5/6; 0xc when a currency sign is next to
the number or `euro euros dollar pfunde usd gbp jpy pfund eur yen` follows)
and tries nine recognizers, the mode's own first (0x10005f20: mode 1→6,
2→8, 4→2, 7→1, 9→4, 10→7, 11→5, 12→3): date 0x10003cb0 (slot 7 in the
`dmy` order, the day dative when the token before starts one of `am vom
zum bis im dem den des einen einem` 0x100272f0, by 0x100024a0; else
0x10002e80: a day before a month name 0x10027a30 and a year after it — one
or two tokens used — or a four-digit year after `bis`, after `bis im jahr
jahre jahren seit von zwischen` 0x10027b28, after `und`/`bis` and a number,
or after a month name), time 0x10003ec0 (`ELQNumIsTime`), ordinal
0x10004050, money 0x100042b0 (a currency name after the number becomes `n`
and is used), real 0x10004630 (mode 9 only), fraction 0x100048e0
(0x10001b40's 14 fractions), phone 0x10004a00 (over six digits starting
with an area code), integer 0x10004df0 (up to five digits, a decimal comma,
dot groups), code 0x10004c30 (mode 2). The renderer 0x10006000 reads kind
0 by `ELQNumConvertRealNumber` (longer pieces by slot 1 in pairs), 4 by
0x10001e30 (ordinals 1–19 from a table, else the cardinal + `ste`), 7 by
0x100051b0 (`h Uhr mm`, ` minuten ` ` sekunden `), 8 by 0x10005c50 (day
ordinal, month word 0x51–0x5c, year `neunzehn hundert neunzig` /
`zweitàusend` / an integer after 2000), 0xb by 0x10001740, 0xc by
0x10002740. The cardinal goes position by position: `eintàusend` for 1xxx,
`einhundert`, `eine Milliòn`/`Milliònen`, `eine Milliàrde`/`Milliàrden`,
`ein und zwanzig` (0x10005b20), a trailing ` # ` dropped.

**French numbers** (`lang/french/numeri.rs`). `LesIniNumeri` 0x10006480
installs slot 0 word by id 0x10001db0 (0x53 `{id, char*}` pairs at
0x1001d020, `""` 0x1001c3f2), 1 phone number 0x10001d30 (area-code length
0x10001cb0: `ELQSearchString` flag 2 over 0xf7 codes at 0x1001cba0), 2
cardinal 0x10005730, 3 digit 0x100056f0 (`zéro`/`un`, else 0x10004970:
words of 1–20 and the tens), 4 by-digits 0x10001d80, 5 currency name
0x10004760 (8 × `{code, unit, units, cent, cents}` at 0x1001cf80), 6
decimals 0x100062c0 (always flag 0), 7 date 0x10001de0 (two-digit years
from 80, from 0 after `/`), 9 the reader 0x10005db0 (slot 8 empty). The
reader is German's in shape: kind 5/6 → mode 4 with +0x48 5/6; mode 0xc
when a currency sign is next to the number or the next token is exactly
(case-sensitive) `euro dollars dollar eur usd USD JPY jpy gbp GBP yen EUR`
(0x1001c428 0x1001c458 0x1001c460 0x1001d704 0x1001d6fc 0x1001d700
0x1001d6f8 0x1001d6f4 0x1001d6d8 0x1001d6dc 0x1001c40c 0x1001d708) or
`livre` 0x1001d6ec with `sterling` 0x1001d6e0 three tokens on. The order
0x10005050 (mode 1→6, 2→8, 4→2, 7→1, 9→4, 10→7, 11→5, 12→3): date
0x10002ca0 (slot 7, or a VoiceXML date in mode 8 with order `b`), time
0x10002e00 (an hour 1–24 before `h` 0x1001d6c8: `:` and the minutes after
it joined to the text when a number below 60 not followed by `h` comes
next — two tokens used — else one; or `ELQNumIsTime`), ordinal 0x10003120
(mode 4, or before `ème éme eme è e`, used, +0x48 5), money 0x10003330
(a currency sign or name after the number: `€` 0x1001c430, `$` 0x1001c468,
`¥` 0x1001c410, `£` 0x1001c444; the next token becomes `n`, for `livre
sterling` and `GBP` the one after it), real 0x10003870, fraction
0x10003b20 (0x10001320), phone 0x10003c40 (no `:/-+` 0x1001d70c, over six
digits, an area code), integer 0x10004050, code 0x10003e80. The renderer
0x10005130: kind 0 `ELQNumConvertRealNumber`, 1 slot 1, 2/3/10/9 a sign
(`moins`/`plus` + space) and the integer, decimals after ` virgule ` by
slot 6; 4 0x10001610 (1–20 from a table, `premièr` + `e` unless a suffix
was read, else the cardinal + `ième` 0x1001d3bc), 7 0x10004430 (` heure `
0x1001d720 / ` heures ` 0x1001d714, ` minutes `, ` secondes `), 8
0x10001e50 (day cardinal, month word 0x51–0x5c, year), 0xb 0x10001000 (14
fractions, `1/2` read `midi`), 0xc 0x10001fa0 (units, the currency name,
two-digit cents as a bare cardinal or `et` + digit + cent name; `une` for
£1). The cardinal: hundreds 0x10004b90 (`cent`, `deux cents`, `trois
cent un`), two digits 0x10004e00 (`vingt et un`, `soixante et onze`,
`quatre vingt un`, `quatre vingt dixneuf`), `mille`, `un million` /
`millions`, `un milliard` / `milliards`, a trailing ` # ` dropped.

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
99.9% → **100%** (20 000 of 20 000) with the text hooks below.

The engine's own tokenizer (LoqTTS7.dll TOP/LES, `lang/common.rs`) calls
the module's text hooks, ported in `italian.rs`:

- `LesPronunciabile` 0x1000b2e0 → 0x1000b140: a word with `'` or an
  accented vowel reads; otherwise, lowered through the character table
  0x10030ed8, one letter `y`/`j` is spelled; a word without a vowel
  (0x100032e0) reads only if listed at 0x1002c368 (0xd bytes); words of
  2, 2–3, 2–4 letters listed at 0x1002c644 (4), 0x1002c648 (0xbd),
  0x1002c708 (0x1f) are spelled; with the engine's char argument 0 (as
  LoqTTS7 0x1003824d passes it) a word of 4+ characters, or one with no
  upper-case letter after its first (0x10003310), reads; else by length:
  1 a/e/o/i, 2 the list 0x1002c378 (0xb2), 3 0x10003390 (list 0x1002c430,
  0x141, then consonant/vowel shapes, `sas` 0x1002fb14), longer 0x10009eb0
  (list 0x1002c578, 0xc9; no letter thrice; every run of 2+ consonants by
  0x100074c0 — pairs at the start, end, inside via 0x1002c9b8 (0xb8),
  triples 0x1002c768 (0x249), quadruples 0x1002c728 (0x3d), `stscr`;
  every run of 2+ vowels by 0x10007ab0, 0x100038b0, 0x100039f0). All
  ELQSearchCompactString lists flags 1, no separator.
- `LesGetSymbolName` 0x100032a0 → 0x10002dd0: the switch (index bytes
  0x10003198 for `c + 0x80`, jumps 0x10003078) is read and its cases run
  (`mov eax, name; ret`, behind `test al, al` when only spelling names
  punctuation): `b` → `bì`, `h` → `àcca`…
- `LesNormalizzaAccento` 0x10002b50: ` ' ’ ´ after a vowel becomes its
  grave (0x100021c0, for `) or acute (0x100022e0) accent; one before a
  final `s` is dropped; then á í ú become grave.
- `LesGetAbbreviazioni` 0x1000b610: `{abbreviation, expansion}` pairs at
  0x100319b8, count 0xc6 from LesGetNumeroAbbreviazioni 0x1000b600
  (`km` → Chilometri, `etc` → Eccetera…).
- `LesEspandiAbbreviazioni` 0x10007430: `s.`/`S.` (0x1002fd40, 0x1002fd3c)
  before a name in the saints list 0x1002d668 (0x2217 bytes, `name=letter`)
  → the title of its letter (index 0x10007410, jumps 0x100073f0: san,
  sant', santa, santo, sacro, santi, santissimo).

`Italian.lex` (its `"cpu" = "Sì Pi U"` among others) is the engine's text
filter (`common`). 0x10001290, the stress of class-`-` participles (35
exception lists 0x1002b3f0–0x1002bb6c, `exceptions` codes 0x4080–0x60c4,
0x10001250 stripping 1–6 letters before the stress table), is ported as
`Italian::participle`.

**Numbers** (`italian/numeri.rs`). `LesIniNumeri` 0x1000b4d0 fills slots
0 word 0x10005680 (0x53 `{id, word}` at 0x1002d3d0), 1 phone 0x100049b0
(area code by 0x100044a0, then `ELQNumConvertPhoneNumber`), 2 cardinal
0x1000a0b0 (hundreds 0x100092e0, two digits 0x10009560, number word
0x100090c0; groups `mille`/`mila`, `unmilione`/`milioni`,
`unmiliardo`/`miliardi` followed by word 0 ` # `, dropped at the end), 3
digit 0x1000a070, 4 by digits 0x10004a10, 5 currency 0x10008eb0 (18
`{code, unit, units, cent, cents}` at 0x1002d268), 6 decimals 0x1000b310,
7 date 0x100056b0 (`ELQNumExtractDate`; two-digit years from 80 in the
1900s unless written with `/`) and 9 the reader 0x1000a810. Slot 9 maps
the engine's type to a mode (5/6 → 4 with +0x48 5/6), forces mode 12 when
the next token is a currency symbol or one of 34 names/codes
(0x1002c310…, 0x1002fcf8…) or the previous one a symbol, copies the token
into the 0xd4-byte object, and runs the recognizers in 0x1000a730's
order (the mode's own first, jumps 0x1000a7d4): date 0x100058a0, time
0x10005a00 (`h.mm`, `h:mm[:ss]`, `3,14` is a time too), ordinal
0x10005cc0 (`°`/`º` next but `°C`, or an apostrophe), currency
0x10005eb0 (symbol before/after, or a name after, which becomes `n` or
empty; ctx+0x18 = 1), type 9 0x10006990, fraction 0x10006c40 (14 known,
0x1002fc10…), phone 0x10009770 (mode 0: the state machine 0x10008360 joins
following digit groups, prefixes 0x1002ca70 country / 0x1002cd5c free
phone / 0x1002cd78 area / 0x1002d1e8 mobile, `ELQSearchCompactString`
mode 2; ctx+0x18 = the groups used; token 0's type set to 1), cardinal
0x10006f30 (up to 5 digits, dotted groups, decimals) and code 0x10006d60;
each failing one restores the token by `ELQNumSetDefaultType`. The renderer
0x10009980 (jumps 0x10009e7c) switches on the kind: 0
`ELQNumConvertRealNumber` (so 6+ digit numbers are read in pairs), 1 phone
0x10004550, 2/3/9/10 integers and `virgola` decimals, 4 ordinals 0x100040a0
(stems 0x1002fc80…, `esim` 0x1002fc14, `o`/`a`), 7 times 0x10008020
(`una`, `e`, ` minuti `/` secondi `), 8 dates 0x10005720 (`primo `, months
word 0x50+m), 11 fractions 0x10003a90, 12 money 0x10004a40 (ranges,
`uno`/`una`/`un`, `e … centesimi`, `zero … di euro`). Replayed on the 426
slot-9 calls traced on `oracle/bench-it-text.txt` and 0…999 999 999 999
plus odd forms (Roberto): all equal in reading, ctx+0x18 and rewritten
tokens. `bench-it-text.txt` whole lines: 5 → 18 of 30 identical (with
0x1000cdf0 classing `e`/`un`/`di` of a reading as function words); the
rest are abbreviations, acronyms, roman numerals and `%` (§11).

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

**Numbers** (`english/numeri.rs`). `LesIniNumeri` 0x1000beb0 fills slots
0–9: word by id 0x10004c50 (0x5b `{id, word}` at 0x1002a088), digits with
`oh` 0x1000b300, cardinal 0x100094a0 (to 15 digits; hundreds 0x10008650,
two digits 0x100088d0, 1–90 words 0x10008430 = index bytes 0x100085e8 →
jumps 0x10008578 → `push id`; " and " 0x1002a8d4 after a hundred, a
trailing ` # ` dropped), digit 0x10009450, by-digits 0x10004c20 (flag, >15
digits, >3 with leading zeros, one digit), currency names 0x100089d0 (12 ×
`{code, unit, units, cent, cents}` at 0x10029f98), decimals 0x1000b3b0,
date extraction 0x10004c80 (ExtractDate step 0 + 1), `oh` 0x10002ee0, and
the reader slot 9 0x1000b540. Slot 9 picks a mode (the engine's type;
0 with `%` next → 10/11 by the fraction test 0x100033f0; 5/6 → 4 with
+0x48 5/6; 0xc when a currency symbol or code is next or a symbol
before), copies the token into the 0xd4-byte object and runs the
recognizers 0x10009150 (ordinal 0x10005380, date 0x10004d30, time
0x100050d0, currency 0x10005590, type 9 0x10005c80, fraction 0x10005f40,
phone 0x10006100, cardinal/decimal 0x100068b0, code 0x10006690; the mode's
own moved first), each swapping `.`/`,` and returning false through
`ELQNumSetDefaultType`; the renderer 0x10009ea0 switches on the kind:
0 `ELQNumConvertRealNumber`, 1 slot 1, 2/3/9/10 integers and decimals
(`point`), 4 ordinals 0x10003720 (endings: compare chain from 0x1002a57c
`1`→`first` … `00`→`hundredth`), 7 times 0x10007c30 (`o'clock`; `am` after
it becomes token `a.m.` of type 4) or durations 0x100080a0 (time format
0x1a), 8 dates 0x10009230 (month word 0x50+m, ordinal day, year
0x10008be0; ranges 0x10009040 `to`/`and`; decades 0x10009d80), 11
fractions 0x100071a0 (plural denominators 0x10002f10), 12 money 0x10007230
(scale words 0x10029dd8, cents). Token access is 0x10001b90 (control tags
skipped, 31 entries). Dates use the engine's date format (ctx+0x20 → +0x34:
0x11 `mdy` for the US voice, 0x10 `dmy` for GB/AU; months 0x10029e38, 0x24
names; year words 0x10029f58, eras 0x10029f80). Replayed on 642 traced
slot-9 calls (num1 and `oracle/bench-en-text.txt`, US/GB/AU): all equal
in reading, ctx+0x18 and rewritten tokens.

Not ported yet (§11): the text hooks the engine's tokenizer calls
(LesPronunciabile 0x10002e90, LesGetSymbolName 0x100021e0 / 0x100021a0,
LesNormalizzaAccento 0x1000c3b0, LesElaboraParola 0x1000a600,
abbreviations 0x100288e0, 0xfc pairs). After FON the voice library's
own phone stage changes what is said (§4e).

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
Leonor (99.8% before); `bench-es-am.txt` 100% for Carlos, Soledad,
Ximena (91.2% before), 99.1% for Diego, Esperanza, Francisca (90.0%
before; the rest is their voice library's phone stage, `h` for `Hh` and
`J` for `Y`, §4c).

**Numbers** (`lang/spanish/numeri.rs`). The engine (0x1003eb80) calls
only slot 9 of the table `LesIniNumeri` (0x10007ec0) fills: 0x10007490,
cdecl `(ctx, type, idx, dmy, token buffer, out, text)`, the tokens used
returned in ctx+0x18 (`Les::leggi_numero`). The reading type comes from a
`\{number=…}` tag before the token (0x10030670: ordinalM 5, ordinalF 6,
telephone 1, time 7, code 2, cardinal/real 10, currency 12, date 8 with
its order), which the `.lex` filters write (`1º` → `\{number=ordinalM}1`);
else 0 and `dmy`. Slot 9 fetches the tokens around (0x10001990: tags
skipped, at most 7 back, an end entry type 7), turns type 5/6 into 4
with the gender at obj+0x48, 12 when a currency symbol or word
(`euro`, `dólares`, `USD`, `pesos`, `libra esterlina`…) is next or a symbol
before, 10 before `%`; then runs nine recognizers (0x100067c0 puts the
type's own first) on the 0xd4-byte number object, the first that accepts
setting its kind, else `ELQNumSetDefaultType`: date 0x10003d90 (slot 7
0x100034a0, `ELQNumExtractDate` in the order, two-digit years from 80
unless `/`), time 0x10003ef0 (`ELQNumIsTime`; an hour before `h` takes
it and its minutes), ordinal 0x10004340 (type 4, or before `°`/`º` not
followed by `c`, or `'`), currency 0x10004510 (the word after becomes
`n`; `.` and `,` swap unless a dot groups thousands), type 9 0x10004ea0,
fraction 0x10005140 (14 fixed ones, `medio`, `tres cuartos`), telephone
0x10005260 (more than six digits after an area code of the 0xf7 at
0x10024ac0), real 0x10005640 (up to five digits, one decimal comma,
dot-grouped thousands) and code 0x10005480 (type 2 only). The renderer
0x100068a0 reads them: plain numbers through `ELQNumConvertRealNumber`
(six digits and more by the telephone slot: pairs, `diez # cero cero #
cero uno`), integers by `ELQNumConvertInteger`/`ConvertBigInteger` and
the cardinal slot 0x10006e10 (one-digit numbers by slot 3; `cien`/`ciento`,
`dosmil`, `veintiun`, `un millón #`, the final `un` + `o`), decimals by
slot 6 0x10007d00 (`coma`, up to three digits as a number, leading zeros
and longer ones digit by digit), ordinals 0x10002090 (`priméro`,
`milésimo …`), times 0x10005a00 (`doce  # treinta`, `minutos`,
`segundos`), dates 0x10003510 (`quince  de marzo  de dosmil
veinticuatro`), amounts 0x10002a00 (`un`/`una` for one unit, cents, `de
dólar`, the names at 0x10024ea0) and fractions 0x10001a80. Checked call
by call (`spanish::numeri::tests::spanish_slot9_trace`, the tracer's
`> NUM`/`< NUM` lines): 311 calls each for America (Carlos) and Spain
(Jorge) on `oracle/bench-es-text.txt`, `bench-es-am.txt` and private
lists of cardinals, grouped and decimal numbers, ordinals, dates, times,
currencies, fractions and telephone numbers: every reading, token count
and rewritten token identical; `bench-es-text.txt` transcriptions 30 of
30 identical (0 before).

## 4e. The voice's own phone stage (`voicefon.rs`)

The module's transcription (`ttsPhoneticTranscription`, `.fon`, event 2
`TTSEVT_LANGUAGEPHONEME`) is not what the voice says (event 3,
`TTSEVT_VOICEPHONEME`): LoqTTS7.dll's phrase loop (0x1002c300) hands each
chunk's phone list, after FonAllofonoLingua, to the voice library. Its one
export (`Allison` 0x10001140…) walks a `{name, function}` table like the
language modules' (Allison 0x1000a8d8, Katrin/Stefan 0x1000aeb8, Jorge
0x10008878, Dave 0x1000c8d0, Steven 0x1000a910, Susan 0x1000b8e8, Kate
0x1000a978, Bernard 0x10008960, …); the binder 0x1002d730 takes
`FonVidAllofonoVoce` (voice +0x18), `FonVidAllofonoAnalisi` (+0x1c),
`FonVidMetaFonema` (+0x20), `FonVidAnalisi2Sintesi` (+0x24) and
`FonVidIngloba` (+0x28); a library without them (Roberto, the Italian
voices) gets the default 0x1002d100 for +0x1c/+0x20 and no +0x18.

1. **Into the voice's inventory**, 0x1002da60. The mapping targets
   (voice +0x10, built by 0x1002d910) are the library's phone table
   (`ELQGetPhonemes`, `{name, id, class, code}` 16 bytes, ids checked
   equal to positions by 0x1002cf20); when the library's `GetVersion`
   minor number is below 3 (0x10051210: Allison 7.0.0, Katrin 7.0.1, Dave
   7.1.1, `LTTSDLLVERSION7.0.1` Bernard) only entries whose code lacks bit
   3 or is a pause (0x18, 0x28), as engine words (0x10050780). Each phone
   (engine code) goes to its nearest target, 0x1003fdc0: an exact match
   (both sides normalised: a `0xc2` palatal/velar with 0x100002 becomes
   `0x1e02` with 0x400002 / 0x200002), else the last equal up to the tone
   bits `0xc` (not index 0); else none for tone `0xc` and for a `0x1e2`
   at place 0x802; affricates (`0x102`) the same up to 0x1ff3 / 0x1fe3,
   else two phones, the nearest stop and fricative (0x1003fb20 below
   15); consonants the nearest by manner (0x1003f980, tables 0x1003fa04,
   0x1003fa98) and place (0x1003f8d0) with feature adjustments, below 5,
   or for the 0x1003fd00 class the best of its class (+1 same tone, +1
   same voicing), an aspirated one preceded by the nearest vowel to
   0x501; vowels and diphthongs by height (0x1003f750), backness
   (0x1003f7b0) and features (0x1003f830, below 100), a diphthong the
   voice lacks as two vowels, a nasalised or 0x200001 vowel the voice
   lacks plus a nasal/glide; pauses the third/fourth target. Unmapped
   phones are dropped. For an older library the target's engine word is
   turned back into the library's (0x10050d50, `voicefon::to_library`)
   and the first table entry with that code (or `| 0x2000004`) taken. The
   list's slots keep their kinds; codes and records are written back.
2. **`FonVidAllofonoVoce`** per phone (`i = 0; while i < len - 1`, the
   length re-read; the function may move `i`), with the chunk's records
   (text, stress grade +0x12, boundary +0x11, the neighbours'), the list
   (`ELQList`: delete, add before/after) and the sentence's record count
   (ctx +0x20).
3. **The phones said**, 0x1002d180: `FonVidMetaFonema(ctx, &in, &out,
   table)` per phone writes table ids into the output records (+0xc) and
   advances both; the default (0x1002d100, and each library's copy) is
   the id of the first entry with the phone's code, else 1. A word's last
   output phone is marked 0x77 when its record's boundary is not 0, the
   last of the phrase 0x69, the closing entry 0x70.
   `FonVidAllofonoAnalisi` replaces MetaFonema only when FON ctx[4]
   +0x214 is set (the engine's analysis mode): the events of every voice
   show none of the analysis-only allophones it writes (Katrin's `HB HD
   HG Hv Hz Hh`, ids 64–86, never appear in 60 cached utterances), so it
   is not ported, nor `FonVidAnalisi2Sintesi` / `FonVidIngloba` (CAT
   side).

Ported per voice (`voicefon/*.rs`; used only when the library's export
name and function table read back as ported; literals and tables read
from the library by address):

| voice | AllofonoVoce | MetaFonema | what they do |
|---|---|---|---|
| Katrin, Stefan | 0x100011e0 | 0x10002ea0 (default), Stefan 0x10002ae0 | a glottal stop `Hk` before a vowel-initial word after a vowel or voiced consonant (stressed word, or after boundary 4), before an unstressed one (grade 3) elsewhere, and between vowels in hiatus inside a word (not in the words at 0x1000af20–0x1000af8c: ideàl, georgie, washington, loquendo…); b d g before a consonant inside a word unreleased (`Hub Hud Hug`); Stefan says unaspirated p t k as b d g |
| Allison | 0x10001450, word replacements 0x10001210 (0x444-byte entries from 0x1000d000 to `-1` 0x1000a994: word, codes, grade) | 0x100037a0 | `because`, `could`, `for`, `our`, `that`, `them`, `your`… replaced (0x48 = the word's last phone); `to/into/onto` weak or flapped (`HTh`); h of unstressed words after a stop or fricative and after `must`/`and`/`there` dropped; d + `you…` → `Dg`; `and` + I/he… → `Hud`; `-tional` (ʃ n̩), `dot it`, `believe`, `tz`; unaspirated p t k said b d g, word-initial `Hw Hj Hg Hk` not said |
| Dave 0x10001210, Steven 0x100014e0, Susan 0x10001210, Kate 0x100014a0, Elizabeth, Simon, Grace, Alan 0x100014a0 | each its own (word dictionaries 0x1000d000, 0x54-byte entries in the 7.3+ libraries) | Dave 0x10004cf0, Steven 0x10003ac0, Susan 0x10004250, Kate 0x100039e0, … | the same kinds of rules, per voice (see each file) |
| Jorge, Carmen, Diego, Carlos | nothing | default; Diego 0x10001280 / Francisca 0x100012c0 say 0x1800104 as `h`; Carlos 0x100013e0, Ximena 0x10001490 map analysis ids to synthesis ones (0x10001370 / 0x10001420) | Leonor 0x100011e0 s + `r:` → 0x501014c, DH after `` `a `` dropped before a vowel/sonorant word; Esperanza `Y` → `J`; Francisca h → 0x1800104 between a vowel and f/t; Soledad 0x10001210 s / dd before the next word; Ximena 0x10001060 |
| Bernard 0x10001230, Juliette 0x10001230, Florence 0x10001210 | | default | Bernard: a nasal vowel + consonant of a word written `-e`/`-es` before a consonant word keeps the `e` (`HEh`); Juliette: s → z before a voiced word, final dd → n after a nasal vowel, else it voices the consonant ending the word before; Florence: voicing assimilation across words after a nasal vowel (not `donc`, `quelconque`, before `million`, `mètre(s)`) |
| Afroditi 0x10001210 | | 0x100012d0 → 0x10001260 (the default) | a word-final s (0x1952) becomes z (0x1942) before a word starting with m (0x1fc2) or n (0x19c2) (the module's own FonAllofonoLingua voices s only before other voiced consonants) |
| Amalia, Eusebio, Gabriela, Fernanda, Felipe | Fernanda, Felipe 0x100014a0; the others 0x10001210 (nothing) | Amalia default; Eusebio default + map; Gabriela/Felipe s → id 0x28 before a voiced consonant; Felipe/Fernanda tap 0x200018d2 → id 0x33 | Fernanda/Felipe: words of the table at 0x1000b000 (0x54-byte entries to 0x100086c4 / 0x100086bc, 0x10001350) replaced, phone adjustments (§4g) |

The stage runs for module-backed voices only (`g2p::Frontend::with_voice`):
the built-in rules of the unported languages give phones the voices say
already. `--fon-list` leaves it out (`without_voice`): it is
compared with the original's `.fonlist`, the module's transcription.
FON ctx +0x18, which Dave, Kate, Grace and Alan read (7–10 → `I`), is
the module's scratch "tokens used" count (LTTS7English 0x10004610,
0x10004d30, 0x10007230); it is stale at FON time and below 7 in ordinary
sentences: taken as not 7–10 (§11).

Sounds identical to the original's events (`--compare`, `QQ_NO_LEXICON=1`),
before → after: Katrin bench-de 95.5% → **100%**, Stefan 91.9% →
**100%** (bench-de2 100% both); Allison bench-en 97.2% → **100%**, Dave
96.8% → **100%**, Steven 98.3% → **100%**, Susan 97.2% → **100%**, Kate
bench-en-gb 96.7% → **100%**, Elizabeth 96.1% → **100%**, Simon 95.9% →
**100%**, Grace bench-en-au 95.3% → **100%**, Alan 92.6% → **100%**;
Jorge, Roberto and the Italian, Spanish and French voices stay or reach
100%. `oracle/bench-en-voice.txt` (42 sentences aimed at these rules):
Steven 100%, the other eight English voices 99.9%, the one phone left
a module difference (`students sent`: our English LES adds a `#50`
pause the original's event 2 does not have, so `Ts` is split).
Afroditi (bench-el, its stage on top of the Greek module, §4f) 100%;
`τις μέρες`, `τους νέους`, `μας με` give the original's `z`.

## 4f. Greek FON, ported from LTTS7Greek.dll (`lang/greek/`)

The export `Greek` (0x10008500) accepts a descriptor whose `Language`
list (split on `,;`, 0x10022378) has `greek` and hands the engine a table
of 32 `{name, function}` pairs at **0x10021d00**: GetVersion 0x10008390
(7.15.0), ELQGetLanguageNativeEncoding and …DefaultAnsiEncoding
0x10008360 (**0x4e5 = 1253**), …IsoEncoding 0x10008370 (0x6fb5 =
ISO-8859-7), …OemEncoding 0x10008380 (0x2e1 = 737), ELQGetPhonemes
0x10007c20 (table 0x10021c40, `{entries, u16 count}`, entries `{symbol,
code}`), ELQGetGraphemes 0x10007c10 (0x100212d0: 8 bytes per character,
lower, upper, flags at +4: 1 letter, 2 acronym, 4 digit, 8 consonant, 0x10
vowel, 0x20 upper, 0x40 lower, 0x80 accented, 0x100 punctuation),
LesElaboraParola 0x10008cb0, LesAccenta 0x10001820, LesPronunciabile
0x100030d0, LesIniNumeri 0x10007b60, LesAnalizza 0x10009620,
LesGetSymbolName 0x10002f50, LesGradiAccentuali 0x10008740,
LesSetModalitaFrase 0x10008920, FonTrasParola 0x1000a0e0,
FonAllofonoLingua 0x1000b470, ELQLanguageLoadReadOnlyData 0x10007d10,
LesGetAbbreviazioni 0x10008240, LesIsPartOfAcronym 0x100086f0 (flag 2),
LesIsLetter 0x10008710 (flag 1), LesIsUpper 0x10008720 (flag 0x20),
LesAllocLanguageLexInfo 0x10008300, LesGetDefaultCurrency 0x10004ec0.
There is no LesConfiniProsodici, FonTrasConfini, LesNormalizzaAccento
nor LesGestisciMaiuscole: the engine binds its own (LoqTTS7.dll
0x10036140: ConfiniProsodici default **0x100409c0**, GestisciMaiuscole
0x10034f60).

**The text is Windows-1253.** The engine stores the module's native
encoding (LoqTTS7 0x1000c3d0 area: `ELQGetLanguageNativeEncoding` →
+0x21c, copied into the LES object's +0x150, which the constructor
0x1002e500 sets to 0x4e4 = 1252 for modules without one) and converts
its UTF-8 input to it; `Les::code_page` (common.rs `to_8bit_cp`).

The loader 0x10007d10 reads `Greek.dbl` (1 251 rows, mostly foreign words
and function words) with `SELECT grafemica, fonetica, gradoaccentuale,
grammaticale, specgramm, gennum FROM lessico` and the abbreviations
(`… WHERE ifnull(abbreviazione,'') <> '' ORDER BY abbreviazione COLLATE
NOCASE DESC`).

| function | address | what it does | data |
|---|---|---|---|
| LesElaboraParola | 0x10008cb0 | the word as one record (0x17, 200, 200), or spelled (each LesGetSymbolName a record 0x15, sub 0x83, the last 0x84, info 9) when the token is one character alone among tags (0x10008b20, symbol mode 3, else 1), the engine's mode asks, or LesPronunciabile rejects it and neither the engine's spell flag nor the token's expansion flag (+0x408) is set | — |
| LesPronunciabile | 0x10002fc0 | readable with `` ` `` or `'`, with Latin and other letters mixed, an accented letter or a non-Latin vowel; else when it has a vowel (0x10002f90) | — |
| LesGetSymbolName | 0x10002400 (character in CL, mode in AL) | names of letters and symbols; mode 0 only letters and `$ % & + / = { } €…`, ≥ 1 punctuation too, 2 `… με τόνο`, 3 `… κεφαλαίο`/`μικρό` | strings 0x1001f0c0…0x10020c88 |
| LesAnalizza | 0x100094e0 | the record's dictionary rows (0x100093b0, `ELQDBTableFind`); the list walked to the first row with a `grammaticale` (0x10009450); no rows or none → class 0x17 becomes 0x2f (0x10008df0); else for class 0x17, or a proper noun (0x15, sub 0xc8) the dictionary calls a function word, class from `grammaticale` (0x10008e10: PRON 0x20, VERBO 0x2a, NONVERBO 0x2f, AVV 0x33, AGG 0x29, SOST 0x15, ARTIC 0x18, ARTPRON 0x16, CONG 0x1f, PREPS 0x19, PREPA 0x1e, PRONCLI 0x21, PRONPC 0x28, PRONR 0x22, VERBOE 0x2c, VERBOA 0x2b, OMOGRAFO 0x2d), subclass from `specgramm` (0x100091b0: ECONG 0x82, NUMU `y`, INTERR `x`), information from `gennum` (0x10009260: FESI 0xd, MASI 0xb, FEPL 0xe, MAPL 0xc, XPLU 0x10, XSIN 0xf, none 9) | — |
| LesAccenta | 0x10001820 | every record not elided and without an accented letter (0x100014e0) gets an accent: mostly Latin lower case (more than half, or one of two) → the ending table (0x10001730 → 0x10001210, as Italian's) marks the n-th vowel from the end with its Latin-1 acute (0x10001400, 0x100012e0); else 0x10001520: `δυο μια μιαν μιας πια πιο` → their accented forms (0x1001efc4…0x1001ef88), a doubled vowel on its second letter, else by vowel groups (0x10001460: `αι ει οι υι αυ ευ ου` once) the first of one or two, the second of three, the third from the end of more (0x10001250). The channel byte +4 (1; `\#Gg`/`\#Ge` tags in records switch it, 0x10008440) picks 0x10001730 over 0x100017a0 | Latin endings 0x1001d1a0 (0x27e × 12) |
| LesSetModalitaFrase | 0x10008920 | the closing punctuation's first character among `.!?:;¶` (0x10022390): `!` 6, `.` 0, `;` `?` 2 (3 with an interrogative `x` in the first four records), `:` `¶` 5, else 7 | — |
| (engine) LesConfiniProsodici | LoqTTS7 0x100409c0 | boundary 0x15 → 3 (class `7` 0x16); punctuation: the sentence end by type (0 → 0x0f, 2 → 0x11, 3 → 0x12, 5 → 0x10, 6 → 0x13, else 0x16) on it and the word before (skipping 0x16), `,` `-` `.` 0x0b, `(` `)` 0x0b between words; then sub `y`/0x84 → 7, `z`/0x83 → 6, `{` → 8 | — |
| LesGradiAccentuali | 0x10008740 | punctuation 0x0f; a function word (classes 0x19 0x1e 0x18 0x20 0x21 0x16 0x22 0x1f 0x2e) of at most 5 letters, a word ending in `'` or sub 0x8f: 3; else 0; rows of the other `gradoaccentuale` (0x10008670: DEA 3) dropped | — |
| FonTrasParola | 0x1000a0e0 | a row with `fonetica` (0x10009450 column 1, 0x10009840 / 0x100097b0) gives the phones; else 0x10009910 lower-cases the word (an unstressed one, grade 3, without accents, 0x10009640), a mostly-Latin word transliterated into Greek letters (`ch` χ, `th` θ, `ks` ξ, `j` ξ…; the FON channel byte +0x44 is 1); the first pass collapses `αι` → ε, `ει` → ι, `οι` → η, `υι` → ι, `αυ ευ ηυ` → φ/β (φ before a voiceless consonant or at the end) and doubled consonants (not γγ νν); then each letter in context: γ before ε/ι palatal (0x1142), `γκ γγ` → N g (word-initial g), `γχ γξ` N; ι/η/υ before a vowel a glide (0x10c2) or full (0x7c1) by the word lists and `ποιος ποιες ποια ποιο` (0x10009720); κ + j before ε; λ/ν + ι + vowel palatal (0x10a2, 0x11c2 + j); `μπ`/`ντ` b/d word-initially, mb/nd inside unless the word is listed; σ z before a voiced consonant; `τσ` Ts, `τζ` Dz; ου u. `local_f0` there is the last letter's index | word-start lists (`ELQSearchString` flag 0x12) 0x10022c68 (15), 0x10022ca8 (41), 0x10022d58 (27), 0x10022dc4 (6), 0x10022de0 (17), 0x10022e28 (32), 0x10022ca4 (1), 0x10022ea8 (37), 0x10022f40 (17); sets 0x10022fdc γδζλμνρ, 0x10022fa8 ΐήίηιϊΰϋ, 0x10022fa4 υύ, 0x10022fa0 ΰύ, 0x10022f9c εέ, 0x10022f98 γκ, 0x10022f94 βσφ |
| FonAllofonoLingua | 0x1000b470 | with the previous, own and next record as Greek letters (0x10009910): n + k across words after `στην την τον των` (0x10022f84) → N g; `δέν` loses its n before DH/Th; j after a palatal (`& 0x1e03 == 0x1002`) deleted; word-final s → z before `μας με μου` (0x10022d4c) or a voiced consonant; k x g before a front vowel or j palatal (0x11f2 0x1152 0x11e2); `κι` loses its i before a vowel; `ότι` 0x10361 → 0x361 and `δέν`/`δέ` 0x10741 → 0x741 in chunks of more than 2 records; t/p after `…ν` of an article → d/b; m before f v s z → 0x1dc2. After a deletion it returns the index before (0x1000b7f5) | — |

Checked call by call (`lang::greek::trace::greek_trace`, the `.les`
trace of `LOQ_LES_DLL=LTTS7Greek.dll LOQ_LES_TABLE=10021d00 LOQ_LES_N=30`):
on the bench (24 sentences of calls) and on 20 000 held-out words
(`oracle/words/el.txt`, every 41st word of LibreOffice `el_GR/el_GR.dic`,
ISO-8859-7), LesAnalizza, LesAccenta, the engine's ConfiniProsodici,
LesGradiAccentuali, FonTrasParola (20 002 calls) and FonAllofonoLingua
give the original's records and codes for **every** call. Whole
transcriptions (`QQ_NO_LEXICON=1`): held-out words 87.2% → **99.995%**
(19 999 of 20 000; `σία`, a dictionary abbreviation spelled through a
`\{spell=yes` tag, §11); 3 082 extra words (English, capitalised and
unaccented upper-case Greek, single letters) 3 081 (`π.χ.`, Greek.lex);
bench `oracle/bench-el.txt` 48.2% → **100%** of sounds, CAT 98.1% →
**100%** (the compare tool now takes Greek `;` as a question, as the
module does). The original's notation glues a word to the next when
FonAllofonoLingua palatalised the next one's first k/x (`τυρί και`,
`δύο χιλιάδες`): the sounds are the same, `--compare` counts those as
different words.

**Greek numbers** (`greek/numeri.rs`). `LesIniNumeri` 0x10007b60 fills
slots 0 0x10002140 (word ids → strings at 0x1001fea0, 0x4d pairs), 1
0x100020c0 (phone: area codes 0x1001fa20 × 0xf7, `ELQSearchString` mode
2, then `ELQNumConvertPhoneNumber` steps 0/1 into its own string), 2
0x10005e70 (cardinal), 3 0x10005e30, 4 0x10002110, 5 0x10005310
(currency names 0x1001fe00, 8 rows {symbol, unit, units, cent, cents};
`"1"` 0x10020d14 takes the singular; a space before and after), 6
0x10006f50, 7 0x10002170 and 9 0x10007740. Slot 9 has the Spanish
skeleton: the window from 0x10001950, reading types 5/6 → mode 4 with
+0x48 = 5/6, mode 0xc for a currency symbol next or before or a next word
among `δρχ` 0x10020ce8, `δράχμη` 0x10020ce0, `δραχμές` 0x10020cd8, `euro`
0x10020cd0, `eur` 0x10020cc8, `EUR` 0x10020ccc, `ευρώ` 0x1001f2b0,
`ΕΥΡΏ` 0x10020cc0, 10 for a next `%`; nine recognizers in 0x10005d50's
order (date 0x100030f0, time 0x10003250, ordinal 0x100036a0, currency
0x10003880, mode 9 0x10003d30, fraction 0x10004010, phone 0x10004130,
real 0x10004510, code 0x10004350), else `ELQNumSetDefaultType`, then the
renderer 0x10007110. Cardinal 0x10005e70: an `m`/`f` in the digits sets
the gender of the last three; hundreds 0x10005710 stems 0x3a–0x43 +
`-α` 0x45 / `-οι` 0x46 / `-ες` 0x44 (`-ες` when the number from that digit
is 1000–999 999); two digits 0x10005a80 (teens `δεκα` 0x10020d18 + unit);
1/3/4 → μία 0x12, τρείς 0x18, τέσσερις 0x1a, ένας 0x13 (masculine after a
zero ten, in the thousands or a given gender); units 0x100054f0 (index
bytes 0x100056a8, `push id` targets 0x10005638). Currency 0x10003880:
drachma words without a comma append `f` (0x10013bf0, +0x5c = 2, the
renderer says μία/τρείς/τέσσερις and no name); euro words (and `EURO`
0x10020cb8) set `€` (0x1001f2b8); a euro word or symbol after the number
becomes `n`, ctx+0x18 = 1. Amounts 0x100048b0: two decimals → units, the
name only for €, `και` and cents; others `τελεία` + slot 6. Times
0x10004ed0: hours 1/21 μία (0x10004f76) / είκοσι μία (0x10020d04), 3 13 23
`είς` (0x10004fa2) and 4 14 `ις` (0x10020204) patched over the ending,
`και` + minutes, `λεπτά` 0x5e / `δευτερόλεπτα` 0x5d. Dates 0x100021e0: day
1 `πρώτη` (0x10020208), the same 3/4 patches (0x100022d0), month word
0x50+m (genitive). Ordinals 0x100065d0 by groups of three: hundreds a
cardinal with οστός/οστή (0x10020e64/0x10020e5c), tens stems 0x10020e2c…
(10/11/12 at 0x10020e3c/0x10020e4c/0x10020e44), units 0x10020dcc… (a
four-digit number's thousands digit the "n-times" forms 0x10020dc8…),
group words 0x10020d20/0x10020d28/0x10020d38, endings ός/ή
0x10020dd8/0x10020dd4 and ος/η 0x10020e38/0x10020e34, more than 12
digits digit by digit. Fractions 14 pairs at 0x10020200…; `°`/`º`
0x10020cb4/0x10020cb0 not before `c` 0x10020cac. Replayed on 497 traced
distinct slot-9 calls (`greek::numeri` test `greek_slot9_trace`,
fixture `greek/numeri.les`): every reading, ctx+0x18 and rewritten token
identical. Only reading types 0, 5 and 6 reach the module untagged
(loqref.c runs with `TaggedText` off): the tag-only entry types 1, 2, 7,
8, 9, 10, 0xc are ported but untested. `oracle/bench-el-text.txt` (30
lines): 98.0% of sounds; the rest is Greek.lex (§11).


## 4g. Portuguese FON, ported from LTTS7Portuguese.dll (`lang/portuguese/`)

The module's exports `PortuguesePt` (0x10009dd0) and `PortugueseBr`
(0x10009d60) hand the engine a table of 26 `{name, function}` pairs each,
**0x1001fb98** and **0x1001fac0**; the `.lde`'s `Library` names the
export. They differ only in ELQGetPhonemes (0x10009520 / 0x10009530,
phone tables 0x1001f8d8 / 0x1001f8e0), LesGetSymbolName (0x10003850 /
0x10003890), FonGetSamples and LesGetDefaultCurrency; the FON code reads
the variant from the engine (`**(ctx+0x44)`, set by 0x10007a30 from the
`Language` names: 0 `Portuguese`, 1 `Brazilian`). Slots: GetVersion
0x10009cb0, ELQGetGraphemes 0x10009510 (characters 0x1001ed98, 8 bytes:
lower, upper, flags 1 letter, 4 digit, 8 consonant, 0x10 vowel, 0x20
upper, 0x80 acute/circumflex, 0x100 punctuation), LesAccenta 0x100015a0,
LesNormalizzaAccento 0x10002c60, FonTrasParola 0x1000c0d0, LesIniNumeri
0x100079e0, LesAnalizza 0x1000bd10, LesPronunciabile 0x10003af0,
LesGradiAccentuali 0x10007ca0, LesConfiniProsodici 0x10008f90,
LesWriteCateg 0x10008780, FonTrasConfini 0x1000e8f0,
ELQLanguageLoadReadOnlyData 0x10009640 (`SELECT grafemica, fonetica,
gradoaccentuale, grammaticale, specgramm, gennum FROM lessico` of the
pack's `Portuguese<Variant>.dbl`), LesGetAbbreviazioni 0x10009b80 /
LesGetNumeroAbbreviazioni 0x10009b30. There is no FonAllofonoLingua.

| function | address | what it does | data |
|---|---|---|---|
| LesAnalizza | 0x1000bd10 → 0x1000b520, 0x1000b700, 0x10009fd0 | each word's rows of the lexical database; class from `grammaticale` (0x1000ade0), subclass from `specgramm` (0x1000b1b0), gender/number from `gennum` (0x1000b2a0); unknown words by ending (0x1000a470, endings tested by 0x1000a410: infinitives `-ar -er -ir -ôr`, `-mente`, nouns/adjectives); feminine nouns by ending; numbers agree with a feminine noun after them | endings in 0x1000a470's code; feminine 0x100216d8 (0x2c), plural 0x10021788 (0x24), `ELQSearchString` mode 8 |
| LesAccenta | 0x100015a0 → 0x100011c0 | words without a written accent get one on the stressed vowel (one vowel itself; else the ending table, the code Italian's port shares); graves become acutes (`à` → `å`… by LesNormalizzaAccento 0x10002c60) | stress endings 0x1001b3c0 (0x161) |
| LesConfiniProsodici | 0x10008f90 → 0x100087d0, 0x100080e0, 0x10008d40, 0x10008450, 0x10008270 | group kinds per word, boundaries, a pause between two sides longer than one word and 0xe letters (and > 1 / > 0x13 for the second test) where 0x10008d40 allows it; locutions kept together; the sentence end by type | locutions 0x1001e1d8 (5 words, 2), 0x1001e1e0 (4, 10), 0x1001e208 (3, 0xd), 0x1001e240 (2, 0x49); placeholders 0x1001e1c8/0x1001e1cc/0x1001e1d0 (`pippo` 0x1001e1bc) |
| LesGradiAccentuali | 0x10007ca0 | punctuation 0x0f; function-word classes unstressed (3) | — |
| FonTrasParola | 0x1000c0d0 | a row with `fonetica` gives the phones; an unstressed word loses its accents (0x1000c040); else the variant's whole words, or the longest word start's phones and the rest letter by letter (the switch at 0x1000c4e2): open/closed e and o by word lists, x by lists (`s z ʃ ks`), nasal vowels before a closing m/n, `tém` (0x1000c890), final `-e`/`-o` reduced, Brazil's `t`/`d` before an i sound affricated, an l closing a syllable `w` in Brazil, dark in Portugal | whole words 0x100296a8 (0x13, Pt) / 0x10029740 (0x19, Br); starts 0x100299b0 (0x48a) / 0x10029808 (0x35); e lists 0x1002be98, 0x1002be00, 0x1002bf20; o lists 0x1002c008, 0x1002c028, 0x1002c07c; x lists 0x1002c100, 0x1002c118, 0x1002c250, 0x1002c2d0; gu/qu 0x1002bef4, 0x1002c090; letter sets 0x1002c3bc, 0x1002c450, 0x1002c444, 0x1002c3a0, 0x1002c348 |
| FonTrasConfini | 0x1000e8f0 | Brazil: s between soft sounds → z, a W glide (0x401ec2) after a final u before a vowel; Portugal: the glide also after a nasal a before `u`, `vínte e` loses the `e` of vinte (deleted from the list), final HEh → J before a vowel or dropped before i after some classes, l + HEh before m → dark l, ʃ → z before a vowel and → ʒ before a voiced consonant; inside the word: HEh dropped between some consonants (not the words 0x10022b68, 0x10022b4c, 0x1002c48c), nasal vowels before a stop get the homorganic nasal | — |
| LesGetSymbolName, LesPronunciabile | 0x10003850 / 0x10003890, 0x10003af0 | letter names read from the switch tables (Pt index bytes 0x10003220, jumps 0x100030cc; Br 0x10003750 / 0x100035fc, bias +0x80, at most 0xfe); pronounceable when it has a vowel | — |

**Numbers** (`lang/portuguese/numeri.rs`): slot 9 is 0x100070c0, the
same machine as Spanish's (§4d) with the module's own data. Slots:
0 0x10002800 (words 0x1001d340, 0x53 `{id, word}`), 1 0x10002780
(telephone, area codes 0x1001ce80, 0xf7, `ELQSearchString` mode 2 by
0x10002700), 2 0x100069f0 (cardinal), 3 0x100069b0, 4 0x100027d0, 5
0x100059e0 (currency names 0x1001d260, 11 rows), 6 0x10007810, 7
0x10002830; no slot 8. Next-token currency words that make the type 12
(slot 9): `euro dólar dólares eur peseta pesetas brl BRL usd USD gbp GBP
jpy JPY esp ESP peso pesos real reais pts pta ptas EUR` (0x1001c6fc…
0x1001dc34). The recognizers (order 0x100063a0) and the token window
(0x10001770) are Spanish's code; they differ in the telephone recognizer
0x10004ef0 (type 1 does not set obj+0x4c) and the currency recognizer
0x10004290 (a currency word after the amount, `euro`/`dólar`/`iene`/
`peseta`/`peso`/`real`/`esterlina` and their codes, becomes `n` and its
symbol `€ $ ¥ | _ ^ £` is kept). The renderer 0x10006480 reads amounts
through LTTS7Util's `ELQNumConvertCurrency`, ordinals by 0x10001e70
(stems 0x1001d6c8… per place, `doismilésimo`), times by 0x10005690
(`hours e minutes`, ` minutos e … segundos `), dates by 0x100028a0 (as
Spanish's), fractions by 0x10001860 (14 pairs from 0x1001d6c4, `meio`,
`treis quartos`). The cardinal 0x100069f0: `cem`/`cento e ` and
`duzentos e …` (0x10005de0, ` e` 0x1001dc4c), `mil ` and `e ` after a
thousand before a round hundred or a lone ten/unit (0x1001dc50), 16
thousand `dezasseis` (0x1001dc54), `um milhão`/`milhões`, `um
bilhão`/`bilhões`. The telephone slot starts its string where the caller
points (`ELQNumConvertRealNumber` passes the end of its string), so the
port reads it into its own buffer. Checked call by call
(`portuguese::numeri::tests::portuguese_slot9_trace`, `QQ_PT_NUM_TRACE`,
`QQ_PT_MT`): 924 calls (Portugal) and 948 (Brazil) on 410 lines of
cardinals, decimals, ordinals, dates, times, currencies, fractions and
telephone numbers plus `oracle/bench-pt-text.txt` and the benches: every
reading, token count and rewritten token identical.

**The voices' stage** (`voicefon/portuguese.rs`, §4e): Amalia
(table 0x10008720), Eusebio and Gabriela have a FonVidAllofonoVoce that
does nothing (0x10001210); Fernanda and Felipe (0x100014a0) replace the
words of their word table (0x10001350, 0x54-byte entries from 0x1000b000
to the end marker 0x100086c4 / 0x100086bc, the code Grace's port shares)
and adjust phones. FonVidMetaFonema: Amalia the default, Eusebio the
default then its Analisi2Sintesi map, Gabriela and Felipe a voiced id
(0x28) for s before a voiced consonant, Felipe and Fernanda id 0x33 for
the tapped r (0x200018d2).

Results (`QQ_NO_LEXICON=1`), before (the built-in rules) → after:
`oracle/bench-pt-pt.txt` Amalia 39.4% → **100%** of sounds, Eusebio →
**100%**; `bench-pt-br.txt` Fernanda 49.3% → **100%**, Felipe and
Gabriela **100%**; half-sounds picked 100% and output samples 100%
identical for all five. `oracle/bench-pt-text.txt` transcriptions 30 of
30 identical for both variants. Held-out words (FON only, every 2nd of
LibreOffice `pt_PT.dic`, every 12th of `pt_BR.dic`, 20 000 each):
Portugal 19 986 (99.93%), Brazil 19 999 (99.995%); the rest are
all-capital acronyms the engine spells and Brazil's `bei` (a
`\{spell=yes` abbreviation) (§11).

## 4h. The text filters, pauses and tags (LTTS7Util.dll, LoqTTS7.dll; `lang/textfilter.rs`, `lang/common.rs`)

**Where.** LesRun 0x10035b20 calls 0x100356e0, which applies the
language's RegExp files (ctx +0x164/+0x168) last-first, then its Lexicon
files (+0x170/+0x174) last-first, through Util's `ELQTextFilterApply`
(0x1003a470, reached through LoqTTS7's pointer 0x100b7690); then Compact,
`ELQNormalizeIPATags`, 0x100352e0, Detokenize (`list,&out,0,1,0,0`) and
the sentence loop 0x10034110. The `.lde` file names are loaded as
0x1002fb00 does: `X`, else `X[0]`, then `X[1]`, …. `lang/mod.rs` wraps
every module in `Filtered`, so the rewritten text is what the module's
LES sees; without `bin/LTTS7Util.dll` (kept by `pack.rs` with a language)
the text goes unfiltered.

**Util, ported.** Character classes read from the DLL: digit 0x10058ab0
(0xe9 entries), space 0x10058c84 (0x19), punctuation 0x10058cb8 (0x1dc),
control 0x10059070 (0x61), lower/upper pairs 0x10059ee0 (0x369). Raw word
list: AddText 0x1001bdb0 (classes 2 number, 3 word, 4 space, 5 punct, 6
tag, 0xc/0xd pattern, 0xe end; tags attach to the next word), entry
emit 0x1001b3f0, entry type/flags 0x1001b340/0x1001b220, Detokenize
0x1001d0e0 (`¶` DAT_10072474 after a paragraph, ` \{$` after a sentence
end), Compact 0x1001d7c0. Regexps: a wide Spencer engine
(ELQRxcFindReplace 0x1001f7e0, regmatch 0x1001f300, regrepeat 0x1001f290
whose ANYOF/ANYBUT both use `wcsspn`, jump table 0x1001f2dc; `\<` `\>`
`\{` `\}` ops 12–15), applied per section by 0x1003bce0. Lexicons: the
matcher 0x10039660 over a sorted table (lookup 0x10039f40, comparator
0x10038950); rule lines parsed by the state machine 0x10039230. Tag
names: the table at 0x10081820 (`{flag, id, name, kind, format}`, 58
entries; `spell` is id 7, `s` 6, `number` 11).

**Pauses** (LoqTTS7 0x1002a200): boundaries 8, 0x0a, 0x0e, 0x14 → 50 ms;
0x0b → 120; 0x0c → 10; 0x0d → 100; 0x10 (`:` `;`) → 250; 0x17 → 0;
anything else 500; a `\pause=N` tag (0x1002a160) sets it. The phrase loop
(0x1002c850) pauses after a punctuation record (grade 0x0f) only for
boundaries 0x0b and 0x0f–0x13 (0x14 only before any phone). A sentence
opening with punctuation (`Mo., 14.02.`) gives a pause with no phones
before it: the original plays both (`@500 #120`); `synth::from_tokens`
and the compare tool add it to the silence before.

**Tags on a token** (0x10030670 from 0x10031410). `\spell=yes`/`=spelling`
spell the token (mode 1; `=acronym` 0, `=dictation` 2, `=phonetic` 3,
`=byexample` 4, `=alphabravo` 5; `=no` reads it): a word goes to
LesElaboraParola with that mode (1 for 0), a symbol is named with mode 1,
an abbreviation skips the initials path and every piece is spelled.
**Abbreviations** (0x10034753) are looked up whatever tag follows the
token (`z.B. \{$` → `zum Beispiel`).

`LOQ_TRACE_TEXT=1` (oracle/loqref.c) writes `<prefix>.txt`: the raw word
lists before/after each filter and every `T` text LesRun hands the
sentence loop (call 0x10035cf8 → 0x10034110). `QQ_TRACE_LES=1` prints
quoquendo's records after each LES stage in the same `W` format as
`LOQ_TRACE_LES`.

## 4i. Dutch FON, ported from LTTS7Dutch.dll (`lang/dutch/`)

The export `Dutch` (0x10001790) accepts a descriptor whose `Language`
list (split on `,;` 0x10012874) has `dutch` and hands the engine a table
of 22 `{name, function}` pairs at **0x100122a0**: GetVersion 0x100016e0
(7.13.2), ELQGetPhonemes 0x10001010 (table 0x10012138, `{entries, u16
count}`, entries `{symbol, code}`), ELQGetGraphemes 0x10001000 (0x10011670:
8 bytes per character, lower, upper, flags at +4 as Greek's),
ELQLanguageLoadReadOnlyData 0x10001100, LesGetAbbreviazioni 0x10001620,
LesGetDefaultCurrency 0x10004cc0, LesNormalizzaAccento 0x10004c90
(`` ` `` `'` 0x92 0xb4 → `'`), LesPronunciabile 0x10003780, LesIniNumeri
0x10006ef0, LesAnalizza 0x10007850, LesGetSymbolName 0x10003550,
LesGradiAccentuali 0x100071b0, LesElaboraParola 0x100074c0, FonTrasParola
0x10008420, FonTrasConfini 0x100089f0, FonAllofonoLingua 0x10009980. No
LesAccenta (the engine's slot stays empty), no LesConfiniProsodici nor
LesSetModalitaFrase (the engine's 0x100409c0, 0x100350e0), no native
encoding (Windows-1252). The loader reads `Dutch.dbl` (3 305 rows) with
`SELECT grafemica, fonetica FROM lessico` and the abbreviations (`…
WHERE abbreviazione!="" ORDER BY abbreviazione COLLATE NOCASE DESC`); the
FON channel (0x10006f40) opens LTTS7GPR.dll's converter on `Dutch.gpr`
(43 letters, no multi-letter graphemes, 87 classes; `gpr.rs`).

| function | address | what it does | data |
|---|---|---|---|
| LesPronunciabile | 0x100035c0 | readable with `'` or an accented letter; with the char flag not otherwise; all capitals: 2 letters never, 3 only V C V or C V C, 4 not when 3 consonants follow at most one vowel; else when it has a vowel (0x10003590); 0x3f characters looked at | — |
| LesGetSymbolName | 0x100030b0 (character in CL, mode in AL) | the switch's targets read as code: a `test al,al; je` target names the character only in mode ≠ 0, the name is the `mov eax, imm32` string | index table 0x10003450, jumps 0x10003338 |
| LesElaboraParola | 0x100074c0 | a readable word (or spell flag, or an expansion's) one record (0x17, 200, 200), or one per compound part (0x10007ce0, ≤ 10, parts but the last boundary 4, lower case); else each character's name (mode 1, through LesNormalizzaAccento) a record (0x15, 0x83/last 0x84, 9) | — |
| compound splitter | 0x10007930 | the word lower-cased (≤ 63); none if the list has it whole or shorter than 6; from the left part, the first cut leaving ≥ 4 letters each side whose right part the list has (or, once one was found, the right part without `en`/`er`/`s`/`n`, 0x10007870 — a buffer only overwritten when an ending comes off); complete when the left part (or it without its ending) is in the list, else it goes on in the left part | `ELQSearchCompactString` flag 1 over 0x848d2 bytes at 0x100148c8 (`aagje`…) |
| LesAnalizza | 0x100077d0 | records 0x17, 0x15 with sub 200, or sub `z`: `strcmp` binary search (0x10007770) in 0x11b `{word, class, sub, info}` | 0x100991a0 |
| LesGradiAccentuali | 0x100071b0 | punctuation 0xf; unset grade (0xe): 3 for `z`/`y` after 0x82 and `z`; 2 after a compound part (boundary 4); 3 for a one-syllable (0x10007090: one vowel, `j`/`w` + vowel, or one of `ee ie oo aa uu eu ui ou` 0x100140e8) function word (0x19 0x1e 0x18 0x20 0x21 0x16 0x22 0x1f 0x2e) before boundary 0–3, 6, 7, 0x16, for sub 0x83, an elided word, and `is` (0x10013f94) before those boundaries; else 0 | — |
| FonTrasParola | 0x10008420 | exactly one dictionary row (`ELQDBTableFind`) with a non-empty `fonetica`: its symbols (0x10007f10: split at ` `/`-`, ≤ 5 characters; 0x10007e80); `het` `van` `een` `eens` (0x10014644 0x10012894 0x1001473c 0x10014734) fixed codes, full vowel after a pause/sentence-end boundary (0x0a 0x08 0x0e 0x0c 0x0d 0x0b 0x0f–0x14), `het` first in a chunk `Ou t`; `eff`/`emm` (grade 3) `E f`/`E m`; else the word with à á ì í ò ó ù ú ý plain (0x10007da0 → 0x10001900) through `ELQGPRRun`, then stress: several stressed vowels (0x100083e0) all unstressed (0x10007fe0) and the first vowel stressed (0x100081e0, schwa → `` `E ``), none stressed the first one unless grade 2/3; grade 3 none, grade 2 none unless after a pause/sentence end | — |
| FonTrasConfini | 0x100089f0 | the list's last phone devoiced (z Gx g Zg d v b); with ≥ 2 phones a final stop before an obstruent/nasal/affricate unreleased (`Hut Huk Hup` not before b d; `Hud Hub`), `R` after a vowel (not schwa) before a consonant or pause `HEhR`; a glide between vowels (`Hj` front, `V` back rounded, `Hy` front rounded); along the word: fricative after a voiceless obstruent (`Gx` → `x`), t k p before b d `Hut Huk`/voiced, n/m before velars/labiodentals `N`/`M`, s z n t d before `j` palatal (the j gone), l before a consonant or at the end `Lh`, schwa R C `HEhR`, x R `HRB`, `-en`/`-ens` schwa `HEh)` (n dropped), `negenen`/`zevenen` (0x10099aac 0x10099aa4), t between obstruents dropped, stops before obstruents/nasals unreleased | — |
| FonAllofonoLingua | 0x10009980 | v/z devoiced after a voiceless obstruent in a compound's later part; a glottal stop `Hg` (element kind byte a stale local, 0xee) before a stressed vowel after `a`, schwa or another word (a word-final `R` then `HEhR`), before a vowel starting a compound part, and before a vowel after a word ending in `a`/schwa; `Aau` → `Oau`; under `\SAMPA=` (0x10099ab4) `Ah` → `Ou`, a schwa's n dropped | — |

Checked call by call (`lang::dutch::trace::dutch_trace`, the `.les`
trace of `LOQ_LES_DLL=LTTS7Dutch.dll LOQ_LES_TABLE=100122a0 LOQ_LES_N=22`):
on the bench and on 20 000 held-out words (`oracle/words/nl.txt`, every
7th alphabetic word of LibreOffice `nl_NL/nl_NL.dic`), LesAnalizza, the
engine's ConfiniProsodici, LesGradiAccentuali, FonTrasParola (25 921
calls), each chunk's list after FonTrasParola + FonTrasConfini and
FonAllofonoLingua give the original's records and codes for **every**
call. Whole transcriptions (`QQ_NO_LEXICON=1`): held-out words 19 998 →
**20 000 of 20 000** once common's `\spell=yes` tags (§4h) spell the
dictionary abbreviations expanding to `\spell=yes cda`/`kro`; bench
`oracle/bench-nl.txt` (Willem) 37.5% → **100%** of sounds, CAT **100%**,
output samples **100%** identical; `oracle/bench-nl-text.txt` (30
lines of numbers, dates, money, abbreviations) **100%** of sounds (§11).

**Dutch numbers** (`dutch/numeri.rs`). The German template (§4):
`LesIniNumeri` 0x10006ef0 fills slots 0 0x10002820 (`{id, char*}` × 0x53 at
0x10013810), 1 0x100027a0 (area codes 0x10013390 × 0xf7, `ELQSearchString`
flag 2), 2 0x10006220 (cardinal: German's but hundreds 0x10005400 `honderd
`/`x honderd `, thousands `duizend` + separator), 3 0x100061e0, 4
0x100027f0, 5 0x10004fd0 (8 × `{code, unit, units, cent, cents}` at
0x10013770; its separators go to the empty tail of the output, so both
are always added), 6 0x10006d30, 7 0x10002850 (two-digit years from 80,
from 0 after `/`), 9 0x10006820: mode 0xc for a currency sign next to the
number or `euro dollars dollar eur usd USD JPY jpy gbp GBP yen EUR`
(`livre` with `sterling` three tokens on) after it; recognizers in
0x10005bd0's order: date 0x100037b0 (slot 7; else 0x10002e00: a day
before a month name 0x10013aa8 and a year after it — the tokens used
counted to them —, a year before `voor`, after `in jaar voor vanaf tot na
uit van sinds sedert` 0x10013ba0 or a month name), time 0x10003960,
ordinal 0x10003b00 (mode 4 only), currency 0x10003bf0 (the name after the
number emptied and used), real 0x10004110, fraction 0x100043c0, phone
0x100044e0, integer 0x100048f0, code 0x10004720; renderer 0x10005cb0
(ordinal 0x100020a0: 1–20 from a table, else cardinal + `ste`; date
0x10005aa0: day cardinal, month word 0x51+m, year 0x10005710 —
`negentien honderd negentig`, `twee duizend vier en twintig`, below 1000
the first two digits only; time 0x10004cd0 ` uur `; money 0x100028c0: two
decimals after the units and currency as a bare cardinal, `en` + digit +
`cent` below ten, cents alone for `0,xx`; fractions 0x10001a90, `1/10`
read `eenelfde`, `2/3` nothing). Replayed on 389 distinct traced slot-9
calls (`dutch::numeri::tests::dutch_slot9_trace`, fixture
`dutch/numeri.les`).

**Willem's stage** (`voicefon/willem.rs`, §4e; table 0x100099b0):
FonVidAllofonoVoce 0x10001210 by code, in words not continuing a compound:
`singapore` (`HEhR` → `R HEh`, g dropped), `conventionele` (`Hut` before
`$`), `johnson` (j `Dg`, o `Ou`, h dropped), `justin`, `materiaal`/
`materiële`/`materieel` (e dropped), `interessant` (schwa dropped),
`british` (i), the `initia…` words (i → `I` before `n i Hut s`/`$`),
`milieu` (schwa), `graham` (h → `I`); FonVidMetaFonema 0x100021a0: `Gx`
id 0x4d, `Hj` 0x35, `Hud` 0x3a, else by code (0x10001d30), then
Analisi2Sintesi 0x10002110 (0x36/0x5f → 0x35, 0x3e → 0x3d, 0x41 → 0x3a,
0x42 → 0x40, 0x49 → 0x48, 0x4c → 0x4b, 0x4e/0x60 → 0x4d, 0x4f → 0x50, 0x5e
→ 0x5c). Its words on `bench-nl` are 100% identical to the original's
events.

## 4j. Swedish FON, ported from LTTS7Swedish.dll (`lang/swedish/`)

The module's one export `Swedish` (0x1000a540) checks that the `.lde`'s
`Language` names `swedish` and hands the engine a table of 26 `{name,
function}` pairs at **0x10046ec0**: GetVersion 0x1000a490, ELQGetGraphemes
0x10009d00 (characters 0x100462d8, 8 bytes: lower, upper, flags 1 letter,
4 digit, 8 consonant, 0x10 vowel, 0x20 upper, 0x40 lower, 0x80 accented,
0x100 punctuation), ELQGetPhonemes 0x10009d10 (phone symbols 0x10046da8,
90 entries `{symbol, code}`; accent 1 `0x1xxxx`, accent 2 `0x3xxxx`),
ELQLanguageLoadReadOnlyData 0x10009f40 (the analyser's grammar descriptor;
the abbreviations by `SELECT grafemica, fonetica, abbreviazione` of the
pack's `Swedish.dbl`, 116 rows), LesGetAbbreviazioni 0x1000a3d0 /
LesGetNumeroAbbreviazioni 0x1000a380, LesNormalizzaAccento 0x10005040,
LesPronunciabile 0x10003bd0, LesAccenta 0x10001ad0, LesIniNumeri
0x10007070, LesAnalizza 0x10009ce0, LesGetSymbolName 0x10003920,
LesGradiAccentuali 0x10007180, LesElaboraParola 0x10007e30, LesWriteCateg
0x10007130, LesAlloc/Free/ResetLanguageLexInfo 0x10009e30 / 0x10009eb0 /
0x10009f30 (0x40 slots of 6 words per word, `Word::data`),
LesConfiniProsodici 0x10008620, FonGetSamples 0x10009d20, FonTrasParola
0x1000ee40, FonAllofonoLingua 0x1000e560.

| function | address | what it does | data |
|---|---|---|---|
| LesElaboraParola → analyser | 0x10007e30 → 0x10009770, 0x10009500, 0x10009440, 0x10008ba0, 0x10008b20, 0x100092e0, 0x10009200, 0x10008eb0, 0x10008da0, 0x1000a6b0 | every word is split into morphemes by a grammar walked from the word's end: each transition matches one morpheme of its sorted list (stems, prefixes, derivational and inflectional suffixes, compound connectors); the two best complete analyses (at most 10 morphemes) are kept | grammar 0x100b9b58 (0x1ba nodes); categories 0x1004a028 (5), features 0x1004a360 (0x53), groups 0x1004a5f8 (0x23); vowels 0x100180ac |
| LesAnalizza | 0x10009ce0 → 0x10009a10 | class, subclass and features of each word from its analyses (`STEM`, `DERIV_SUFFIX`, `FLEX_SUFFIX`, `PREFIX`) | — |
| LesAccenta | 0x10001ad0 → 0x10001100, 0x10001890 | the stressed vowel and its accent (1 or 2) per morpheme: stress rules on the stem and suffix groups (`FLEX GRP_f_last_1`, `f_last_1_or` use the analysis count), compounds stress their first part with accent 2 | — |
| LesConfiniProsodici | 0x10008620 → 0x10008190, 0x10007390, 0x100074c0, 0x10007860, 0x10007680 | group kinds per word by a state machine on the classes, the boundaries, locutions kept together (read from the code's immediates), `pippo` placeholders | locutions in 0x10007885–0x10007b67: 5 lists (offset, count, length) (0xc, 2, 6), (0x14, 5, 5), (0x50, 0x16, 4), (0xa8, 0x24, 3), (0x28, 10, 2); 0x100076a5 |
| LesGradiAccentuali | 0x10007180 | stress grades: punctuation, function words unstressed | — |
| LesNormalizzaAccento, LesPronunciabile, LesGetSymbolName | 0x10005040, 0x10003bd0 → 0x100039d0, 0x10003920 → 0x10003460 | accents folded; pronounceable words (a vowel, no impossible cluster: 2- and 3-letter lists); letter names from the switch (index bytes 0x10003814, jumps 0x100036f4), digits from the tens/units lists | spell lists 0x10017530 (8), 0x10017550 (0xe); tens 0x10016214 (8), units 0x10016234 (9) |
| FonTrasParola | 0x1000ee40 → 0x1000a960 | the word's spelling rebuilt from its analysis (each morpheme's stress spelling, +0xc, when it has one), letter by letter into phone codes; morpheme boundaries steer vowel length and clusters; accented vowels take accent 1 or 2; `jag`, `av`, `trettio`… by word | vowels 0x10047458, consonants 0x10047464, front vowels 0x10047478; connector 0x100462c0; morpheme kinds STEM 0x10016500, DERIV 0x100163a0, PREFIX 0x10016274, FLEX 0x1001630c; JAG 0x1001afd0, AV 0x10028e3c, TRETTIO 0x100161dc |
| FonAllofonoLingua | 0x1000e560 | an r merges into a following dental (0x19a2 + 0x1952 → 0x2001752 RT; RTS 0x20017f2, RTJ 0x20017e2, RN 0x20017c2, RL 0x20016a2) and spreads over following dentals; a few phones of `åkk`, `jag`, `det` deleted (`ELQListDeleteElement`) | åkk 0x1003d8e0, jag 0x1001afd0, för 0x100291c4, det 0x1001b018 |

**Numbers** (`lang/swedish/numeri.rs`). LesIniNumeri fills slots 0–7 and
9 like German's (§4, German): slot 0 0x10003160 (`{id, text}` pairs at
0x10017298, 0x53; `""` 0x10016505), slot 1 0x100030e0 (area codes
0x10016e18, 0xf7, `ELQSearchString` flag 2 by 0x10003060, then
`ELQNumConvertPhoneNumber` steps 0/1), slot 2 0x10005cf0 (cardinals
position by position; four digits 1100–1999 read as hundreds by
0x100063a0, `nittonhundraåttiosju`, `hundra` 0x10016624; words 1–20 and
tens 0x100056c0, two digits 0x10005af0 tens then units, hundreds
0x100058e0 ids 0x3a/0x3b; `tusen` 0x47/0x48, `en miljon`/`miljoner`
0x4a/0x4b, `en miljard`/`miljarder` 0x4d/0x4e; a trailing word 0 ` # `
removed), slot 3 0x10005cb0, slot 4 0x10003130, slot 5 0x100054b0
(currencies 0x100171f8, 8 × `{code, unit, units, cent, cents}`, singular
after `1`, a space before and after), slot 6 0x10006520, slot 7
0x10003190 (`ELQNumExtractDate`, two-digit years from 80 unless the
separator is `/`). Slot 9 (0x10006ea0) sets the mode (5/6 → 4 with +0x48
5/6), orders the recognizers by 0x10005bd0 (the mode's own first:
1→phone, 2→code, 4→ordinal, 7→time, 9→real, 10→integer, 11→fraction,
12→money) and renders by 0x10006990. Recognizers: date 0x10003c60
(outside mode 8 the date order becomes `ymd`; slot 7, else a year range
0x10003230 `1990-2000` or `1990 - 2000` (dash 0x100177a8; the two tokens
after it consumed), else a VoiceXML date in mode 8 with order `b`
0x100179a0), time 0x10003e00 (also `h:mm,dd`), ordinal 0x100040c0 (mode
4; before `°`/`º` 0x100179b0 / 0x100179ac unless `c` 0x100179a8 follows;
an apostrophe not before a number; `:a`/`:e` 0x10017860 / 0x10017848,
consumed, refused when the text has `-/,` 0x100179a4), money 0x10004360
(a currency sign after or before; no currency names, no forced money
mode), real 0x100044c0, fraction 0x10004770 (list 0x10002330), phone
0x10004890, integer 0x10004ca0, code 0x10004ad0 (separators `:/-+`
0x100179b4). The renderer differs from German's: ordinals only with
+0x48 5/6 (0x10002620: tens and hundreds as cardinals with a space after
each, `tjugo femte`, appending `0` 0x100177a4; a round ten or a longer
number ends in `nde` 0x100177a0; 1–20 their own words
0x10017694–0x1001779c); dates 0x100066e0 (day ordinal, month word
0x50+m, a year 1100–1999 as hundreds, else an integer and ` # `); year
ranges 0x100068a0 (`till ` 0x100179c8); times 0x10005110 (separators
`.,:` 0x100179c4, ` och` 0x100179bc after a non-zero hour, `och`
0x10017950 before the seconds); fractions 0x10002010
(0x10017588–0x10017690); money by `ELQNumConvertCurrency` with no
point/comma swap; slot 6 gets flag 1 only for codes (kind 2).

**Voices.** Annika (`voicefon/annika.rs`, LTTS7Annika.dll table
0x10009a40): FonVidAllofonoVoce 0x10001210 aspirates stops before a
stressed vowel and leaves them unreleased before a consonant (labial
0x1ff2 0x1fe2, velar 0xff2 0xfe2, dental 0x19f2 0x19e2, each with
0x19a2/0x18a2/0x10c2), FonVidMetaFonema 0x10002140 (→ 0x10001b80, the
default). Sven (`voicefon/sven.rs`, LTTS7Sven.dll table 0x10009a70):
FonVidAllofonoVoce 0x10001440 looks each word up in a dictionary
(0x1000c000 to 0x10009afc; `bort` 0x10009b00, `kanske` 0x10009b08), then
runs Annika's stop rules; FonVidMetaFonema 0x10002630 maps analysis ids
to synthesis ones (0x50→0x45, 0x51→0x47, 0x5b→0x57, 0x63/0x64→0x62).

**Checked**: `swedish_trace` (`lang/swedish/trace.rs`, `QQ_SV_TRACE`)
replays LesAnalizza, LesAccenta, LesGradiAccentuali, LesConfiniProsodici
and FonTrasParola call by call against `LOQ_TRACE_LES` traces of 20 000
held-out words, `bench-sv.txt` and `bench-sv-text.txt`: 20 108 / 20 108
LES calls and 20 590 / 20 590 FonTrasParola calls identical.
`numeri::tests::replay_slot9` (`QQ_SV_NUM`): 506 / 506 slot-9 calls (0–120,
big numbers, times, dates, year ranges, ordinals, money, fractions,
phones). `--fon-list oracle/words/sv.txt` (20 000 Hunspell `sv_SE` words,
evenly sampled): 99.955%. `bench-sv.txt` and `bench-sv-text.txt`, Annika
and Sven: FON 100% sounds, CAT 100%, samples 100% identical (before the
port: FON 42.2%, CAT 99.6% / 99.2%).

## 4k. Danish FON, ported from LTTS7Danish.dll (`lang/danish/`)

The export `Danish` (0x10003120) accepts a descriptor whose `Language`
list (split on 0x10025b54) has `danish` and hands the engine a table of
**46** `{name, function}` pairs at **0x100251b8** (LesGetSymbolName
0x100034c0, LesPronunciabile 0x10004640, LesElaboraParola 0x10004d60,
LesWriteCateg 0x100046e0, LesAccenta 0x100021c0, LesNormalizzaAccento
0x100019d0, FonTrasParola 0x1000bc20, FonAllofonoLingua 0x10017400,
LesConfiniProsodici 0x10005c50, LesAnalizza 0x1000b620,
LesGradiAccentuali 0x10004730, ELQLanguageLoadReadOnlyData 0x10002aa0,
LesIsLetter 0x10004fc0, the number hooks LesGetMorphTable 0x100081e0,
LesGetOrdinalStringTable 0x100081f0, LesGetClassifyTable 0x100081c0,
LesGetCurrencyTable 0x100081d0, LesClassifyFunc 0x10008420, LesParseData
0x10008e20, LesReadData 0x100082e0, LesReadFrazione 0x10008200,
LesReadCodice 0x10007770, LesDoComposed/LesDoOrdinalDay 0x100030a0,
LesDoAndBetweenTenthsUnits 0x100030b0, LesDoTenthInversion 0x100030d0,
LesDoYearSplit 0x100030e0, LesDoAndBetweenThousandsHundreds 0x100030f0).
No LesIniNumeri: numbers go through LoqTTS7.dll's generic number reader
(§4n). Native encoding 0x4e4 (1252, 0x100024b0); characters
0x10024200 (8 bytes: lower, upper, flags +4); phones 0x10024de0 → 124
`{symbol, code}` at 0x10024a00.

**The word analyser.** The module links Loquendo's "WordAnalyzer"
(`lang/wordanalyzer.rs`, shared with Russian; name table 0x100731f0,
initAnalyzer 0x1001bbc0 → 0x1001bb40: 0x1ad nodes at 0x10076308, types
`COMPOUND_CONNECTOR…` 0x1008094c (5), feature pairs 0x10073298 (0x48 × 8),
attributes 0x10080960 (0xb), values 0x10080990 (0x56), group names
0x10080cd0 (0x218 = getGroupingCount 0x1001bc30), group details 0x10078790
(0x18 bytes: +4/+8 the A/B marks, +0xc FA and +0x10 FS stress function,
+0x14 key, +0x16 override entry), override patterns 0x1007d88c, stress
function names 0x100d9df0; its +0x3c callback 0x1001bb30 returns 0). Its
lexicon entries carry a respelling after their spelling (`godmorgen` →
`gom\xe5\xe5n`, `mod` → `mo~d`: `:` long, `~`, `?` stød, `^` `"` stress).
The loader (0x10002aa0) reads `Danish.dbl`'s `TD_Abbreviations` (NOCASE
DESC) and `TD_Phonetizations` (`PH_Graphemic, PH_Phonetic`, BINARY ASC;
one test row in the shipped file).

| function | address | what it does |
|---|---|---|
| LesElaboraParola | 0x10004d60 → 0x10004a30 | the token (0xac as `-`) lowered (≤ 63 letters; `xxversionxx` 0x10026070 → 0x10026058) and analysed when mode 0 and readable, spelled, or expanded; else one record per LesGetSymbolName (mode 1) name, 0x15/0x83, the last 0x84. No analysis: split at the first `-` after the first letter, or the letters' names run together (0x100042b0) as 0x15/0x84 when unreadable. Kept: the analyses up to the first with more morphemes than the best (0x100048a0), each morpheme's getMorphemeInfo (0x1001bd60) fields in the LexInfo buffer (`{count, 10 × 0x194}`), a stem's group overridden from the next morpheme's key (0x10004930), an inflection after it ungrouped |
| LesPronunciabile | 0x10004640 → 0x10004420 | V/C pattern: no vowel, one vowel not in `aiAI`, ≥ 5 in a row, `^CC$` 0x10025e80, `^CCC$` 0x10025e78, `^CCCCC`, `CCCCC$` (≤ 7 letters) unreadable; > 6 letters readable |
| LesGetSymbolName | 0x100034c0 | marked respellings of letters and symbols (strings 0x10025b64…0x10025e64, short ones as code immediates); punctuation only with the char argument set |
| LesNormalizzaAccento | 0x100019d0 | drops `` ` `` 0x92 `'` 0xb4 |
| LesAnalizza | 0x1000b620 → 0x10009140, 0x10009650, 0x1000b460 | phonetization rows; the solution by the neighbours' parts of speech (patterns 0x10026f94…0x10026fd4); class 0x1000acc0, info 0x1000ae60, sub 0x1000b2a0 from the morphemes' features |
| LesAccenta | 0x100021c0 → 0x10001730, 0x100010f0, 0x10001380, 0x100020d0, 0x10001570, 0x10001d30, 0x10001bf0, 0x10001ab0 | per morpheme (LexInfo +0xc, 0x20 bytes: marks A/B `{vowel, kind 1 ^ / 2 " / 3 ^^, stød 1 on it / 2 on the next consonant}`) the groups' FA functions (FA0, FA1, FACancAPStem, FAlig, FAor, FAShiftSuStemAPB1) then FS (FSCanc, FSCancSillabaAdiacente, FSIns, FSInsPoliAccB1), on the morpheme or its stem (a suffix's nearest stem before, a prefix's next); a word without a solution stressed on the vowel before its first accented letter (0x10001030) or its first; only the first main stress stays (0x10001570), none → grade 3 unless alone between punctuation; the respelling (+0x250) with per-letter flags (+0x14c: morpheme, type, 0x10 compound start, 0x20 `-`, 0x40/0x80 first/last, 0x100 `^`, 0x200 `^^`, 0x400 `"`, 0x800 `?`, 0x1000 `~`, 0x2000 `:`, 0x4000 respelled) |
| LesConfiniProsodici | 0x10005c50 → 0x10005780, 0x100054d0, 0x10005300, 0x10005000, 0x10005130 | group kinds by a class state machine, fixed phrases 0x10026008…0x1002601c, pause 0x0a, punctuation boundaries |
| LesGradiAccentuali | 0x10004730 | punctuation 0x0f, 0x0e → 0 |
| FonTrasParola | 0x1000bc20 (loop 0x1000bcf0) | per letter of the marked respelling: punctuation 2 (`#` `,`) / 3; doubled consonants once (not `rr` `cc`); respelled letters (0x4000), `£`, 0x80 by the switch at 0x100131bb, the others by the spelling rules at 0x1000bfa6; mark tests 0x1000b790…0x1000b850, stressed vowel ahead 0x1000b8b0 (after a preposition, in the next word: the sentence's records); phonetization rows parsed by 0x1000bb50/0x1000bac0; sets vowels 0x10027108, consonants 0x10027250, back 0x1002714c, front 0x10027164; morphs `inde` `agtig` `ig` `isk` `be` `af` `et` `und` (0x100271b0…0x10027204) |
| FonAllofonoLingua | 0x10017400 | ~70 ordered phone-sequence rules over the chunk list (first match wins); `\SAMPA=` (0x10027280) tags handled by the tail 0x1001a093 |

**Frida** (`voicefon/frida.rs`; export 0x100011a0, table 0x10009c28):
FonVidAllofonoVoce 0x10001210 (`20001ff2`/`20000ff2`/`200019f2` →
`5ff2`/`4ff2`/`59f2` before a 0x1e2 consonant; `a` → `Ae` in
`dansk`/`danmark` 0x10009cb4/0x10009cc0; `har` 0x10009cbc loses its r;
medial `501` dropped before `n` → `219c2`), FonVidMetaFonema 0x10001c10
(0x9f2 after a `w`/`p` phone merges into its stød id, 0x10001ba0).

**Checked** (`QQ_DA_TRACE=<file.les>`, traces of `LOQ_LES_DA=1`,
`oracle/lestrace.h`): bench (`oracle/bench-da.txt`) and 20 000 held-out
words (`oracle/words/da.txt`, LibreOffice `da_DK/da_DK.dic` evenly
sampled): LesElaboraParola 174/174 and 19 920/19 920 analysed words,
LesAnalizza, LesConfiniProsodici, LesGradiAccentuali 46/46 and 19 994 /
19 994 calls, LesAccenta 174/174 and 20 106/20 106 records,
FonTrasParola 174/174 and 20 106/20 106, FonAllofonoLingua 86/86 and
39 988/39 988 chunks; Frida's stage explains every event-2/event-3
difference of the bench. Whole transcriptions (`QQ_NO_LEXICON=1`): held-out
words 19 994 of 20 000 (the 6 left are Greek letters outside Windows-1252,
empty in both); bench `oracle/bench-da.txt` FON 27.2% → 88.8% → **100%**
since the numbers (§4n), CAT 100%, samples 100% identical;
`oracle/bench-da-text.txt` 69.2% → **100%** (§4n).

## 4l. Polish FON, ported from LTTS7Polish.dll (`lang/polish/`)

The export `Polish` (0x1000e050) accepts a descriptor whose `Language`
list has `polish` and hands the engine a table of 30 `{name, function}`
pairs at **0x10022d78**: GetVersion 0x1000df70 (7.13.0),
ELQGetLanguageNativeEncoding and …DefaultAnsiEncoding 0x1000e020
(**0x4e2 = 1250**), …IsoEncoding 0x1000e030 (0x6fb0 = ISO-8859-2),
…OemEncoding 0x1000e040 (0x354 = 852), ELQGetGraphemes 0x1000d890
(0x100222a0: 8 bytes per character, lower, upper, flags at +4 as Greek's),
ELQGetPhonemes 0x1000d8a0 (0x10022c78, `{entries, u16 count}`, 59 entries
`{symbol, code}`), ELQLanguageLoadReadOnlyData 0x1000d990,
LesGetAbbreviazioni 0x1000deb0 / LesGetNumeroAbbreviazioni 0x1000de60,
LesGetDefaultCurrency 0x1000a5a0, LesNormalizzaAccento 0x100088c0,
LesAccenta 0x10001280, LesPronunciabile 0x10008f30, LesIniNumeri
0x1000c450, LesAnalizza 0x1000e640, LesGetSymbolName 0x10008d90,
LesGradiAccentuali 0x1000c510, LesConfiniProsodici 0x1000d470,
FonTrasParola 0x10010f60, FonTrasConfini 0x1000e9e0, FonAllofonoLingua
0x100105a0, FonGetSamples 0x1000d8b0, LesIsPartOfAcronym 0x1000cdb0 (flag
2), LesIsLetter 0x1000cdd0 (flag 1), LesIsUpper 0x1000cde0 (flag 0x20). No
LesElaboraParola, LesSetModalitaFrase nor LesGestisciMaiuscole: the
engine's own. The text is Windows-1250 (`Les::code_page`, `to_8bit_cp`
as Greek's 1253). The loader 0x1000d990 reads `Polish.dbl` (353 rows) with
`SELECT grafemica, fonetica FROM lessico` and the 145 abbreviations (`…
WHERE abbreviazione!="" ORDER BY abbreviazione COLLATE NOCASE DESC`); the
rows are searched by LTTS7Util's `ELQDBTableFind` (a binary search with
`ELQstricmp`, A–Z folded, then back over equal rows).

The module writes stress into the spelling: LesAccenta gives the
stressed vowel its accented letter (0x10001050: `a` `á`, `ą` `ã`, `ę`
`ě`, `o` `ô`, `ó`/`u` `ú`, `e` `é`, `i` `í`, `y` `ý`), and the later
stages read those letters as the stressed vowels (0x1000e660 takes them
back off).

| function | address | what it does | data |
|---|---|---|---|
| LesAnalizza | 0x1000e640 | 0x1000e470: a record of class 0x17, a proper noun (0x15, sub 0xc8) or of subclass `z` found in the closed-class words (binary search by `strcmp`) takes their class, subclass, information and grade; 0x1000e600: a record still 0x17 is classed by its reversed text in the ending table (0x1000e500 → 0x1000e3d0, searched as Italian's): 1 adjective 0x29, 2 adverb 0x33, 3 verb 0x2a sub 0x8b, 5 homograph 0x2d, 6 verb sub 0x88, 7 verb, else 0x2f | words 0x1003d960 (0x201 × `{word, class, sub, info, grade}`); endings 0x1003b2f0 (0x334 × 12 bytes) |
| LesAccenta | 0x10001280 | every record not elided and without an accented letter (0x10001240) gets one vowel marked (0x100011c0, counted from the end, an `i` before a vowel not counted, 0x10001000): the word lists' vowel; `arcy…` of 3 vowels, `eks…` of 2, `wice…` of 3 the last; a verb ending `byśmy`/`byście` the 4th from the end, `śmy` `ście` `bym` `byś` (and `by` after `y`) the 3rd; `eś`, `set`, `kroć` the 3rd; a verb after `nie`/`nié` the 2nd, or none for one vowel (the `nie` then grade 0); else the last of one vowel, the 2nd from the end of more. A text over 63 letters stops the function | lists (`ELQSearchString` flag 0x11): 0x1001e608 × 5 (1st from the end), 0x1001e620 × 0x163 (3rd), 0x1001ebb0 × 0x37 (4th), 0x1001ec90 × 0x29 (2nd); `nie` 0x1001ed3c, `nié` 0x1001ed38 |
| LesConfiniProsodici | 0x1000d470 | 0x1000ce00 groups the words by a state machine over the classes (`*` `+` `,` verbs, non-finite subclasses 0x88 0x8e 0x89; `a` `al` `de` `del` 0x10020f64 0x10021dc0 0x10020f50 0x10021dbc keep a group open before a verb), 0x1000cb60 / 0x1000c980 keep multi-word expressions together (texts joined by spaces, unaccented, 0x1000c920); then by group type the boundary 5 at a phrase end (0x0a on the phrase before when it may break, 0x1000c780, and is long, 0x1000c650: more than 3 words and 14 letters on both sides), 9, 1, 1/3 for function words / others, punctuation as the engine's own (`common::confini_default`), class `7` 0x16; then subclass `y`/0x84 7, `z`/0x83 6, `{` 8. 0x1000c780 keeps `môże` (0x10021d8c) with a verb | expressions 4/3/2 words 0x10021adc × 4, 0x10021af0 × 0x27, 0x10021b90 × 0x7f; 3/2/1 0x10021ab0 × 1, 0x10021ab4 × 1, 0x10021ab8 × 9 |
| LesGradiAccentuali | 0x1000c510 | punctuation and class `7` 0x0f; a word of grade 3 (from the closed-class words) after a boundary 0–3, 5–7 or 0x16 keeps it when it has at most 7 letters; subclass 0x83 3; else 0 | — |
| LesNormalizzaAccento | 0x100088c0 | `` ` `` `'` `’` `´` → `'` | — |
| LesPronunciabile | 0x10008f30 → 0x10008e00 | with `` ` `` or `'`, an accented letter or a vowel other than `aeiouy` it reads; `z`, `w` alone read (0x100210dc, 0x100210d8); else when it has a vowel (0x10008dd0) | — |
| LesGetSymbolName | 0x10008d90 → 0x100088f0 | the character's signed value + 0x80 indexes the bytes at 0x10008c90 (above 0xfe the default), which index the jump table 0x10008b70; each case loads its name (`mov eax, name`), the names given only in mode ≠ 0 behind `test al, al`; the default 0x10008b67 `""` | — |
| FonTrasParola | 0x10010f60 | the word without stress marks (0x1000e780) in the dictionary: a `fonetica` other than `""` (0x1001efd7) converted symbol by symbol (0x1000e890: symbols split at space or `-`, at most 5 characters; 0x1000e800 looks each up in the phone table, unknown 0x10); else the word (stress marks off only at grade 3, 0x1000e960) may start with a spelling of the word-start table (0x10010e20 flag 2, a prefix), whose phonetic string gives the first codes; every other letter by its left and right context (`strncmp` lists of the code's own literals): voicing and devoicing within the word, `rz` `sz` `cz` `ch` `dz` `dź` `dż`, `i` before a vowel softening the consonant, nasal vowels before fricatives and stops, `n` before velars, final devoicing. Punctuation `"` `'` nothing, `#` `,` 0x13, other 0x17. The `dż` + `żmy` test compares 2 characters only (`żm`) | word starts 0x1003f098 (0x29 × `{spelling, phonetic}`); `ELQSearchString` flag 0x12 (an entry starting the word): `s` + `i` read 0x1952 0x1003efb4 × 0xe, `rz` read r + z 0x1003eff0 × 0xc, `u` glide 0x1003ef98 × 6 |
| FonTrasConfini | 0x1000e9e0 | before the incoming word's codes (first code `next[0]`, its text, the word before's text and boundary +0x11; `""`/0x0f for a chunk's first) the list's last phone is recoded: after `lúb` 0x1ff2 → 0x1fe2, `w wbréw przéciw naprzéciw` 0x1d52 → 0x1d42, `ôd pôd spôd przéd nád znád sprzéd wśrúd pônad` 0x1bf2 → 0x1be2, `béz przéz pôprzez z` 0x1952 → 0x1942 (also before a voiced obstruent), `áż íż gdziéż czýż gdýż níż` 0x2001752 → 0x2001742, each before a sonorant (`m n r l ł j`, not `rz`) or a vowel; 0x100002e1 + 0x101003e1 → + 0x1fc2 before 0x1ff2/0x1fe2, → one 0x2e1 before 0x401ec2/0x1003e1; voiceless → voiced before a voiced obstruent (`& 3 == 2`, bit 0x10 clear, not a sonorant): 0x1ff2 0x1d52 0x1912 0x1001712 0x1bf2 0x1952 0x1001752 0xff2 0x2001752 0x2001712; 0x19c2 → 0xfc2 before a velar; 0xff2 → 0x200ff2 before 0x7c1/0x10c2; 0x18a2 → 0x2018a2 before `i`/`í` + a consonant; a word ending `j` + one letter (0x7c1) before a pause gets a second 0x7c1 (`ELQListAddElementAfter`, kind 0x69) | lists 0x1003f024 × 1, 0x1003f02c × 4, 0x1003f040 × 9, 0x1003f068 × 4, 0x1003f07c × 6 (flag 0x11) |
| FonAllofonoLingua | 0x100105a0 | a voiceless obstruent before 0x1942/0x1d42 and then a voiced obstruent or a vowel voiced (0x1ff2 0x1952 0xff2 0x1912 0x1bf2 0x1d52 0x2001712 0x1001712 0x1001752 0x2001752 → the voiced code); 0x1942 0x1001742 0x2001742 devoiced before a voiceless obstruent; 0x1be2 + 0x1be2 of the next word: a 0x501 between them (`ELQListFindElement` + `ELQListAddElementAfter`). Then only for a word whose tag holds `\SAMPA=` (0x1003f238): glides and palatals (0x401ec2 → 0x1003e1, 0x101003e1 after a marked vowel, 0x1fc2/0x19c2 → 0x1dc2, 0x11c2 → 0x101007c1, 0x19c2 → 0x2019c2/0x101003e1, 0xfe2/0xff2 → 0x200fe2/0x200ff2, 0x10c2 deleted after 0x200ff2/0x200fe2 or → 0x1007c1). An added entry's kind is the stack's byte (the local element sets word and code only): 0xe3 in every trace | — |

**Numbers** (`polish/numeri.rs`). `LesIniNumeri` 0x1000c450 fills slots 0
0x10008690 (word ids → strings, `{id, word}` 0x1001f078 × 0x53, `""`
0x1001efd7 else), 1 0x10008610 (phone: area codes 0x1001f020 × 2,
`0801`/`0800`, `ELQSearchString` mode 2 a prefix, then
`ELQNumConvertPhoneNumber` steps 0/1), 2 0x1000b790 (cardinal), 3
0x1000b750 (one digit: 0 id 0xf, 1 id 0x10, else 0x1000aad0), 4
0x10008660, 5 0x1000a8c0 (currency names 0x1001f028, 4 rows `{symbol,
unit, units, cent, cents}` for `$ € $. €.`; `"1"` 0x1002055c the
singular), 6 0x1000c290 (decimals), 7 0x100086c0 (date parts), 9
0x1000bf00. Slot 9 has Greek's skeleton but reads the neighbouring
tokens by raw index (tags included): reading types 5/6 → mode 4 with
+0x48 = 5/6; mode 0xc for a currency symbol next or before or a next
`euro` 0x1001e578 / `EUR` 0x10021118 / `eur` 0x10021114 (`€` 0x1001efe8)
or `dolar` 0x1001f004 / `dolary` 0x1001effc / `USD` 0x10021110 / `usd`
0x1002110c (`$` 0x1001f00c); nine recognizers 0x1000b150 in order (date
0x10008f50, time 0x100090b0, ordinal 0x100094d0 with `°` 0x10021108 /
`º` 0x10021104 not before `c` 0x10021100, currency 0x10009680, type 9
0x10009a20, fraction 0x10009cd0, phone 0x10009df0, real 0x1000a200, code
0x1000a030), then the renderer 0x1000b230 (as Greek's except kind 4 →
0x10001cc0, 7 → 0x1000a5b0, 8 → 0x10008730, 0xb → 0x100016b0, 0xc swaps
`,`/`.` for `ELQNumConvertCurrency`). Cardinal 0x1000b790: hundreds
0x1000acf0, tens and units 0x1000aad0 (a code switch), two digits
0x1000af40; after a group digit 2–4 `miliardy ` 0x1002114c, ` miliony `
0x10021140, ` tysiące ` 0x10021134 unless a tens digit 1 was read
anywhere before in the number (the flag is never reset: `12 002 000 000`
says `milionów`, as the original); a trailing word 0 (`" # "`) is cut.
Ordinals: 0x10001cc0 (masculine/feminine) matches the text exactly
against `"0"`…`"100"` in its string blocks, read by walking consecutive
NUL-padded strings from 100 down (masculine `{word, "n"}` 0x1001fc04,
202 strings; feminine 0x1001f42c, 101), else a cardinal; 0x100065a0
(genitive, 0x1002056c, 100 strings 100…1) after stripping leading zeros.
Time 0x1000a5b0: the hour a feminine ordinal, minutes and seconds
cardinals joined by word 0xd, ` sekundy ` word 0x5d after seconds (`h`
0x100210fc, `:` 0x100210f8). Date 0x10008730: masculine ordinal day,
month word 0x50 + m, `ltoa(y / 100 * 100)` as a cardinal, then the
genitive of `y % 100` (VXML order 0x100210f4). Fractions 0x100016b0 (14
pairs, keys 0x1001f428…0x1001f320, readings 0x1001f41c…0x1001f310;
0x100019d0 tests membership). `:/-+` 0x10021120, `.,:` 0x10021130.

**Voices** (`voicefon/polish.rs`). Krzysztof (LTTS7Krzysztof.dll, table
0x100087b0): FonVidAllofonoVoce 0x10001210 does nothing, FonVidMetaFonema
0x100015c0 (→ 0x10001220) the default. Zosia (LTTS7Zosia.dll, table
0x100087b8): FonVidAllofonoVoce 0x10001060 adds a 0x501 after a 0x1be2
followed in its word by 0x1be2/0x1001702/0x1902, and after a 0xfe2
followed by the next word's 0x1be2 or in its word by those three (not
before the run 0x1001702 0x6c1 0x107c1 0x2019c2 0x1001702 0x6c1 0x1007c1);
FonVidMetaFonema 0x10001590 (→ 0x100011f0) the default. Both libraries'
FonVidAllofonoAnalisi (0x10001290, 0x10001260) run only in the engine's
analysis mode.

**Checked** (`lang::polish::trace::polish_trace`, `QQ_PL_TRACE=<file.les>`,
traces of `LOQ_LES_DLL=LTTS7Polish.dll LOQ_LES_TABLE=10022d78
LOQ_LES_N=30`): on the bench, 60 written sentences, 1 500 generated ones
(closed-class and held-out words, punctuation) and 20 000 held-out words
(`oracle/words/pl.txt`, evenly sampled from LibreOffice `pl_PL/pl_PL.dic`,
ISO-8859-2), LesAnalizza, LesAccenta, LesConfiniProsodici and
LesGradiAccentuali (26 924 calls each), LesNormalizzaAccento,
LesPronunciabile, LesGetSymbolName, FonTrasParola (45 192 calls;
`polish_fontras_trace` in `parola.rs` replays it alone), FonTrasConfini
(each chunk's list against the one FonAllofonoLingua is handed) and
FonAllofonoLingua (28 322 chunks) give the original's records and codes
for **every** call. Numbers:
`polish_slot9_trace` (fixture `polish/numeri.les`) replays 960 traced
slot-9 calls identically; only reading type 0 reaches the module untagged.
Whole transcriptions (`QQ_NO_LEXICON=1`): held-out words 1.0% → **100%**
(20 000 / 20 000); `bench-pl.txt`, Krzysztof and Zosia: FON 36.9% / 36.8%
→ **100%** of sounds, CAT 99.5% / 98.4% → **100%**, samples 100%
identical; `bench-pl-text.txt` (30 lines of numbers, dates, money,
abbreviations): FON 19.5% / 19.4% → **100%**, CAT 99.7% (§11).

## 4m. Russian FON, ported from LTTS7Russian.dll (`lang/russian/`)

The export `Russian` (0x1000a1b0) hands the engine a table of 56
`{name, function}` pairs at **0x1009f270** (LesGetSymbolName 0x100014f0,
LesPronunciabile 0x10002c60, LesElaboraParola 0x10003280, LesAccenta
0x10001280, LesNormalizzaAccento 0x100013e0, FonTrasParola 0x1000a620,
FonAllofonoLingua 0x1000d570, LesConfiniProsodici 0x10003d80, LesAnalizza
0x10008f90, LesGradiAccentuali 0x10002d60, LesIsPartOfAcronym/IsLetter/
IsUpper 0x10003530/0x10003550/0x10003560, and the old number interface:
LesGetMorphTable 0x100059c0 → 0x100a2e30, LesGetClassifyTable 0x100059a0 →
0x100a2218, LesGetCurrencyTable 0x100059b0 → 0x100a2258,
LesGetOrdinalStringTable 0x100059d0 → 0x100a2e40, LesClassifyFunc
0x10004860, LesRead* 0x10005f70…0x10007d70, LesDo* 0x100046b0…0x100047c0,
Les{Set,Get}NumberReadingMode 0x10006ab0/0x10006af0). There is no
LesIniNumeri, LesSetModalitaFrase nor LesGestisciMaiuscole (the engine's
run). `ELQGetLanguageNativeEncoding` 0x10009d70 = **0x4e3 (1251)**, Iso
0x6fb3, Oem 0x362: the engine converts its input to Windows-1251. The
module calls LTTS7Util through 0x61 pointers it fills by name (`{name,
dest}` at 0x10119ea8, 0x10009b90). Character classes (`ELQGetGraphemes`
0x10009c90 → 0x1009e760, 8 bytes per character, flags +4: 1 letter, 2
acronym, 8 consonant, 0x10 vowel, 0x20 upper, 0x80 accented, 0x100
punctuation); phones (`ELQGetPhonemes` 0x10009ca0 → 0x1009f168).

**There is no dictionary file** (`Russian.lde` names only `Russian.lex`,
the text filter). The lexicon is a morphological analyser linked into the
module (`lang/wordanalyzer.rs`, the same code as LTTS7Danish.dll's): its
read-only object (0x100094a0) has 0x4e1 nodes at 0x100975d8 (0x14 bytes:
id, name, start +8, final +9, transition count +0xc, transitions +0x10);
from the start node `MAIN_END` it strips morphs off the end of the word
(transitions: next nodes +4 `{count, ids}`, morph list +8 `{?, count +4,
entries +8}` sorted by reversed spelling; a morph `{spelling, id +4,
class +8 (i16), length +0xa, flag +0xb}`, its spelling followed by its
**stressed form**) until `MAIN_START` with nothing left; 37 555 morphs.
0x1000ff80 matches a morph, 0x10010840/0x10010920 walk, 0x100106e0 and
0x10010600 skip to the next candidate morph, 0x100102d0 keeps at most 10
analyses (≤ 10 segments) ranked by 0x10010190 (bubble sort 0x10010270:
fewer segments first, then the last morph's length, then the root's
class), 0x10008fc0 accepts a path whose root (flag < 0) is no longer
than the rest, 0x100103c0 reverses the segments (root first). The
feature table at +0x24 (0x1008d6c0, 0x135 entries `{u8 count, (key,
value)*}`, 0x10010040/0x10010060) gives each morph's features (6 part of
speech, 7 government, 1 case, 3/4 gender/number, 8 verb form).

| function | address | what it does | data |
|---|---|---|---|
| LesElaboraParola | 0x10003280 → 0x10002f60 | lower-cases the token (0x1009e760), `xxversionxx` → 0x10066880; one record (0x17, 200, 9, grade 0x0e, boundary 0x15; 0x10002dd0) with the analyses in its LexInfo (0x10009eb0: analyses, rank in use +4, stressed vowel +0xc, stressed spelling +0x10); without an analysis split at the first hyphen (`-то` 0x1006687c kept with its hyphen) or spelled (0x10002640: class 0x15, sub 0x83/0x84, boundary 7/8, each LesGetSymbolName stressed on its last vowel) when LesPronunciabile says no; a one-character token between sentence boundaries is spelled | — |
| LesPronunciabile | 0x10002c30 → 0x10002880 | a single letter unless a consonant other than `ж в к с`; one of 0x51 letter groups (0x100a2000, 0x10002790); otherwise a vowel needed, then readable with `ьъы` (0x10065f28), more than 6 letters, or a C/V shape other than `^VCCCCC$ ^CCCCCV$ ^VCCCC$ ^CCCCV$ ^VCCC$ ^CCCV$` (0x10065f1c…0x10065ef0) | — |
| LesGetSymbolName | 0x100014f0 | letter names (`б~е`, `и кр^аткое`…, with their stress marks), digit and symbol names (most only with the third argument set) | strings 0x10065a24…0x10065ee0, 0x1001c344, 0x10020500 |
| LesAnalizza | 0x10008f90 | 0x100083c0 chooses each word's analysis (LexInfo +4) by its neighbours' part of speech: after `был/было/были/была` (0x10067d14…), `все` (0x10067cf8), `сам…` (0x10067ce4…), prepositions (government feature 7 → patterns 0x100a3134…0x100a31fc, word patterns 0x100a2f38 ×29), agreement patterns 0x100a2e4c/0x100a2e58/0x100a2e60 (0x10008230), feature patterns 0x100a2f14…0x100a31d4 (0x10008300); then 0x10008e10: class from feature 6 (0x10008a20: 6 → 0x29, 7 → 0x33, 0xc → 0x1f, `7` → 0x19, `1` → 0xc9, 0x2e for `не`/`нет`, 0x1d → 0xca, `8` → 0x20, `^` → 0x2a, `$'()-./` → 0x15), information from gender/number (0x10008b60), subclass from verb form/case (0x10008ca0); number words and `/` words class `y` (the one before `z`) | — |
| LesAccenta | 0x10001280 → 0x10009520 | the stressed spelling: the root's and first suffix's stressed forms; the vowel = the root class's base (0x1008e068 `{i16 vowel, i16 pattern}`) + the suffix's shift in that movement pattern (0x1008e8d8, 0x10009100); a word the analyser lacks: brand names for spelled records (0x10119b70 ×68, 0x100092c0) or Latin transliterated (0x10009170), stress from the ending table 0x10059cc8 ×0xf9a (`common::lookup`, 0x10001460); `не` before `быть/был/было/были` (0x1009d830…) takes the stress, `быть…` after `не` none, `на плацу` the 2nd vowel; a marked spelling (`^`, `ё`) keeps its mark; vowel index by 0x10009000; grade 3 when unstressed | — |
| LesConfiniProsodici | 0x10003d80 | 0x10003a60 groups words by class (fixed expressions 0x10066630 ×0x17, 0x10066690 ×0x6d, 0x100037b0), then boundaries 5/3/1/9/0x16 by group kind, 0x0a where a long phrase splits (0x100035b0, 0x10003690), punctuation by sentence type (0x0f–0x13) or `,)(-.` 0x0b (0x100668dc), number words 6/7/8 | — |
| LesGradiAccentuali | 0x10002d60 | punctuation and `7` grade 0x0f, unset 0 | — |
| FonTrasParola | 0x1000a620 | the stressed spelling letter by letter, each by its neighbours (skipping `^`): soft consonants before `яюеёиь` (0x1009f7d0), voicing before `бгджз`, devoicing before `цкшщхфпчст` and at the word's end, `сч сщ зч` → щ, silent `д л т в` (`здн`, `лнц`, `стн`, `вств`), `что`, `ться`, akanye/ikanye of unstressed о а е я by whether the next syllable is stressed (0x1000a350, into the next word after a preposition), word-final reductions in a chunk's last two words | letter sets 0x1009f758…0x1009f800, 0x1001b5e4…0x100658b4 |
| FonAllofonoLingua | 0x1000d570 | inside a word: е/и after hard/soft consonants and j, final `ий` loses й, regressive voicing, double consonants said once (0x1000d520 position in word), `ссч` → щ; at a word's start the previous word's last consonant voiced before a voiced obstruent (and after `об над под перед из`), ы → и after a one-phone word | — |

Voices Dmitri (table 0x10008728) and Olga (0x10008750), the same code at
the same addresses (`voicefon/russian.rs`): FonVidAllofonoVoce 0x10001210
softens s/z before tʲ dʲ nʲ and t before nʲ inside a word and turns a
word-final `ə j ə` into the unit 0xe6c5; FonVidMetaFonema 0x100014b0 is
the default but for the code 0x942, which the module never gives.

Checked call by call (`russian::trace::russian_trace`; `LOQ_LES_RU=1
LOQ_LES_DLL=LTTS7Russian.dll LOQ_LES_TABLE=1009f270 LOQ_LES_N=56`, which
also dumps each record's LexInfo; `oracle/lestrace.h` now hooks up to 64
slots): on the bench the analyser (138 words), LesAnalizza, LesAccenta,
LesConfiniProsodici, LesGradiAccentuali (46 sentences each),
FonTrasParola (158) and FonAllofonoLingua (50) give the original's
analyses, records, stress and codes for every call. Whole transcriptions
(`QQ_NO_LEXICON=1`): held-out words (`oracle/words/ru.txt`, every 7th
alphabetic word of LibreOffice `ru_RU/ru_RU.dic`) **100%** (20 000 /
20 000); `bench-ru.txt`, Dmitri and Olga: FON 38.9% → 90.4% → **100%**
of sounds since the numbers (§4n), CAT and samples 100%;
`bench-ru-text.txt` (30 lines): 70.2% → **100%** (§4n).

## 4n. The engine's generic number reader (LoqTTS7.dll; `lang/loqnum.rs`)

**Who uses it.** A module without `LesIniNumeri` (Danish, Russian) leaves
slot 9 empty; 0x1003f520 then calls `LesNumero2Testo` (0x1003ebf0) with
the escape the number tags ask for (kind 1→1, 2→2, 4/5→4, 6→5, 7→7, 8→8,
9→0xb, 10→6, 0xb→10, 0xc→9, else 0). Its first call builds the whole
reading in a buffer; the next ones `strtok` it on spaces (a `#` piece
marks the record before 0x7b, as for slot 9). The module fills the
binder's number slots by name (LES 0x10036140, from 0x1003667a): +0x50
LesNumlettFunc, +0x54 LesClassifyFunc, +0x58…+0x88 LesParse* (Telefonico,
Codice, Intero, Reale, Ordinale, Ora, Data, Valuta, Frazione, Angolare,
Misura, Risultato, UserDefined), +0x8c…+0xbc LesRead* (the same order),
+0xc0…+0x10c the 20 LesDo* hooks
(YearSplit, TenthInversion, AndBetweenTenthsUnits,
AndBetweenHundredsTenths, AndBetweenThousandsHundreds, Truncation,
TruncateTenth, OrdinalTenth, Composed, Gender, Plural, OverOnFraction,
OrdinalDay, OfBetweenDayMonth, OfBetweenMonthYear, OClock, FeminineHour,
Hour, Minutes, Seconds; called `(tables, value, num, type)`, the engine's
all answer 0), +0x110…+0x11c the morph, classify (`{regexp, escape}`),
currency and ordinal-string tables. Empty slots get the engine's
functions (addresses in mechanics.json, `numbers-engine`).

**The driver** (0x1003ebf0): `ELQNumIni` (LTTS7Util 0x10015220: a 0x524
byte object, layout in mechanics.json), parse and reading mode = ctx+0x32c
(`dmy` unless a `date…` tag), escape; escape 0 asks LesClassifyFunc. The
escape's parser runs; if it refuses, the fixed order Data, Ora, Valuta,
Angolare, Frazione, Telefonico, Reale, Misura, Risultato, Intero,
Ordinale, Codice (types 8, 7, 9, 0xb, 10, 1, 6, 0xc, 0xd, 3, 4, 2); none →
no reading. Then the type's reader adds morph ids (`ELQNumAddMorph`),
LesNumlettFunc (0x10038d90) writes each morph's first morph-table string
and ctx+0x18 takes the skip count (+0x21c).

**The engine's parts** (`loqnum.rs`): the classifier 0x10038a20 (the
first classify regexp matching the whole text, `ELQRxcFindReplace` at
offset 0; patterns and text converted from 1252); the parsers 0x10038e40
(telephone: `+`/`00` and the country codes at 0x10071350, so
`pack.rs` now keeps `bin/LoqTTS7.dll` with a language), 0x10039010
(integers, big above 2147483647), 0x10039180 (ordinals: `°`/`º` not
before `c`, or `ª`, used up), 0x10039400 / 0x1003a280 (reals and amounts:
the separator rule, the amount's currency from the next token, used up, or
the previous), 0x100399e0 + 0x10039770 (times, ranges with one `-`),
0x1003a040 + 0x10039bd0 (dates in the parse mode's order, years 0–19 →
20xx, 20–99 → 19xx, 100–999 +1000), 0x1003a720 (fractions, `¼ ½ ¾`); the
readers 0x1003c060…0x1003de40 and the cardinal builder 0x1003aa30
(stem variant), 0x1003aaa0 (ending variant), 0x1003aba0 (digit),
0x1003aed0 (0–99), 0x1003b4d0 (0–999), 0x1003b790 (int), 0x1003bf10
(digits by pairs), 0x1003bcf0 (a character), 0x1003c840 (month names),
0x1003c580 (times), 0x1003c970 (dates); morph ids in mechanics.json. The
token helper 0x10038930 steps over tag tokens (type 5) and stops at the
end (type 7) or index 14; quoquendo gives it the number's sentence.

**Roman numerals** (0x100315a4, every language): a word after a
`number=ordinalM`/`ordinalF`/`cardinal` tag that is a canonical Roman
numeral (`ELQNumArabic2Roman` 0x10014c70: value 1–3999 written back by
0x10014f90 and compared, any case; a one-letter one may keep a dot)
becomes its number before the token is read (`common::roman_value`):
`XXI век`, `Ludwig XIV.`, `Christian IV`.

**Russian** (`russian/numeri.rs`): LesClassifyFunc 0x10004860 (the table
part as the engine's, plus dates: before a month name mode `d`, a four-digit
number before `году`… or after `году`/`год^у` mode `y`, after a month
name `y` or `d`), the engine's parsers, every reader its own
(0x10005f70…0x10007d70) on its builder copy 0x10004d40…0x100059e0 (a
separator morph argument: 2 for readings, 1 in codes; two before
thousands 0x6b, 200 0x397, 300/400 0x398, thousand forms 0x3bd/0x3be/0x3ac
by the last digit, million forms 0x3e3/0x3e4/0x3d2), dates with an
ordinal day and year (`пятн^адцатое марта две тысячи двадцать
четвёртого`), LesDoOrdinalTenth/OrdinalDay/Gender. The reading-mode user
data (LesGetNumberReadingMode 0x10006af0, slot +0x17c) is empty with the
engine's defaults, so the modes 0x1d/0x1e (reals) and 0x1f (codes) are not
modelled.

**Danish** (`danish/numeri.rs`): LesClassifyFunc 0x10008420 (as Russian's
with Danish words: `i siden år før efter inden indtil f.Kr. e.Kr. ad bc`
before a year, `f.Kr. e.Kr. ad bc`, `s`, `'s` after it), LesParseData
0x10008e20 (the engine's), LesReadData 0x100082e0 (0x100078a0: years
before 2000 split, `nitten hundrede og syvogfirs`), LesReadFrazione
0x10008200 (`en halv`, numerator and ordinal denominator with `del`/`dele`),
LesReadCodice 0x10007770 (digits spelled), the engine's readers for the
rest, and LesDoComposed/OrdinalDay 1, AndBetweenTenthsUnits,
TenthInversion (`femogtyve`), YearSplit, AndBetweenThousandsHundreds.

**Checked** (`LOQ_TRACE_NUM=1`, oracle/loqref.c: the token buffer and the
ELQNum object with its morphs after every reading; `QQ_TRACE_NUM=1` prints
quoquendo's): type, escape, skip, mode and morph list identical for every
number of `bench-ru` (3), `bench-ru-text` (38), `bench-da` (3),
`bench-da-text` (39) and two stress lists of telephone numbers, amounts,
fractions, times, ranges, big integers and dates (32 Russian, 33 Danish).

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

Every other voice with a benchmark: 100% except Dave 99.1%, Dmitri 98.9%,
Elizabeth 99.8%, Grace 99.3%, Olga 99.3%, Simon 98.9%, Stefan 99.7% (§11).
Annika and Sven reached 100% (from 99.6% and 99.2%) once their voice
stages were ported (§4j), Krzysztof and Zosia (from 99.5% and 98.4%) with
the Polish module (§4l).
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
Stefan, Steven, Susan, Sven, Zosia, and the eight 44 kHz CodecMsx voices
with a benchmark through the engine's own resampler (§2c): Alan, Dmitri,
Florence, Grace, Marcello, Olga, Willem, Ximena.

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
`LOQ_TRACE_RESAMPLE` (every chunk SIG hands the output resampler to
`<prefix>.res`; `=2` with its samples, `=3` also every call of 0x1001c370
with its input, history and time before and after, and every phrase's
flush, §2c),
`LOQ_TRACE_LES` (`oracle/lestrace.h`: every slot of the language module's
function table swapped for a logging thunk — each LES function's word
records before and after, FonTrasParola's word and codes, FonTrasConfini's
arguments — to `<prefix>.les`; `LOQ_LES_DLL`/`LOQ_LES_TABLE` for another
module than Italian's, with `LOQ_LES_N` its pair count (default 19;
hooking past a shorter table corrupts the next one and the voice fails to
load with 15). The slot-9 number reader `LesIniNumeri` installs is
hooked too: `> NUM`/`< NUM` lines with the type, index, date order, text,
reading and ctx+0x18, and the token buffer before (`T`) and after (`U`);
`LOQ_NUM_ONLY=1` logs only those. `LOQ_TRACE_NUM=1` logs the generic
number reader's work to `<prefix>.num` (§4n: the token buffer, then the
ELQNum object and its morphs after each reading; `QQ_TRACE_NUM=1` is
quoquendo's). `oracle/parse_les.py` turns it into JSON lines,
§4b). quoquendo's counterparts: `QQ_TRACE_CAT=1`,
`QQ_CHECK_MARKS=<trace.join>`, `--check-decode <trace.dec>`,
`QQ_RESAMPLE_TRACE=<prefix>.res` (`codec::tests::replays_the_engines_resampler`),
`QQ_DEBUG_JOIN=1`.

`LOQ_LINES=1` treats every line as an utterance; `LOQ_FON_ONLY=1` writes
`<prefix>.fonlist` with one transcription per line (87 421 words in 6 s).
Results are cached in `oracle/cache`. In the window, picking "Original engine"
for a stage, or "Compare with the original", runs it in the background.

## 9. Learned from the original

The Italian ending tables once learned from the original's transcriptions
(`it-position.tsv`, `it-quality.tsv`) are gone with the fitted rules: the
module's own tables (§4b) give the stress.

Then the transcriptions themselves became dictionaries (`--learn-lexicon`,
`lexicons/<MotherTongue>.tsv`: Italian 87 421 words, US English 128 376,
French 635 540) that brought the fitted rules to 96–100% of sounds on the
benchmarks. They are gone too, with `QQ_NO_LEXICON`: every language a voice
of the pack speaks has its module ported (§4b–§4m), and the results that
cite `QQ_NO_LEXICON=1` above are the ports alone.

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

- Polish (§4l): the original starts a new word at the 0x501 that
  FonAllofonoLingua adds between two d's of adjacent words (`brat 2.`:
  `b r a dd | HEh dd v a`); our voice stage (`voicefon.rs` 0x1002d180, word
  ends from record changes) leaves it on the word before. Behind a 0x501
  that Zosia's FonVidAllofonoVoce adds (`Rok 2000`, `pociąg do`), the
  original marks no word start for the rest of the phrase and for the
  first words of the next one. Sounds, pieces and samples are identical;
  only the word grouping differs (bench-pl Zosia 97.4% of words,
  bench-pl-text 98.5% / 97.6%). On `bench-pl-text.txt` CAT given the
  original's FON differs in `Zdobył 1. nagrodę, a jego brat 2. miejsce.`
  (68 of 76 half-sounds, both voices), the line with that 0x501; the
  samples of the whole file are still identical.
- Portuguese (§4g): the held-out words left are all-capital acronyms the
  engine spells (Portugal: `IBM PC AD ANC BMW CDU EDP NBA PAIGC PCP PSD PSR
  SPA TVI`) and Brazil's `bei`, which the module lists as an abbreviation
  read with `\{spell=yes` (the engine-wide text layer, `lang/common.rs`).
  In `dois mil e vinte e quatro` the voice's events end no word between
  `vinte` and `e` (`v i) n tt i`), ours do after `tt`: FonTrasConfini
  deletes vinte's last phone (0x1000ea87) and the record keeps boundary
  0x0e, so 0x1002d180 as ported (`voicefon.rs`, record boundary) marks a
  word end the original does not. Sounds, pieces and samples are
  identical; only the word grouping differs.
- Swedish (§4j): FON, CAT and samples are 100% identical on `bench-sv`
  and `bench-sv-text` (Annika, Sven). Held-out words 99.955%: the 9 of
  20 000 left differ only in a word-end `-` ours print and the original
  leaves out after a word whose last phone FonAllofonoLingua deleted
  (`torsdageftermiddag` → `… m `I d a`, `Nasdaq`), a rendering of the
  phone list, not its sounds. The `.fon` words metric
  (94.6% / 93.5%) misses word separators the original leaves out before a
  word starting with an aspirated stop or `h` (`n `2E s t a th `o:`) and
  after a deleted final phone; sounds, pieces and samples are identical.
  LesPronunciabile's look at the previous record (0x100039d0) is modelled
  by the previous token.
- The engine's text filters (§4h) run for every language. Left: Util's
  tag parser (ELQTagParser) and its `&…;` entities are not ported, so
  input that already holds `\}`, `\{(`, `\t`-like tags or `&amp;` can be
  rewritten differently (stress tests only; every `T` line of the five
  `bench-*-text.txt` is identical). Left on those benches after the
  filters (module FON, phones): English `\{spell=yes US` (the original
  spells `US`; the `spell` tag is read by `common::spell_tags` but the
  English path still reads `us`) and French `45 67` (below; the Roman
  ordinals after `\{number=ordinalM` are read since §4n). The English
  `#50` in "Seventy students sent…"
  (`oracle/bench-en-voice.txt`) is not found yet: the original's
  LesConfiniProsodici gives seventy 0x03, students 0x05, sent 0x05
  (`/tmp/qq-tl/s7.les`), no pause boundary.
- Italian (§4b): every LES/FON function of LTTS7Italian.dll the engine
  calls is ported; bench-it and the 20 000 held-out words are 100%
  identical. 0x10001290 (participles) is checked on 91 sentences with
  participles after an auxiliary (all identical), not on a held-out list.
- The output resampler's upsampling (linear interpolation, 0x1001c0e0,
  chosen by 0x10028b6d when the output rate is the higher) is not ported:
  quoquendo always speaks at 32 kHz and no voice is below it. Its small
  filter and SrcUP (0x1001c960, 0x1001bd10) are never reached from
  VoiceResamplerRun, which asks for the large filter (0x10028bc5).
- Two joins of 1 082 traced (Roberto, Bernard) differ in the next unit's
  tail margin only: the joined buffer is shorter than `b.pre + b.len +
  b.post` and the engine copies the rest out of its scratch memory
  (0x100b747c/0x100b74b4, the buffer reused across joins), whose stale
  contents we fill with zeros. Neither reaches the output on the benchmarks
  (replays 100% identical). One voiced half of 423 (Bernard) gets other
  pitch marks than the original's, also without effect there.
- CAT given the original's FON: 100% on 35 voices (Afroditi 98.1% →
  100%: Greek asks with `;`, which the compare tool took as no question;
  Annika and Sven since §4j, Krzysztof and Zosia since §4l);
  Dave 99.1%, Dmitri 98.9%, Elizabeth 99.8%, Grace 99.3%, Olga 99.3%,
  Simon 98.9%, Stefan 99.7% not
  yet diffed term by term (`LOQ_TRACE_CAT` against `QQ_TRACE_CAT`). The
  original's automatic pauses (TOP/LES) are a FON matter: given its FON the
  picker already gets them as pauses. Speed and pitch controls.
- The app's piece player (`src/app.rs`) and the JNI `analyse` read pieces
  from `db.audio`; pieces from a Gilded database need `db.bank(unit)`.
- LinLin (Chinese) has no LTTS 7.0 language data in any installer found
  (the original refuses her: with the voice unpacked into a full engine,
  `ttsLoadPersona` fails with 12 "Invalid function or API parameter").
  LTTS 7.2's Chinese is in Loquendo's Android build (archive.org
  `loquendo-tts-offline-v-1`): `libLTTS7Chinese.so`, ARM only, not ported,
  and its dictionary `Chinese.chd`, 58 742 UTF-8 `词\tpinyin` lines with
  tone sandhi inside words applied (`一万 yi2wan4`). `--unpack Chinese.chd`
  writes `data/languages/Chinese/` with an `.lde` naming it (7.2 has no
  `.lde`). `g2p::chinese` splits text by the longest dictionary word, reads
  typed pinyin (her `DemoSentence` is pinyin) and digits, applies sandhi
  across words (3-3, 不, 一) and turns syllables into the phones her 3 982
  recorded sentences use: initial, glide (`j w Hy`), toned final (`xuan2` →
  c Hy E2 n), with the allophones they show (`d` voiced in tone 5 and after
  a nasal, `Hg` on a vowel-initial syllable inside a phrase, `fu` → f Hfw U,
  `in`/`ing` before a pause as `i)`/`I)`). All 1 686 syllables of the
  dictionary map to recorded phones but `er1` (`ARR1`, never recorded; read
  `ARR2`). The 7.2 module's phone table (`ELQGetPhonemes`) is LinLin's
  alphabet; its `FonTrasParola`/`FonAllofonoLingua` are what this fits.
- Federica (Italian) exists only as an LTTS 7.2 bundle (archive.org
  `loquendo-7-federica-italian`, an Inno installer of the whole 7.2
  engine): voices and languages under `modules/<Name>/`, configured by one
  XML `global.ttsconf` instead of `.vcf`/`.vde`/`.lde`, her main database
  split in two (`VocalDBName` `Federica_1`, `Federica_2`; 7.0 names the
  first after the voice, the loader now takes `VocalDBName[0]`).
  `pack::place_ttsconf` writes a `.vcf`/`.vde` from her `<voice>` entry
  (`ttsconf`) and keeps only her files: the bundle's 7.2 language and engine
  DLLs differ from the 7.0 ones the ported modules are matched to, so she
  reads Italian from any 7.0 Italian installer. Its `global.ttsconf` lists
  26 more voices than the pack (Veena, Lisheng, Otoya, Laila…); none was
  found as 7.x data. The pack's `Loquendo 6/` Juan, Montserrat, Saskia,
  Silvana and Valentina are LTTS 6 (6.0/6.1).
- Jordi (Catalan), Empar (Valencian) and Carmela (Galician) have no module
  in their voice installers either; theirs come in Loquendo's separate
  language distributions (`Catalan & Valencian Distribution.exe`,
  `Galician Distribution.exe`, Wise MSIs like the voices'; one
  `data/languages/Catalan` folder holds `CatalanCe` and `CatalanVa`). Their
  `LTTS7Catalan.dll`/`LTTS7Galician.dll` are not ported: the dictionaries
  (`.dbl`) are read, and the built-in rules (`g2p::catalan`, `galician`)
  are fitted to those dictionaries and the voices' recordings (no plain
  `t`/`d`; Empar has `v` and `Dg` but no `Zg`/`HEh`). Carmela's library
  lists nasal vowels (`a)`) that her recordings never use (`u N`, not
  `u) N`): her unported phone stage makes them oral, so the dictionary's
  are read oral. Against the dictionaries' own words (irregular ones,
  mostly) the rules miss 25% (Central), 18% (Valencian) and 20% (Galician,
  nasality aside) of the phones. The `German Distribution.exe` of that
  archive is damaged (`unzip -t` reports a bad CRC; its cabinet's MSZIP
  blocks fail their checksums from the fifth on); German's data comes with
  Katrin's and Stefan's voice installers anyway. With their distributions'
  language data unpacked into the engine (Wise MSI, like the voices), the
  three voices do run in the original: `ttsLoadPersona` needs each language's
  `.lcf` also at `data/` root, a copy of the one in
  `data/languages/<Language>/` (`data/Catalan.lcf` BaseLanguage `CatalanCe`,
  `data/Valencian.lcf` `CatalanVa`, `data/Galician.lcf` `Galician`; the
  distributions ship them nested only), and the engine finds nothing without
  it (`ttsLoadPersona` 12). Scored on `oracle/bench-{ca,va,gl}.txt` (Jordi,
  Empar, Carmela): FON 82/86/83% of sounds (the built-in rules, above),
  CAT given the original's FON 98.9/100/99.5% of half-sounds, decoder and
  replay bit-exact.
- German numbers (§4): slot 9 matches the original on every traced call
  (541, `numeri::tests::replay_slot9`). What still differs on
  `oracle/bench-de-text.txt` is the engine's side: the reading's words are
  not split like typed ones (`neun-zehn`, `fünf-hundert` in the original),
  the text filters' `\{number=ordinalM}` before `3.`/`1. Mai`, the `Uhr`
  dropped after a time, `030 1234567` joined into one kind-10 number and
  `3-5` read `bis`.
- French numbers (§4): slot 9 matches the original on every traced call
  (282, `numeri::tests::french_replay_slot9` over `/tmp/qq-num-fr/*.les`).
  On `oracle/bench-fr-text.txt` one line differs, on the engine's side: a
  number followed by another number (`45 67`) gets the `t` of `quarante`
  before `cinq` in the original (`le 45 67.`: `Aa) tt s`), ours `Aa) n s`
  as for a lone `45`; the readings themselves are identical.
- Spanish all-capital acronyms (`ABS`, `CEE`, `PNV`, `UGT`): the original
  spells them letter by letter (`` `a-BH `e-`e s e ``); our tokenizer reads
  them as words. These are the 6–7 held-out words of 20 000 left (99.97%
  Spain, 99.96% America); the module's own LesPronunciabile and
  LesGetSymbolName are ported, the engine's acronym decision is not.
- English numbers (§4c): slot 9 matches the original on every traced call.
  Left on `oracle/bench-en-text.txt`: after `11:45 am` the module rewrites
  the next token to `a.m.` of type 4, which the original then spells
  (`` `HEI-Hg `E m ``); quoquendo's tokenizer side still reads `am`. The
  engine's date/time format settings (ctx+0x20 +0x34, +0x30) are taken
  as the defaults seen in the traces, not from `\{…}` tags.
- Dutch (§4i): `oracle/bench-nl-text.txt` is 100% identical in sounds
  and half-sounds; `--compare` still counts 6 of its 30 lines as
  differing in word grouping only: after FonAllofonoLingua inserts a
  glottal stop before a vowel-initial word the original's events glue
  the two words (`d R `i Hg Oa…`), as Greek's palatalised k does (§4f).
  The original engine under Wine crashes now and then (an ntdll page
  fault) in long `LOQ_LINES` runs, independent of the line: rerunning the
  rest of the list goes through.
- Danish (§4k): the `\SAMPA=` tail of FonAllofonoLingua and the
  phonetization path of FonTrasParola are ported but no trace exercises
  them.
- Danish and Russian numbers (§4n): every traced reading is the
  original's, and sounds, pieces and samples of `bench-da`,
  `bench-da-text`, `bench-ru` and `bench-ru-text` are identical. Left: the
  word grouping of the events (`bench-da-text` 99.1%, `bench-ru` 98.6%,
  `bench-ru-text` 97.3% of words): after a date's reading (`15 марта 2024
  года`, `12.05.2023`, `9 мая 1945`, `до 1 апреля`, `31 декабря`) and from
  the word before some Danish cardinals (`har 1024 sider`, `har 28
  elever`) the original marks no word start up to the sentence's end,
  ours one per word. Consecutive numbers (`+7 495 123 45 67`, as French
  `45 67`): the original gives each reading but the last the last-word
  subclass 0x7b (then boundary 8 and a 50 ms pause), ours 0x79; the
  number readings themselves are identical, the flag's source in
  0x10031410 (its +0x84 local or 0x1002ddc0) is not found yet. The
  reading-mode user data (`LesSetNumberReadingMode`) is not modelled.
