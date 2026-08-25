use std::{env, path::PathBuf};

fn required_directory(variable: &str) -> PathBuf {
    let value = env::var(variable).unwrap_or_else(|_| {
        panic!(
            "{variable} must be set explicitly for the ASIO bridge; run the repository ASIO preflight before Cargo so asio-sys cannot use its download fallback"
        )
    });
    if value.trim().is_empty() {
        panic!("{variable} must not be empty for the ASIO bridge");
    }

    let path = PathBuf::from(value);
    if !path.is_dir() {
        panic!(
            "{variable} must name an existing directory: {}",
            path.display()
        );
    }
    path
}

fn main() {
    println!("cargo:rerun-if-env-changed=CPAL_ASIO_DIR");
    println!("cargo:rerun-if-env-changed=LIBCLANG_PATH");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // The application loads the bridge dynamically and never links an import library.
        // Avoiding the unused .lib/.exp pair also keeps MSVC's import-library progress line from
        // being promoted to Rust's `linker_messages` warning while preserving the DLL exports.
        println!("cargo:rustc-cdylib-link-arg=/NOIMPLIB");
        println!("cargo:rustc-cdylib-link-arg=/NOEXP");
    }

    if env::var_os("CARGO_FEATURE_ASIO").is_none() {
        return;
    }
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        panic!("the ASIO bridge can only be built for Windows targets");
    }

    let sdk = required_directory("CPAL_ASIO_DIR");
    for relative in ["common/asio.h", "common/asiosys.h", "host/asiodrivers.h"] {
        let file = sdk.join(relative);
        if !file.is_file() {
            panic!(
                "CPAL_ASIO_DIR is missing required SDK file: {}",
                file.display()
            );
        }
    }

    let libclang = required_directory("LIBCLANG_PATH").join("libclang.dll");
    if !libclang.is_file() {
        panic!(
            "LIBCLANG_PATH is missing libclang.dll: {}",
            libclang.display()
        );
    }
}
