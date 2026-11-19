//! The port checked call by call against the original's own calls: a
//! `LOQ_TRACE_LES` trace of LTTS7Dutch.dll's function-table slots
//! (oracle/loqref.c, `LOQ_LES_DLL=LTTS7Dutch.dll LOQ_LES_TABLE=100122a0
//! LOQ_LES_N=22`) replayed through the port, each stage fed the original's
//! own input: LesAnalizza, the engine's LesConfiniProsodici (seen between
//! LesAnalizza's output and LesGradiAccentuali's input),
//! LesGradiAccentuali, FonTrasParola, the chunk's phone list as
//! FonTrasParola and FonTrasConfini build it (FonAllofonoLingua's first
//! input) and FonAllofonoLingua.
//!
//!   QQ_NL_TRACE=<prefix.les> [QQ_NL_LTTS7=<LTTS7 dir>] [QQ_NL_MAX=<examples>]
//!   cargo test -p quoquendo --release dutch_trace -- --ignored --nocapture

use super::super::common::{field::*, Les, Phone, Word};
use super::Dutch;

pub(super) fn load() -> Dutch {
    let root = std::env::var("QQ_NL_LTTS7").unwrap_or_else(|_| "/tmp/qqv/LTTS7".into());
    let read = |p: String| std::fs::read(&p).unwrap_or_else(|e| panic!("{p}: {e}"));
    let files = super::super::ModuleFiles {
        dll: read(format!("{root}/bin/LTTS7Dutch.dll")),
        mother_tongue: "Dutch".into(),
        dbl: Some(read(format!("{root}/data/languages/Dutch/Dutch.dbl"))),
        gpr: Some(read(format!("{root}/data/languages/Dutch/Dutch.gpr"))),
        ..Default::default()
    };
    Dutch::from_files(&files).expect("LTTS7Dutch.dll")
}

/// A `W` line: the record's text and its bytes 0x0c..0x18.
fn record(line: &str) -> Option<Word> {
    let mut it = line.split_whitespace();
    if it.next()? != "W" {
        return None;
    }
    it.next()?;
    let hex: String = (0..6).filter_map(|_| it.next()).collect();
    if hex.len() != 48 {
        return None;
    }
    let bytes: Vec<u8> = (0..24).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap_or(0)).collect();
    let text = line.find(" s4=\"").map(|p| unescape(&line[p + 5..])).unwrap_or_default();
    let mut w = super::super::common::record(text, 0, 0, 0);
    w.f[0xc..0x18].copy_from_slice(&bytes[0xc..0x18]);
    Some(w)
}

/// A traced string up to its closing quote (`\xNN`, `\"`, `\\`).
fn unescape(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() && b[i] != b'"' {
        if b[i] == b'\\' && i + 1 < b.len() {
            if b[i + 1] == b'x' && i + 3 < b.len() {
                out.push(u8::from_str_radix(&s[i + 2..i + 4], 16).unwrap_or(0));
                i += 4;
            } else {
                out.push(b[i + 1]);
                i += 2;
            }
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

fn phones(s: &str) -> Vec<Phone> {
    s.split_whitespace()
        .filter_map(|p| {
            let mut it = p.split(':');
            Some(Phone {
                word: it.next()?.parse().ok()?,
                code: u32::from_str_radix(it.next()?, 16).ok()?,
                kind: u8::from_str_radix(it.next()?, 16).ok()?,
            })
        })
        .collect()
}

fn show(w: &[Word]) -> String {
    w.iter()
        .map(|r| format!("{}:{:02x?}", String::from_utf8_lossy(&r.text), &r.f[0xc..0x14]))
        .collect::<Vec<_>>()
        .join(" ")
}

fn same(a: &[Word], b: &[Word]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.text == y.text && x.f[0xc..0x14] == y.f[0xc..0x14])
}

/// The chunk's phone list as the phrase loop builds it (0x1002c300):
/// FonTrasParola, FonTrasConfini, the word's phones, the pause after a
/// pausing boundary; the closing sentinel.
fn chunk_list(d: &Dutch, sentence: &[Word], a: usize, n: usize) -> Vec<Phone> {
    let w = &sentence[a..a + n];
    let mut ph: Vec<Phone> = Vec::new();
    for i in 0..n {
        let r = &w[i];
        let punct = r.f[CLASS] == b'2';
        let mut new = if punct { vec![if r.f[SUB] == b'|' { 0x17 } else { 0x13 }] } else { d.tras_parola(w, i) };
        d.tras_confini_in(sentence, a, i, 0, &mut ph, &mut new);
        if punct {
            if let Some(&c) = new.first() {
                ph.push(Phone { word: i as u32, code: c, kind: 0x70 });
            }
            continue;
        }
        let m = new.len();
        ph.extend(new.iter().enumerate().map(|(k, &code)| Phone {
            word: i as u32,
            code,
            kind: if k + 1 == m && r.f[BOUND] != 0 { 0x77 } else { 0x69 },
        }));
        if matches!(r.f[BOUND], 0x0a | 0x08 | 0x0e | 0x0c | 0x0d) {
            ph.push(Phone { word: i as u32, code: 0x13, kind: 0x70 });
        }
    }
    ph
}

#[test]
#[ignore]
fn dutch_trace() {
    let Ok(path) = std::env::var("QQ_NL_TRACE") else { return };
    let max: usize = std::env::var("QQ_NL_MAX").ok().and_then(|m| m.parse().ok()).unwrap_or(10);
    let d = load();
    let text = std::fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(&text);
    let lines: Vec<&str> = text.lines().collect();
    let block = |k: usize| -> Vec<Word> { lines[k + 1..].iter().take_while(|l| l.starts_with("  W")).filter_map(|l| record(l.trim())).collect() };
    let mut stats = std::collections::BTreeMap::<&str, (usize, usize)>::new();
    let mut shown = 0usize;
    let mut fail = |stats: &mut std::collections::BTreeMap<&str, (usize, usize)>, name: &'static str, ok: bool, why: String| {
        let e = stats.entry(name).or_default();
        e.0 += 1;
        if !ok {
            e.1 += 1;
            if shown < max {
                shown += 1;
                eprintln!("{name}: {why}");
            }
        }
    };
    let mut analysed: Vec<Word> = Vec::new();
    let mut sentence: Vec<Word> = Vec::new();
    let mut pos = 0usize;
    let mut chunk_start = 0usize;
    let mut pending: Option<Vec<Phone>> = None;
    let mut k = 0;
    while k < lines.len() {
        let l = lines[k];
        let name = l.split_whitespace().nth(1).unwrap_or("");
        if l.starts_with("> ") {
            let end = (k + 1..lines.len()).find(|&j| lines[j].starts_with("< ") && lines[j].split_whitespace().nth(1) == Some(name));
            let Some(end) = end else { break };
            match name {
                "LesAnalizza" => {
                    let input = block(k);
                    let output = block(end);
                    let mut ours = input.clone();
                    d.analizza(&mut ours);
                    analysed = output.clone();
                    fail(&mut stats, "LesAnalizza", same(&ours, &output), format!("\n  in   {}\n  orig {}\n  ours {}", show(&input), show(&output), show(&ours)));
                }
                "LesGradiAccentuali" => {
                    let input = block(k);
                    let output = block(end);
                    let mut c = analysed.clone();
                    let mode = Les::modalita_frase(&d, &c);
                    super::super::common::confini_default(&mut c, mode);
                    fail(&mut stats, "LesConfiniProsodici (engine)", same(&c, &input), format!("mode {mode}\n  in   {}\n  orig {}\n  ours {}", show(&analysed), show(&input), show(&c)));
                    let mut ours = input.clone();
                    d.gradi(&mut ours);
                    sentence = output.clone();
                    pos = 0;
                    fail(&mut stats, "LesGradiAccentuali", same(&ours, &output), format!("\n  in   {}\n  orig {}\n  ours {}", show(&input), show(&output), show(&ours)));
                }
                "FonTrasParola" => {
                    let word = l.find(" word=\"").map(|p| unescape(&l[p + 7..])).unwrap_or_default();
                    let idx: usize = l.rsplit("idx=").next().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                    let codes: Vec<u32> = lines[end]
                        .split(" out=")
                        .nth(1)
                        .map(|s| s.split_whitespace().next().unwrap_or("").split(',').filter_map(|c| u32::from_str_radix(c, 16).ok()).collect())
                        .unwrap_or_default();
                    if let Some(at) = (pos..sentence.len()).find(|&j| sentence[j].text == word) {
                        if idx == 0 || at < idx {
                            chunk_start = at - idx.min(at);
                        }
                        chunk_start = at - idx.min(at);
                        pos = at + 1;
                        let ours = d.tras_parola(&sentence[chunk_start..], idx);
                        fail(&mut stats, "FonTrasParola", ours == codes, format!("{} grade {} bound {:x}: orig {codes:x?} ours {ours:x?}", String::from_utf8_lossy(&word), sentence[at].f[GRADE], sentence[at].f[BOUND]));
                    }
                }
                "FonAllofonoLingua" => {
                    if let Some(p) = lines[k + 1..end].iter().find(|l| l.starts_with("  P ")) {
                        let input = phones(&p[4..]);
                        let n = input.iter().map(|p| p.word as usize + 1).max().unwrap_or(0);
                        if chunk_start + n <= sentence.len() {
                            let mut ours = chunk_list(&d, &sentence, chunk_start, n);
                            let last = ours.last().map_or(0, |p| p.word);
                            ours.push(Phone { word: last, code: 0, kind: 0x70 });
                            let want: Vec<Phone> = input.clone();
                            // the trace's list has no sentinel
                            let ours_cmp: Vec<Phone> = ours[..ours.len() - 1].to_vec();
                            fail(&mut stats, "FonTrasParola+FonTrasConfini (chunk)", ours_cmp == want, format!("{}\n  orig {want:x?}\n  ours {ours_cmp:x?}", show(&sentence[chunk_start..chunk_start + n])));
                        }
                        pending = Some(input);
                    }
                    if let (Some(q), Some(input)) = (lines.get(end + 1).filter(|l| l.starts_with("  Q  P ")), pending.as_ref()) {
                        let want = phones(&q[7..]);
                        let mut ph = input.clone();
                        let last = ph.last().map_or(0, |p| p.word);
                        ph.push(Phone { word: last, code: 0, kind: 0x70 });
                        let mut i = 0usize;
                        while i < ph.len() - 1 {
                            i = d.allofono_in(&sentence, chunk_start, &[], &mut ph, i).wrapping_add(1);
                        }
                        ph.pop();
                        fail(&mut stats, "FonAllofonoLingua", ph == want, format!("\n  in   {input:x?}\n  orig {want:x?}\n  ours {ph:x?}"));
                        pending = None;
                    }
                }
                _ => {}
            }
            k = end + 1;
            continue;
        }
        k += 1;
    }
    for (n, (calls, bad)) in &stats {
        eprintln!("{n}: {calls} calls, {bad} differ");
    }
    assert!(stats.values().all(|&(_, bad)| bad == 0));
}
