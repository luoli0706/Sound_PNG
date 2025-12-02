fn main() {
    #[cfg(windows)]
    {
        // Increase stack size to 8MB
        if std::env::var("CARGO_CFG_TARGET_ENV").unwrap() == "msvc" {
            println!("cargo:rustc-link-arg=/STACK:8388608");
        } else {
            println!("cargo:rustc-link-arg=-Wl,--stack,8388608");
        }
    }
    slint_build::compile("src/ui.slint").unwrap();
}
