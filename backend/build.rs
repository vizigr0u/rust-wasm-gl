// build.rs
use std::process::Command;

fn main() {
    let git_hash = Command::new("git")
        .args(["describe", "--abbrev=4", "--dirty", "--always", "--tags"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
