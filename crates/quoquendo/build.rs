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
    for f in std::fs::read_dir("speex/libspeex").unwrap() {
        let p = f.unwrap().path();
        if p.extension().is_some_and(|e| e == "c") {
            b.file(&p);
        }
    }
    b.compile("speex");
    println!("cargo:rerun-if-changed=speex");
}
