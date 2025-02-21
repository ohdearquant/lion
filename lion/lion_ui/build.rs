fn main() {
    if std::env::var("CARGO_FEATURE_DESKTOP").is_ok() {
        // Only build Tauri when desktop feature is enabled
        tauri_build::build();
    }

    // Always print this for cargo to track the config file
    println!("cargo:rerun-if-changed=tauri.conf.json");

    // Also track changes to the frontend files
    println!("cargo:rerun-if-changed=templates/index.html");
    println!("cargo:rerun-if-changed=src");
}
