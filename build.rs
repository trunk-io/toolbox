use std::process::Command;

fn main() {
    // This is a workaround to allow the value of HORTON_RELEASE during `cargo build` to control
    // the built version, as opposed to pulling the package version from Cargo.toml.
    // See https://github.com/rust-lang/cargo/issues/6583#issuecomment-1259871885
    println!("cargo:rerun-if-env-changed=HORTON_RELEASE");
    if let Ok(val) = std::env::var("HORTON_RELEASE") {
        println!("cargo:rustc-env=CARGO_PKG_VERSION={}", val);
    } else {
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .unwrap();
        let git_ref = String::from_utf8(output.stdout).unwrap();
        println!("cargo:rustc-env=CARGO_PKG_VERSION={}", git_ref);
    }
}
