//! What every language module port shares: reading the module's DLL as data
//! (tables are located by the addresses its code uses) and the word records
//! the engine's LES stage hands the module.

use std::sync::Arc;

/// A PE image read as data: virtual addresses → bytes. Nothing in it runs.
#[derive(Clone)]
pub struct Pe {
    data: Arc<[u8]>,
    /// (virtual address, size, file offset, raw size)
    sections: Vec<(u32, u32, u32, u32)>,
}

fn u16_at(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?))
}

fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

impl Pe {
    pub fn parse(data: Vec<u8>) -> Option<Pe> {
        if data.get(0..2)? != b"MZ" {
            return None;
        }
        let nt = u32_at(&data, 0x3c)? as usize;
        if data.get(nt..nt + 4)? != b"PE\0\0" {
            return None;
        }
        let count = u16_at(&data, nt + 6)? as usize;
        let opt_size = u16_at(&data, nt + 20)? as usize;
        let base = u32_at(&data, nt + 24 + 28)?;
        let mut sections = Vec::new();
        for i in 0..count {
            let s = nt + 24 + opt_size + 40 * i;
            let (vsize, va, rsize, raw) = (u32_at(&data, s + 8)?, u32_at(&data, s + 12)?, u32_at(&data, s + 16)?, u32_at(&data, s + 20)?);
            sections.push((base.wrapping_add(va), vsize.max(rsize), raw, rsize));
        }
        Some(Pe { data: data.into(), sections })
    }

    /// The image's bytes from `va` to the end of its section (zeros past the
    /// raw data are not in the file and read as empty).
    pub fn at(&self, va: u32) -> &[u8] {
        self.sections
            .iter()
            .find(|(v, size, _, _)| va >= *v && va - v < *size)
            .and_then(|&(v, _, raw, rsize)| {
                let off = va - v;
                (off < rsize).then(|| &self.data[(raw + off) as usize..(raw + rsize) as usize])
            })
            .unwrap_or(&[])
    }

    pub fn u8(&self, va: u32) -> u8 {
        self.at(va).first().copied().unwrap_or(0)
    }

    pub fn u16(&self, va: u32) -> u16 {
        u16_at(self.at(va), 0).unwrap_or(0)
    }

    pub fn u32(&self, va: u32) -> u32 {
        u32_at(self.at(va), 0).unwrap_or(0)
    }

    /// The NUL-terminated string at `va` (without the NUL).
    pub fn cstr(&self, va: u32) -> &[u8] {
        let b = self.at(va);
        &b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]
    }
}

/// C `strcmp` on two byte strings without their NULs: the sign of the
/// first difference, shorter first.
pub fn strcmp(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    a.cmp(b)
}

/// `strncmp(a, b, n) == 0` for NUL-free strings: the first `n` bytes agree,
/// a string ending before `n` only matching one that ends there too.
pub fn starts(a: &[u8], b: &[u8], n: usize) -> bool {
    let (x, y) = (&a[..a.len().min(n)], &b[..b.len().min(n)]);
    x == y
}

/// LTTS7Util.dll's `ELQSearchCompactString` (0x100289f0) over a blob of
/// NUL-separated strings, for the modes the language modules use:
/// `flags & 8`, an entry that ends the key; otherwise a binary search of the
/// sorted blob, `flags & 2` an entry the key starts with, else the key
/// itself (entries may be `key<sep>value`). The match's offset in the blob
/// (after the separator when `sep` is not 0).
///
/// The binary search stops when its bounds meet without looking at the
/// entry there, as the original does.
pub fn search_compact(key: &[u8], blob: &[u8], flags: u32, sep: u8) -> Option<usize> {
    let k0 = *key.first()?;
    let size = blob.len() as i64;
    if flags & 8 != 0 {
        let mut i = 0usize;
        if size <= 1 {
            return None;
        }
        loop {
            let e = &blob[i..];
            let len = e.iter().position(|&c| c == 0).unwrap_or(e.len());
            if len <= key.len() && key[key.len() - len..] == e[..len] {
                return Some(i);
            }
            i += 1 + len;
            if size <= i as i64 + 1 {
                return None;
            }
        }
    }
    let b = |i: i64| blob.get(i as usize).copied().unwrap_or(0);
    let (mut lo, mut hi) = (0i64, size - 1);
    if hi <= 0 {
        return None;
    }
    loop {
        let sum = hi + lo;
        let mut e = sum / 2;
        while e > 0 && b(e - 1) != 0 {
            e -= 1;
        }
        let mut k = 0i64;
        if b(e) == k0 {
            let mut c = k0;
            loop {
                if c == 0 || b(e + k) == sep {
                    break;
                }
                c = at(key, k as usize + 1);
                k += 1;
                if b(e + k) != c {
                    break;
                }
            }
        }
        if flags & 2 != 0 && b(e + k) == 0 {
            return Some(e as usize);
        }
        if at(key, k as usize) == 0 && b(e + k) == sep {
            return Some(if sep != 0 { (e + k + 1) as usize } else { e as usize });
        }
        if at(key, k as usize) < b(e + k) {
            hi = e - 1;
        } else {
            lo = sum / 2 + 1;
        }
        if hi <= lo {
            return None;
        }
    }
}

/// `strchr(set, c) != NULL` for `c != 0`.
pub fn among(set: &[u8], c: u8) -> bool {
    c != 0 && set.contains(&c)
}

/// The byte at `i`, 0 past the end (the C strings' terminator and beyond).
#[inline]
pub fn at(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

/// One word of a phrase as the engine's LES stage keeps it: a 24-byte
/// record (`+4` its text, `+0` a tag) whose byte fields the language module
/// reads and writes. The fields are kept at their offsets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    /// the text, in the engine's 8-bit (Windows-1252) characters
    pub text: Vec<u8>,
    /// the tag the engine attached (`\{pause=0` before a sentence end)
    pub tag: Option<Vec<u8>>,
    /// the record's bytes; only 0x0c..0x18 are meaningful here
    pub f: [u8; 24],
}

/// Field offsets in a word record.
pub mod field {
    /// word class (0x17 unknown, 0x2f noun/other, 0x2a verb, 0x18 article…)
    pub const CLASS: usize = 0xe;
    /// subclass (verb form 0x87…, 0xc8 none)
    pub const SUB: usize = 0xf;
    /// further information (gender/number, punctuation kind…)
    pub const INFO: usize = 0x10;
    /// prosodic boundary after the word
    pub const BOUND: usize = 0x11;
    /// stress grade (0 stressed, 3 unstressed, 0x0e not set yet)
    pub const GRADE: usize = 0x12;
}

impl Word {
    pub fn class(&self) -> u8 {
        self.f[field::CLASS]
    }
}
