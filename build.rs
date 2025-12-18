use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    // Ensure changes to Askama templates trigger a rebuild.
    // Askama templates are read at compile time, but without explicit cargo hints
    // it's easy to end up with a stale binary during dev.
    rerun_if_changed_dir("templates");

    // Helpful dev marker so we can see whether the running server is actually
    // the newest binary.
    let build_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "dev".to_string());
    println!("cargo:rustc-env=GOAMET_BUILD_ID={}", build_id);
}

fn rerun_if_changed_dir(dir: impl AsRef<Path>) {
    let dir = dir.as_ref();
    if !dir.exists() {
        return;
    }
    let mut stack: Vec<PathBuf> = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        let Ok(entries) = fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            if p.extension().and_then(|s| s.to_str()) == Some("html") {
                println!("cargo:rerun-if-changed={}", p.display());
            }
        }
    }
}
