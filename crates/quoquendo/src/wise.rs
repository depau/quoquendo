//! Reading the Wise installers some Loquendo voices and the language
//! distributions ship in (an MSI holding one Microsoft cabinet) without
//! running them: the cabinet is found by its `MSCF` signature, its file
//! table read, and its MSZIP data inflated. Works on bytes, so it runs in a
//! browser too.

use miniz_oxide::inflate::core::{decompress, inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF, DecompressorOxide};
use miniz_oxide::inflate::TINFLStatus;

fn u16le(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?))
}

fn u32le(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

/// The installed name of a cabinet member: MSI file keys are the file name
/// plus a number (`LTTS7Jordi.dll1`, `Marcello.sde6`, `German.lde4`),
/// dropped here.
pub fn member_name(key: &str) -> String {
    const EXTENSIONS: [&str; 14] = [".sde", ".bin", ".vde", ".vcf", ".dll", ".gde", ".pdf", ".ico", ".lde", ".dbl", ".lex", ".rex", ".lcf", ".ycf"];
    let stem = key.trim_end_matches(|c: char| c.is_ascii_digit());
    let low = stem.to_ascii_lowercase();
    if stem.len() < key.len() && EXTENSIONS.iter().any(|e| low.ends_with(e)) {
        stem.to_string()
    } else {
        key.to_string()
    }
}

struct Member {
    name: String,
    size: usize,
    offset: usize,
    folder: usize,
}

struct Folder {
    first_block: usize,
    blocks: usize,
    compression: u16,
}

struct Cabinet<'a> {
    cab: &'a [u8],
    folders: Vec<Folder>,
    members: Vec<Member>,
    data_reserve: usize,
}

/// The cabinet starting at `at`, if a whole and sane one starts there.
fn cabinet(exe: &[u8], at: usize) -> Option<Cabinet<'_>> {
    let size = u32le(exe, at + 8)? as usize;
    let cab = exe.get(at..at.checked_add(size)?)?;
    let files_at = u32le(cab, 16)? as usize;
    let (folders, files, flags) = (u16le(cab, 26)? as usize, u16le(cab, 28)? as usize, u16le(cab, 30)?);
    let mut o = 36;
    let (mut header_reserve, mut folder_reserve, mut data_reserve) = (0, 0, 0);
    if flags & 4 != 0 {
        header_reserve = u16le(cab, o)? as usize;
        folder_reserve = *cab.get(o + 2)? as usize;
        data_reserve = *cab.get(o + 3)? as usize;
        o += 4;
    }
    o += header_reserve;
    // previous / next cabinet names: two strings each
    let skip_string = |o: usize| -> Option<usize> { Some(o + cab.get(o..)?.iter().position(|&c| c == 0)? + 1) };
    for bit in [1, 2] {
        if flags & bit != 0 {
            o = skip_string(skip_string(o)?)?;
        }
    }
    let mut fs = Vec::with_capacity(folders);
    for _ in 0..folders {
        fs.push(Folder {
            first_block: u32le(cab, o)? as usize,
            blocks: u16le(cab, o + 4)? as usize,
            compression: u16le(cab, o + 6)?,
        });
        o += 8 + folder_reserve;
    }
    let mut ms = Vec::with_capacity(files);
    let mut o = files_at;
    for _ in 0..files {
        let name_end = skip_string(o + 16)?;
        ms.push(Member {
            size: u32le(cab, o)? as usize,
            offset: u32le(cab, o + 4)? as usize,
            folder: u16le(cab, o + 8)? as usize,
            name: member_name(&String::from_utf8_lossy(&cab[o + 16..name_end - 1])),
        });
        o = name_end;
    }
    Some(Cabinet {
        cab,
        folders: fs,
        members: ms,
        data_reserve,
    })
}

/// A folder's data, uncompressed up to `need` bytes.
fn unpack_folder(c: &Cabinet, f: &Folder, need: usize) -> Result<Vec<u8>, String> {
    let short = || "the cabinet is cut short".to_string();
    let mut out: Vec<u8> = Vec::with_capacity(need);
    let mut o = f.first_block;
    for _ in 0..f.blocks {
        if out.len() >= need {
            break;
        }
        let packed = u16le(c.cab, o + 4).ok_or_else(short)? as usize;
        let unpacked = u16le(c.cab, o + 6).ok_or_else(short)? as usize;
        let at = o + 8 + c.data_reserve;
        let data = c.cab.get(at..at + packed).ok_or_else(short)?;
        o = at + packed;
        match f.compression & 0xf {
            0 => out.extend_from_slice(data),
            1 => {
                let deflate = data.strip_prefix(b"CK").ok_or("bad MSZIP block")?;
                // each block is its own deflate stream, but it may refer back
                // into the previous 32 KiB of output
                let base = out.len().saturating_sub(32 * 1024);
                let was = out.len();
                out.resize(was + unpacked, 0);
                let (status, _, written) = decompress(
                    &mut DecompressorOxide::new(),
                    deflate,
                    &mut out[base..],
                    was - base,
                    TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
                );
                if status != TINFLStatus::Done || written != unpacked {
                    return Err("bad MSZIP block".into());
                }
            }
            _ => return Err("unsupported CAB compression".into()),
        }
    }
    Ok(out)
}

/// Every file of the voice or language cabinet inside a Wise installer, by
/// its installed name (`member_name`), with its contents. The voice
/// installers hold one cabinet with a `.vcf`; the language distributions
/// one with `.lde`s instead.
pub fn files(exe: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let holds = |c: &Cabinet, ext: &str| c.members.iter().any(|m| m.name.to_ascii_lowercase().ends_with(ext));
    let signature = b"MSCF\0\0\0\0";
    let c = exe
        .windows(signature.len())
        .enumerate()
        .filter(|(_, w)| w == signature)
        .filter_map(|(at, _)| cabinet(exe, at))
        .find(|c| holds(c, ".vcf") || holds(c, ".lde"))
        .ok_or("no Loquendo cabinet in this installer")?;
    let mut out = Vec::with_capacity(c.members.len());
    for (i, f) in c.folders.iter().enumerate() {
        let mine: Vec<&Member> = c.members.iter().filter(|m| m.folder == i).collect();
        let need = mine.iter().map(|m| m.offset + m.size).max().unwrap_or(0);
        if need == 0 {
            continue;
        }
        let data = unpack_folder(&c, f, need)?;
        for m in mine {
            let bytes = data.get(m.offset..m.offset + m.size).ok_or("a file runs past its cabinet folder")?;
            out.push((m.name.clone(), bytes.to_vec()));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixed-Huffman deflate block (final) of `lits` then one back
    /// reference of length 10 at distance 20.
    fn fixed_block_with_match(lits: &[u8]) -> Vec<u8> {
        let mut bits: Vec<bool> = Vec::new();
        let put_lsb = |v: u32, n: u32, bits: &mut Vec<bool>| (0..n).for_each(|i| bits.push(v >> i & 1 != 0));
        let put_msb = |v: u32, n: u32, bits: &mut Vec<bool>| (0..n).rev().for_each(|i| bits.push(v >> i & 1 != 0));
        put_lsb(1, 1, &mut bits); // final
        put_lsb(1, 2, &mut bits); // fixed Huffman
        for &l in lits {
            put_msb(0x30 + l as u32, 8, &mut bits);
        }
        put_msb(264 - 256, 7, &mut bits); // length 10
        put_msb(8, 5, &mut bits); // distance 17..24
        put_lsb(3, 3, &mut bits); // + 3 = 20
        put_msb(0, 7, &mut bits); // end of block
        bits.chunks(8).map(|c| c.iter().enumerate().fold(0u8, |b, (i, &x)| b | (x as u8) << i)).collect()
    }

    /// Two MSZIP blocks, the second referring back into the first: both
    /// members come out whole and named as installed.
    #[test]
    fn reads_a_two_block_mszip_cabinet() {
        let first = b"Loquendo voice cabinet test: 0123456789 abcdefghij".to_vec();
        let block1 = miniz_oxide::deflate::compress_to_vec(&first, 6);
        let block2 = fixed_block_with_match(b"xy");
        let mut whole = first.clone();
        whole.extend_from_slice(b"xy");
        for _ in 0..10 {
            whole.push(whole[whole.len() - 20]);
        }
        let names: [&[u8]; 2] = [b"Test.vcf3\0", b"LTTS7Test.dll12\0"];
        let split = 30;
        let header = 36 + 8;
        let files_len: usize = names.iter().map(|n| 16 + n.len()).sum();
        let data_at = header + files_len;
        let mut data = Vec::new();
        for (b, n) in [(&block1, first.len()), (&block2, whole.len() - first.len())] {
            data.extend_from_slice(&0u32.to_le_bytes());
            data.extend_from_slice(&((b.len() + 2) as u16).to_le_bytes());
            data.extend_from_slice(&(n as u16).to_le_bytes());
            data.extend_from_slice(b"CK");
            data.extend_from_slice(b);
        }
        let size = data_at + data.len();
        let mut cab = Vec::new();
        cab.extend_from_slice(b"MSCF\0\0\0\0");
        cab.extend_from_slice(&(size as u32).to_le_bytes());
        cab.extend_from_slice(&0u32.to_le_bytes());
        cab.extend_from_slice(&(header as u32).to_le_bytes());
        cab.extend_from_slice(&0u32.to_le_bytes());
        cab.extend_from_slice(&[3, 1]);
        for v in [1u16, 2, 0, 0, 0] {
            cab.extend_from_slice(&v.to_le_bytes());
        }
        cab.extend_from_slice(&(data_at as u32).to_le_bytes());
        cab.extend_from_slice(&2u16.to_le_bytes());
        cab.extend_from_slice(&1u16.to_le_bytes());
        for (name, (off, len)) in names.iter().zip([(0, split), (split, whole.len() - split)]) {
            cab.extend_from_slice(&(len as u32).to_le_bytes());
            cab.extend_from_slice(&(off as u32).to_le_bytes());
            cab.extend_from_slice(&[0; 8]);
            cab.extend_from_slice(name);
        }
        cab.extend_from_slice(&data);
        let mut exe = b"MZ wise stub MSCF\0\0\0\0 not a cabinet".to_vec();
        exe.extend_from_slice(&cab);
        exe.extend_from_slice(b"trailer");
        let got = files(&exe).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0], ("Test.vcf".to_string(), whole[..split].to_vec()));
        assert_eq!(got[1], ("LTTS7Test.dll".to_string(), whole[split..].to_vec()));
    }

    #[test]
    fn names_as_installed() {
        assert_eq!(member_name("LTTS7Jordi.dll1"), "LTTS7Jordi.dll");
        assert_eq!(member_name("Alan.44000.LTTS7CodecMsx.58300.10.bin1"), "Alan.44000.LTTS7CodecMsx.58300.10.bin");
        assert_eq!(member_name("Marcello.44000.LTTS7CodecMsx.58300.10.bin"), "Marcello.44000.LTTS7CodecMsx.58300.10.bin");
        assert_eq!(member_name("Loquendo_TTS_7_Installation_Manual.pdf7"), "Loquendo_TTS_7_Installation_Manual.pdf");
        assert_eq!(member_name("German.dbl5"), "German.dbl");
        assert_eq!(member_name("Logo.ico"), "Logo.ico");
    }
}
