//! The original SIG stage's joins (LoqTTS7.dll): the pitch marks of a
//! half-phone (0x1001e3a0) and the join at the end of every run of
//! half-phones (0x10024660): pitch-synchronous between two voiced halves,
//! a cross-fade (0x100244a0) otherwise. Floating point as the DLL runs it
//! (x87 control word 0x027f, MECHANICS.md §2d): double arithmetic, `float`
//! where it stores one, and its runtime's SSE2 `cos`.

/// 2π as LoqTTS7.dll writes it (0x100a5be0): 6.28318530717958, 7 ulps
/// under `std::f64::consts::TAU`.
const TWO_PI: f64 = f64::from_bits(0x4019_21fb_5444_2d11);

/// LoqTTS7.dll's window of type 2 (0x100241e0, case at 0x100242ba): n points
/// of `0.5 − 0.5·cos(2π (i+1) / (n+1))`.
pub fn hann_window(n: usize) -> Vec<f64> {
    (0..n).map(|i| 0.5 - 0.5 * msvc_cos(TWO_PI * (i + 1) as f64 / (n + 1) as f64)).collect()
}

/// The engine's rounding of a sample (`modf` 0x10059ce0, +1 when the
/// fraction is at least a half, so positives round half up and negatives
/// truncate), held to 16 bits.
fn round_sample(v: f64) -> i16 {
    let t = v.trunc();
    let r = if v - t >= 0.5 { t + 1.0 } else { t };
    r.clamp(-32768.0, 32767.0) as i16
}

/// `_ftol2` (0x10069060, SSE2 `cvttsd2si`): truncation to 32 bits, 0x80000000
/// when out of range.
fn ftol(v: f64) -> i32 {
    if v.is_nan() || v >= 2147483648.0 || v <= -2147483649.0 {
        i32::MIN
    } else {
        v as i32
    }
}

/// `cos` as LoqTTS7.dll's runtime computes it: 0x10069510 takes the SSE2
/// routine 0x1006b13e (SSE2 present, every exception masked), which this is
/// operation by operation for |x| from 2^-252 to 2^... (the high word of x
/// in [0x3030, 0x40f5]); the window passes (0, 2π). Elsewhere the
/// platform's `cos` stands in.
pub fn msvc_cos(x: f64) -> f64 {
    let hi = ((x.to_bits() >> 48) as u16) & 0x7fff;
    if hi.wrapping_sub(0x3030) > 0x10c5 {
        return x.cos();
    }
    let c = |a: u64| f64::from_bits(a);
    // 32/π, the 1.5·2⁵² shifter and π/32 in three parts (0x100aa7b0..d8)
    let (inv, shift, p1, p2, p3) = (
        c(0x4024_5f30_6dc9_c883),
        c(0x4338_0000_0000_0000),
        c(0x3fb9_21fb_5440_0000),
        c(0x3d90_b461_1a60_0000),
        c(0x3b63_198a_2e03_7073),
    );
    let x1 = inv * x;
    let ki = x1.round_ties_even() as i32;
    let k = (x1 + shift) - shift;
    let t = &COS_TABLE[(ki.wrapping_add(0x1c7610) & 0x3f) as usize];
    let t = [c(t[0]), c(t[1]), c(t[2]), c(t[3])];
    let x2 = k * p2;
    let r1 = x - p1 * k;
    let r = r1 - x2;
    let x5a = c(0x3ec7_1de3_a556_c734) * r1 * r;
    let x5b = c(0x3efa_01a0_1a01_a01a) * r1 * r;
    let x7 = t[1] * r;
    let corr = k * p3 - ((r1 - r) - x2);
    let t03 = t[0] + t[3];
    let x7 = x7 - t03;
    let r2 = r * r;
    let x6a = c(0x3f81_1111_1111_1111) * r2;
    let x6b = c(0x3fa5_5555_5555_5555) * r2;
    let x3 = t[3] * r;
    let x2a = t03 * r * r2;
    let x2b = t[1] * r2;
    let r4 = r2 * r2;
    let x5a = (x5a + c(0xbf2a_01a0_1a01_a01a)) * r4;
    let x5b = (x5b + c(0xbf56_c16c_16c1_6c17)) * r4;
    let x4 = r * t[0];
    let x6a = (x6a + c(0xbfc5_5555_5555_5555) + x5a) * x2a;
    let x6b = (x6b + c(0xbfe0_0000_0000_0000) + x5b) * x2b;
    let x3b = x3 + t[1];
    let x1 = corr * x7 + t[2];
    let hi4 = x4 + x3b;
    let x5 = (t[1] - x3b) + x3;
    let x3c = (x3b - hi4) + x4;
    hi4 + (((((x1 + x5) + x3c) + x6a) + x6b))
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
                        let score = if marks.len() == 1 {
                            ((height * 3 - distance) + corr) * 2
                        } else {
                            (height - distance) * 3 + corr * 4
                        };
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
                if next == 0 || len_u <= ((cur * 3) >> 2) + next {
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
    if best_lag != 0 {
        best_lag
    } else {
        0x96
    }
}

/// One half-phone as the engine joins it (its unit, LoqTTS7.dll 0x1001faa0):
/// `pre + len + post` samples, the half itself after `pre`, and its pitch
/// marks counted from its first sample (+0x1c, count at +0x41c: 0 and `len`
/// included). `voiced` is the unit's flag at +0x640.
#[derive(Clone, Debug, PartialEq)]
pub struct JoinUnit {
    pub buf: Vec<i16>,
    pub pre: usize,
    pub len: usize,
    pub post: usize,
    pub marks: Vec<usize>,
    pub voiced: bool,
}

/// The join at the end of a run (algorithm 1, LoqTTS7.dll 0x10024660): `a`
/// is the run's last half-phone, `b` the next run's first. Two voiced halves
/// are joined pitch-synchronously ([`voiced_join`]); any other pair is
/// cross-faded (0x100244a0) from `a`'s last samples through its tail margin
/// into `b`'s lead-in margin and first samples, the sum written into both.
pub fn join(a: &mut JoinUnit, b: &mut JoinUnit) {
    if a.voiced && b.voiced {
        voiced_join(a, b);
        return;
    }
    // 0x100244a0: from a.len − b.pre + 1 + a.pre (unsigned) up to the end of
    // a's buffer, n points cross-faded with a window of 2n; at most 500, and
    // no more than b's buffer
    let total = (a.pre + a.len + a.post) as u32;
    let start = (a.len as u32).wrapping_sub(b.pre as u32).wrapping_add(1).wrapping_add(a.pre as u32).min(total);
    let n = (total - start) as usize;
    if 2 * n > 1000 || n > b.pre + b.len + b.post {
        return;
    }
    let w = hann_window(2 * n);
    for i in 0..n {
        let v = a.buf[start as usize + i] as f64 * w[n + i] + b.buf[i] as f64 * w[i];
        let v = ftol(v.clamp(-32768.0, 32767.0)) as i16;
        a.buf[start as usize + i] = v;
        b.buf[i] = v;
    }
}

/// 0x10024360: overlap `frame` onto `out` from `start` (at most `out`'s
/// length): the n samples there are cross-faded (window of 2n, falling half
/// on `out`, truncated, not clamped), the rest appended. More than 500 to
/// overlap leaves `out` as it was.
fn overlap(out: &mut Vec<i16>, frame: &[i16], start: u32) {
    let len = out.len();
    let start = (start as usize).min(len);
    let n = len - start;
    if 2 * n > 1000 {
        return;
    }
    let w = hann_window(2 * n);
    for k in 0..n {
        let v = out[start + k] as f64 * w[n + k] + *frame.get(k).unwrap_or(&0) as f64 * w[k];
        out[start + k] = ftol(v) as i16;
    }
    out.extend_from_slice(frame.get(n..).unwrap_or(&[]));
}

/// The voiced join, LoqTTS7.dll 0x10024660, as `join` calls it. Both halves
/// are cut into pitch periods, each with the unit's margins around it (at
/// most 50 periods, at least two each side, under 1000 samples each). Their
/// loudness (√Σs² over the frame's length) is eased with a gain ramp on the
/// louder side toward the other's mean (never under 0.5); when the mean
/// periods differ by 0.5–14 semitones the period lengths ease toward the
/// other side (weight x = periods before / all, kept in [0.2, 0.5]), undone
/// if they change by more than 10% on average; then the periods are
/// overlap-added back (changes of ±1 sample dropped) and the result split
/// into the two units again, with their new lengths and marks. Anything the
/// engine refuses leaves both units as they were.
fn voiced_join(a: &mut JoinUnit, b: &mut JoinUnit) {
    let (ma, mb) = (a.marks.len(), b.marks.len());
    if ma == 0 || mb == 0 || ma + mb - 2 > 0x32 {
        return;
    }
    let total = ma + mb - 2;
    let na = ma - 1;
    let nb = total - na;
    if na <= 1 || nb <= 1 {
        return;
    }
    struct Frame {
        samples: Vec<i16>,
        pre: usize,
        post: usize,
    }
    let mut frames = Vec::with_capacity(total);
    let mut period = Vec::with_capacity(total);
    let mut energy = Vec::with_capacity(total);
    for (u, n) in [(&*a, na), (&*b, nb)] {
        for p in 0..n {
            let m0 = u.marks[p];
            let m1 = u.marks[p + 1] as i64 - 1;
            if m1 <= m0 as i64 {
                return;
            }
            let count = u.pre + u.post + 1 + m1 as usize - m0;
            if count >= 1000 {
                return;
            }
            let samples: Vec<i16> = (0..count).map(|i| *u.buf.get(m0 + i).unwrap_or(&0)).collect();
            // √Σs² / count as a float (the sum is exact in a double)
            let sum: f64 = samples.iter().map(|&s| s as f64 * s as f64).sum();
            energy.push((sum.sqrt() / count as f64) as f32);
            period.push((m1 as usize + 1 - m0) as i16);
            frames.push(Frame {
                samples,
                pre: u.pre,
                post: u.post,
            });
        }
    }
    // loudness: each side's mean, summed in floats; a ramp on the louder side
    let mean = |v: &[f32]| v.iter().fold(0f32, |acc, &e| e + acc) / v.len() as f32;
    let (mean_a, mean_b) = (mean(&energy[..na]), mean(&energy[na..]));
    let mut gain = vec![1f32; total];
    let ramp = |k: usize, slope: f64, d: f32, base: f64| -> f32 {
        let g = (k as f64 * slope / d as f64 + base) as f32;
        if g < 0.5 {
            0.5
        } else {
            g
        }
    };
    if mean_a >= mean_b {
        if mean_a > 0.0 {
            let q = mean_b as f64 / mean_a as f64 - 1.0;
            for (k, g) in gain[..na].iter_mut().enumerate() {
                *g = ramp(k, q, (na - 1) as f32, 1.0);
            }
        }
    } else if mean_b > 0.0 {
        let r = mean_a as f64 / mean_b as f64;
        for (j, g) in gain[na..].iter_mut().enumerate() {
            *g = ramp(j, 1.0 - r, (nb - 1) as f32, r);
        }
    }
    // period lengths
    let mut new = period.clone();
    let mut x = (na as f32 as f64 / total as f64) as f32;
    if (x as f64) < f64::from_bits(0x3fc9_9999_a000_0000) {
        x = 0.2;
    }
    if x > 0.5 {
        x = 0.5;
    }
    let one_minus_x = (1.0 - x as f64) as f32;
    let mean_period = |v: &[i16]| v.iter().fold(0f32, |acc, &p| p as f32 + acc) / v.len() as f32;
    let (pa, pb) = (mean_period(&period[..na]), mean_period(&period[na..]));
    let mut err = 0f32;
    if pa > 0.0 && pb > 0.0 {
        let st = ((pb as f64 / pa as f64).ln() * 12.0 / 2f64.ln()).abs();
        if 0.5 < st && st < 14.0 {
            let (last_a, first_b) = (period[na - 1] as i32, period[na] as i32);
            let d = (na - 1) as f32 as f64;
            for k in 0..na {
                let o = period[k] as f64;
                let v = ftol((first_b - last_a) as f64 * x as f64 * (k as f64 / d) + o) as i16;
                new[k] = v;
                err += ((v as i32 - period[k] as i32).abs() as f64 / o) as f32;
            }
            let d = (nb - 1) as f32 as f64;
            for j in na..total {
                let o = period[j] as f64;
                let v = ftol((last_a - first_b) as f64 * one_minus_x as f64 * ((total - j - 1) as f64 / d) + o) as i16;
                new[j] = v;
                err += ((v as i32 - period[j] as i32).abs() as f64 / o) as f32;
            }
            err = (err as f64 / total as f64) as f32;
        }
    }
    if err as f64 > 0.1 {
        new = period.clone();
    }
    // overlap-add the periods back; `marks` in the joined buffer
    let mut out: Vec<i16> = Vec::new();
    let mut marks = vec![frames[0].pre as i64; total + 1];
    let limit = 2 * (a.len + b.len) as u32;
    for (p, f) in frames.iter().enumerate() {
        let count = f.samples.len() as i64;
        let mut d = (new[p] - period[p]) as i64;
        if count + d <= 0 || (d + 1) as u32 <= 2 {
            d = 0;
        }
        if p == 0 {
            out.extend_from_slice(&f.samples);
        } else {
            let samples: Vec<i16> = f.samples.iter().map(|&s| round_sample((s as f64 * gain[p] as f64).min(32767.0))).collect();
            out.truncate(out.len() - frames[p - 1].post);
            let start = (out.len() as u32).wrapping_sub(f.pre as u32).wrapping_add(d as u32);
            if start.wrapping_add(count as u32) >= limit {
                return;
            }
            overlap(&mut out, &samples, start);
        }
        marks[p + 1] = marks[p] - f.post as i64 - f.pre as i64 + count + d;
    }
    let (m0, mna, mt) = (marks[0], marks[na], marks[total]);
    let take = |from: usize, n: usize| -> Vec<i16> { (from..from + n).map(|i| *out.get(i).unwrap_or(&0)).collect() };
    a.len = (mna - m0) as usize;
    b.len = (mt - mna) as usize;
    a.buf = take(0, a.pre + a.len + a.post);
    b.buf = take(a.pre + a.len - b.pre, b.pre + b.len + b.post);
    a.marks = (0..ma).map(|i| (marks[i] - m0) as usize).collect();
    b.marks = (0..mb).map(|i| (marks[na + i] - mna) as usize).collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two traced joins of the original (oracle/loqref.c LOQ_TRACE_JOIN on
    /// "Attenzione, il treno…"): half lengths after the join.
    #[test]
    fn periods_ease_like_the_original() {
        let unit = |periods: &[usize]| {
            let mut marks = vec![0];
            for p in periods {
                marks.push(marks.last().unwrap() + p);
            }
            let len = *marks.last().unwrap();
            JoinUnit {
                buf: vec![1000; len + 128],
                pre: 64,
                len,
                post: 64,
                marks,
                voiced: true,
            }
        };
        let (mut a, mut b) = (unit(&[274, 267, 269, 278, 290]), unit(&[345, 335]));
        join(&mut a, &mut b);
        assert_eq!((a.len, b.len), (1444, 652));
        let (mut a, mut b) = (unit(&[225, 225, 225, 227]), unit(&[222, 221, 221, 198]));
        join(&mut a, &mut b);
        assert_eq!((a.len, b.len), (897, 864));
    }

    /// The runtime's cos is within an ulp of the correctly rounded one over
    /// the window's arguments.
    #[test]
    fn msvc_cos_is_a_cosine() {
        for n in 1..1001 {
            let x = TWO_PI * n as f64 / 1001.0;
            let (ours, libm) = (msvc_cos(x), x.cos());
            assert!((ours.to_bits() as i64 - libm.to_bits() as i64).abs() <= 1 || (ours - libm).abs() < 1e-16, "{x}: {ours} {libm}");
        }
    }
}

/// The runtime's table for [`msvc_cos`] (0x100a9f70): for j = 0..63 four
/// doubles, the parts of sin and cos of jπ/32 its polynomial combines.
#[rustfmt::skip]
static COS_TABLE: [[u64; 4]; 64] = [
    [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x3ff0000000000000],
    [0xbf73b92e176d6d31, 0x3fb917a6bc29b42c, 0xbc3e2718e0000000, 0x3ff0000000000000],
    [0xbf93ad06011469fb, 0x3fc8f8b83c69a60b, 0xbc626d19c0000000, 0x3ff0000000000000],
    [0xbfa60bea939d225a, 0x3fd294062ed59f06, 0xbc75d28da0000000, 0x3ff0000000000000],
    [0xbfb37ca1866b95cf, 0x3fd87de2a6aea963, 0xbc672cede0000000, 0x3ff0000000000000],
    [0xbfbe3a6873fa1279, 0x3fde2b5d3806f63b, 0x3c5e0d8920000000, 0x3ff0000000000000],
    [0xbfc592675bc57974, 0x3fe1c73b39ae68c8, 0x3c8b25dd20000000, 0x3ff0000000000000],
    [0xbfcd0dfe53aba2fd, 0x3fe44cf325091dd6, 0x3c68076a20000000, 0x3ff0000000000000],
    [0x3fca827999fcef32, 0x3fe6a09e667f3bcd, 0xbc8bdd3420000000, 0x3fe0000000000000],
    [0x3fc133cc94247758, 0x3fe8bc806b151741, 0xbc82c5e120000000, 0x3fe0000000000000],
    [0x3fac73b39ae68c87, 0x3fea9b66290ea1a3, 0x3c39f630e0000000, 0x3fe0000000000000],
    [0xbf9d4a2c7f909c4e, 0x3fec38b2f180bdb1, 0xbc76e0b180000000, 0x3fe0000000000000],
    [0xbfbe087565455a75, 0x3fed906bcf328d46, 0x3c7457e620000000, 0x3fe0000000000000],
    [0x3fa4a03176acf82d, 0x3fee9f4156c62dda, 0x3c8760b1e0000000, 0x3fd0000000000000],
    [0xbfac1d1f0e5967d5, 0x3fef6297cff75cb0, 0x3c75621720000000, 0x3fd0000000000000],
    [0xbf9ba1650f592f50, 0x3fefd88da3d12526, 0xbc887df640000000, 0x3fc0000000000000],
    [0x0000000000000000, 0x3ff0000000000000, 0x0000000000000000, 0x0000000000000000],
    [0x3f9ba1650f592f50, 0x3fefd88da3d12526, 0xbc887df640000000, 0xbfc0000000000000],
    [0x3fac1d1f0e5967d5, 0x3fef6297cff75cb0, 0x3c75621720000000, 0xbfd0000000000000],
    [0xbfa4a03176acf82d, 0x3fee9f4156c62dda, 0x3c8760b1e0000000, 0xbfd0000000000000],
    [0x3fbe087565455a75, 0x3fed906bcf328d46, 0x3c7457e620000000, 0xbfe0000000000000],
    [0x3f9d4a2c7f909c4e, 0x3fec38b2f180bdb1, 0xbc76e0b180000000, 0xbfe0000000000000],
    [0xbfac73b39ae68c87, 0x3fea9b66290ea1a3, 0x3c39f630e0000000, 0xbfe0000000000000],
    [0xbfc133cc94247758, 0x3fe8bc806b151741, 0xbc82c5e120000000, 0xbfe0000000000000],
    [0xbfca827999fcef32, 0x3fe6a09e667f3bcd, 0xbc8bdd3420000000, 0xbfe0000000000000],
    [0x3fcd0dfe53aba2fd, 0x3fe44cf325091dd6, 0x3c68076a20000000, 0xbff0000000000000],
    [0x3fc592675bc57974, 0x3fe1c73b39ae68c8, 0x3c8b25dd20000000, 0xbff0000000000000],
    [0x3fbe3a6873fa1279, 0x3fde2b5d3806f63b, 0x3c5e0d8920000000, 0xbff0000000000000],
    [0x3fb37ca1866b95cf, 0x3fd87de2a6aea963, 0xbc672cede0000000, 0xbff0000000000000],
    [0x3fa60bea939d225a, 0x3fd294062ed59f06, 0xbc75d28da0000000, 0xbff0000000000000],
    [0x3f93ad06011469fb, 0x3fc8f8b83c69a60b, 0xbc626d19c0000000, 0xbff0000000000000],
    [0x3f73b92e176d6d31, 0x3fb917a6bc29b42c, 0xbc3e2718e0000000, 0xbff0000000000000],
    [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0xbff0000000000000],
    [0x3f73b92e176d6d31, 0xbfb917a6bc29b42c, 0x3c3e2718e0000000, 0xbff0000000000000],
    [0x3f93ad06011469fb, 0xbfc8f8b83c69a60b, 0x3c626d19c0000000, 0xbff0000000000000],
    [0x3fa60bea939d225a, 0xbfd294062ed59f06, 0x3c75d28da0000000, 0xbff0000000000000],
    [0x3fb37ca1866b95cf, 0xbfd87de2a6aea963, 0x3c672cede0000000, 0xbff0000000000000],
    [0x3fbe3a6873fa1279, 0xbfde2b5d3806f63b, 0xbc5e0d8920000000, 0xbff0000000000000],
    [0x3fc592675bc57974, 0xbfe1c73b39ae68c8, 0xbc8b25dd20000000, 0xbff0000000000000],
    [0x3fcd0dfe53aba2fd, 0xbfe44cf325091dd6, 0xbc68076a20000000, 0xbff0000000000000],
    [0xbfca827999fcef32, 0xbfe6a09e667f3bcd, 0x3c8bdd3420000000, 0xbfe0000000000000],
    [0xbfc133cc94247758, 0xbfe8bc806b151741, 0x3c82c5e120000000, 0xbfe0000000000000],
    [0xbfac73b39ae68c87, 0xbfea9b66290ea1a3, 0xbc39f630e0000000, 0xbfe0000000000000],
    [0x3f9d4a2c7f909c4e, 0xbfec38b2f180bdb1, 0x3c76e0b180000000, 0xbfe0000000000000],
    [0x3fbe087565455a75, 0xbfed906bcf328d46, 0xbc7457e620000000, 0xbfe0000000000000],
    [0xbfa4a03176acf82d, 0xbfee9f4156c62dda, 0xbc8760b1e0000000, 0xbfd0000000000000],
    [0x3fac1d1f0e5967d5, 0xbfef6297cff75cb0, 0xbc75621720000000, 0xbfd0000000000000],
    [0x3f9ba1650f592f50, 0xbfefd88da3d12526, 0x3c887df640000000, 0xbfc0000000000000],
    [0x0000000000000000, 0xbff0000000000000, 0x0000000000000000, 0x0000000000000000],
    [0xbf9ba1650f592f50, 0xbfefd88da3d12526, 0x3c887df640000000, 0x3fc0000000000000],
    [0xbfac1d1f0e5967d5, 0xbfef6297cff75cb0, 0xbc75621720000000, 0x3fd0000000000000],
    [0x3fa4a03176acf82d, 0xbfee9f4156c62dda, 0xbc8760b1e0000000, 0x3fd0000000000000],
    [0xbfbe087565455a75, 0xbfed906bcf328d46, 0xbc7457e620000000, 0x3fe0000000000000],
    [0xbf9d4a2c7f909c4e, 0xbfec38b2f180bdb1, 0x3c76e0b180000000, 0x3fe0000000000000],
    [0x3fac73b39ae68c87, 0xbfea9b66290ea1a3, 0xbc39f630e0000000, 0x3fe0000000000000],
    [0x3fc133cc94247758, 0xbfe8bc806b151741, 0x3c82c5e120000000, 0x3fe0000000000000],
    [0x3fca827999fcef32, 0xbfe6a09e667f3bcd, 0x3c8bdd3420000000, 0x3fe0000000000000],
    [0xbfcd0dfe53aba2fd, 0xbfe44cf325091dd6, 0xbc68076a20000000, 0x3ff0000000000000],
    [0xbfc592675bc57974, 0xbfe1c73b39ae68c8, 0xbc8b25dd20000000, 0x3ff0000000000000],
    [0xbfbe3a6873fa1279, 0xbfde2b5d3806f63b, 0x3c5e0d8920000000, 0x3ff0000000000000],
    [0xbfb37ca1866b95cf, 0xbfd87de2a6aea963, 0x3c672cede0000000, 0x3ff0000000000000],
    [0xbfa60bea939d225a, 0xbfd294062ed59f06, 0x3c75d28da0000000, 0x3ff0000000000000],
    [0xbf93ad06011469fb, 0xbfc8f8b83c69a60b, 0x3c626d19c0000000, 0x3ff0000000000000],
    [0xbf73b92e176d6d31, 0xbfb917a6bc29b42c, 0x3c3e2718e0000000, 0x3ff0000000000000],
];
