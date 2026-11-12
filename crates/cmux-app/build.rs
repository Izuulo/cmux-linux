fn main() {
    // Feature `browser`: cef-dll-sys copies libcef.so next to the binary but adds no rpath.
    // `$ORIGIN/..` is for test binaries in target/<profile>/deps.
    if std::env::var_os("CARGO_FEATURE_BROWSER").is_some()
        && std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux")
    {
        println!("cargo::rustc-link-arg=-Wl,-rpath,$ORIGIN:$ORIGIN/..");
    }
}
