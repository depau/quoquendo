//! The port checked call by call against the original's own calls: a
//! `LOQ_TRACE_LES` trace of LTTS7Polish.dll's function-table slots
//! (oracle/loqref.c, `LOQ_LES_DLL=LTTS7Polish.dll LOQ_LES_TABLE=10022d78
//! LOQ_LES_N=30`) replayed through the port, each stage fed the original's
//! own input: LesAnalizza, LesAccenta, LesConfiniProsodici,
//! LesGradiAccentuali, FonTrasParola, FonTrasConfini (the chunk's phone
//! list built from the original's FonTrasParola output, against the list
//! FonAllofonoLingua is first handed) and FonAllofonoLingua.
//!
//!   QQ_PL_TRACE=<prefix.les> [QQ_PL_LTTS7=<LTTS7 dir>] [QQ_PL_MAX=<examples>]
//!   cargo test -p quoquendo --release polish_trace -- --ignored --nocapture

use super::super::common::{field::BOUND, Les, Phone, Word};
use super::Polish;

pub(super) fn load() -> Polish {
    let root = std::env::var("QQ_PL_LTTS7").unwrap_or_else(|_| "/tmp/qqv/LTTS7".into());
    let read = |p: String| std::fs::read(&p).unwrap_or_else(|e| panic!("{p}: {e}"));
    let files = super::super::ModuleFiles {
        dll: read(format!("{root}/bin/LTTS7Polish.dll")),
        mother_tongue: "Polish".into(),
        dbl: Some(read(format!("{root}/data/languages/Polish/Polish.dbl"))),
        ..Default::default()
    };
    Polish::from_files(&files).expect("LTTS7Polish.dll")
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

fn show_phones(p: &[Phone]) -> String {
    p.iter().map(|p| format!("{}:{:x}:{:x}", p.word, p.code, p.kind)).collect::<Vec<_>>().join(" ")
}

fn same(a: &[Word], b: &[Word]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.text == y.text && x.f[0xc..0x14] == y.f[0xc..0x14])
}

/// An `out=` list of hex codes.
fn codes(l: &str) -> Vec<u32> {
    l.split(" out=")
        .nth(1)
        .map(|s| s.split_whitespace().next().unwrap_or("").split(',').filter_map(|c| u32::from_str_radix(c, 16).ok()).collect())
        .unwrap_or_default()
}

#[test]
#[ignore]
fn polish_trace() {
    let Ok(path) = std::env::var("QQ_PL_TRACE") else { return };
    let max: usize = std::env::var("QQ_PL_MAX").ok().and_then(|m| m.parse().ok()).unwrap_or(10);
    let g = load();
    let text = std::fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(&text);
    let lines: Vec<&str> = text.lines().collect();
    // the records of a call's `>` and `<` lines
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
    let mut sentence: Vec<Word> = Vec::new();
    let mut pos = 0usize; // the sentence record after the last FonTrasParola
    let mut chunk_start = 0usize;
    // the chunk's phone list as 0x1002d060 builds it, through our
    // FonTrasConfini, from the original's FonTrasParola output
    let mut built: Vec<Phone> = Vec::new();
    let mut incoming: Vec<u32> = Vec::new();
    let mut allo_in: Option<Vec<Phone>> = None;
    let mut allo_out: Option<Vec<Phone>> = None;
    let mut k = 0;
    let flush = |stats: &mut _, fail: &mut dyn FnMut(&mut std::collections::BTreeMap<&str, (usize, usize)>, &'static str, bool, String), sentence: &[Word], chunk_start: usize, input: Option<Vec<Phone>>, want: Option<Vec<Phone>>| {
        let (Some(input), Some(want)) = (input, want) else { return };
        let n = input.iter().map(|p| p.word as usize + 1).max().unwrap_or(0);
        let end = (chunk_start + n).min(sentence.len());
        let chunk = &sentence[chunk_start.min(end)..end];
        if chunk.len() != n {
            return;
        }
        let mut ph = input.clone();
        let last = ph.last().map_or(0, |p| p.word);
        ph.push(Phone { word: last, code: 0, kind: 0x70 });
        let mut i = 0usize;
        while i < ph.len() - 1 {
            i = Les::allofono_in(&g, sentence, chunk_start, chunk, &mut ph, i).wrapping_add(1);
        }
        ph.pop();
        fail(stats, "FonAllofonoLingua", ph == want, format!("{}\n  in   {}\n  orig {}\n  ours {}", show(chunk), show_phones(&input), show_phones(&want), show_phones(&ph)));
    };
    while k < lines.len() {
        let l = lines[k];
        let name = l.split_whitespace().nth(1).unwrap_or("");
        if l.starts_with("> ") {
            let end = (k + 1..lines.len()).find(|&j| lines[j].starts_with("< ") && lines[j].split_whitespace().nth(1) == Some(name));
            let Some(end) = end else { break };
            if name != "FonAllofonoLingua" && allo_in.is_some() {
                flush(&mut stats, &mut fail, &sentence, chunk_start, allo_in.take(), allo_out.take());
            }
            match name {
                "LesPronunciabile" => {
                    let s = l.find(" s=\"").map(|p| unescape(&l[p + 4..])).unwrap_or_default();
                    let c: u8 = l.rsplit("c=").next().and_then(|c| c.trim().parse().ok()).unwrap_or(0);
                    let want = lines[end].contains(" ok=1");
                    let ours = Les::pronunciabile_flag(&g, &s, c);
                    fail(&mut stats, "LesPronunciabile", ours == want, format!("{} flag {c}: orig {want} ours {ours}", String::from_utf8_lossy(&s)));
                }
                "LesGetSymbolName" => {
                    let a: Vec<&str> = l.split_whitespace().collect();
                    let arg = |k: usize| u32::from_str_radix(a.get(k).copied().unwrap_or("0"), 16).unwrap_or(0);
                    let (c, mode) = (arg(3) as u8, arg(4) as u8);
                    let want = lines[end].find(" s=\"").map(|p| unescape(&lines[end][p + 4..])).unwrap_or_default();
                    let ours = Les::symbol_name(&g, c, mode);
                    fail(&mut stats, "LesGetSymbolName", ours == want, format!("{c:02x} mode {mode}: orig {:?} ours {:?}", String::from_utf8_lossy(&want), String::from_utf8_lossy(&ours)));
                }
                "LesNormalizzaAccento" => {
                    let mut s = l.find(" s=\"").map(|p| unescape(&l[p + 4..])).unwrap_or_default();
                    let want = lines[end].find(" s=\"").map(|p| unescape(&lines[end][p + 4..])).unwrap_or_default();
                    Les::normalizza_accento(&g, &mut s);
                    fail(&mut stats, "LesNormalizzaAccento", s == want, format!("orig {:?} ours {:?}", String::from_utf8_lossy(&want), String::from_utf8_lossy(&s)));
                }
                "LesAnalizza" | "LesAccenta" | "LesConfiniProsodici" | "LesGradiAccentuali" => {
                    let input = block(k);
                    let output = block(end);
                    let mut ours = input.clone();
                    let why = |ours: &[Word]| format!("\n  in   {}\n  orig {}\n  ours {}", show(&input), show(&output), show(ours));
                    match name {
                        "LesAnalizza" => {
                            g.analizza(&mut ours);
                            fail(&mut stats, "LesAnalizza", same(&ours, &output), why(&ours));
                        }
                        "LesAccenta" => {
                            g.accenta(&mut ours);
                            fail(&mut stats, "LesAccenta", same(&ours, &output), why(&ours));
                        }
                        "LesConfiniProsodici" => {
                            let mode = Les::modalita_frase(&g, &ours);
                            g.confini(&mut ours, mode);
                            fail(&mut stats, "LesConfiniProsodici", same(&ours, &output), format!("mode {mode}{}", why(&ours)));
                        }
                        _ => {
                            Les::gradi(&g, &mut ours);
                            sentence = output.clone();
                            pos = 0;
                            chunk_start = 0;
                            built.clear();
                            fail(&mut stats, "LesGradiAccentuali", same(&ours, &output), why(&ours));
                        }
                    }
                }
                "FonTrasParola" => {
                    let word = l.find(" word=\"").map(|p| unescape(&l[p + 7..])).unwrap_or_default();
                    let idx: usize = l.rsplit("idx=").next().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                    let out = codes(lines[end]);
                    if let Some(at) = (pos..sentence.len()).find(|&j| sentence[j].text == word) {
                        if idx == 0 {
                            chunk_start = at;
                            built.clear();
                        }
                        pos = at + 1;
                        let ours = g.tras_parola(&sentence[at..=at], 0);
                        fail(&mut stats, "FonTrasParola", ours == out, format!("{} grade {}: orig {out:x?} ours {ours:x?}", String::from_utf8_lossy(&word), sentence[at].f[0x12]));
                    }
                    incoming = out;
                }
                "FonTrasConfini" => {
                    // args: ctx, list, ?, word index, phones so far, …
                    let a: Vec<&str> = l.split_whitespace().collect();
                    let i = usize::from_str_radix(a.get(5).copied().unwrap_or("0"), 16).unwrap_or(0);
                    let code = l.split("code=").nth(1).and_then(|s| u32::from_str_radix(s.split_whitespace().next()?, 16).ok()).unwrap_or(0);
                    if chunk_start + i >= sentence.len() {
                        eprintln!("FonTrasConfini: word {i} past the sentence ({} records from {chunk_start})", sentence.len());
                        k = end + 1;
                        continue;
                    }
                    // a word's codes, or a pause (a punctuation record's
                    // or the short pause after a word)
                    let word = incoming.first() == Some(&code);
                    let mut next = if word { std::mem::take(&mut incoming) } else { vec![code] };
                    let bound = sentence.get(chunk_start + i).map_or(0, |r| r.f[BOUND]);
                    Les::tras_confini_in(&g, &sentence, chunk_start, i, 0, &mut built, &mut next);
                    let n = next.len();
                    if !word {
                        if let Some(&c) = next.first() {
                            built.push(Phone { word: i as u32, code: c, kind: 0x70 });
                        }
                    } else {
                        built.extend(next.iter().enumerate().map(|(k, &code)| Phone {
                            word: i as u32,
                            code,
                            kind: if k + 1 == n && bound != 0 { 0x77 } else { 0x69 },
                        }));
                    }
                }
                "FonAllofonoLingua" => {
                    if let Some(p) = lines[k + 1..end].iter().find(|l| l.starts_with("  P ")) {
                        let input = phones(&p[4..]);
                        fail(&mut stats, "FonTrasConfini", input == built, format!("{}\n  orig {}\n  ours {}", show(&sentence[chunk_start..]), show_phones(&input), show_phones(&built)));
                        allo_in = Some(input);
                    }
                    if let Some(q) = lines.get(end + 1).filter(|l| l.starts_with("  Q  P ")) {
                        allo_out = Some(phones(&q[7..]));
                    }
                }
                _ => {}
            }
            k = end + 1;
            continue;
        }
        k += 1;
    }
    flush(&mut stats, &mut fail, &sentence, chunk_start, allo_in.take(), allo_out.take());
    for (n, (calls, bad)) in &stats {
        eprintln!("{n}: {calls} calls, {bad} differ");
    }
    assert!(stats.values().all(|&(_, bad)| bad == 0));
}
