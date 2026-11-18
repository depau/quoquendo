fn main() {
    let mut b = cc::Build::new();
    b.include("speex/include")
        .include("speex/libspeex")
        .define("HAVE_CONFIG_H", None)
        .warnings(false);
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        b.define("USE_ALLOCA", None); // MSVC has no C99 variable length arrays
    } else {
        b.define("VAR_ARRAYS", None);
    }
    // experiment: the original codec (MSVC, 32-bit) computes on the x87 FPU
    if std::env::var_os("QQ_X87").is_some() && std::env::var("CARGO_CFG_TARGET_ARCH").map(|a| a == "x86_64" || a == "x86").unwrap_or(false) {
        b.flag("-mfpmath=387");
    }
    for f in std::fs::read_dir("speex/libspeex").unwrap() {
        let p = f.unwrap().path();
        if p.extension().is_some_and(|e| e == "c") {
            b.file(&p);
        }
    }
    b.compile("speex");
    println!("cargo:rerun-if-changed=speex");
    println!("cargo:rerun-if-env-changed=QQ_X87");
}
