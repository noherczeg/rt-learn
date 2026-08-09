//! Build script: make the linker aware of `memory.x`.
//!
//! `memory.x` is copied into Cargo's `OUT_DIR` (which is on the linker search
//! path) so that cortex-m-rt's `link.x` can `INCLUDE memory.x`. The script re-runs
//! only when the memory map or the script itself changes.

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is always set by cargo"));

    File::create(out_dir.join("memory.x"))
        .expect("failed to create memory.x in OUT_DIR")
        .write_all(include_bytes!("memory.x"))
        .expect("failed to write memory.x");

    println!("cargo:rustc-link-search={}", out_dir.display());

    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=build.rs");
}
