//! 构建脚本：为 RISC-V64 目标自动生成链接脚本。
//!
//! 这里复用 `tg-linker` 提供的 `NOBIOS_SCRIPT`，确保导出
//! `KernelLayout::locate()` 所需的链接符号（如 `__start/__sbss/__ebss`）。

fn main() {
    use std::{env, fs, path::PathBuf};

    println!("cargo:rerun-if-changed=build.rs");

    if env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default() == "riscv64" {
        let ld = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("linker.ld");
        fs::write(&ld, tg_linker::NOBIOS_SCRIPT)
            .unwrap_or_else(|err| panic!("failed to write {}: {}", ld.display(), err));
        println!("cargo:rustc-link-arg=-T{}", ld.display());
    }
}
