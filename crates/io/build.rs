fn main() {
    configure_ndi_delay_load();
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

fn configure_ndi_delay_load() {
    if std::env::var_os("CARGO_FEATURE_NDI").is_none()
        || std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
    {
        return;
    }
    assert_eq!(
        std::env::var("CARGO_CFG_TARGET_ENV").as_deref(),
        Ok("msvc"),
        "NDI runtime guarding requires the MSVC delay-load linker on Windows",
    );
    let dll = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("x86_64") => "Processing.NDI.Lib.x64.dll",
        Ok("x86") => "Processing.NDI.Lib.x86.dll",
        _ => panic!("NDI runtime guarding does not support this Windows architecture"),
    };
    // The application and io test binaries each need this final-link option.
    // The io library's link arguments do not propagate to downstream binaries.
    println!("cargo:rustc-link-lib=delayimp");
    println!("cargo:rustc-link-arg=/DELAYLOAD:{dll}");
}
