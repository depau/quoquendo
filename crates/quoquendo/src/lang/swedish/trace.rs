//! The LES port checked call by call against the original's own calls: a
//! trace of LTTS7Swedish.dll's function-table slots (oracle/loqref.c with
//! LOQ_TRACE_LES, LOQ_LES_DLL=LTTS7Swedish.dll, LOQ_LES_TABLE=10046ec0,
//! LOQ_LES_N=26), each stage fed the original's own records (the LexInfo,
//! which the trace does not show, from the port's analyser as
//! LesElaboraParola fills it).
//!
//!   QQ_SV_TRACE=<a.les,b.les> [QQ_SV_LTTS7=<LTTS7 dir>] [QQ_SV_MAX=<shown>]
//!   cargo test -p quoquendo --release swedish_trace -- --ignored --nocapture

use super::{LexInfo, Swedish};
use crate::lang::common::{field::*, Word};

#[derive(Clone)]
struct Rec {
    text: Vec<u8>,
    tag: Option<Vec<u8>>,
    f: [u8; 12],
}

struct Call {
    name: String,
    mode: i32,
    before: Vec<Rec>,
    after: Vec<Rec>,
    /// FonTrasParola: (word, idx, codes)
    tp: Option<(Vec<u8>, usize, Vec<u32>)>,
}

fn unesc(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
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

fn field(line: &str, key: &str) -> Option<Vec<u8>> {
    let k = format!(" {}=\"", key);
    let a = line.find(&k)? + k.len();
    let rest = &line.as_bytes()[a..];
    let mut e = 0;
    while e < rest.len() {
        if rest[e] == b'\\' {
            e += 2;
            continue;
        }
        if rest[e] == b'"' {
            break;
        }
        e += 1;
    }
    Some(unesc(&line[a..a + e]))
}

fn parse(text: &[u8]) -> Vec<Call> {
    let text = crate::loquendo::latin1(text);
    let mut calls: Vec<Call> = Vec::new();
    let mut side = 0;
    for line in text.lines() {
        if let Some(r) = line.strip_prefix("> Les") {
            let name = r.split(' ').next().unwrap_or("").to_string();
            if ["Analizza", "Accenta", "ConfiniProsodici", "GradiAccentuali"].contains(&name.as_str()) {
                let mode = line.find(" ctx=").map_or(0, |p| {
                    let v: Vec<&str> = line[p + 5..].split(',').collect();
                    i32::from_str_radix(v.get(2).unwrap_or(&"0"), 16).unwrap_or(0)
                });
                calls.push(Call { name, mode, before: Vec::new(), after: Vec::new(), tp: None });
                side = 1;
                continue;
            }
            side = 0;
            continue;
        }
        if line.starts_with("< Les") {
            side = if side == 1 { 2 } else { 0 };
            continue;
        }
        if line.starts_with("> FonTrasParola") {
            let idx = line.split(" idx=").nth(1).and_then(|v| v.trim().parse().ok()).unwrap_or(0);
            calls.push(Call { name: "FonTrasParola".into(), mode: 0, before: Vec::new(), after: Vec::new(), tp: Some((field(line, "word").unwrap_or_default(), idx, Vec::new())) });
            side = 0;
            continue;
        }
        if line.starts_with("< FonTrasParola") {
            if let Some(c) = calls.last_mut() {
                if let Some(tp) = c.tp.as_mut() {
                    let o = line.split(" out=").nth(1).unwrap_or("").split(' ').next().unwrap_or("");
                    tp.2 = o.split(',').filter(|x| !x.is_empty()).map(|x| u32::from_str_radix(x, 16).unwrap_or(0)).collect();
                }
            }
            side = 0;
            continue;
        }
        if line.starts_with("> ") || line.starts_with("< ") {
            side = 0;
            continue;
        }
        if let Some(r) = line.strip_prefix("  W ") {
            if side == 0 {
                continue;
            }
            let p: Vec<&str> = r.split(' ').collect();
            if p.len() < 7 {
                continue;
            }
            let mut f = [0u8; 12];
            for (g, s) in p[4..7].iter().enumerate() {
                for k in 0..4 {
                    f[g * 4 + k] = u8::from_str_radix(&s[2 * k..2 * k + 2], 16).unwrap_or(0);
                }
            }
            let rec = Rec {
                text: field(line, "s4").unwrap_or_default(),
                tag: field(line, "s0"),
                f,
            };
            let c = calls.last_mut().unwrap();
            if side == 1 {
                c.before.push(rec);
            } else {
                c.after.push(rec);
            }
        }
    }
    calls
}

fn word(r: &Rec) -> Word {
    let mut w = crate::lang::common::record(r.text.clone(), 0, 0, 0);
    w.f[0xc..0x18].copy_from_slice(&r.f);
    w.tag = r.tag.clone();
    w
}

fn show(w: &[Word]) -> String {
    w.iter()
        .map(|r| format!("{}:{:02x}/{:02x}/{:02x}/{:02x}/{:02x}", String::from_utf8_lossy(&r.text), r.f[CLASS], r.f[SUB], r.f[INFO], r.f[BOUND], r.f[GRADE]))
        .collect::<Vec<_>>()
        .join(" ")
}

fn module() -> Option<Swedish> {
    let root = std::env::var("QQ_SV_LTTS7").unwrap_or_else(|_| "/tmp/qqv/LTTS7".into());
    let dll = std::fs::read(format!("{root}/bin/LTTS7Swedish.dll")).ok()?;
    let dbl = std::fs::read(format!("{root}/data/languages/Swedish/Swedish.dbl")).ok();
    Swedish::new(dll, dbl.as_deref())
}

#[test]
#[ignore]
fn swedish_trace() {
    let Some(sv) = module() else { return };
    let files = std::env::var("QQ_SV_TRACE").unwrap_or_else(|_| "/tmp/qq-sv/tr-annika.les,/tmp/qq-sv/text.les,/tmp/qq-sv/w2k.les".into());
    let max: usize = std::env::var("QQ_SV_MAX").ok().and_then(|v| v.parse().ok()).unwrap_or(10);
    let mut stats: std::collections::BTreeMap<String, (usize, usize)> = Default::default();
    for file in files.split(',') {
        let Ok(data) = std::fs::read(file) else { continue };
        let calls = parse(&data);
        // the LexInfo each record carries, from the analyser, per sentence
        let mut lex: Vec<Vec<u32>> = Vec::new();
        // the sentence after LesGradiAccentuali, and the next FON record
        let mut done: Vec<Word> = Vec::new();
        let mut fon_at = 0usize;
        for c in &calls {
            if let Some((word, _idx, want)) = &c.tp {
                // the next non-punctuation record, within its FON chunk
                while fon_at < done.len() && done[fon_at].f[CLASS] == b'2' {
                    fon_at += 1;
                }
                if fon_at >= done.len() {
                    continue;
                }
                let mut a = fon_at;
                while a > 0 && !(done[a - 1].f[CLASS] == b'2' || matches!(done[a - 1].f[BOUND], 0x0a | 0x08 | 0x0e | 0x0c | 0x0d)) {
                    a -= 1;
                }
                let got = sv.tras_parola(&done[a..], fon_at - a);
                let e = stats.entry("FonTrasParola".into()).or_default();
                e.0 += 1;
                if &got != want || done[fon_at].text != *word {
                    e.1 += 1;
                    if e.1 <= max {
                        let li = LexInfo::of(&done[fon_at]);
                        let morphs: Vec<String> = li.best().unwrap_or(&[]).iter().map(|m| format!("{}/{}/{}{}", String::from_utf8_lossy(sv.m_str(m.m)), String::from_utf8_lossy(sv.category(sv.m_cat(m.m)).unwrap_or(b"-")), String::from_utf8_lossy(sv.group(sv.m_grp(m.m)).unwrap_or(b"-")), sv.m_alt(m.m).map(|a| format!("({})", String::from_utf8_lossy(a))).unwrap_or_default())).collect();
                        println!("FON {} [{}] grade {:02x} slots {:?}\n  ours {:x?}\n  orig {:x?}", String::from_utf8_lossy(word), morphs.join("+"), done[fon_at].f[GRADE], &li.slots[..li.best().map_or(1, |b| b.len().max(1))], got, want);
                    }
                }
                fon_at += 1;
                continue;
            }
            let mut w: Vec<Word> = c.before.iter().map(word).collect();
            if c.name == "Analizza" {
                lex = w
                    .iter()
                    .map(|r| {
                        let from_elabora = r.f[SUB] == 0x17 || (r.f[CLASS] == 0x15 && matches!(r.f[SUB], b'z' | b'y' | 0x7b));
                        let mut x = r.clone();
                        let analyses = if from_elabora && r.f[CLASS] != b'2' {
                            sv.analyse_word(&r.text.iter().map(|&ch| sv.lower(ch)).collect::<Vec<u8>>())
                        } else {
                            Vec::new()
                        };
                        LexInfo { slots: vec![[0; 6]; super::SLOTS], analyses }.store(&mut x);
                        x.data
                    })
                    .collect();
            }
            for (r, d) in w.iter_mut().zip(&lex) {
                r.data = d.clone();
            }
            match c.name.as_str() {
                "Analizza" => sv.analizza(&mut w),
                "Accenta" => sv.accenta(&mut w),
                "ConfiniProsodici" => sv.confini(&mut w, c.mode),
                _ => sv.gradi(&mut w),
            }
            for (k, r) in w.iter().enumerate() {
                if k < lex.len() {
                    lex[k] = r.data.clone();
                }
            }
            let want: Vec<Word> = c.after.iter().map(word).collect();
            if c.name == "GradiAccentuali" {
                done = want.clone();
                for (r, d) in done.iter_mut().zip(&lex) {
                    r.data = d.clone();
                }
                fon_at = 0;
            }
            let same = w.len() == want.len() && w.iter().zip(&want).all(|(a, b)| a.text == b.text && a.f[0xc..0x14] == b.f[0xc..0x14]);
            let e = stats.entry(c.name.clone()).or_default();
            e.0 += 1;
            if !same {
                e.1 += 1;
                if e.1 <= max {
                    println!("{} mode {}\n  in   {}\n  ours {}\n  orig {}", c.name, c.mode, show(&c.before.iter().map(word).collect::<Vec<_>>()), show(&w), show(&want));
                }
            }
        }
    }
    for (k, (n, bad)) in &stats {
        println!("{k}: {} of {n} calls identical", n - bad);
    }
}
