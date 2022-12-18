use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;


fn copy_memory_x() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));
    let out_path = out_dir.join("memory.x");

    File::create(&out_path)
        .expect("failed to create memory.x output file")
        .write_all(include_bytes!("src/memory.x"))
        .expect("failed to write memory.x output file");
    println!("cargo:rustc-link-search={}", out_dir.display());
    println!("cargo:rerun-if-changed=src/memory.x");
}


fn main() {
    copy_memory_x();
}
