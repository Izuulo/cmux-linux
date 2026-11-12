fn main() {
    // cef-dll-sys copies libcef.so and its resources next to the binary; let the loader find it there.
    println!("cargo::rustc-link-arg=-Wl,-rpath,$ORIGIN");
}
