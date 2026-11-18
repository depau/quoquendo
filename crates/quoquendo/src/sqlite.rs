//! Just enough of SQLite 3 to read every row of one table from a database
//! file's bytes: the language dictionaries (`<Language>.dbl`) are SQLite
//! files. Read-only, no SQL, no dependencies (so it builds for any target).

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Int(i64),
    Real(f64),
    Text(Vec<u8>),
    Blob(Vec<u8>),
}

impl Value {
    /// Text as Latin-1 (what the Loquendo dictionaries are written in).
    pub fn text(&self) -> Option<String> {
        match self {
            Value::Text(b) => Some(b.iter().map(|&c| c as char).collect()),
            _ => None,
        }
    }
}

struct Db<'a> {
    data: &'a [u8],
    page_size: usize,
    usable: usize,
}

fn varint(b: &[u8], mut at: usize) -> Option<(u64, usize)> {
    let mut v: u64 = 0;
    for i in 0..9 {
        let c = *b.get(at)?;
        at += 1;
        if i == 8 {
            return Some(((v << 8) | c as u64, at));
        }
        v = (v << 7) | (c & 0x7f) as u64;
        if c & 0x80 == 0 {
            return Some((v, at));
        }
    }
    None
}

impl<'a> Db<'a> {
    fn page(&self, n: u32) -> Option<&'a [u8]> {
        let start = (n as usize).checked_sub(1)? * self.page_size;
        self.data.get(start..start + self.page_size)
    }

    /// The whole payload of a cell, following overflow pages.
    fn payload(&self, page: &[u8], at: usize, len: usize) -> Option<Vec<u8>> {
        let u = self.usable;
        let x = u - 35;
        if len <= x {
            return page.get(at..at + len).map(|s| s.to_vec());
        }
        let m = ((u - 12) * 32 / 255) - 23;
        let k = m + (len - m) % (u - 4);
        let local = if k <= x { k } else { m };
        let mut out = page.get(at..at + local)?.to_vec();
        let mut next = u32::from_be_bytes(page.get(at + local..at + local + 4)?.try_into().ok()?);
        while out.len() < len && next != 0 {
            let p = self.page(next)?;
            next = u32::from_be_bytes(p.get(0..4)?.try_into().ok()?);
            let take = (len - out.len()).min(u - 4);
            out.extend_from_slice(p.get(4..4 + take)?);
        }
        Some(out)
    }

    fn record(rec: &[u8]) -> Option<Vec<Value>> {
        let (hlen, mut h) = varint(rec, 0)?;
        let mut body = hlen as usize;
        let mut out = Vec::new();
        while h < hlen as usize {
            let (t, nh) = varint(rec, h)?;
            h = nh;
            let int = |n: usize, body: usize| -> Option<i64> {
                let s = rec.get(body..body + n)?;
                let mut v: i64 = if s.first().map(|b| b & 0x80 != 0).unwrap_or(false) { -1 } else { 0 };
                for &b in s {
                    v = (v << 8) | b as i64;
                }
                Some(v)
            };
            let (v, n) = match t {
                0 => (Value::Null, 0),
                1 => (Value::Int(int(1, body)?), 1),
                2 => (Value::Int(int(2, body)?), 2),
                3 => (Value::Int(int(3, body)?), 3),
                4 => (Value::Int(int(4, body)?), 4),
                5 => (Value::Int(int(6, body)?), 6),
                6 => (Value::Int(int(8, body)?), 8),
                7 => (Value::Real(f64::from_bits(int(8, body)? as u64)), 8),
                8 => (Value::Int(0), 0),
                9 => (Value::Int(1), 0),
                t if t >= 12 && t % 2 == 0 => {
                    let n = ((t - 12) / 2) as usize;
                    (Value::Blob(rec.get(body..body + n)?.to_vec()), n)
                }
                t if t >= 13 => {
                    let n = ((t - 13) / 2) as usize;
                    (Value::Text(rec.get(body..body + n)?.to_vec()), n)
                }
                _ => (Value::Null, 0),
            };
            body += n;
            out.push(v);
        }
        Some(out)
    }

    /// Every row of the table b-tree rooted at `root`, in rowid order.
    fn rows(&self, root: u32, out: &mut Vec<Vec<Value>>, depth: usize) -> Option<()> {
        if depth > 64 {
            return None;
        }
        let page = self.page(root)?;
        let hdr = if root == 1 { 100 } else { 0 };
        let kind = *page.get(hdr)?;
        let cells = u16::from_be_bytes(page.get(hdr + 3..hdr + 5)?.try_into().ok()?) as usize;
        let ptrs = hdr + if kind == 0x05 { 12 } else { 8 };
        for c in 0..cells {
            let at = u16::from_be_bytes(page.get(ptrs + 2 * c..ptrs + 2 * c + 2)?.try_into().ok()?) as usize;
            match kind {
                0x05 => {
                    let child = u32::from_be_bytes(page.get(at..at + 4)?.try_into().ok()?);
                    self.rows(child, out, depth + 1)?;
                }
                0x0d => {
                    let (len, at) = varint(page, at)?;
                    let (_rowid, at) = varint(page, at)?;
                    let rec = self.payload(page, at, len as usize)?;
                    out.push(Db::record(&rec)?);
                }
                _ => return None,
            }
        }
        if kind == 0x05 {
            let right = u32::from_be_bytes(page.get(hdr + 8..hdr + 12)?.try_into().ok()?);
            self.rows(right, out, depth + 1)?;
        }
        Some(())
    }
}

/// Every row of table `name` (case-insensitive) of an SQLite database.
pub fn read_table(data: &[u8], name: &str) -> Result<Vec<Vec<Value>>, String> {
    if data.get(0..16) != Some(b"SQLite format 3\0") {
        return Err("not an SQLite database".into());
    }
    let ps = u16::from_be_bytes([data[16], data[17]]) as usize;
    let page_size = if ps == 1 { 65536 } else { ps };
    let usable = page_size - data[20] as usize;
    let db = Db { data, page_size, usable };
    let mut schema = Vec::new();
    db.rows(1, &mut schema, 0).ok_or("damaged database schema")?;
    // sqlite_schema rows: type, name, tbl_name, rootpage, sql
    let root = schema
        .iter()
        .find(|r| {
            r.first().and_then(Value::text).as_deref() == Some("table") && r.get(1).and_then(Value::text).map(|n| n.eq_ignore_ascii_case(name)).unwrap_or(false)
        })
        .and_then(|r| match r.get(3) {
            Some(Value::Int(p)) => Some(*p as u32),
            _ => None,
        })
        .ok_or_else(|| format!("no table {name}"))?;
    let mut rows = Vec::new();
    db.rows(root, &mut rows, 0).ok_or("damaged table")?;
    Ok(rows)
}
