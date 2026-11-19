//! The port checked call by call against the original's own calls: a
//! `LOQ_TRACE_LES` trace of LTTS7Greek.dll's function-table slots
//! (oracle/loqref.c, `LOQ_LES_DLL=LTTS7Greek.dll LOQ_LES_TABLE=10021d00
//! LOQ_LES_N=30`) replayed through the port, each stage fed the original's
//! own input: LesAnalizza, LesAccenta, the engine's LesConfiniProsodici
//! (seen between LesAccenta's output and LesGradiAccentuali's input),
//! LesGradiAccentuali, FonTrasParola and FonAllofonoLingua.
//!
//!   QQ_EL_TRACE=<prefix.les> [QQ_EL_LTTS7=<LTTS7 dir>] [QQ_EL_MAX=<examples>]
//!   cargo test -p quoquendo --release greek_trace -- --ignored --nocapture

use super::super::common::{Les, Phone, Word};
use super::Greek;

fn load() -> Greek {
    let root = std::env::var("QQ_EL_LTTS7").unwrap_or_else(|_| "/tmp/qqv/LTTS7".into());
    let read = |p: String| std::fs::read(&p).unwrap_or_else(|e| panic!("{p}: {e}"));
    let files = super::super::ModuleFiles {
        dll: read(format!("{root}/bin/LTTS7Greek.dll")),
        mother_tongue: "Greek".into(),
        dbl: Some(read(format!("{root}/data/languages/Greek/Greek.dbl"))),
        ..Default::default()
    };
    Greek::from_files(&files).expect("LTTS7Greek.dll")
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

#[test]
#[ignore]
fn greek_trace() {
    let Ok(path) = std::env::var("QQ_EL_TRACE") else { return };
    let max: usize = std::env::var("QQ_EL_MAX").ok().and_then(|m| m.parse().ok()).unwrap_or(10);
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
    let mut analysed: Vec<Word> = Vec::new();
    let mut accented: Vec<Word> = Vec::new();
    let mut sentence: Vec<Word> = Vec::new();
    let mut pos = 0usize; // the sentence record of the last FonTrasParola
    let mut chunk_start = 0usize;
    let mut pending: Option<Vec<Phone>> = None;
    let mut k = 0;
    while k < lines.len() {
        let l = lines[k];
        let name = l.split_whitespace().nth(1).unwrap_or("");
        if l.starts_with("> ") {
            // find the matching `<`
            let end = (k + 1..lines.len()).find(|&j| lines[j].starts_with("< ") && lines[j].split_whitespace().nth(1) == Some(name));
            let Some(end) = end else { break };
            match name {
                "LesAnalizza" | "LesAccenta" | "LesGradiAccentuali" => {
                    let input = block(k);
                    let output = block(end);
                    let mut ours = input.clone();
                    match name {
                        "LesAnalizza" => {
                            g.analizza(&mut ours);
                            analysed = ours.clone();
                            fail(&mut stats, "LesAnalizza", same(&ours, &output), format!("\n  in   {}\n  orig {}\n  ours {}", show(&input), show(&output), show(&ours)));
                        }
                        "LesAccenta" => {
                            g.accenta(&mut ours);
                            accented = output.clone();
                            fail(&mut stats, "LesAccenta", same(&ours, &output), format!("\n  in   {}\n  orig {}\n  ours {}", show(&input), show(&output), show(&ours)));
                        }
                        _ => {
                            // the engine's confini between them
                            let mut c = accented.clone();
                            let mode = Les::modalita_frase(&g, &c);
                            super::les::confini(&mut c, mode);
                            fail(&mut stats, "LesConfiniProsodici (engine)", same(&c, &input), format!("mode {mode}\n  in   {}\n  orig {}\n  ours {}", show(&accented), show(&input), show(&c)));
                            for (o, a) in ours.iter_mut().zip(&analysed) {
                                o.data = a.data.clone();
                            }
                            g.gradi(&mut ours);
                            sentence = ours.clone();
                            pos = 0;
                            fail(&mut stats, "LesGradiAccentuali", same(&ours, &output), format!("\n  in   {}\n  orig {}\n  ours {}", show(&input), show(&output), show(&ours)));
                        }
                    }
                }
                "FonTrasParola" => {
                    let word = l.find(" word=\"").map(|p| unescape(&l[p + 7..])).unwrap_or_default();
                    let idx: usize = l.rsplit("idx=").next().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                    let out = &lines[end];
                    let codes: Vec<u32> = out
                        .split(" out=")
                        .nth(1)
                        .map(|s| s.split_whitespace().next().unwrap_or("").split(',').filter_map(|c| u32::from_str_radix(c, 16).ok()).collect())
                        .unwrap_or_default();
                    if let Some(at) = (pos..sentence.len()).find(|&j| sentence[j].text == word) {
                        if idx == 0 {
                            chunk_start = at;
                        }
                        pos = at + 1;
                        let ours = g.tras_parola(&sentence[at..=at], 0);
                        fail(&mut stats, "FonTrasParola", ours == codes, format!("{} grade {}: orig {codes:x?} ours {ours:x?}", String::from_utf8_lossy(&word), sentence[at].f[0x12]));
                    }
                }
                "FonAllofonoLingua" => {
                    if let Some(p) = lines[k + 1..end].iter().find(|l| l.starts_with("  P ")) {
                        pending = Some(phones(&p[4..]));
                    }
                    if let (Some(q), Some(input)) = (lines.get(end + 1).filter(|l| l.starts_with("  Q  P ")), pending.as_ref()) {
                        let want = phones(&q[7..]);
                        let n = input.iter().map(|p| p.word as usize + 1).max().unwrap_or(0);
                        let chunk = &sentence[chunk_start.min(sentence.len())..(chunk_start + n).min(sentence.len())];
                        if chunk.len() == n {
                            let mut ph = input.clone();
                            let mut i = 0usize;
                            while i < ph.len() - 1 {
                                i = g.allofono(chunk, &mut ph, i).wrapping_add(1);
                            }
                            fail(&mut stats, "FonAllofonoLingua", ph == want, format!("{}\n  orig {want:x?}\n  ours {ph:x?}", show(chunk)));
                        }
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
