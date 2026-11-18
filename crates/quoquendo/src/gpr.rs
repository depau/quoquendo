//! Loquendo's letter-to-sound models (`<Language>.gpr`, read by
//! LTTS7GPR.dll as "LoqG2P"): for every letter, an ordered list of rules
//! with a left and a right context; the first rule whose contexts match the
//! letter's surroundings in `-word-` gives its output class, else the
//! letter's default class. A class is a list of phones as engine feature
//! words (`phoneset::engine_features` format), possibly empty (silent).
//!
//! File layout (little-endian), from the loader at LTTS7GPR.dll 0x10001710:
//! byte flag; u32 file size; u8 letters, u8 multi-letter graphemes, u8
//! classes, u8; u32 × 8 offsets (the last: start of the context string pool);
//! the alphabet; graphemes (u8 length + letter indexes); classes (u8 count +
//! u32 features each); u32 rule bytes per letter; u8 default class per
//! letter; u32 file offset of each letter's rules. A rule (0x10001220):
//! u8 (left length << 4 | right length), u8 class, then a 3-byte pool offset
//! for each non-empty context.

#[derive(Clone, Debug)]
struct Rule {
    left: Vec<u8>,
    right: Vec<u8>,
    class: u8,
}

pub struct Gpr {
    alphabet: Vec<u8>,
    classes: Vec<Vec<u32>>,
    default: Vec<u8>,
    rules: Vec<Vec<Rule>>,
}

fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

fn u24_at(d: &[u8], o: usize) -> Option<usize> {
    let b = d.get(o..o + 3)?;
    Some(b[0] as usize | (b[1] as usize) << 8 | (b[2] as usize) << 16)
}

impl Gpr {
    pub fn parse(d: &[u8]) -> Option<Gpr> {
        let (letters, multi, nclasses) = (*d.get(5)? as usize, *d.get(6)? as usize, *d.get(7)? as usize);
        let pool_at = u32_at(d, 9 + 4 * 7)? as usize;
        let pool = d.get(pool_at..)?;
        let mut o = 0x29;
        let alphabet = d.get(o..o + letters)?.to_vec();
        o += letters;
        for _ in 0..multi {
            // ponytail: multi-letter graphemes are skipped (no shipped English
            // model has any); add them when a language's model does
            let n = *d.get(o)? as usize;
            o += 1 + n;
        }
        let mut classes = Vec::with_capacity(nclasses);
        for _ in 0..nclasses {
            let n = *d.get(o)? as usize;
            o += 1;
            classes.push((0..n).map(|j| u32_at(d, o + 4 * j)).collect::<Option<Vec<u32>>>()?);
            o += 4 * n;
        }
        let symbols = letters + multi;
        o += 4 * symbols;
        let default = d.get(o..o + symbols)?.to_vec();
        o += symbols;
        let offsets: Vec<usize> = (0..symbols).map(|k| u32_at(d, o + 4 * k).map(|v| v as usize)).collect::<Option<_>>()?;
        let mut rules = Vec::with_capacity(symbols);
        for k in 0..symbols {
            let end = offsets.get(k + 1).copied().unwrap_or(pool_at);
            let mut p = offsets[k];
            let mut list = Vec::new();
            while p < end {
                let b = *d.get(p)?;
                let class = *d.get(p + 1)?;
                let (ln, rn) = ((b >> 4) as usize, (b & 15) as usize);
                let mut q = p + 2;
                let mut left = Vec::new();
                let mut right = Vec::new();
                if ln > 0 {
                    let at = u24_at(d, q)?;
                    left = pool.get(at..at + ln)?.to_vec();
                }
                if ln > 0 && rn > 0 {
                    q = p + 5;
                }
                if rn > 0 {
                    let at = u24_at(d, q)?;
                    right = pool.get(at..at + rn)?.to_vec();
                }
                list.push(Rule { left, right, class });
                p = q + 3;
            }
            rules.push(list);
        }
        Some(Gpr {
            alphabet,
            classes,
            default,
            rules,
        })
    }

    /// The phones of a lower-case word, as engine feature words.
    pub fn transcribe(&self, word: &[u8]) -> Vec<u32> {
        let mut buf = Vec::with_capacity(word.len() + 2);
        buf.push(b'-');
        buf.extend_from_slice(word);
        buf.push(b'-');
        let mut out = Vec::new();
        for i in 1..buf.len() - 1 {
            let Some(k) = self.alphabet.iter().position(|&c| c == buf[i]) else { continue };
            let mut class = self.default[k];
            for r in &self.rules[k] {
                if r.left.is_empty() && r.right.is_empty() {
                    continue;
                }
                if !r.left.is_empty() && (r.left.len() > i || buf[i - r.left.len()..i] != r.left[..]) {
                    continue;
                }
                if !r.right.is_empty() && buf.get(i + 1..i + 1 + r.right.len()) != Some(&r.right[..]) {
                    continue;
                }
                class = r.class;
                break;
            }
            if let Some(c) = self.classes.get(class as usize) {
                out.extend(c.iter().copied().filter(|&f| f != 0));
            }
        }
        out
    }
}
