fn main() {
    println!("cargo:rerun-if-changed=src/macos_ftdi_transport.c");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("src/macos_ftdi_transport.c")
            .warnings(true)
            .flag("-Werror")
            .compile("syndocal_macos_ftdi");
        println!("cargo:rustc-link-lib=framework=IOKit");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    }
}
