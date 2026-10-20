use std::process::Command;

fn run(command: &mut Command, description: &str) {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("{description}: {error}"));
    assert!(status.success(), "{description} failed: {status}");
}

fn build_syphon() {
    let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let frameworks = manifest.join("../../target/syphon/frameworks");
    let frameworks = frameworks.canonicalize().unwrap_or_else(|_| {
        panic!("Syphon.framework is missing. On macOS run `bash app/scripts/prepare-syphon.sh` from the repository root before Cargo/Tauri builds.")
    });
    assert!(
        frameworks
            .join("Syphon.framework/Headers/SyphonMetalServer.h")
            .is_file(),
        "The prepared Syphon.framework is missing its Metal server headers"
    );
    println!(
        "cargo:rerun-if-changed={}",
        frameworks.join("Syphon.framework").display()
    );

    let sdk = Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
        .expect("Syphon requires full Xcode with the macOS SDK");
    assert!(
        sdk.status.success(),
        "Could not resolve the macOS SDK using xcrun"
    );
    let sdk = String::from_utf8(sdk.stdout).expect("macOS SDK path is not UTF-8");
    let architecture = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86_64") => "x86_64",
        other => panic!("Unsupported Syphon macOS architecture: {other:?}"),
    };
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let object = out.join("syphon_bridge.o");
    run(
        Command::new("xcrun")
            .args([
                "--sdk",
                "macosx",
                "clang",
                "-c",
                "-fobjc-arc",
                "-fobjc-exceptions",
                "-fobjc-arc-exceptions",
                "-fblocks",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-O2",
                "-arch",
                architecture,
                "-mmacosx-version-min=12.0",
                "-isysroot",
                sdk.trim(),
            ])
            .arg("-F")
            .arg(&frameworks)
            .arg(manifest.join("native/syphon_bridge.m"))
            .arg("-o")
            .arg(&object),
        "Compile the Syphon Objective-C bridge",
    );
    run(
        Command::new("xcrun")
            .args(["--sdk", "macosx", "ar", "crs"])
            .arg(out.join("libsyndocal_syphon_bridge.a"))
            .arg(&object),
        "Archive the Syphon bridge",
    );

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-search=framework={}", frameworks.display());
    println!("cargo:rustc-link-lib=static=syndocal_syphon_bridge");
    for framework in ["Syphon", "Foundation", "Metal"] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
    println!("cargo:rustc-link-lib=objc");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
    if std::env::var("PROFILE").as_deref() == Ok("debug") {
        // Cargo tests live in target/debug/deps and developer binaries are not
        // app bundles. Keep this build-machine rpath out of release packages.
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", frameworks.display());
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=SYNDOCAL_UPDATE_ENDPOINT");
    println!("cargo:rerun-if-env-changed=SYNDOCAL_UPDATE_PUBKEY");
    println!("cargo:rerun-if-env-changed=SYNDOCAL_UPDATE_CHANNEL");
    println!("cargo:rerun-if-changed=native/syphon_bridge.m");
    println!("cargo:rerun-if-changed=../scripts/prepare-syphon.sh");
    // Target metadata is required here: build scripts run on the host, which
    // need not have the same OS or architecture as the application being built.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos")
        && std::env::var_os("CARGO_FEATURE_SYPHON").is_some()
    {
        build_syphon();
    }
    tauri_build::build();
}
