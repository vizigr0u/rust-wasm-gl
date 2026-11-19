//! Parse + validate every `backend/shaders/*.wgsl` with naga, against the
//! WebGPU-core capability subset (no push constants, no f64, ...).
use naga::valid::{Capabilities, ValidationFlags, Validator};
use std::path::PathBuf;

fn shader_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../backend/shaders")
}

#[test]
fn all_wgsl_shaders_validate() {
    let mut files: Vec<PathBuf> = std::fs::read_dir(shader_dir())
        .expect("backend/shaders must exist")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "wgsl"))
        .collect();
    files.sort();

    let mut failures = Vec::new();
    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let src = std::fs::read_to_string(path).unwrap();
        let module = match naga::front::wgsl::parse_str(&src) {
            Ok(m) => m,
            Err(e) => {
                failures.push(format!("{name}: parse: {}", e.emit_to_string(&src)));
                continue;
            }
        };
        // WebGPU subset: no push constants, no f64, no optional features.
        let mut validator = Validator::new(ValidationFlags::all(), Capabilities::empty());
        if let Err(e) = validator.validate(&module) {
            failures.push(format!("{name}: validate: {}", e.emit_to_string(&src)));
        }
    }

    println!("validated {} wgsl shader(s)", files.len());
    assert!(failures.is_empty(), "WGSL failures:\n{}", failures.join("\n"));
}
