//! The port checked call by call against the original's own calls: a
//! `LOQ_TRACE_LES` trace of LTTS7Russian.dll's function-table slots, with
//! the records' LexInfo (oracle/loqref.c, `LOQ_LES_RU=1
//! LOQ_LES_DLL=LTTS7Russian.dll LOQ_LES_TABLE=1009f270 LOQ_LES_N=56`)
//! replayed through the port, each stage fed the original's own input:
//! the analyser (on LesAnalizza's input), LesAnalizza, LesAccenta,
//! LesConfiniProsodici, LesGradiAccentuali, FonTrasParola and
//! FonAllofonoLingua.
//!
//!   QQ_RU_TRACE=<prefix.les> [QQ_RU_LTTS7=<LTTS7 dir>] [QQ_RU_MAX=<examples>]
//!   cargo test -p quoquendo --release russian_trace -- --ignored --nocapture

use super::super::common::{Les, Phone, Word};
use super::super::wordanalyzer::morph;
use super::{Lex, Russian};

fn load() -> Russian {
    let root = std::env::var("QQ_RU_LTTS7").unwrap_or_else(|_| "/tmp/qqv/LTTS7".into());
    let read = |p: String| std::fs::read(&p).unwrap_or_else(|e| panic!("{p}: {e}"));
    let files = super::super::ModuleFiles {
        dll: read(format!("{root}/bin/LTTS7Russian.dll")),
        mother_tongue: "Russian".into(),
        ..Default::default()
    };
    Russian::from_files(&files).expect("LTTS7Russian.dll")
}

/// A traced record's LexInfo: stressed vowel, spelling, analysis in use,
/// and the analyses by rank as (start, end, spelling VA, id, class).
#[derive(Clone, Debug, Default, PartialEq)]
struct TLex {
    pos: i32,
    form: Vec<u8>,
    cur: u32,
    ranks: Vec<Vec<(i32, i32, u32, i32, i32)>>,
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

/// The records of a call's block, each with its traced LexInfo.
fn block(lines: &[&str], k: usize) -> Vec<(Word, TLex)> {
    let mut out: Vec<(Word, TLex)> = Vec::new();
    for l in &lines[k + 1..] {
        let t = l.trim_start();
        if !l.starts_with("  ") {
            break;
        }
        if t.starts_with("W ") {
            if let Some(w) = record(t) {
                out.push((w, TLex { pos: -1, ..Default::default() }));
            }
        } else if let Some(rest) = t.strip_prefix("L ") {
            let Some((_, x)) = out.last_mut() else { continue };
            x.pos = rest.split_whitespace().next().and_then(|p| p.strip_prefix("pos=")).and_then(|p| p.parse().ok()).unwrap_or(-1);
            x.form = rest.find("form=\"").map(|p| unescape(&rest[p + 6..])).unwrap_or_default();
            x.cur = rest.rsplit("cur=").next().and_then(|c| c.trim().parse().ok()).unwrap_or(0);
        } else if let Some(rest) = t.strip_prefix("A ") {
            let Some((_, x)) = out.last_mut() else { continue };
            let segs = rest
                .split_whitespace()
                .skip(2)
                .filter_map(|g| {
                    let (se, m) = g.split_once(':')?;
                    let (s, e) = se.split_once('-')?;
                    let mut it = m.split('/');
                    Some((s.parse().ok()?, e.parse().ok()?, u32::from_str_radix(it.next()?, 16).ok()?, it.next()?.parse().ok()?, it.next()?.parse().ok()?))
                })
                .collect();
            x.ranks.push(segs);
        } else {
            break;
        }
    }
    out
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

impl Russian {
    /// The analyses of `w` as the trace shows them.
    fn traced_ranks(&self, w: &[u8]) -> Vec<Vec<(i32, i32, u32, i32, i32)>> {
        let an = self.analyse_word(w);
        an.order
            .iter()
            .map(|&s| {
                an.slots[s]
                    .iter()
                    .map(|g| {
                        let m = morph(&self.pe, g.morph);
                        (g.start, g.end, m.str_va(), m.id(), m.class() as i32)
                    })
                    .collect()
            })
            .collect()
    }

    /// Our records for traced ones: the analyses ours, the analysis in
    /// use, stressed vowel and spelling the trace's.
    fn with_lex(&self, b: &[(Word, TLex)]) -> Vec<Word> {
        b.iter()
            .map(|(w, x)| {
                let mut w = w.clone();
                if w.f[0xe] != b'2' {
                    Lex {
                        an: if x.ranks.is_empty() { Default::default() } else { self.analyse_word(&w.text) },
                        cur: x.cur,
                        pos: x.pos,
                        form: x.form.clone(),
                    }
                    .store(&mut w);
                }
                w
            })
            .collect()
    }
}

#[test]
#[ignore]
fn russian_trace() {
    let Ok(path) = std::env::var("QQ_RU_TRACE") else { return };
    let max: usize = std::env::var("QQ_RU_MAX").ok().and_then(|m| m.parse().ok()).unwrap_or(10);
    let g = load();
    let text = std::fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(&text);
    let lines: Vec<&str> = text.lines().collect();
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
    let cy = |b: &[u8]| -> String { b.iter().map(|&c| if c >= 0xc0 { char::from_u32(0x410 + (c - 0xc0) as u32).unwrap() } else { c as char }).collect() };
    let mut sentence: Vec<Word> = Vec::new();
    let mut pos = 0usize;
    let mut chunk_start = 0usize;
    let mut chunk_len = 0usize;
    let mut pending: Option<Vec<Phone>> = None;
    let mut k = 0;
    while k < lines.len() {
        let l = lines[k];
        let name = l.split_whitespace().nth(1).unwrap_or("");
        if l.starts_with("> ") {
            let end = (k + 1..lines.len()).find(|&j| lines[j].starts_with("< ") && lines[j].split_whitespace().nth(1) == Some(name));
            let Some(end) = end else { break };
            if name == "FonTrasParola" {
                // the chunk's record count, from the FonAllofonoLingua
                // call after its words
                if let Some(c) = lines[k..].iter().find(|l| l.starts_with("  C ")) {
                    chunk_len = c.split("ch4=").nth(1).and_then(|s| s.split_whitespace().next()).and_then(|s| s.parse().ok()).unwrap_or(0);
                }
            }
            match name {
                "LesAnalizza" | "LesAccenta" | "LesConfiniProsodici" | "LesGradiAccentuali" => {
                    let input = block(&lines, k);
                    let output = block(&lines, end);
                    let mode: i32 = l.find("ctx=").and_then(|p| l[p + 4..].split(',').nth(2)).and_then(|s| i32::from_str_radix(s, 16).ok()).unwrap_or(0);
                    let mut ours = g.with_lex(&input);
                    let want: Vec<Word> = output.iter().map(|x| x.0.clone()).collect();
                    match name {
                        "LesAnalizza" => {
                            for (w, x) in &input {
                                if w.f[0xe] == 0x17 {
                                    let r = g.traced_ranks(&w.text);
                                    fail(&mut stats, "analyser", r == x.ranks, format!("{}\n  orig {:x?}\n  ours {:x?}", cy(&w.text), x.ranks, r));
                                }
                            }
                            g.analizza(&mut ours);
                            let curs_ok = ours.iter().zip(&output).all(|(o, (_, x))| o.data.is_empty() || Lex::of(o).cur == x.cur);
                            fail(&mut stats, "LesAnalizza", same(&ours, &want) && curs_ok, format!("\n  in   {}\n  orig {}\n  ours {}\n  cur orig {:?} ours {:?}", cy(show(&input.iter().map(|x| x.0.clone()).collect::<Vec<_>>()).as_bytes()), show(&want), show(&ours), output.iter().map(|x| x.1.cur).collect::<Vec<_>>(), ours.iter().map(|o| Lex::of(o).cur).collect::<Vec<_>>()));
                        }
                        "LesAccenta" => {
                            g.accenta(&mut ours);
                            let lex_ok = ours.iter().zip(&output).all(|(o, (_, x))| o.f[0xe] == b'2' || (Lex::of(o).pos == x.pos && Lex::of(o).form == x.form));
                            fail(
                                &mut stats,
                                "LesAccenta",
                                same(&ours, &want) && lex_ok,
                                format!(
                                    "\n  orig {}\n  ours {}\n  orig {:?}\n  ours {:?}",
                                    show(&want),
                                    show(&ours),
                                    output.iter().map(|x| (x.1.pos, cy(&x.1.form))).collect::<Vec<_>>(),
                                    ours.iter().map(|o| (Lex::of(o).pos, cy(&Lex::of(o).form))).collect::<Vec<_>>()
                                ),
                            );
                        }
                        "LesConfiniProsodici" => {
                            g.confini(&mut ours, mode);
                            fail(&mut stats, "LesConfiniProsodici", same(&ours, &want), format!("mode {mode}\n  orig {}\n  ours {}", show(&want), show(&ours)));
                        }
                        _ => {
                            super::les::gradi(&mut ours);
                            sentence = g.with_lex(&output);
                            pos = 0;
                            fail(&mut stats, "LesGradiAccentuali", same(&ours, &want), format!("\n  orig {}\n  ours {}", show(&want), show(&ours)));
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
                        let ours = g.tras_parola(&sentence, chunk_start, chunk_len, at - chunk_start);
                        fail(&mut stats, "FonTrasParola", ours == codes, format!("{} grade {} lex {:?}: orig {codes:x?} ours {ours:x?}", cy(&word), sentence[at].f[0x12], (Lex::of(&sentence[at]).pos, cy(&Lex::of(&sentence[at]).form))));
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
                                i = g.allofono_in(&sentence, chunk_start, chunk, &mut ph, i).wrapping_add(1);
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
