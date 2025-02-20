use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=frontend");

    // Get the manifest directory (where Cargo.toml lives)
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Source directory containing frontend files
    let frontend_dir = Path::new(&manifest_dir).join("frontend");

    // Ensure the frontend directory exists
    if !frontend_dir.exists() {
        fs::create_dir_all(&frontend_dir).unwrap();
    }

    // We don't need to copy files since we're using include_str! in main.rs
    // This build script mainly ensures the frontend directory exists and
    // triggers rebuilds when frontend files change

    // Print the frontend directory path for debugging
    println!(
        "cargo:warning=Frontend directory: {}",
        frontend_dir.display()
    );
}
