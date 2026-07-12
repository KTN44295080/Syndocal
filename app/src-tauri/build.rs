fn main() {
    println!("cargo:rerun-if-env-changed=SYNDOCAL_UPDATE_ENDPOINT");
    println!("cargo:rerun-if-env-changed=SYNDOCAL_UPDATE_PUBKEY");
    println!("cargo:rerun-if-env-changed=SYNDOCAL_UPDATE_CHANNEL");
    tauri_build::build();
}
