use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=c/hyprdrop-detect-selection.c");

    let out_dir = match std::env::var("OUT_DIR") {
        Ok(d) => d,
        Err(_) => return,
    };

    let out_path = PathBuf::from(&out_dir);
    // target/{debug|release}/build/... -> target/{debug|release} is ancestors(3)
    let target_dir = match out_path.ancestors().nth(3) {
        Some(p) => p,
        None => return,
    };

    let dest_bin = target_dir.join("hyprdrop-detect-selection");

    let pkg_check = Command::new("pkg-config")
        .args(["--cflags", "--libs", "atspi-2", "gobject-2.0", "glib-2.0"])
        .output();

    if let Ok(pkg_out) = pkg_check {
        if pkg_out.status.success() {
            let flags = String::from_utf8_lossy(&pkg_out.stdout);
            let mut args = vec![
                "-O3",
                "c/hyprdrop-detect-selection.c",
                "-o",
                dest_bin.to_str().unwrap(),
            ];
            for arg in flags.split_whitespace() {
                args.push(arg);
            }
            if let Ok(status) = Command::new("gcc").args(&args).status() {
                if status.success() {
                    println!("cargo:warning=Compiled native hyprdrop-detect-selection helper to {:?}", dest_bin);
                }
            }
        }
    }
}
