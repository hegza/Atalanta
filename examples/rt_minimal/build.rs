use std::{env, fs, path::PathBuf};

fn add_linker_script() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Put the linker script somewhere the linker can find it.
    fs::write(out_dir.join("memory.x"), include_bytes!("memory.x")).unwrap();
    println!("cargo:rustc-link-search={}", out_dir.display());
    println!("cargo:rerun-if-changed=memory.x");
}

fn main() {
    add_linker_script();

    println!("cargo:rerun-if-changed=build.rs");
}
