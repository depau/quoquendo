//! The original SIG stage's joins (LoqTTS7.dll), for plain-copy units: the
//! pitch marks of a half-phone (0x1001e3a0) and the pitch-synchronous join of
//! two voiced half-phones (0x10024660), whose cross-fade (0x10024360,
//! 0x100244a0) is also used for every other join (`synth::render`).

/// LoqTTS7.dll's cross-fade window (0x100241e0, type 2 at 0x100242ba): n
/// points of `0.5 · (1 − cos(2π (i+1) / (n+1)))`.
pub fn hann_window(n: usize) -> Vec<f64> {
    (0..n).map(|i| 0.5 * (1.0 - (std::f64::consts::TAU * (i + 1) as f64 / (n + 1) as f64).cos())).collect()
}

/// The engine's rounding of a sample (`modf`, +1 when the fraction is at
/// least a half, so positives round half up and negatives truncate), as
/// 16 bits.
fn round_sample(v: f64) -> i16 {
    let t = v.trunc();
    let r = if v - t >= 0.5 { t + 1.0 } else { t };
    r.clamp(-32768.0, 32767.0) as i16
}

/// A half-phone's voicing as the engine marks it (unit info +0x150): `'0'`
/// voiced, `'u'` voiceless, or neither (pauses).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Voicing {
    Voiced,
    Voiceless,
    Other,
}

impl Voicing {
    /// From the phone's engine feature word (`phoneset::engine_features`).
    pub fn of_features(f: u32) -> Voicing {
        match f & 3 {
            1 => Voicing::Voiced,
            2 if f & 0x10 != 0 => Voicing::Voiceless,
            2 => Voicing::Voiced,
            _ => Voicing::Other,
        }
    }
}

/// Pitch marks of one half-phone, LoqTTS7.dll 0x1001e3a0: positions in the
/// half's own samples, from 0 to `len` included. `s(i)` reads its sample i
/// (i may be −1 or run past `len` into the decoded margins). Voiceless:
/// evenly every `rate/125` samples. Voiced: the period is `rate / f0` (or
/// found by autocorrelation when f0 is 0, default 150); each next mark is
/// the best rising zero crossing between 0.5 and 1.5 periods on, scored by
/// its correlation with the last period, its peak against the largest and
/// its distance from one period.
pub fn pitch_marks(s: &dyn Fn(isize) -> i32, len: usize, rate: u32, f0: u32, voicing: Voicing) -> Vec<usize> {
    let mut marks = vec![0usize];
    let len_u = len as u32;
    match voicing {
        Voicing::Other => {}
        Voicing::Voiceless => {
            let mut step = rate / 125;
            let mut n = len_u / step;
            if n != 0 {
                if len_u % step > step >> 1 {
                    n += 1;
                    step = len_u / n;
                }
                let mut pos = 0u32;
                for _ in 1..n {
                    pos += step + (len_u % step) / n;
                    marks.push(pos as usize);
                }
            }
        }
        Voicing::Voiced => {
            let period = if f0 != 0 { rate / f0 } else { autocorrelation_period(s, len_u) };
            let mut last = 0u32; // local_50
            let mut cur = period; // local_58
            loop {
                let lo = cur * 5 / 10 + last;
                if len_u < lo {
                    break;
                }
                let mut hi = cur * 15 / 10 + last;
                if len_u < hi {
                    hi = len_u;
                }
                let mut best_score = 0i32;
                let mut best = 0u32;
                let mut top = 0i32;
                for i in lo + 1..hi {
                    top = top.max(s(i as isize));
                }
                let mut i = lo + 1;
                let mut dist_from_last = i - last; // local_28
                let mut off_period = (i - last) as i64 - cur as i64; // local_24
                let mut span = cur; // local_44
                while i < hi {
                    if s(i as isize - 1) < 1 && s(i as isize) > 0 {
                        let mut peak = s(i as isize);
                        let mut zc = i;
                        let mut j = i;
                        while j < hi - 1 {
                            let v = s(j as isize);
                            if peak < v {
                                peak = v;
                            }
                            if v < 1 {
                                let gap = j - zc;
                                zc = j;
                                if gap > 5 {
                                    break;
                                }
                            }
                            j += 1;
                        }
                        if len_u < i + span {
                            span = len_u - i;
                            if span < cur / 3 {
                                // too little left: stop looking, keep the best so far
                                break;
                            }
                        }
                        let end = span + last;
                        let mut energy = 0i32;
                        for k in last..end {
                            let v = s(k as isize);
                            energy = energy.wrapping_add((v * v) >> 10);
                        }
                        let mut corr_max = 0i32;
                        for lag in -1i64..2 {
                            let mut c = 0i32;
                            for k in last..end {
                                let other = (dist_from_last as i64 + lag + k as i64) as isize;
                                c = c.wrapping_add((s(k as isize) * s(other)) >> 10);
                            }
                            corr_max = corr_max.max(c);
                        }
                        let corr = if energy != 0 { corr_max * 10 / energy } else { 0 };
                        let height = if top != 0 { peak * 10 / top } else { 0 };
                        let distance = if cur != 0 { (off_period.unsigned_abs() as u32 * 10 / cur) as i32 } else { 0 };
                        let score = if marks.len() == 1 { ((height * 3 - distance) + corr) * 2 } else { (height - distance) * 3 + corr * 4 };
                        if best_score < score {
                            best = i;
                            best_score = score;
                        }
                    }
                    dist_from_last += 1;
                    off_period += 1;
                    i += 1;
                }
                let next = if best == 0 { last + period } else { best };
                if next == 0 || len_u <= (cur * 3 >> 2) + next {
                    break;
                }
                marks.push(next as usize);
                last = next;
                if marks.len() > 2 {
                    cur = next / (marks.len() as u32 - 1);
                }
            }
        }
    }
    marks.push(len);
    marks
}

/// The period guess when a voiced half has no F0 (0x1001e3a0): the lag in
/// [50, 199) samples after the first peak with the largest correlation, or 150.
fn autocorrelation_period(s: &dyn Fn(isize) -> i32, len: u32) -> u32 {
    let quarter = (len >> 2).saturating_sub(1);
    let (mut peak_at, mut first, mut top) = (1u32, 1u32, 0i32);
    for i in 1..quarter.max(1) {
        let v = s(i as isize);
        if s(i as isize - 1) < v && s(i as isize + 1) < v {
            first = i;
        }
        if top < v {
            peak_at = first;
            top = v;
        }
    }
    let base = peak_at;
    let (mut best, mut best_lag) = (0i32, 0u32);
    let mut lag = 0x32u32;
    while lag < 199 {
        let at = base + lag;
        if len <= base + 200 + (lag - 0x32) {
            break;
        }
        if s(at as isize - 1) < s(at as isize) && s(at as isize + 1) < s(at as isize) {
            let mut c = 0i32;
            let mut k = base;
            while k < base + 0x96 {
                c = c.wrapping_add((s(k as isize) * s((k + lag) as isize)) >> 10);
                k += 1;
            }
            if best < c {
                best = c;
                best_lag = lag;
            }
        }
        lag += 1;
    }
    if best_lag != 0 { best_lag } else { 0x96 }
}

/// One half-phone at a join: its samples with `pre` before and `post`
/// after, and its pitch marks (relative to its first sample).
pub struct JoinHalf {
    pub buf: Vec<i16>,
    pub pre: usize,
    pub len: usize,
    pub post: usize,
    pub marks: Vec<usize>,
}

/// The voiced join, LoqTTS7.dll 0x10024660: the two halves cut into pitch
/// periods (each with the margins around it), period lengths eased toward
/// each other when the pitch differs by 0.5–14 semitones (weight
/// x = periods before / all, kept in [0.2, 0.5]; changes of ±1 sample
/// dropped; undone if they average over 10%), loudness eased with a gain
/// ramp (never under 0.5), then the periods overlap-added back with the
/// cross-fade. Returns the two halves' new samples (core only) and the
/// second half's new marks (the engine keeps them for its next join), or None
/// when the engine would leave them alone.
pub fn voiced_join(a: &JoinHalf, b: &JoinHalf) -> Option<(Vec<i16>, Vec<i16>, Vec<usize>)> {
    let na = a.marks.len().checked_sub(1)?;
    let nb = b.marks.len().checked_sub(1)?;
    let total = na + nb;
    if total >= 0x33 || na < 2 || nb < 2 {
        return None;
    }
    // frames: one per period, with the half's margins around it
    struct Frame {
        samples: Vec<f64>,
        period: i32,
        pre: usize,
        post: usize,
    }
    let mut frames = Vec::with_capacity(total);
    for (h, n) in [(a, na), (b, nb)] {
        for p in 0..n {
            let (m0, m1) = (h.marks[p], h.marks[p + 1]);
            // the buffer starts `pre` samples before the half: period p's
            // frame starts `pre` before its first mark
            let from = m0;
            let count = (m1 - m0) + h.pre + h.post;
            if count > 999 {
                return None;
            }
            let samples: Vec<f64> = (0..count).map(|i| *h.buf.get(from + i).unwrap_or(&0) as f64).collect();
            frames.push(Frame { samples, period: (m1 - m0) as i32, pre: h.pre, post: h.post });
        }
    }
    let energy: Vec<f32> = frames.iter().map(|f| (f.samples.iter().map(|v| v * v).sum::<f64>().sqrt() / f.samples.len() as f64) as f32).collect();
    let mean_a = energy[..na].iter().sum::<f32>() / na as f32;
    let mean_b = energy[na..].iter().sum::<f32>() / nb as f32;
    let mut gain = vec![1.0f32; total];
    if mean_b <= mean_a {
        if mean_a > 0.0 {
            let r = mean_b / mean_a - 1.0;
            for k in 0..na {
                gain[k] = (k as f32 * r / (na - 1) as f32 + 1.0).max(0.5);
            }
        }
    } else if mean_b > 0.0 {
        let r = mean_a / mean_b;
        for j in 0..nb {
            gain[na + j] = (j as f32 * (1.0 - r) / (nb - 1) as f32 + r).max(0.5);
        }
    }
    // period lengths
    let orig: Vec<i32> = frames.iter().map(|f| f.period).collect();
    let mut new = orig.clone();
    let x = (na as f32 / total as f32).clamp(0.2, 0.5);
    let mean_pa = orig[..na].iter().map(|&v| v as f32).sum::<f32>() / na as f32;
    let mean_pb = orig[na..].iter().map(|&v| v as f32).sum::<f32>() / nb as f32;
    if mean_pa > 0.0 && mean_pb > 0.0 {
        let st = ((mean_pa / mean_pb) as f64).ln() * 12.0 / 2f64.ln();
        if st.abs() > 0.5 && st.abs() < 14.0 {
            let (last_a, first_b) = (orig[na - 1], orig[na]);
            let mut err = 0.0f32;
            for k in 0..na {
                let v = (orig[k] as f64 + (first_b - last_a) as f64 * x as f64 * k as f64 / (na - 1) as f64) as i32;
                new[k] = v;
                err += (v - orig[k]).abs() as f32 / orig[k] as f32;
            }
            for j in 0..nb {
                let v = (orig[na + j] as f64 + (last_a - first_b) as f64 * (1.0 - x) as f64 * (nb - 1 - j) as f64 / (nb - 1) as f64) as i32;
                new[na + j] = v;
                err += (v - orig[na + j]).abs() as f32 / orig[na + j] as f32;
            }
            if err / total as f32 > 0.1 {
                new = orig.clone();
            }
        }
    }
    // overlap-add the periods back
    let mut out: Vec<i16> = Vec::new();
    let mut marks = vec![frames[0].pre as i64];
    for (p, f) in frames.iter().enumerate() {
        let mut d = (new[p] - orig[p]) as i64;
        if (f.samples.len() as i64 + d) < 1 || d.abs() <= 1 {
            d = 0;
        }
        let samples: Vec<i16> = f
            .samples
            .iter()
            .map(|&v| if p == 0 { round_sample(v) } else { round_sample((v as f32 * gain[p]).min(32767.0) as f64) })
            .collect();
        if p == 0 {
            out.extend_from_slice(&samples);
        } else {
            let prev_post = frames[p - 1].post;
            out.truncate(out.len().saturating_sub(prev_post));
            let start = (out.len() as i64 - f.pre as i64 + d).max(0) as usize;
            let start = start.min(out.len());
            let n = out.len() - start;
            if 2 * n > 1000 {
                return None;
            }
            let w = hann_window(2 * n);
            for i in 0..n.min(samples.len()) {
                let v = out[start + i] as f64 * w[n + i] + samples[i] as f64 * w[i];
                out[start + i] = v.clamp(-32768.0, 32767.0) as i16;
            }
            out.extend_from_slice(&samples[n.min(samples.len())..]);
        }
        let last = *marks.last().unwrap();
        marks.push(last + f.period as i64 + d);
    }
    let (m0, ma, mb) = (marks[0] as usize, marks[na] as usize, marks[total] as usize);
    if mb > out.len() || ma < m0 {
        return None;
    }
    let b_marks = marks[na..].iter().map(|&v| (v - ma as i64) as usize).collect();
    Some((out[m0..ma].to_vec(), out[ma..mb].to_vec(), b_marks))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two traced joins of the original (oracle/loqref.c LOQ_TRACE_JOIN on
    /// "Attenzione, il treno…"): period lengths before and after.
    #[test]
    fn periods_ease_like_the_original() {
        let half = |periods: &[usize]| {
            let mut marks = vec![0];
            for p in periods {
                marks.push(marks.last().unwrap() + p);
            }
            let len = *marks.last().unwrap();
            JoinHalf { buf: vec![1000; len + 128], pre: 64, len, post: 64, marks }
        };
        let (a, b, _) = voiced_join(&half(&[274, 267, 269, 278, 290]), &half(&[345, 335])).unwrap();
        assert_eq!((a.len(), b.len()), (1444, 652));
        let (a, b, _) = voiced_join(&half(&[225, 225, 225, 227]), &half(&[222, 221, 221, 198])).unwrap();
        assert_eq!((a.len(), b.len()), (897, 864));
    }
}
