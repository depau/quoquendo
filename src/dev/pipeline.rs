//! The swappable pipeline, named after the original engine's own modules
//! (its module table in LoqTTS7.dll reads TOP, LES, FON, CAT, SIG, ACU):
//!
//!   FON  text -> sounds to say      builtin rules | original engine | any command
//!   CAT  sounds -> recorded pieces  builtin selection | original engine
//!   SIG  pieces -> audio            like the original | smoothed
//!
//! and `compare`, which runs the original engine beside ours and scores each
//! stage against it.

use super::oracle::{self, Reference};
use quoquendo::g2p::{analyse, Frontend};
use quoquendo::loquendo::VoiceDatabase;
use quoquendo::synth::{self, JoinStyle, Phrase, Settings, Speech, TargetPhrase};

#[derive(Clone, Debug, PartialEq)]
pub enum Fon {
    /// quoquendo's own letter-to-sound rules
    Rules,
    /// the original engine's transcription (runs it under Wine)
    Original,
    /// any program: gets the text on stdin, prints a transcription in the
    /// engine's notation (see `synth::parse_transcription`) on stdout
    Command(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cat {
    Builtin,
    /// the pieces the original engine chose (it uses its own FON then)
    Original,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stages {
    pub fon: Fon,
    pub cat: Cat,
    pub sig: JoinStyle,
}

impl Default for Stages {
    fn default() -> Self {
        Stages {
            fon: Fon::Rules,
            cat: Cat::Builtin,
            sig: JoinStyle::Original,
        }
    }
}

pub struct Run {
    pub targets: Vec<TargetPhrase>,
    pub plan: Vec<Phrase>,
}

pub fn run_command(cmd: &str, text: &str) -> Result<String, String> {
    use std::io::Write;
    let shell = if cfg!(windows) { ("cmd", "/C") } else { ("sh", "-c") };
    let mut child = std::process::Command::new(shell.0)
        .args([shell.1, cmd])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("{cmd}: {e}"))?;
    child.stdin.take().unwrap().write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("{cmd} failed ({})", out.status));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

impl Stages {
    /// Whether any stage needs the original engine's results.
    pub fn need_original(&self) -> bool {
        self.fon == Fon::Original || self.cat == Cat::Original
    }
}

/// FON and CAT for one text; SIG is `synth::render` on `plan`. `reference`
/// is the original engine's result for `text` when a stage needs it (fetched
/// here, under Wine, when not given).
pub fn run(text: &str, fe: &Frontend, voice: &str, db: &VoiceDatabase, stages: &Stages, reference: Option<Reference>) -> Result<Run, String> {
    let reference = match reference {
        Some(r) => Some(r),
        None if stages.need_original() => Some(oracle::run_many(voice, &[text.to_string()])?.remove(0)),
        None => None,
    };
    let targets = match &stages.fon {
        Fon::Rules => synth::from_tokens(&analyse(text, fe)),
        Fon::Original => reference.as_ref().unwrap().target_phrases(),
        Fon::Command(cmd) => synth::parse_transcription(&run_command(cmd, text)?),
    };
    let plan = match stages.cat {
        Cat::Builtin => synth::select_all(db, &targets),
        Cat::Original => reference.as_ref().unwrap().phrases(db)?,
    };
    Ok(Run { targets, plan })
}

/// Every half-phone a plan plays, as (sentence, half index).
fn halves(plan: &[Phrase]) -> Vec<(u32, u32)> {
    plan.iter()
        .flat_map(|p| &p.pieces)
        .flat_map(|piece| (piece.first_half..piece.first_half + piece.halves).map(move |h| (piece.unit, h)))
        .collect()
}

/// Words of a FON result, each as its phones joined by spaces.
fn words(t: &[TargetPhrase]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for p in t.iter().flat_map(|p| &p.phones) {
        if p.word_start || out.is_empty() {
            out.push(p.sym.clone());
        } else {
            let w = out.last_mut().unwrap();
            w.push(' ');
            w.push_str(&p.sym);
        }
    }
    out
}

/// Longest common subsequence: pairs of matching indexes.
fn lcs(a: &[String], b: &[String]) -> Vec<(usize, usize)> {
    let mut t = vec![vec![0u16; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            t[i][j] = if a[i] == b[j] { t[i + 1][j + 1] + 1 } else { t[i + 1][j].max(t[i][j + 1]) };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, Vec::new());
    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            out.push((i, j));
            i += 1;
            j += 1;
        } else if t[i + 1][j] >= t[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

/// Signal-to-noise ratio of `ours` against `orig` (dB) and the share of
/// samples within ±2 of the original.
pub fn audio_match(orig: &[i16], ours: &[i16]) -> (f64, f64) {
    let n = orig.len().min(ours.len());
    if n == 0 {
        return (0.0, 0.0);
    }
    let (mut s, mut r, mut close) = (0f64, 0f64, 0usize);
    for i in 0..n {
        let (o, q) = (orig[i] as f64, ours[i] as f64);
        s += o * o;
        r += (o - q) * (o - q);
        close += ((o - q).abs() <= 2.0) as usize;
    }
    // length difference counts as error too
    for &o in orig[n..].iter().chain(ours[n..].iter()) {
        r += (o as f64) * (o as f64);
    }
    (10.0 * ((s + 1.0) / (r + 1.0)).log10(), close as f64 / orig.len().max(ours.len()) as f64)
}

/// How well each piece's audio matches the original inside the piece: every
/// piece is looked up near where it should sit in the original's audio (joins
/// may shift it by a few ms) and scored on its middle half, so the decoder is
/// judged apart from join timing. Returns the median dB.
///
/// 44 kHz voices reach the original's 32 kHz output through its resampler,
/// whose grid falls on each piece wherever the piece sits in the utterance:
/// the piece is resampled at each of the grid's 11 offsets against the
/// recording and the best one counts.
fn pieces_match(plan: &[Phrase], db: &VoiceDatabase, orig: &[i16]) -> f64 {
    let Some(bank) = db.audio.as_ref() else { return 0.0 };
    let (rate_in, rate_out) = (bank.rate as usize, quoquendo::codec::OUTPUT_RATE as usize);
    let gcd = |mut a: usize, mut b: usize| {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    };
    let offsets = rate_in / gcd(rate_in, rate_out);
    let mut cursor = 0usize;
    let mut scores = Vec::new();
    for phrase in plan {
        for piece in &phrase.pieces {
            let len_in = piece.to.saturating_sub(piece.from) as usize;
            let len = len_in * rate_out / rate_in;
            let candidates: Vec<Vec<i16>> = if rate_in == rate_out {
                vec![bank.read(piece.from as usize, len)]
            } else {
                let margin = 256.min(piece.from as usize);
                let buf = bank.read(piece.from as usize - margin, margin + len_in + 256);
                (0..offsets)
                    .map(|s| {
                        let r = quoquendo::codec::resample(&buf[s..], bank.rate, rate_out as u32);
                        let start = (margin.saturating_sub(s) * rate_out).div_ceil(rate_in);
                        let mut v = r[start.min(r.len())..].to_vec();
                        v.resize(len, 0);
                        v
                    })
                    .collect()
            };
            let (m0, m1) = (len / 4, len * 3 / 4);
            let probe = &candidates[0][m0..m1.min(m0 + 1024)];
            let lo = cursor.saturating_sub(2000);
            let hi = (cursor + 2000).min(orig.len().saturating_sub(len));
            // the exact spot has (almost) no difference at all
            let best = (lo..hi.max(lo)).min_by_key(|&off| probe.iter().zip(&orig[off + m0..]).map(|(a, b)| (*a as i64 - *b as i64).pow(2)).sum::<i64>());
            if let Some(off) = best {
                let near = if candidates.len() == 1 { off..off + 1 } else { off.saturating_sub(2)..(off + 3).min(hi.max(lo + 1)) };
                let snr = candidates
                    .iter()
                    .flat_map(|ours| near.clone().map(move |o| audio_match(&orig[o + m0..o + m1], &ours[m0..m1]).0))
                    .fold(f64::MIN, f64::max);
                scores.push(snr);
                cursor = off + len;
            }
        }
        cursor += phrase.pause_ms as usize * rate_out / 1000;
    }
    scores.sort_by(|a, b| a.total_cmp(b));
    scores.get(scores.len() / 2).copied().unwrap_or(0.0)
}

fn to_i16(s: &Speech) -> Vec<i16> {
    s.samples.iter().map(|v| (v * 32768.0).round().clamp(-32768.0, 32767.0) as i16).collect()
}

#[derive(Default)]
pub struct Totals {
    pub words: usize,
    pub same_words: usize,
    pub phones: usize,
    pub same_phones: usize,
    pub halves: usize,
    pub same_halves: usize,
    pub replay_snr: Vec<f64>,
    pub inside_snr: Vec<f64>,
    pub replay_close: Vec<f64>,
    /// share of the replay's samples identical to the original's
    pub replay_exact: Vec<f64>,
    /// the original's decoder requests (`LOQ_TRACE_DECODE`), and how many
    /// ours answer bit for bit
    pub dec_requests: usize,
    pub dec_same: usize,
    pub orig_s: f64,
    pub ours_s: f64,
}

/// Run the original engine and ours on every text and score each stage.
/// Writes `<n>-original.wav`, `<n>-replay.wav` (original's pieces, our
/// decoder and joins) and `<n>-ours.wav` into `out_dir` when given.
pub fn compare(refs: &[Reference], voice: &quoquendo::Voice, out_dir: Option<&std::path::Path>) -> Result<(String, Totals), String> {
    let db = &voice.db;
    use std::fmt::Write;
    let texts: Vec<&String> = refs.iter().map(|r| &r.text).collect();
    let mut rep = String::new();
    let mut tot = Totals::default();
    let settings = Settings {
        volume: 1.0,
        join: JoinStyle::Original,
    };
    for (n, (text, r)) in texts.iter().zip(refs).enumerate() {
        let _ = writeln!(rep, "\n[{n}] {text}");
        // FON
        let ours_t = synth::from_tokens(&analyse(text, &voice.frontend));
        let orig_t = r.target_phrases();
        let (wo, wq) = (words(&orig_t), words(&ours_t));
        let same = lcs(&wo, &wq);
        tot.words += wo.len();
        tot.same_words += same.len();
        // sounds too: the original groups some words together in its notation
        let flat = |t: &[TargetPhrase]| t.iter().flat_map(|p| p.phones.iter().map(|x| x.sym.clone())).collect::<Vec<_>>();
        let (po, pq) = (flat(&orig_t), flat(&ours_t));
        let same_p = lcs(&po, &pq).len();
        tot.phones += po.len();
        tot.same_phones += same_p;
        let _ = writeln!(rep, "  FON  {} of {} sounds the same ({} of {} words)", same_p, po.len(), same.len(), wo.len());
        let _ = writeln!(rep, "       original: {}", synth::format_transcription(&orig_t));
        let _ = writeln!(rep, "       ours:     {}", synth::format_transcription(&ours_t));
        let matched_o: std::collections::HashSet<usize> = same.iter().map(|p| p.0).collect();
        let matched_q: std::collections::HashSet<usize> = same.iter().map(|p| p.1).collect();
        let diff_o: Vec<&str> = wo.iter().enumerate().filter(|(i, _)| !matched_o.contains(i)).map(|(_, w)| w.as_str()).collect();
        let diff_q: Vec<&str> = wq.iter().enumerate().filter(|(i, _)| !matched_q.contains(i)).map(|(_, w)| w.as_str()).collect();
        if !diff_o.is_empty() || !diff_q.is_empty() {
            let _ = writeln!(rep, "       differs:  original [{}]  ours [{}]", diff_o.join(" | "), diff_q.join(" | "));
        }
        // CAT, given the original's own FON so only the picking differs
        let orig_plan = r.phrases(db)?;
        // the original's transcription string glues some words together
        // (`h `HAI HEh v s `E v…`) though its picker still knows where each
        // ends: take word starts from ours wherever the sounds line up
        let mut orig_words = orig_t.clone();
        {
            let starts: Vec<bool> = ours_t.iter().flat_map(|p| p.phones.iter().map(|x| x.word_start)).collect();
            let mut flat: Vec<&mut synth::Target> = orig_words.iter_mut().flat_map(|p| p.phones.iter_mut()).collect();
            for (a, b) in lcs(&po, &pq) {
                flat[a].word_start = flat[a].word_start || starts[b];
            }
        }
        let ours_plan = synth::select_all(db, &orig_words);
        let (ho, hq) = (halves(&orig_plan), halves(&ours_plan));
        let set: std::collections::HashSet<_> = hq.iter().collect();
        let same_h = ho.iter().filter(|h| set.contains(h)).count();
        tot.halves += ho.len();
        tot.same_halves += same_h;
        let pieces = |p: &[Phrase]| p.iter().map(|x| x.pieces.len()).sum::<usize>();
        let _ = writeln!(
            rep,
            "  CAT  {} of {} half-sounds from the same recording and place; pieces: original {}, ours {}",
            same_h,
            ho.len(),
            pieces(&orig_plan),
            pieces(&ours_plan)
        );
        // SIG: the original's own pieces through our decoder and joins
        let replay = to_i16(&synth::render(&orig_plan, db, settings)?);
        // the decoder alone: the original's requests, in order, through ours
        // (its own recordings only; Gilded databases have other decoders)
        if let (Some(bank), Some(first)) = (db.audio.as_ref(), r.decodes.first()) {
            let mine: Vec<_> = r.decodes.iter().filter(|d| d.0 == first.0).collect();
            let same = mine.iter().filter(|(_, pos, theirs)| bank.read(*pos, theirs.len()) == *theirs).count();
            tot.dec_requests += mine.len();
            tot.dec_same += same;
            let _ = writeln!(rep, "  DEC  {same} of {} decoder requests bit-identical", mine.len());
        }
        let (snr, close) = audio_match(&r.audio, &replay);
        tot.replay_snr.push(snr);
        tot.replay_close.push(close);
        let exact = r.audio.iter().zip(&replay).filter(|(a, b)| a == b).count() as f64 / r.audio.len().max(replay.len()).max(1) as f64;
        tot.replay_exact.push(exact);
        let inside = pieces_match(&orig_plan, db, &r.audio);
        tot.inside_snr.push(inside);
        let _ = writeln!(
            rep,
            "  SIG  original pieces through our decoder: inside pieces {inside:.1} dB (median); whole file {snr:.1} dB, {:.1}% of samples within ±2, {:.1}% identical",
            close * 100.0,
            exact * 100.0
        );
        // all of ours
        let ours = synth::render(&synth::select_all(db, &ours_t), db, settings)?;
        let (os, qs) = (r.audio.len() as f64 / 32000.0, ours.duration_s() as f64);
        tot.orig_s += os;
        tot.ours_s += qs;
        let _ = writeln!(rep, "  ALL  original {os:.2} s, ours {qs:.2} s");
        if let Some(dir) = out_dir {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            let wav = |samples: Vec<i16>| {
                synth::to_wav_bytes(&Speech {
                    samples: samples.iter().map(|s| *s as f32 / 32768.0).collect(),
                    rate: 32000,
                })
            };
            let _ = std::fs::write(dir.join(format!("{n}-original.wav")), wav(r.audio.clone()));
            let _ = std::fs::write(dir.join(format!("{n}-replay.wav")), wav(replay));
            let _ = std::fs::write(dir.join(format!("{n}-ours.wav")), synth::to_wav_bytes(&ours));
        }
    }
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
    let _ = writeln!(
        rep,
        "\nTOTAL  FON {:.1}% sounds the same, {:.1}% words | CAT {:.1}% half-sounds the same | SIG inside pieces {:.1} dB, whole file {:.1} dB, {:.1}% samples within ±2, {:.2}% identical | length ours/original {:.2}",
        100.0 * tot.same_phones as f64 / tot.phones.max(1) as f64,
        100.0 * tot.same_words as f64 / tot.words.max(1) as f64,
        100.0 * tot.same_halves as f64 / tot.halves.max(1) as f64,
        mean(&tot.inside_snr),
        mean(&tot.replay_snr),
        100.0 * mean(&tot.replay_close),
        100.0 * mean(&tot.replay_exact),
        tot.ours_s / tot.orig_s.max(1e-9)
    );
    Ok((rep, tot))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lcs_finds_shared_words() {
        let a: Vec<String> = ["il", "tt r `E n o", "e"].iter().map(|s| s.to_string()).collect();
        let b: Vec<String> = ["`i l", "tt r `E n o", "e"].iter().map(|s| s.to_string()).collect();
        assert_eq!(lcs(&a, &b), vec![(1, 1), (2, 2)]);
    }

    #[test]
    fn identical_audio_scores_high() {
        let a = vec![100i16, -200, 300, 0, 5];
        let (snr, close) = audio_match(&a, &a);
        assert!(snr > 50.0 && close == 1.0);
    }
}
