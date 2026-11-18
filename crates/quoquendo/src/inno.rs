//! Reading Loquendo's voice installers (Inno Setup 5.2.3) without running
//! them or any other program: the loader's offset table, the two LZMA
//! header blocks, the file entries and their places in the compressed
//! chunks, and Inno's x86 call filter undone for programs and libraries.
//! Works on bytes, so it runs in a browser too.

fn u32le(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

fn u64le(b: &[u8], o: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(o..o + 8)?.try_into().ok()?))
}

fn lzma(data: &[u8], size: Option<u64>) -> Result<Vec<u8>, String> {
    use std::io::Write;
    let options = lzma_rs::decompress::Options {
        unpacked_size: lzma_rs::decompress::UnpackedSize::UseProvided(size),
        memlimit: None,
        allow_incomplete: true,
    };
    // Inno writes no end marker: the stream simply stops
    let mut s = lzma_rs::decompress::Stream::new_with_options(&options, Vec::new());
    // it takes nothing more once its stream has ended
    let mut rest = data;
    while !rest.is_empty() {
        match s.write(rest) {
            Ok(0) => break,
            Ok(n) => rest = &rest[n..],
            Err(e) if e.kind() == std::io::ErrorKind::WriteZero => break,
            Err(e) => return Err(format!("LZMA: {e}")),
        }
    }
    let mut out = s.finish().map_err(|e| format!("LZMA: {e:?}"))?;
    if let Some(n) = size {
        out.truncate(n as usize);
    }
    Ok(out)
}

/// One header block: CRC, stored size, compressed flag, then the data in
/// 4 KiB pieces each after its own CRC.
fn block(exe: &[u8], pos: &mut usize) -> Result<Vec<u8>, String> {
    let bad = || "the installer's header is cut short".to_string();
    let stored = u32le(exe, *pos + 4).ok_or_else(bad)? as usize;
    let compressed = *exe.get(*pos + 8).ok_or_else(bad)? != 0;
    *pos += 9;
    let raw = exe.get(*pos..*pos + stored).ok_or_else(bad)?;
    *pos += stored;
    let data: Vec<u8> = raw.chunks(4096 + 4).flat_map(|p| p.get(4..).unwrap_or(&[]).iter().copied()).collect();
    if compressed {
        lzma(&data, None)
    } else {
        Ok(data)
    }
}

/// Undo Inno Setup 5.2's CALL/JMP filter: in each 64 KiB block, the 3 low
/// bytes of an E8/E9 operand whose high byte is 00/FF were made absolute.
pub fn unfilter_calls(data: &mut [u8]) {
    for (b, blk) in data.chunks_mut(0x10000).enumerate() {
        let base = b * 0x10000;
        let size = blk.len().saturating_sub(4);
        let mut i = 0;
        while i < size {
            if blk[i] == 0xe8 || blk[i] == 0xe9 {
                i += 1;
                if blk[i + 3] == 0 || blk[i + 3] == 0xff {
                    let addr = ((base + i + 4) & 0xff_ffff) as u32;
                    let rel = (blk[i] as u32 | (blk[i + 1] as u32) << 8 | (blk[i + 2] as u32) << 16).wrapping_sub(addr);
                    blk[i] = rel as u8;
                    blk[i + 1] = (rel >> 8) as u8;
                    blk[i + 2] = (rel >> 16) as u8;
                }
                i += 4;
            } else {
                i += 1;
            }
        }
    }
}

struct Location {
    start: u64,
    suboffset: u64,
    size: u64,
    compressed_size: u64,
    calls_filtered: bool,
    compressed: bool,
}

/// Bytes of a file entry after its strings and versions, for 5.2.3:
/// location, attributes, external size, permissions, options, type.
const FILE_TAIL: usize = 4 + 4 + 8 + 2 + 4 + 1;

/// Every file in an Inno Setup 5.2.3 installer whose path (`pf/Loquendo/…`,
/// the folder constant without its braces, as innoextract writes it) `want`
/// accepts, with its contents. Later entries for the same path replace
/// earlier ones, as installing would.
pub fn files(exe: &[u8], want: impl Fn(&str) -> bool) -> Result<Vec<(String, Vec<u8>)>, String> {
    // the loader's offset table: magic, revision, total size, exe offset,
    // exe size, exe CRC, header offset, data offset
    let magic = b"rDlPtS\xcd\xe6\xd7{\x0b*";
    let (header, data_offset) = exe
        .windows(magic.len())
        .enumerate()
        .filter(|(_, w)| w == magic)
        .filter_map(|(p, _)| Some((u32le(exe, p + 32)? as usize, u32le(exe, p + 36)? as usize)))
        .find(|&(h, _)| exe.get(h..h + 22) == Some(b"Inno Setup Setup Data ".as_slice()))
        .ok_or("not an Inno Setup 5 installer")?;
    let version = String::from_utf8_lossy(&exe[header..header + 64]).trim_end_matches('\0').to_string();
    if !version.contains("(5.2.") {
        return Err(format!("{version}: only Inno Setup 5.2 installers are read"));
    }
    let mut pos = header + 64;
    let entries = block(exe, &mut pos)?;
    let locations_raw = block(exe, &mut pos)?;
    // file location entries, 70 bytes each
    let locations: Vec<Location> = locations_raw
        .chunks_exact(70)
        .map(|r| {
            let flags = u16::from_le_bytes([r[68], r[69]]);
            Location {
                start: u32le(r, 8).unwrap() as u64,
                suboffset: u64le(r, 12).unwrap(),
                size: u64le(r, 20).unwrap(),
                compressed_size: u64le(r, 28).unwrap(),
                calls_filtered: flags & 1 << 4 != 0,
                compressed: flags & 1 << 7 != 0,
            }
        })
        .collect();
    // file entries: source (empty), destination, font, six condition
    // strings, two Windows versions, then the tail; they follow each other
    let string = |o: usize| -> Option<(usize, &[u8])> {
        let n = u32le(&entries, o)? as usize;
        (n < 4096).then_some(())?;
        Some((o + 4 + n, entries.get(o + 4..o + 4 + n)?))
    };
    let entry = |o: usize| -> Option<(usize, String, usize)> {
        let (o, source) = string(o)?;
        let (mut o, dest) = string(o)?;
        if !source.is_empty() || dest.first() != Some(&b'{') || !dest.contains(&b'}') {
            return None;
        }
        for _ in 0..7 {
            o = string(o)?.0;
        }
        o += 20;
        let location = u32le(&entries, o)? as usize;
        let external = u64le(&entries, o + 8)?;
        (location < locations.len() && external == 0).then_some(())?;
        let path: String = dest
            .iter()
            .map(|&c| if c == b'\\' { '/' } else { c as char })
            .filter(|&c| c != '{' && c != '}')
            .collect();
        Some((o + FILE_TAIL, path, location))
    };
    // the first entry that another one follows (a lone match can be a folder entry)
    let first = (0..entries.len())
        .find(|&o| entry(o).and_then(|(next, ..)| entry(next)).is_some())
        .ok_or("the installer lists no files")?;
    let mut picked: Vec<(String, usize)> = Vec::new();
    let mut o = first;
    while let Some((next, path, location)) = entry(o) {
        if want(&path) {
            picked.retain(|(p, _)| *p != path);
            picked.push((path, location));
        }
        o = next;
    }
    // unpack each chunk once, up to the last byte a wanted file needs
    let mut chunks: std::collections::BTreeMap<u64, Vec<u8>> = std::collections::BTreeMap::new();
    for (_, l) in &picked {
        let l = &locations[*l];
        let end = l.suboffset + l.size;
        let need = chunks.get(&l.start).map(|c| (c.len() as u64) < end).unwrap_or(true);
        if !need {
            continue;
        }
        let at = data_offset + l.start as usize;
        if exe.get(at..at + 4) != Some(b"zlb\x1a".as_slice()) {
            return Err("a compressed chunk is not where the installer says".into());
        }
        let raw = exe.get(at + 4..at + 4 + l.compressed_size as usize).ok_or("the installer is cut short")?;
        let mut need_end = end;
        for (_, m) in &picked {
            let m = &locations[*m];
            if m.start == l.start {
                need_end = need_end.max(m.suboffset + m.size);
            }
        }
        let data = if l.compressed { lzma(raw, Some(need_end))? } else { raw.to_vec() };
        chunks.insert(l.start, data);
    }
    picked
        .into_iter()
        .map(|(path, l)| {
            let l = &locations[l];
            let chunk = &chunks[&l.start];
            let mut data = chunk
                .get(l.suboffset as usize..(l.suboffset + l.size) as usize)
                .ok_or("a file runs past its chunk")?
                .to_vec();
            if l.calls_filtered {
                unfilter_calls(&mut data);
            }
            Ok((path, data))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    /// QQ_INNO=<installer> QQ_INNO_REF=<innoextract's folder for it>: every
    /// file we read is byte for byte what innoextract wrote.
    #[test]
    fn reads_like_innoextract() {
        let (Some(exe), Some(reference)) = (std::env::var_os("QQ_INNO"), std::env::var_os("QQ_INNO_REF")) else {
            return;
        };
        let exe = std::fs::read(exe).unwrap();
        let files = super::files(&exe, |_| true).unwrap();
        assert!(!files.is_empty());
        let mut same = 0;
        for (path, data) in &files {
            let want = std::path::Path::new(&reference).join(path);
            let Ok(want) = std::fs::read(&want) else { continue };
            assert!(want == *data, "{path} differs ({} vs {} bytes)", data.len(), want.len());
            same += 1;
        }
        eprintln!("{same} of {} files checked", files.len());
        assert!(same > files.len() / 2);
    }
}
