//! The records LesElaboraParola (0x10003280) makes for a word token: the
//! word lower-cased and analysed (0x10002f60, the analyser's analyses kept
//! in the record's LexInfo), split at a hyphen when the analyser knows
//! neither the whole nor misses `-то`, or spelled letter by letter
//! (0x10002640) when LesPronunciabile rejects it; and the letter and
//! symbol names (LesGetSymbolName 0x100014f0).

use super::{les, Lex, Russian, CONSONANT, VOWEL};
use crate::lang::common::{field::*, record, Tok, Word};

/// `-то` (0x1006687c): a hyphenated word ending in it is analysed as the
/// word before and `-то`.
const TO: u32 = 0x1006687c;
/// The spelling `xxversionxx` reads as (0x10066880).
const VERSION: u32 = 0x10066880;
/// The 0x51 lower-case letter groups LesPronunciabile reads although
/// their vowels and consonants say no (0x100a2000, sorted pointers;
/// 0x10002790).
const READABLE: u32 = 0x100a2000;
const READABLE_COUNT: i32 = 0x51;
/// `ьъы` (0x10065f28): a word with one of them is readable when it has a
/// vowel.
const SOFT: u32 = 0x10065f28;
/// The consonant/vowel shapes of at most six letters that are not
/// readable: `^VCCCCC$`, `^CCCCCV$`, `^VCCCC$`, `^CCCCV$`, `^VCCC$`,
/// `^CCCV$` (0x10002880).
const UNREADABLE: [u32; 6] = [0x10065f1c, 0x10065f10, 0x10065f08, 0x10065f00, 0x10065ef8, 0x10065ef0];

/// A name LesGetSymbolName copies: a string of the module, or the bytes
/// its code stores.
#[derive(Clone, Copy)]
enum Name {
    At(u32),
    Bytes(&'static [u8]),
}

/// `LesGetSymbolName` 0x100014f0: (characters, named only when the third
/// argument asks for symbols too, the name).
const SYMBOLS: &[(&[u8], bool, Name)] = &[
    (&[0x21], true, Name::At(0x10065e1c)), // восклицательный знак
    (&[0x22], true, Name::At(0x10065e04)), // знак угловой секунды
    (&[0x23], true, Name::At(0x10065dfc)), // решётка
    (&[0x24], false, Name::At(0x10065df4)), // доллар
    (&[0x25], false, Name::At(0x10065dec)), // процент
    (&[0x26], false, Name::At(0x10065de0)), // амперсанд
    (&[0x27], true, Name::At(0x10065dc0)), // Знак угловой минуты; апостроф
    (&[0x28], true, Name::At(0x10065d9c)), // открывающая круглая скобка скобка
    (&[0x29], true, Name::At(0x10065d80)), // закрывающая круглая скобка
    (&[0x2a], false, Name::At(0x10065d74)), // звёздочка
    (&[0x2b], false, Name::At(0x10065d6c)), // плюс
    (&[0x2c], true, Name::At(0x10065d64)), // запятая
    (&[0x2d], false, Name::At(0x10065d5c)), // дефис
    (&[0x2e], true, Name::At(0x10065d54)), // точка
    (&[0x2f], true, Name::At(0x10065d4c)), // слэш
    (&[0x30], false, Name::At(0x10065e7c)), // ноль
    (&[0x31], false, Name::At(0x10065e74)), // один
    (&[0x32], false, Name::Bytes(b"\xe4\xe2\xe0")), // два
    (&[0x33], false, Name::Bytes(b"\xf2\xf0\xe8")), // три
    (&[0x34], false, Name::At(0x10065e6c)), // четыре
    (&[0x35], false, Name::At(0x10065e64)), // пять
    (&[0x36], false, Name::At(0x10065e5c)), // шесть
    (&[0x37], false, Name::At(0x10065e54)), // семь
    (&[0x38], false, Name::At(0x10065e4c)), // восемь
    (&[0x39], false, Name::At(0x10065e44)), // девять
    (&[0x3a], true, Name::At(0x10065d40)), // двоеточие
    (&[0x3b], true, Name::At(0x10065d30)), // точка с запятой
    (&[0x3c], true, Name::At(0x10065d28)), // меньше
    (&[0x3d], false, Name::At(0x10065d20)), // равно
    (&[0x3e], true, Name::At(0x10065d18)), // больше
    (&[0x3f], true, Name::At(0x10065d04)), // вопросительный знак
    (&[0x40], true, Name::At(0x10065cfc)), // собака
    (&[0x41, 0x61], false, Name::Bytes(b"\xe0")), // а
    (&[0x48, 0x68], false, Name::At(0x10020500)), // аш
    (&[0x4a, 0x6a], false, Name::At(0x10065eac)), // й^от
    (&[0x4c, 0x6c], false, Name::At(0x10065ea4)), // ~ель
    (&[0x51, 0x71], false, Name::At(0x10065ea0)), // ку
    (&[0x57, 0x77], false, Name::At(0x10065e94)), // д^убль в~е
    (&[0x58, 0x78], false, Name::Bytes(b"\xe8\xea\xf1")), // икс
    (&[0x59, 0x79], false, Name::At(0x10065e8c)), // ^игрик
    (&[0x5a, 0x7a], false, Name::At(0x10065e84)), // з~^ета
    (&[0x5b], true, Name::At(0x10065cdc)), // открывающая квадратная скобка
    (&[0x5c], true, Name::At(0x10065e34)), // обратный слэш
    (&[0x5d], true, Name::At(0x10065cbc)), // закрывающая квадратная скобка
    (&[0x5e], true, Name::At(0x10065ca4)), // знак вставки; крышечка
    (&[0x5f], true, Name::At(0x10065c98)), // нижнее тире
    (&[0x60], true, Name::At(0x10065c90)), // седиль
    (&[0x7b], true, Name::At(0x10065c78)), // левая фигурная скобка
    (&[0x7c], true, Name::At(0x10065c64)), // вертикальная черта
    (&[0x7d], true, Name::At(0x10065c4c)), // правая фигурная скобка
    (&[0x7e], true, Name::At(0x10065c44)), // тильда
    (&[0x82], true, Name::At(0x10065d64)), // запятая
    (&[0x84], true, Name::At(0x10065abc)), // нижние кавычки-лапки
    (&[0x85], true, Name::At(0x10065a88)), // многоточие
    (&[0x86], true, Name::At(0x10065ab4)), // крестик
    (&[0x87], true, Name::At(0x10065aa4)), // двойной крестик
    (&[0x88], false, Name::At(0x10065a3c)), // евро
    (&[0x89], false, Name::At(0x10065a7c)), // промилле
    (&[0x8b], true, Name::At(0x10065a60)), // открывающие угловая скобка
    (&[0x91], true, Name::At(0x10065b34)), // открывающие одинарные верхние кавычки
    (&[0x92], true, Name::At(0x10065b0c)), // закрыващие одинарные верхние кавычки
    (&[0x93], true, Name::At(0x10065af0)), // открывающие верхние кавычки
    (&[0x94], true, Name::At(0x10065ad4)), // закрывающие верхние кавычки
    (&[0x95], true, Name::At(0x10065a94)), // простой маркер
    (&[0x96], true, Name::At(0x10065b6c)), // узкое тире
    (&[0x97], true, Name::At(0x10065b5c)), // широкое тире
    (&[0x99], false, Name::At(0x10065a24)), // торговая марка
    (&[0x9b], true, Name::At(0x10065a44)), // закрывающая угловая скобка
    (&[0xa4], true, Name::At(0x10065c2c)), // Знак денежной единицы
    (&[0xa6], true, Name::At(0x10065c14)), // вертикальный пунктир
    (&[0xa7], false, Name::At(0x10065c08)), // параграф
    (&[0xa8, 0xb8], false, Name::Bytes(b"\xb8")), // ё
    (&[0xa9], false, Name::At(0x10065bfc)), // копирайт
    (&[0xab], true, Name::At(0x10065be8)), // открывающие кавычки
    (&[0xad], true, Name::At(0x10065d5c)), // дефис
    (&[0xae], false, Name::At(0x10065bcc)), // Зарегистрированный товарный
    (&[0xb0], true, Name::At(0x10065bc4)), // градус
    (&[0xb1], true, Name::At(0x10065bb8)), // плюс минус
    (&[0xb5], true, Name::At(0x10065bb0)), // микро
    (&[0xb6], true, Name::At(0x10065ba4)), // знак абзаца
    (&[0xb7], true, Name::At(0x10065b94)), // средняя точка
    (&[0xb9], false, Name::At(0x10065a34)), // номер
    (&[0xbb], true, Name::At(0x10065b78)), // закрывающие кавычки-ёлочки
    (&[0xc0, 0xe0], false, Name::Bytes(b"\xe0")), // а
    (&[0xc1, 0xe1, 0x42, 0x62], false, Name::Bytes(b"\xe1\x7e\xe5")), // б~е
    (&[0xc2, 0xe2, 0x56, 0x76], false, Name::Bytes(b"\xe2\x7e\xe5")), // в~е
    (&[0xc3, 0xe3, 0x47, 0x67], false, Name::Bytes(b"\xe3\x7e\xe5")), // г~е
    (&[0xc4, 0xe4, 0x44, 0x64], false, Name::Bytes(b"\xe4\x7e\xe5")), // д~е
    (&[0xc5, 0xe5], false, Name::Bytes(b"\xe5")), // е
    (&[0xc6, 0xe6], false, Name::Bytes(b"\xe6\x7e\xe5")), // ж~е
    (&[0xc7, 0xe7], false, Name::Bytes(b"\xe7\x7e\xe5")), // з~е
    (&[0xc8, 0xe8, 0x49, 0x69], false, Name::Bytes(b"\xe8")), // и
    (&[0xc9, 0xe9], false, Name::At(0x10065ee4)), // и кр^аткое
    (&[0xca, 0xea, 0x4b, 0x6b], false, Name::At(0x1001e190)), // ка
    (&[0xcb, 0xeb], false, Name::Bytes(b"\x7e\xe5\xeb")), // ~ел
    (&[0xcc, 0xec, 0x4d, 0x6d], false, Name::Bytes(b"\x7e\xe5\xec")), // ~ем
    (&[0xcd, 0xed, 0x4e, 0x6e], false, Name::Bytes(b"\x7e\xe5\xed")), // ~ен
    (&[0xce, 0xee, 0x4f, 0x6f], false, Name::Bytes(b"\xee")), // о
    (&[0xcf, 0xef, 0x50, 0x70], false, Name::Bytes(b"\xef\x7e\xe5")), // п~е
    (&[0xd0, 0xf0, 0x52, 0x72], false, Name::Bytes(b"\x7e\xe5\xf0")), // ~ер
    (&[0xd1, 0xf1, 0x53, 0x73], false, Name::Bytes(b"\x7e\xe5\xf1")), // ~ес
    (&[0xd2, 0xf2, 0x54, 0x74], false, Name::Bytes(b"\xf2\x7e\xe5")), // т~е
    (&[0xd3, 0xf3, 0x55, 0x75], false, Name::Bytes(b"\xf3")), // у
    (&[0xd4, 0xf4, 0x46, 0x66], false, Name::Bytes(b"\x7e\xe5\xf4")), // ~еф
    (&[0xd5, 0xf5], false, Name::At(0x1001c344)), // ха
    (&[0xd6, 0xf6, 0x43, 0x63], false, Name::Bytes(b"\xf6\x7e\xe5")), // ц~е
    (&[0xd7, 0xf7], false, Name::At(0x10065ee0)), // че
    (&[0xd8, 0xf8], false, Name::At(0x10065edc)), // ша
    (&[0xd9, 0xf9], false, Name::At(0x10065ed8)), // ща
    (&[0xda, 0xfa], false, Name::At(0x10065ec8)), // твёрдый зн^ак
    (&[0xdb, 0xfb], false, Name::Bytes(b"\xfb")), // ы
    (&[0xdc, 0xfc], false, Name::At(0x10065eb8)), // м^ягкий зн^ак
    (&[0xdd, 0xfd, 0x45, 0x65], false, Name::At(0x10065eb4)), // ~е
    (&[0xde, 0xfe], false, Name::Bytes(b"\xfe")), // ю
    (&[0xdf, 0xff], false, Name::Bytes(b"\xff")), // я
];

/// `LesNormalizzaAccento` (0x100013e0): `` ` ``, `’`, `'` and `´` become `'`.
pub(super) fn normalizza_accento(w: &mut Vec<u8>) {
    for c in w.iter_mut() {
        if matches!(*c, b'`' | 0x92 | b'\'' | 0xb4) {
            *c = b'\'';
        }
    }
}

/// `strcmp(a, b) == 0` for C strings.
fn eq(a: &[u8], b: &[u8]) -> bool {
    a == b
}

impl Russian {
    /// `LesGetSymbolName` (0x100014f0): the name of character `c`; the
    /// names of punctuation and most symbols only when `mode` is not 0.
    pub(super) fn symbol_name(&self, c: u8, mode: u8) -> Vec<u8> {
        for &(cs, guarded, name) in SYMBOLS {
            if cs.contains(&c) {
                if guarded && mode == 0 {
                    return Vec::new();
                }
                return match name {
                    Name::At(va) => self.s(va).to_vec(),
                    Name::Bytes(b) => b.to_vec(),
                };
            }
        }
        Vec::new()
    }

    /// 0x10002790: the lower-cased word (at most 0x3f letters) is one of
    /// the readable letter groups.
    fn readable_group(&self, w: &[u8]) -> bool {
        let low: Vec<u8> = w.iter().take(0x3f).map(|&c| self.lower(c)).collect();
        if low.first().copied().unwrap_or(0) == 0 {
            return false;
        }
        let (mut lo, mut hi) = (0i32, READABLE_COUNT - 1);
        while lo <= hi {
            let mid = (lo + hi) / 2;
            match low.as_slice().cmp(self.s(self.pe.u32(READABLE + 4 * mid as u32))) {
                std::cmp::Ordering::Less => hi = mid - 1,
                std::cmp::Ordering::Greater => lo = mid + 1,
                std::cmp::Ordering::Equal => return true,
            }
        }
        false
    }

    /// `LesPronunciabile` (0x10002c60 → 0x10002c30 → 0x10002880): never
    /// with the char argument set; a single letter unless it is a
    /// consonant (or `Ъ`/`Ь` by lower case) other than `ж в к с`; a
    /// readable letter group; otherwise a word with a vowel (`ь ъ` are
    /// not), readable when it has `ь ъ ы` or more than six letters, or its
    /// consonant/vowel shape is not one of six.
    pub(super) fn pronunciabile(&self, w: &[u8], flag: u8) -> bool {
        if flag != 0 {
            return false;
        }
        if w.len() == 1 {
            let lower = self.lower(w[0]);
            let cons = self.has(w[0], CONSONANT) || lower == 0xda || lower == 0xdc;
            return !(cons && !matches!(lower, 0xe6 | 0xe0 | 0xe2 | 0xe8 | 0xea | 0xee | 0xf1 | 0xf3 | 0xff));
        }
        if self.readable_group(w) {
            return true;
        }
        let mut shape = vec![b'^'];
        let (mut letters, mut vowels, mut soft) = (0usize, 0usize, false);
        for &c in w.iter().take(0x3f) {
            letters += 1;
            if !soft && self.s(SOFT).contains(&c) {
                soft = true;
            }
            if self.has(c, VOWEL) && c != 0xfc && c != 0xfa {
                vowels += 1;
                shape.push(b'V');
            } else if self.has(c, CONSONANT) {
                shape.push(b'C');
            }
        }
        shape.push(b'$');
        if vowels == 0 {
            return false;
        }
        if soft || letters > 6 {
            return true;
        }
        !UNREADABLE.iter().any(|&p| eq(&shape, self.s(p)))
    }

    /// `LesElaboraParola` (0x10003280): the records for the word token
    /// `toks[i]` (`¬` read as `-`). A token of one character between
    /// sentence boundaries is spelled unless `spell` is set; otherwise
    /// LesPronunciabile decides, and the word is read when it is
    /// readable, `spell` is set or the token comes from an expansion, and
    /// `mode` is 0.
    pub(super) fn elabora_parola(&self, toks: &mut [Tok], i: usize, _idx: usize, spell: u8, mode: u8) -> Vec<Word> {
        for c in toks[i].text.iter_mut() {
            if *c == 0xac {
                *c = b'-';
            }
        }
        let t = toks[i].text.clone();
        let kind = |k: usize| toks.get(k).map_or(7, |x| x.kind);
        let text0 = |k: usize| toks.get(k).and_then(|x| x.text.first().copied()).unwrap_or(0);
        let first = i == 0 || (kind(i - 1) == 6 && text0(i - 1) != b'"');
        let last = kind(i + 1) == 7 || (kind(i + 1) == 6 && text0(i + 1) != b'"') || (kind(i + 1) == 2 && kind(i + 2) == 6);
        let readable = if first && last && t.len() < 2 && spell == 0 { false } else { self.pronunciabile(&t, 0) };
        let mut out = Vec::new();
        if mode == 0 && (readable || spell != 0 || toks[i].expanded) {
            if !t.is_empty() {
                let mut low: Vec<u8> = t.iter().take(0x40).map(|&c| self.lower(c)).collect();
                if low.eq_ignore_ascii_case(b"xxversionxx") {
                    low = self.s(VERSION)[..self.s(VERSION).len().min(0x40)].to_vec();
                }
                self.word_records(&low, spell, &mut out);
            }
        } else {
            self.spelled(&t, 0, &mut out);
        }
        out
    }

    /// 0x10002f60: one word's records with its analyses.
    fn word_records(&self, w: &[u8], spell: u8, out: &mut Vec<Word>) {
        let an = self.analyse_word(w);
        if an.slots.is_empty() {
            // the first hyphen after the first letter
            if let Some(h) = (1..w.len()).find(|&k| w[k] == b'-') {
                if h + 1 < w.len() {
                    let (head, tail) = (&w[..h], &w[h..]);
                    self.word_records(head, spell, out);
                    if eq(tail, self.s(TO)) {
                        self.word_records(tail, spell, out);
                    } else {
                        self.word_records(&tail[1..], spell, out);
                    }
                    return;
                }
            } else if !self.pronunciabile(w, 0) && spell == 0 {
                self.spelled(w, 0, out);
                return;
            }
        }
        let mut r = word_record(&w[..w.len().min(0x3f)], 0x17, 200, 9, 0x0e, 0x15);
        Lex {
            an,
            pos: -1,
            ..Default::default()
        }
        .store(&mut r);
        out.push(r);
    }

    /// 0x10002640: `w` spelled, one record per character (class 0x15, sub
    /// 0x83, the last 0x84; boundary 8 when every letter is a vowel, else
    /// 7, and 8 on the last), each name with its stress marked on its last
    /// vowel unless it marks its own.
    pub(super) fn spelled(&self, w: &[u8], mode: u8, out: &mut Vec<Word>) {
        let mut bound = if w.iter().all(|&c| self.has(c, VOWEL)) { 8 } else { 7 };
        for (k, &c) in w.iter().enumerate() {
            let sub = if k + 1 == w.len() {
                bound = 8;
                0x84
            } else {
                0x83
            };
            let name = self.symbol_name(c, mode);
            let pos = les::vowel_at(self, &name, -1);
            let text = if !les::has_stress_mark(self, &name) && pos >= 0 { les::mark_stress(&name, pos, 0x40) } else { name };
            out.push(word_record(&text[..text.len().min(0x3f)], 0x15, sub, 9, 0x0e, bound));
        }
    }
}

/// 0x10002dd0: a record (`+0xc` 0xffff) with its class, subclass,
/// information, grade and boundary.
pub(super) fn word_record(text: &[u8], class: u8, sub: u8, info: u8, grade: u8, bound: u8) -> Word {
    let mut r = record(text.to_vec(), class, sub, info);
    r.f[0xc] = 0xff;
    r.f[0xd] = 0xff;
    r.f[GRADE] = grade;
    r.f[BOUND] = bound;
    r
}
