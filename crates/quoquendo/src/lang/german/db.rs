//! The module's dictionary tables (ELQLanguageLoadReadOnlyData 0x10007720)
//! and their lookup, LTTS7Util.dll's `ELQDBTableFindCaseSensitive`
//! (0x10005f70).

use crate::sqlite::Value;

/// One query's result: rows of columns, in the database's row order (the
/// queries have no ORDER BY; `German.dbl` is stored sorted by `grafemica`,
/// which the lookup's binary search relies on). `None` is SQL NULL.
#[derive(Default)]
pub struct Table {
    pub rows: Vec<Vec<Option<Vec<u8>>>>,
}

/// `lessico`'s columns: grafemica, fonetica, gradoaccentuale, grammaticale,
/// gennum, tempomodo, abbreviazione, altreinfo.
fn col(r: &[Value], i: usize) -> Option<Vec<u8>> {
    match r.get(i) {
        Some(Value::Text(b)) => Some(b.clone()),
        _ => None,
    }
}

impl Table {
    /// `SELECT grafemica, gradoaccentuale, grammaticale, tempomodo, gennum
    /// FROM lessico WHERE grammaticale!="" AND gradoaccentuale!=""` (NULLs
    /// fail the test)
    pub fn grammar(rows: &[Vec<Value>]) -> Table {
        let set = |v: &Option<Vec<u8>>| v.as_ref().is_some_and(|b| !b.is_empty());
        Table {
            rows: rows
                .iter()
                .filter(|r| set(&col(r, 3)) && set(&col(r, 2)))
                .map(|r| vec![col(r, 0), col(r, 2), col(r, 3), col(r, 5), col(r, 4)])
                .collect(),
        }
    }

    /// `SELECT grafemica, fonetica FROM lessico`
    pub fn phonetic(rows: &[Vec<Value>]) -> Table {
        Table { rows: rows.iter().map(|r| vec![col(r, 0), col(r, 1)]).collect() }
    }

    /// `SELECT grafemica,abbreviazione FROM lessico WHERE abbreviazione!=""
    /// AND altreinfo=="" ORDER BY abbreviazione COLLATE NOCASE DESC` (data
    /// +0x10): SQLite's sort is stable, NOCASE folds ASCII only.
    pub fn abbreviations(rows: &[Vec<Value>]) -> Table {
        let fold = |b: &[u8]| b.iter().map(u8::to_ascii_lowercase).collect::<Vec<u8>>();
        let mut t: Vec<Vec<Option<Vec<u8>>>> = rows
            .iter()
            .filter(|r| col(r, 6).is_some_and(|a| !a.is_empty()) && col(r, 7).is_some_and(|a| a.is_empty()))
            .map(|r| vec![col(r, 0), col(r, 6)])
            .collect();
        t.sort_by(|a, b| fold(b[1].as_deref().unwrap_or(b"")).cmp(&fold(a[1].as_deref().unwrap_or(b""))));
        Table { rows: t }
    }

    /// `ELQDBTableFindCaseSensitive` on the first column: a binary search
    /// (byte `strcmp`), then back to the first of the equal rows. The number
    /// of equal rows and the first one's index; (0, _) when none.
    pub fn find(&self, key: &[u8]) -> (usize, usize) {
        let k = |i: usize| self.rows[i][0].as_deref().unwrap_or(b"");
        let (mut lo, mut hi) = (0i64, self.rows.len() as i64 - 1);
        while lo <= hi {
            let mid = (lo + hi) / 2;
            match key.cmp(k(mid as usize)) {
                std::cmp::Ordering::Less => hi = mid - 1,
                std::cmp::Ordering::Greater => lo = mid + 1,
                std::cmp::Ordering::Equal => {
                    let mut first = mid as usize;
                    while first > 0 && k(first - 1) == key {
                        first -= 1;
                    }
                    let mut n = mid as usize + 1 - first;
                    while first + n < self.rows.len() && k(first + n) == key {
                        n += 1;
                    }
                    return (n, first);
                }
            }
        }
        (0, 0)
    }

    /// Column `c` of row `r` (`None` for NULL).
    pub fn get(&self, r: usize, c: usize) -> Option<&[u8]> {
        self.rows.get(r)?.get(c)?.as_deref()
    }
}
