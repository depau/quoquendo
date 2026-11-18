//! A voice's phone set, read from its own library (`LTTS7<Voice>.dll`, which
//! the voice installer puts in the engine's `bin` folder), and the CAT
//! tables LoqTTS7.dll derives from it when the voice loads (MECHANICS.md §5).
//!
//! The library holds the phone table as 16-byte records
//! `{ name pointer, id (u8), u8, 2 pad, base features (u32), u32 }` with ids
//! 0, 1, 2… in order; record 0 is `{ "", 0, 0, 0xffffffff }`. It is found by
//! that shape, not by an offset, so any voice's library works.

/// One phone of the voice: its symbol and the library's feature word.
#[derive(Clone, Debug)]
pub struct PhoneDef {
    pub name: String,
    pub base: u32,
}

struct Pe<'a> {
    data: &'a [u8],
    /// (virtual address, size, file offset)
    sections: Vec<(u32, u32, u32)>,
}

fn u16_at(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?))
}

fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

impl<'a> Pe<'a> {
    fn parse(data: &'a [u8]) -> Option<Pe<'a>> {
        if data.get(0..2)? != b"MZ" {
            return None;
        }
        let nt = u32_at(data, 0x3c)? as usize;
        if data.get(nt..nt + 4)? != b"PE\0\0" {
            return None;
        }
        let count = u16_at(data, nt + 6)? as usize;
        let opt_size = u16_at(data, nt + 20)? as usize;
        let base = u32_at(data, nt + 24 + 28)?;
        let mut sections = Vec::new();
        for i in 0..count {
            let s = nt + 24 + opt_size + 40 * i;
            let (vsize, va, rsize, raw) = (u32_at(data, s + 8)?, u32_at(data, s + 12)?, u32_at(data, s + 16)?, u32_at(data, s + 20)?);
            sections.push((base.wrapping_add(va), vsize.max(rsize), raw));
        }
        Some(Pe { data, sections })
    }

    fn offset(&self, va: u32) -> Option<usize> {
        self.sections
            .iter()
            .find(|(v, size, _)| va >= *v && va - v < *size)
            .map(|(v, _, raw)| (raw + (va - v)) as usize)
    }

    /// A short printable name at `va` (phone symbols are 1–8 ASCII characters).
    fn name(&self, va: u32) -> Option<String> {
        let o = self.offset(va)?;
        let rest = self.data.get(o..)?;
        let end = rest.iter().position(|&b| b == 0)?;
        let s = &rest[..end];
        (s.len() <= 8 && s.iter().all(|b| b.is_ascii_graphic())).then(|| String::from_utf8_lossy(s).into_owned())
    }
}

/// The phone table of a voice library's bytes.
pub fn phone_table(data: &[u8]) -> Option<Vec<PhoneDef>> {
    let pe = Pe::parse(data)?;
    let rec = |o: usize| Some((u32_at(data, o)?, *data.get(o + 4)?, u32_at(data, o + 8)?, u32_at(data, o + 12)?));
    for start in (0..data.len().saturating_sub(32)).step_by(4) {
        let Some((ptr, id, base, extra)) = rec(start) else { break };
        if id != 0 || base != 0 || extra != 0xffff_ffff || pe.name(ptr).as_deref() != Some("") {
            continue;
        }
        let mut out = vec![PhoneDef { name: String::new(), base: 0 }];
        while let Some((ptr, id, base, _)) = rec(start + 16 * out.len()) {
            match pe.name(ptr) {
                Some(name) if id as usize == out.len() && !name.is_empty() => out.push(PhoneDef { name, base }),
                _ => break,
            }
        }
        if out.len() > 10 {
            return Some(out);
        }
    }
    None
}

/// LoqTTS7.dll 0x10050780: the library's feature word → the engine's
/// (`&3` 1 vowel, 2 consonant, 3 pause; `0x10000` stressed; `0x10` unvoiced;
/// `&0x1e3` manner family; `0x1e03` place).
pub fn engine_features(p: u32) -> u32 {
    match p {
        0 => return 0,
        0x28 => return 0x17,
        0x18 => return 0x13,
        0x48 => return 0x10,
        _ => {}
    }
    let has = |m: u32| p & m == m;
    let b = p & 0xff;
    if p & 1 != 0 {
        let mut u = if has(0x401) {
            0x601
        } else if has(0x801) {
            0x401
        } else if has(0x1001) {
            0x201
        } else {
            1
        };
        for (m, v) in [
            (0x2001, 0x1c1),
            (0x4001, 0x181),
            (0x8001, 0x141),
            (0x10001, 0x101),
            (0x20001, 0xc1),
            (0x40001, 0x81),
            (0x80001, 0x41),
        ] {
            if has(m) {
                u |= v;
                break;
            }
        }
        if has(0x100001) {
            u |= 0x21;
        }
        if p & 2 != 0 {
            for (m, v) in [(0x200001, 0xc001), (0x400001, 0x4001), (0x800001, 0x8001)] {
                if has(m) {
                    u |= v;
                    break;
                }
            }
            for (m, v) in [(0x1000001, 0x3001), (0x2000001, 0x2001), (0x4000001, 0x1001), (0x8000001, 1)] {
                if has(m) {
                    u |= v;
                    break;
                }
            }
            if has(0x8000_0001) {
                u |= 0x801;
            }
        }
        if has(0x101) {
            u |= 0x1000_0000;
        }
        if has(0x201) {
            u |= 0x200001;
        }
        if b & 0x81 == 0x81 {
            u |= 4;
        }
        if b & 0x21 == 0x21 {
            u |= 0x10001;
        } else if b & 0x11 == 0x11 {
            u |= 1;
        }
        if b & 0x41 != 0x41 || has(0x10001) || has(0x40001) {
            return u;
        }
        return u | 0x100001;
    }
    if p & 4 == 0 {
        return 0;
    }
    let u = if has(0x14) && b & 0x84 != 0x84 {
        0x1e2
    } else if b & 0x24 == 0x24 {
        0x1c2
    } else if b & 0x44 == 0x44 {
        0x1a2
    } else if b & 0x84 == 0x84 {
        0x182
    } else if has(0x104) {
        0x142
    } else if has(0x204) {
        0xe2
    } else if has(0x404) {
        0xc2
    } else if has(0x804) {
        0xa2
    } else if has(0x1004) {
        0x102
    } else {
        2
    };
    let v = if !has(0x2004) {
        if has(0x4004) {
            u | 0x1c02
        } else if has(0x8004) {
            u | 0x1a02
        } else if has(0x10004) {
            let mut v = u | if b & 0x84 == 0x84 && !has(0x14) { 0x1a02 } else { 0x1802 };
            if has(0x80004) {
                v |= 0x200002;
            }
            if has(0x100004) {
                v |= 0x400002;
            }
            v
        } else if has(0x20004) {
            let mut v = if has(0x100004) { u | 0x401602 } else { u | 0x1602 };
            if has(0x80004) {
                v |= 0x200002;
            }
            v
        } else if has(0x40004) {
            u | 0x1402
        } else if !has(0x100004) {
            if has(0x80004) {
                if has(0x404) && !has(0x8000_0004) {
                    u | 0x100e
                } else {
                    u | 0x1002
                }
            } else if has(0x200004) {
                u | 0xc02
            } else if has(0x400004) {
                u | 0xa02
            } else if has(0x800004) {
                u | 0x802
            } else {
                u
            }
        } else {
            let mut v = u | 0xe02;
            if has(0x80004) {
                v |= 0x200002;
            }
            v
        }
    } else if has(0x404) && (has(0x80004) || has(0x100004)) {
        let u = if has(0x80004) { u | 0x101002 } else { u | 0x100e02 };
        if has(0x8000_0004) {
            u
        } else {
            u | 0xc
        }
    } else {
        u | 0x1e02
    };
    let mut v = v;
    if !has(0x1000004) {
        v |= 0x10;
    }
    for (m, add) in [
        (0x8000004, 0x20002),
        (0x1000_0004, 0x2002),
        (0x2000_0004, 0x4002),
        (0x4000_0004, 0x2000_0000),
        (0x4000004, 4),
    ] {
        if has(m) {
            v |= add;
        }
    }
    v
}

/// LoqTTS7.dll 0x100457b0: a phone's weight class (0–10) for the picker:
/// unvoiced stops 1, fricatives 2–3, nasals 5, liquids 4–6, vowels 7–10.
pub fn weight_class(e: u32) -> u8 {
    let low = e & 0xc != 4;
    if e & 3 == 1 {
        if e & 0x10003 == 1 {
            if e & 0xc003 != 1 {
                return 9;
            }
            if e & 0x8000_0000 != 0 {
                return 5;
            }
            if e & 0x100001 == 0x100001 {
                return 7;
            }
            if e & 0x200001 == 0x200001 {
                return 8;
            }
            return 7 + low as u8;
        }
        if e & 0x10003 == 0x10001 {
            if e & 0xc003 != 1 {
                return 10;
            }
            if e & 0x8000_0000 != 0 {
                return 6;
            }
            return 8 + low as u8;
        }
    }
    if e & 3 != 2 {
        return 0;
    }
    let m = e & 0x1e3;
    let stop_like = |e: u32| e & 0x2002 == 0x2002 || (e & 0x1e03 == 0x1602 && e & 0x200_0002 == 0x200_0002);
    if e & 0x10 != 0 {
        return match m {
            0x1e2 if stop_like(e) => 2,
            0x1e2 | 0x102 | 0x82 | 0x62 => 1,
            _ => 2,
        };
    }
    let trill = |e: u32| e & 0x400002 == 0x400002 || e & 0x200002 == 0x200002;
    match m {
        0xc2 => {
            let c = e & 0xc;
            if c == 0xc {
                6
            } else if e & 0x20002 == 0x20002 {
                7
            } else if e & 0x8000_0000 != 0 {
                5
            } else if e & 0x1e03 == 0x1002 {
                if c != 4 {
                    9
                } else {
                    7
                }
            } else if e & 0x1e03 == 0x1e02 && trill(e) {
                if c == 4 {
                    7
                } else {
                    9
                }
            } else {
                6
            }
        }
        0xa2 => {
            if e & 0x20002 == 0x20002 {
                7
            } else if e & 0x8000_0000 == 0 && e & 0x1e03 != 0x1002 {
                5 + low as u8
            } else {
                4
            }
        }
        0x142 | 0xe2 => {
            if e & 0x8000_0000 != 0 {
                2
            } else if e & 0x1e03 == 0xc02 {
                5
            } else if e & 0x1e03 == 0x1002 || e & 0x20002 == 0x20002 {
                7
            } else {
                3
            }
        }
        0x1c2 => {
            if e & 0x20002 == 0x20002 {
                6
            } else {
                4 + (e & 0x8000_0000 == 0) as u8
            }
        }
        0x1e2 => {
            if stop_like(e) {
                3
            } else {
                2
            }
        }
        0x102 | 0x82 | 0x62 => 2,
        _ => 5,
    }
}

fn place(e: u32) -> i32 {
    match e & 0x1e03 {
        0x1e02 => 0,
        0x1c02 => 1,
        0x1a02 => 3,
        0x1802 => 4,
        0x1602 | 0x1402 => 5,
        0x1002 => 7,
        0xe02 => 9,
        0xc02 => 10,
        0xa02 => 12,
        _ => 13,
    }
}

fn manner(e: u32) -> i32 {
    match e & 0x1e3 {
        0x1c2 => 0,
        0x1e2 | 0x102 => 1,
        0x1a2 => 2,
        0x142 => 3,
        0xa2 | 0xc2 => 4,
        _ => 5,
    }
}

fn height(e: u32) -> i32 {
    match e & 0x1c3 {
        0x41 => 6,
        0x81 => 5,
        0xc1 => 4,
        0x101 => 3,
        0x141 => 2,
        0x181 => 1,
        _ => 0,
    }
}

/// LoqTTS7.dll 0x100495b0 (with 0x10049470, 0x10049550, 0x10049500): how
/// alike two phones sound, 0–30. The same phone (or two pauses) 30; two
/// consonants 17 − place and manner distance, +1 same voicing, +3 unless just
/// one is a nasal; two vowels or glides 13 (21 both stressed), +4 when alike
/// in rounding, − height distance, − 2/4 for front/back mismatches, − 1 for
/// length; a vowel and a liquid-like glide 18; else 0.
pub fn similarity(a: u32, c: u32) -> u8 {
    if a == c {
        return if a != 0 { 30 } else { 0 };
    }
    let (a3, c3) = (a & 3, c & 3);
    if a3 == 3 && c3 == 3 {
        return 30;
    }
    let vowelish = |x: u32| x & 3 == 1 || x & 0x1e3 == 0xc2;
    if !vowelish(a) || !vowelish(c) {
        if a3 == 2 && c3 == 2 {
            let mut v = 17 - (place(c) - place(a)).abs() - (manner(c) - manner(a)).abs();
            if (a ^ c) & 0x10 == 0 {
                v += 1;
            }
            if (a & 0x1e3 == 0x1c2) == (c & 0x1e3 == 0x1c2) {
                v += 3;
            }
            return v as u8;
        }
        let (x, y) = if a3 == 1 { (a, c) } else { (c, a) };
        if x & 0x603 == 0x401 && x & 0x1c3 == 0x101 && y & 0x1e03 == 0x1802 && y & 0x20002 == 0x20002 && matches!(y & 0x1e3, 0xc2 | 0xa2) {
            return 0x12;
        }
        return 0;
    }
    let mut v: i32 = if a & 0x10003 == 0x10001 && c & 0x10003 == 0x10001 { 21 } else { 13 };
    let round = |x: u32| x & 0x23 == 0x21 || x & 0x1e03 == 0x1e02;
    if round(a) == round(c) {
        v += 4;
    }
    v -= (height(c) - height(a)).abs();
    let (ua, uc) = (a & 0x603, c & 0x603);
    let front = |x: u32, u: u32| u == 0x601 || x & 0x1e03 == 0x1002;
    let back = |x: u32, u: u32| u == 0x201 || x & 0x1e03 == 0xe02;
    let (fa, fc, ba, bc) = (front(a, ua), front(c, uc), back(a, ua), back(c, uc));
    if (fa && uc == 0x401) || (fc && ua == 0x401) {
        v -= 2;
    }
    if (ua == 0x401 && bc) || (uc == 0x401 && ba) {
        v -= 2;
    }
    if (fa && bc) || (fc && ba) {
        v -= 4;
    }
    if (a & 0x1000_0000 != 0) != (c & 0x1000_0000 != 0) {
        v -= 1;
    }
    v as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tables computed from Roberto's library equal the ones the original
    /// builds (dumped by oracle/loqref.c LOQ_DUMP_TABLES into oracle/out/tables).
    #[test]
    fn tables_match_the_original() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for voice in ["Roberto", "Allison", "Bernard"] {
            let lib = root.join(format!("oracle/prefix/drive_c/Program Files (x86)/Loquendo/LTTS7/bin/LTTS7{voice}.dll"));
            let (Ok(data), Ok(dump), Ok(sim)) = (
                std::fs::read(&lib),
                std::fs::read_to_string(root.join(format!("oracle/out/tables/{voice}.phones"))),
                std::fs::read(root.join(format!("oracle/out/tables/{voice}.sim"))),
            ) else {
                continue;
            };
            let phones = phone_table(&data).expect("phone table");
            let rows: Vec<Vec<&str>> = dump.lines().map(|l| l.split('\t').collect()).collect();
            assert_eq!(phones.len(), rows.len(), "{voice} phone count");
            let f: Vec<u32> = phones.iter().map(|p| engine_features(p.base)).collect();
            for (i, r) in rows.iter().enumerate() {
                assert_eq!(phones[i].name, r[1]);
                assert_eq!(f[i], u32::from_str_radix(r[2], 16).unwrap(), "{voice} {} features", r[1]);
                assert_eq!(weight_class(f[i]).to_string(), r[3], "{voice} {} class", r[1]);
            }
            let n = f.len();
            for i in 0..n {
                for j in 0..n {
                    assert_eq!(similarity(f[i], f[j]), sim[i * n + j], "{voice} {} {}", rows[i][1], rows[j][1]);
                }
            }
        }
    }
}
