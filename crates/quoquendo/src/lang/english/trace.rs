//! The port checked call by call against the original's own calls: a
//! trace of LTTS7English.dll's function-table slots (oracle/loqref.c with
//! LOQ_TRACE_LES, reduced by the conversion script MECHANICS.md §4c names)
//! replayed through the port, stage by stage, each stage fed the original's
//! own input.
//!
//!   QQ_EN_TRACE=<trace.txt> QQ_EN_VARIANT=us|gb|au [QQ_EN_LTTS7=<LTTS7 dir>]
//!   [QQ_EN_MAX=<examples>] cargo test -p quoquendo --release english_trace -- --ignored --nocapture

use super::super::common::{field::*, record, Les, Phone, Word};
use super::English;

struct Sentence {
    /// stage → (records in, records out): (f[0xc..0x18], text)
    stages: Vec<(String, Vec<([u8; 12], Vec<u8>)>, Vec<([u8; 12], Vec<u8>)>)>,
    /// the sentence type LesConfiniProsodici gets (ctx+8)
    mode: i32,
    /// FonTrasParola: (index in its chunk, word, codes)
    tp: Vec<(usize, Vec<u8>, Vec<u32>)>,
    /// FonAllofonoLingua: (phones in, phones out), one per chunk
    al: Vec<(Vec<Phone>, Vec<Phone>)>,
}

fn unhex(s: &str) -> Vec<u8> {
    if s == "-" {
        return Vec::new();
    }
    (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap_or(0)).collect()
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

fn parse(text: &str) -> Vec<Sentence> {
    let mut out: Vec<Sentence> = Vec::new();
    let mut pending: Option<Vec<Phone>> = None;
    for line in text.lines() {
        let (tag, rest) = line.split_at(1);
        let rest = rest.trim_start();
        if tag == "S" {
            out.push(Sentence { stages: Vec::new(), mode: 0, tp: Vec::new(), al: Vec::new() });
            continue;
        }
        let Some(s) = out.last_mut() else { continue };
        match tag {
            "G" => s.stages.push((rest.to_string(), Vec::new(), Vec::new())),
            "M" => s.mode = rest.parse().unwrap_or(0),
            "I" | "O" => {
                let p: Vec<&str> = rest.split(' ').collect();
                if p.len() < 3 {
                    continue;
                }
                let b = unhex(p[1]);
                let mut f = [0u8; 12];
                f.copy_from_slice(&b[..12]);
                let r = (f, unhex(p[2]));
                if let Some(st) = s.stages.last_mut() {
                    if tag == "I" {
                        st.1.push(r)
                    } else {
                        st.2.push(r)
                    }
                }
            }
            "T" => {
                let p: Vec<&str> = rest.split(' ').collect();
                let codes = if p[2] == "-" { Vec::new() } else { p[2].split(',').map(|c| u32::from_str_radix(c, 16).unwrap_or(0)).collect() };
                s.tp.push((p[0].parse().unwrap_or(0), unhex(p[1]), codes));
            }
            "P" => pending = Some(phones(rest)),
            "Q" => {
                if let Some(p) = pending.take() {
                    s.al.push((p, phones(rest)));
                }
            }
            _ => {}
        }
    }
    out
}

fn load(variant: &str) -> English {
    let root = std::env::var("QQ_EN_LTTS7").unwrap_or_else(|_| "/tmp/qqv/LTTS7".into());
    let mt = match variant {
        "gb" => "EnglishGb",
        "au" => "EnglishAu",
        _ => "EnglishUs",
    };
    let read = |p: String| std::fs::read(&p).unwrap_or_else(|e| panic!("{p}: {e}"));
    let files = super::super::ModuleFiles {
        dll: read(format!("{root}/bin/LTTS7English.dll")),
        mother_tongue: mt.into(),
        dbl: Some(read(format!("{root}/data/languages/English/{mt}.dbl"))),
        gpr: Some(read(format!("{root}/data/languages/English/{mt}.gpr"))),
        ..Default::default()
    };
    English::from_files(&files).expect("LTTS7English.dll")
}

fn show(r: &([u8; 12], Vec<u8>)) -> String {
    format!("{} {:02x?}", String::from_utf8_lossy(&r.1), &r.0[..8])
}

fn show_phones(en: &English, p: &[Phone]) -> String {
    p.iter().map(|p| format!("{}:{}({:x}):{:x}", p.word, en.symbol(p.code).unwrap_or("?"), p.code, p.kind)).collect::<Vec<_>>().join(" ")
}

#[test]
#[ignore]
fn english_trace() {
    let Ok(path) = std::env::var("QQ_EN_TRACE") else { return };
    let variant = std::env::var("QQ_EN_VARIANT").unwrap_or_else(|_| "us".into());
    let max: usize = std::env::var("QQ_EN_MAX").ok().and_then(|s| s.parse().ok()).unwrap_or(10);
    let en = load(&variant);
    let sentences = parse(&std::fs::read_to_string(&path).unwrap());
    let names = ["LesGestisciMaiuscole", "LesAnalizza", "LesConfiniProsodici", "LesGradiAccentuali"];
    let mut total = [0usize; 6];
    let mut bad = [0usize; 6];
    for s in &sentences {
        let Some(first) = s.stages.first() else { continue };
        let mut w: Vec<Word> = first
            .1
            .iter()
            .map(|(f, t)| {
                let mut r = record(t.clone(), 0, 0, 0);
                r.f[0xc..0x14].copy_from_slice(&f[..8]);
                r
            })
            .collect();
        let sentence_text = || w.iter().map(|r| String::from_utf8_lossy(&r.text).into_owned()).collect::<Vec<_>>().join(" ");
        let text0 = sentence_text();
        for (k, name) in names.iter().enumerate() {
            let Some(st) = s.stages.iter().find(|st| st.0 == *name) else { continue };
            if st.1.len() != w.len() || st.2.len() != w.len() {
                continue;
            }
            for (r, i) in w.iter_mut().zip(&st.1) {
                r.f[0xc..0x14].copy_from_slice(&i.0[..8]);
                r.text = i.1.clone();
            }
            match k {
                0 => en.gestisci_maiuscole(&mut w),
                1 => Les::analizza(&en, &mut w),
                2 => Les::confini(&en, &mut w, s.mode),
                _ => Les::gradi(&en, &mut w),
            }
            total[k] += 1;
            let wrong: Vec<usize> = (0..w.len()).filter(|&i| w[i].f[0xc..0x14] != st.2[i].0[..8] || w[i].text != st.2[i].1).collect();
            if !wrong.is_empty() {
                bad[k] += 1;
                if bad[k] <= max {
                    println!("{name} [{text0}]");
                    for &i in &wrong {
                        println!("  {i}: in {}  want {}  got {} {:02x?}", show(&st.1[i]), show(&st.2[i]), String::from_utf8_lossy(&w[i].text), &w[i].f[0xc..0x14]);
                    }
                }
            }
            for (r, o) in w.iter_mut().zip(&st.2) {
                r.f[0xc..0x14].copy_from_slice(&o.0[..8]);
                r.text = o.1.clone();
            }
        }
        // FON chunks: records up to a punctuation record
        let mut chunks: Vec<std::ops::Range<usize>> = Vec::new();
        let mut a = 0;
        for i in 0..w.len() {
            if w[i].f[CLASS] == b'2' {
                chunks.push(a..i + 1);
                a = i + 1;
            }
        }
        if a < w.len() {
            chunks.push(a..w.len());
        }
        let mut c = 0usize;
        let mut last = None;
        for (idx, word, want) in &s.tp {
            if last.is_some_and(|l| *idx <= l) {
                c += 1;
            }
            last = Some(*idx);
            let Some(ch) = chunks.get(c) else { break };
            let chunk = &w[ch.clone()];
            if *idx >= chunk.len() {
                continue;
            }
            let got = Les::tras_parola(&en, chunk, *idx);
            total[4] += 1;
            if &got != want {
                bad[4] += 1;
                if bad[4] <= max {
                    let sy = |v: &[u32]| v.iter().map(|&c| format!("{}({:x})", en.symbol(c).unwrap_or("?"), c)).collect::<Vec<_>>().join(" ");
                    println!(
                        "FonTrasParola {} [{text0}] class {:x} grade {:x}\n  want {}\n  got  {}",
                        String::from_utf8_lossy(word),
                        chunk[*idx].f[CLASS],
                        chunk[*idx].f[GRADE],
                        sy(want),
                        sy(&got)
                    );
                }
            }
        }
        for (k, (pin, pout)) in s.al.iter().enumerate() {
            let Some(ch) = chunks.get(k) else { break };
            let mut p = pin.clone();
            let lastw = p.last().map(|x| x.word).unwrap_or(0);
            p.push(Phone { word: lastw, code: 0, kind: 0x70 });
            let mut i = 0usize;
            while i < p.len().saturating_sub(1) {
                i = en.allofono(&w, ch.start, &mut p, i).wrapping_add(1);
            }
            p.pop();
            total[5] += 1;
            if &p != pout {
                bad[5] += 1;
                if bad[5] <= max {
                    println!("FonAllofonoLingua [{text0}]\n  in   {}\n  want {}\n  got  {}", show_phones(&en, pin), show_phones(&en, pout), show_phones(&en, &p));
                }
            }
        }
    }
    let labels = ["LesGestisciMaiuscole", "LesAnalizza", "LesConfiniProsodici", "LesGradiAccentuali", "FonTrasParola", "FonAllofonoLingua"];
    for k in 0..6 {
        println!("{:22} {} of {} differ", labels[k], bad[k], total[k]);
    }
}

/// A voice's phone table (symbol, feature word): QQ_EN_VOICE=<name>.
#[test]
#[ignore]
fn english_voice_phones() {
    let Ok(name) = std::env::var("QQ_EN_VOICE") else { return };
    let root = std::env::var("QQ_EN_LTTS7").unwrap_or_else(|_| "/tmp/qqv/LTTS7".into());
    let d = crate::loquendo::discover_voices(std::path::Path::new(&format!("{root}/data")));
    let v = d.iter().find(|v| v.name.eq_ignore_ascii_case(&name)).expect("voice");
    let voice = crate::Voice::open(v).unwrap();
    for (s, f) in voice.db.tables.as_ref().unwrap().phone_features() {
        print!("{s}={f:x} ");
    }
    println!();
}
