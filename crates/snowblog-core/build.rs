use std::path::Path;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir");
    let lock_path = Path::new(&manifest_dir).join("../../Cargo.lock");
    println!("cargo:rerun-if-changed={}", lock_path.display());
    let version = std::fs::read_to_string(&lock_path)
        .ok()
        .and_then(|lock| typst_version(&lock))
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=SNOWBLOG_TYPST_VERSION={version}");
}

fn typst_version(lock: &str) -> Option<String> {
    let mut in_typst_package = false;
    for line in lock.lines() {
        let line = line.trim();
        if line == "name = \"typst\"" {
            in_typst_package = true;
        } else if in_typst_package {
            return line
                .strip_prefix("version = \"")
                .and_then(|rest| rest.strip_suffix('"'))
                .map(str::to_string);
        }
    }
    None
}
