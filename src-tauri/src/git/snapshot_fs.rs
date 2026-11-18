use std::fs;
use std::path::{Path, PathBuf};

pub fn collect_untracked(repo: &Path) -> Vec<PathBuf> {
    let out = crate::git::cli::git(repo, &["ls-files", "--others", "--exclude-standard", "-z"]);
    let Ok(out) = out else { return Vec::new() };
    if out.status != 0 {
        return Vec::new();
    }
    out.stdout
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect()
}

const MAX_FILES: usize = 5000;
const MAX_BYTES: u64 = 200 * 1024 * 1024;

fn is_ignored(path: &Path) -> bool {
    path.components().any(|c| {
        matches!(
            c.as_os_str().to_string_lossy().as_ref(),
            "node_modules" | "target" | ".git" | "dist" | ".next" | ".turbo"
        )
    })
}

/// Copy untracked files into a snapshot directory, respecting caps.
/// Returns (copied, skipped, bytes).
pub fn copy_untracked(repo: &Path, dest: &Path) -> (usize, usize, u64) {
    let mut copied = 0;
    let mut skipped = 0;
    let mut bytes = 0u64;
    for rel in collect_untracked(repo) {
        if copied >= MAX_FILES || bytes >= MAX_BYTES {
            skipped += 1;
            continue;
        }
        if is_ignored(&rel) {
            skipped += 1;
            continue;
        }
        let src = repo.join(&rel);
        let Ok(meta) = fs::metadata(&src) else { continue };
        if !meta.is_file() {
            skipped += 1;
            continue;
        }
        if bytes + meta.len() > MAX_BYTES {
            skipped += 1;
            continue;
        }
        let target = dest.join(&rel);
        if let Some(parent) = target.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::copy(&src, &target).is_ok() {
            copied += 1;
            bytes += meta.len();
        } else {
            skipped += 1;
        }
    }
    (copied, skipped, bytes)
}

pub fn restore_untracked(dest: &Path, repo: &Path) -> (usize, usize) {
    let mut restored = 0;
    let mut failed = 0;
    fn walk(dir: &Path, base: &Path, repo: &Path, restored: &mut usize, failed: &mut usize) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, base, repo, restored, failed);
            } else {
                let Ok(rel) = path.strip_prefix(base) else { continue };
                let target = repo.join(rel);
                if let Some(parent) = target.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if fs::copy(&path, &target).is_ok() {
                    *restored += 1;
                } else {
                    *failed += 1;
                }
            }
        }
    }
    walk(dest, dest, repo, &mut restored, &mut failed);
    (restored, failed)
}
