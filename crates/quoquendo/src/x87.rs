//! The floating-point environment the codec DLLs run in: the x87 FPU as
//! LTTS7Codec210.dll and LTTS7CodecMsx.dll use it, and the MSVC C runtime
//! math they call (both link it statically). MECHANICS.md §2d.
//!
//! - **Precision control 53 bits.** The x87 control word is 0x027f at every
//!   decoder request (oracle/loqref.c `LOQ_TRACE_DECODE` logs it), so each
//!   `fadd`/`fsub`/`fmul`/`fdiv` rounds to a double's 53-bit mantissa. With
//!   the decoder's magnitudes (nothing near a double's exponent limits) that
//!   is exactly IEEE double arithmetic, and the `fstp dword` after each
//!   statement is `as f32`: the port computes in `f64` and needs no 80-bit
//!   arithmetic.
//! - **sqrt** (`_CIsqrt`, Codec210 0x100105c0, CodecMsx 0x10005ed0) is
//!   `fsqrt` under the caller's precision control: the correctly rounded
//!   double square root, which `f64::sqrt` is on every platform.
//! - **exp** (`_CIexp`, Codec210 0x10011aec, CodecMsx 0x10006dac) takes the
//!   runtime's SSE2 routine when the processor has SSE2 (flag 0x100300cc /
//!   0x1001c184, set at load by 0x10015195 / 0x10007fd9) and every exception
//!   is masked in both MXCSR and the x87 control word (MXCSR 0x1fa0, control
//!   word 0x027f): the x87 argument rounded to a double, then a 64-entry
//!   table and a degree-5 polynomial in double arithmetic (Codec210
//!   0x100175ee, CodecMsx 0x1000b1ce, the same code and constants). [`exp`]
//!   is that routine operation by operation, so every platform gets the
//!   DLL's bits whatever its own libm returns.

/// 64/ln 2 (0x10027010).
const INV_LN2_64: f64 = f64::from_bits(0x4057_1547_652b_82fe);
/// 1.5·2⁵²: adding it leaves round(x·64/ln 2) in the low mantissa bits (0x10026ff0).
const SHIFTER: f64 = f64::from_bits(0x4338_0000_0000_0000);
/// ln 2/64 in two parts (0x10027020, 0x10027030).
const LN2_64_HI: f64 = f64::from_bits(0x3f86_2e42_fefa_0000);
const LN2_64_LO: f64 = f64::from_bits(0x3d1c_f79a_bc9e_3b3a);
/// Polynomial coefficients, two lanes each: ≈1/120, 1/6 (0x10027040) and
/// ≈1/24, 1/2 (0x10027050).
const C4: [f64; 2] = [f64::from_bits(0x3f81_1074_b1d1_08e5), f64::from_bits(0x3fc5_5555_5566_a45a)];
const C5: [f64; 2] = [f64::from_bits(0x3fa5_5557_26ec_ed80), f64::from_bits(0x3fdf_ffff_ffff_e17b)];

/// 2^(j/64), j = 0..63 (0x10027060): a relative correction, then the
/// mantissa bits the exponent is ORed into.
#[rustfmt::skip]
static TABLE: [u64; 128] = [
    0x0000000000000000, 0x0000000000000000, 0x3cad7bbf0e03754d, 0x00002c9a3e778060,
    0x3c8cd2523567f613, 0x000059b0d3158574, 0x3c60f74e61e6c861, 0x0000874518759bc8,
    0x3c979aa65d837b6c, 0x0000b5586cf9890f, 0x3c3ebe3d702f9cd1, 0x0000e3ec32d3d1a2,
    0x3ca3516e1e63bcd8, 0x00011301d0125b50, 0x3ca4c55426f0387b, 0x0001429aaea92ddf,
    0x3ca9515362523fb6, 0x000172b83c7d517a, 0x3c8b898c3f1353bf, 0x0001a35beb6fcb75,
    0x3c9aecf73e3a2f5f, 0x0001d4873168b9aa, 0x3c8a6f4144a6c38d, 0x0002063b88628cd6,
    0x3c968efde3a8a894, 0x0002387a6e756238, 0x3c80472b981fe7f2, 0x00026b4565e27cdd,
    0x3c82f7e16d09ab31, 0x00029e9df51fdee1, 0x3c8b3782720c0ab3, 0x0002d285a6e4030b,
    0x3c834d754db0abb6, 0x000306fe0a31b715, 0x3c8fdd395dd3f84a, 0x00033c08b26416ff,
    0x3ca12f8ccc187d29, 0x000371a7373aa9ca, 0x3ca7d229738b5e8b, 0x0003a7db34e59ff6,
    0x3c859f48a72a4c6d, 0x0003dea64c123422, 0x3ca8b846259d9205, 0x0004160a21f72e29,
    0x3c4363ed60c2ac12, 0x00044e086061892d, 0x3c6ecce1daa10379, 0x000486a2b5c13cd0,
    0x3c7690cebb7aafb0, 0x0004bfdad5362a27, 0x3ca083cc9b282a09, 0x0004f9b2769d2ca6,
    0x3ca509b0c1aae707, 0x0005342b569d4f81, 0x3c93350518fdd78e, 0x00056f4736b527da,
    0x3c9063e1e21c5409, 0x0005ab07dd485429, 0x3c9432e62b64c035, 0x0005e76f15ad2148,
    0x3ca0128499f08c0a, 0x0006247eb03a5584, 0x3c99f0870073dc06, 0x0006623882552224,
    0x3c998d4d0da05571, 0x0006a09e667f3bcc, 0x3ca52bb986ce4786, 0x0006dfb23c651a2e,
    0x3ca32092206f0dab, 0x00071f75e8ec5f73, 0x3ca061228e17a7a6, 0x00075feb564267c8,
    0x3ca244ac461e9f86, 0x0007a11473eb0186, 0x3c65ebe1abd66c55, 0x0007e2f336cf4e62,
    0x3c96fe9fbbff67d0, 0x00082589994cce12, 0x3c951f1414c801df, 0x000868d99b4492ec,
    0x3c8db72fc1f0eab4, 0x0008ace5422aa0db, 0x3c7bf68359f35f44, 0x0008f1ae99157736,
    0x3ca360ba9c06283c, 0x00093737b0cdc5e4, 0x3c95e8d120f962aa, 0x00097d829fde4e4f,
    0x3c71affc2b91ce27, 0x0009c49182a3f090, 0x3c9b6d34589a2ebd, 0x000a0c667b5de564,
    0x3c95277c9ab89880, 0x000a5503b23e255c, 0x3c8469846e735ab3, 0x000a9e6b5579fdbf,
    0x3c8c1a7792cb3387, 0x000ae89f995ad3ad, 0x3ca22466dc2d1d96, 0x000b33a2b84f15fa,
    0x3ca1112eb19505ae, 0x000b7f76f2fb5e46, 0x3c74ffd70a5fddcd, 0x000bcc1e904bc1d2,
    0x3c736eae30af0cb3, 0x000c199bdd85529c, 0x3c84e08fd10959ac, 0x000c67f12e57d14b,
    0x3c676b2c6c921968, 0x000cb720dcef9069, 0x3c93700936df99b3, 0x000d072d4a07897b,
    0x3c74a385a63d07a7, 0x000d5818dcfba487, 0x3c8e5a50d5c192ac, 0x000da9e603db3285,
    0x3c98bb731c4a9792, 0x000dfc97337b9b5e, 0x3c74b604603a88d3, 0x000e502ee78b3ff6,
    0x3c916f2792094926, 0x000ea4afa2a490d9, 0x3c8ec3bc41aa2008, 0x000efa1bee615a27,
    0x3c8a64a931d185ee, 0x000f50765b6e4540, 0x3c77893b4d91cd9d, 0x000fa7c1819e90d8,
];

/// `exp` as the codec DLLs compute it (see the module notes): `x` is the
/// double the x87 argument rounds to.
///
/// Inputs from 2^-54 up to the scale limit (|x| < 619, every input the
/// decoders have: they lie in [−1.25, 8.86]) follow the DLL's fast path
/// exactly, smaller ones give `1 + x` as it does. Beyond 619 the DLL
/// finishes on the x87 at 64-bit precision and reports range errors; that
/// path is not ported and the platform's `exp` stands in.
pub fn exp(x: f64) -> f64 {
    let hi = (x.to_bits() >> 48) as u32 & 0x7fff;
    if !(0x3c90..=0x408f).contains(&hi) {
        // 0x1001785e: tiny arguments
        if (x.to_bits() >> 32) as u32 & 0x7fff_ffff < 0x4090_0000 {
            return x + 1.0;
        }
        return x.exp();
    }
    let t = x * INV_LN2_64 + SHIFTER;
    let n = t - SHIFTER;
    let r = x - n * LN2_64_HI - n * LN2_64_LO;
    let k = t.to_bits() as u32 as i32;
    let (j, m) = ((k & 63) as usize, k >> 6);
    if !(-894..=1022).contains(&m) {
        return x.exp();
    }
    let r2 = r * r;
    let p_lo = C5[0] + C4[0] * r;
    let p_hi = C5[1] + C4[1] * r;
    let r4 = r2 * r2;
    let u = r + f64::from_bits(TABLE[2 * j]) + r4 * p_lo;
    let v = r2 * p_hi + u;
    let scale = f64::from_bits(TABLE[2 * j + 1] | ((m + 1023) as u64) << 52);
    v * scale + scale
}

/// `sqrt` as the codec DLLs compute it: `fsqrt` at 53-bit precision.
pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

#[cfg(test)]
mod tests {
    /// The DLL's own results (its `_CIexp`, LTTS7Codec210.dll 0x10011aec,
    /// called under Wine) for every argument the decoders pass.
    #[test]
    fn exp_like_the_codec_dlls() {
        for (x, want) in DLL_EXP {
            assert_eq!(super::exp(f64::from_bits(x)).to_bits(), want, "exp({})", f64::from_bits(x));
        }
    }

    /// (argument, result) bits: q/3.5 for q = 0..31, q·0.27027027027027023 −
    /// 0.15556 for q = 0..15, (q − 10)·0.125 for q = 0..31.
    #[rustfmt::skip]
    const DLL_EXP: [(u64, u64); 80] = [
        (0x0000000000000000, 0x3ff0000000000000),
        (0x3fd2492492492492, 0x3ff54a98df86cb6b),
        (0x3fe2492492492492, 0x3ffc552d168a1807),
        (0x3feb6db6db6db6db, 0x4002d9f1e98dd529),
        (0x3ff2492492492492, 0x400915f19f38ef20),
        (0x3ff6db6db6db6db7, 0x4010b0e126b3dc0d),
        (0x3ffb6db6db6db6db, 0x401635f90da7dff4),
        (0x4000000000000000, 0x401d8e64b8d4ddae),
        (0x4002492492492492, 0x4023aa58950a59bd),
        (0x4004924924924925, 0x402a2b4402544cd4),
        (0x4006db6db6db6db7, 0x40316965b31b7de4),
        (0x4009249249249249, 0x40372b834df64368),
        (0x400b6db6db6db6db, 0x403ed522f6acd920),
        (0x400db6db6db6db6e, 0x404483bf1e811e13),
        (0x4010000000000000, 0x404b4c902e273a58),
        (0x4011249249249249, 0x405229e21611df21),
        (0x4012492492492492, 0x40582ba7fe421df3),
        (0x40136db6db6db6db, 0x406014feab8b5f6f),
        (0x4014924924924925, 0x40656689031fc94e),
        (0x4015b6db6db6db6e, 0x406c7a5a86d72952),
        (0x4016db6db6db6db7, 0x4072f2ae65efa169),
        (0x4018000000000000, 0x407936dc5690c08f),
        (0x4019249249249249, 0x4080c6c7eb0e4dc9),
        (0x401a492492492492, 0x4086531e0becf9b7),
        (0x401b6db6db6db6db, 0x408db52d281e27c4),
        (0x401c924924924925, 0x4093c42686dac49d),
        (0x401db6db6db6db6e, 0x409a4d9a9ec13601),
        (0x401edb6db6db6db7, 0x40a1803e9617f601),
        (0x4020000000000000, 0x40a749ea7d470c6e),
        (0x4020924924924925, 0x40aefd98247eb667),
        (0x4021249249249249, 0x40b49eaa54d4013a),
        (0x4021b6db6db6db6e, 0x40bb70626681d8fd),
        (0xbfc3e963dc486ad3, 0x3feb63d34e345835),
        (0x3fbd5da6fb537d5e, 0x3ff1f1dcc824addc),
        (0x3fd8a3856bcdf418, 0x3ff7836c51e78b0f),
        (0x3fe4f7d08c63846d, 0x3ffecf5c9ae866cb),
        (0x3fed9dde62e00ecd, 0x40042f7a6968686e),
        (0x3ff321f61cae4c97, 0x400a730adf2f103e),
        (0x3ff774fd07ec91c8, 0x4011541c7aed969c),
        (0x3ffbc803f32ad6f8, 0x4016b4b8054b15ed),
        (0x40000d856f348e14, 0x401dc083659ef625),
        (0x40023708e4d3b0ac, 0x40237e077f394876),
        (0x4004608c5a72d344, 0x40298a87249d0734),
        (0x40068a0fd011f5dc, 0x4030bbc6f517ddf4),
        (0x4008b39345b11875, 0x4035ed1cd7eb06da),
        (0x400add16bb503b0d, 0x403cbaf7328dd3fe),
        (0x400d069a30ef5da5, 0x4042d2ac85a7e3e3),
        (0x400f301da68e803d, 0x4048a9ff6eb2d2c0),
        (0xbff4000000000000, 0x3fd25618372a584f),
        (0xbff2000000000000, 0x3fd4c71b2477ab20),
        (0xbff0000000000000, 0x3fd78b56362cef38),
        (0xbfec000000000000, 0x3fdaadde095dad4b),
        (0xbfe8000000000000, 0x3fde3b40ebefcd7e),
        (0xbfe4000000000000, 0x3fe120dc934993e8),
        (0xbfe0000000000000, 0x3fe368b2fc6f960a),
        (0xbfd8000000000000, 0x3fe5fe4615e98e8f),
        (0xbfd0000000000000, 0x3fe8ebef9eac820b),
        (0xbfc0000000000000, 0x3fec3d6a24ed8222),
        (0x0000000000000000, 0x3ff0000000000000),
        (0x3fc0000000000000, 0x3ff2216045b6f5cd),
        (0x3fd0000000000000, 0x3ff48b5e3c3e8186),
        (0x3fd8000000000000, 0x3ff747a513dbef6a),
        (0x3fe0000000000000, 0x3ffa61298e1e069c),
        (0x3fe4000000000000, 0x3ffde455df80e3c0),
        (0x3fe8000000000000, 0x4000ef9db467dcf8),
        (0x3fec000000000000, 0x400330e587b62b28),
        (0x3ff0000000000000, 0x4005bf0a8b145769),
        (0x3ff2000000000000, 0x4008a448bb358768),
        (0x3ff4000000000000, 0x400bec38edb0faf0),
        (0x3ff6000000000000, 0x400fa3ff43d0eaf9),
        (0x3ff8000000000000, 0x4011ed3fe64fc541),
        (0x3ffa000000000000, 0x4014504d148049fe),
        (0x3ffc000000000000, 0x401704b6905bacbf),
        (0x3ffe000000000000, 0x401a15519b83cc1b),
        (0x4000000000000000, 0x401d8e64b8d4ddae),
        (0x4001000000000000, 0x4020beec6b6888c8),
        (0x4002000000000000, 0x4022f9b882f47e81),
        (0x4003000000000000, 0x40258084cce2201b),
        (0x4004000000000000, 0x40285d6fd931e0bb),
        (0x4005000000000000, 0x402b9bf125d0ba5d),
    ];
}
