use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::dto::SnapshotInfo;
use crate::error::{AppError, AppResult};
use crate::git::{cli, snapshot_fs};
use crate::store;

/// Create a recovery snapshot before a destructive operation.
/// Committed history -> backup ref, index/worktree -> stash commit, untracked -> files on disk.
pub fn create(
    conn: &Connection,
    repo_id: &str,
    repo: &Path,
    op: &str,
    data_dir: &Path,
) -> AppResult<SnapshotInfo> {
    let created_at = store::chrono_now();
    let safe_op: String = op.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    let id = format!("{created_at}-{}", if safe_op.is_empty() { "snapshot" } else { &safe_op });

    // 1. Committed HEAD (if any).
    let head_ref = if cli::git(repo, &["rev-parse", "--verify", "--quiet", "HEAD"])?.status == 0 {
        let reference = format!("refs/backups/{id}");
        cli::git_ok(repo, &["update-ref", &reference, "HEAD"])?;
        Some(reference)
    } else {
        None
    };

    // 2. Staged + unstaged tracked changes as a stash commit.
    let stash_ref = match cli::git(repo, &["stash", "create"]) {
        Ok(out) if out.status == 0 && !out.stdout.trim().is_empty() => {
            let oid = out.stdout.trim().to_string();
            let reference = format!("refs/backups/{id}-stash");
            cli::git_ok(repo, &["update-ref", &reference, &oid])?;
            Some(reference)
        }
        _ => None,
    };

    // 3. Untracked files on disk (capped).
    let untracked_dir = data_dir.join("snapshots").join(&id).join("untracked");
    let (copied, _skipped, _bytes) = snapshot_fs::copy_untracked(repo, &untracked_dir);
    let untracked_dir = if copied > 0 {
        Some(untracked_dir.to_string_lossy().to_string())
    } else {
        None
    };

    let info = SnapshotInfo {
        id,
        repo_id: repo_id.to_string(),
        op: op.to_string(),
        head_ref,
        stash_ref,
        untracked_dir,
        created_at,
    };
    store::insert_snapshot(conn, &info)?;
    Ok(info)
}

/// Restore a snapshot. Returns a human-readable report of what was restored.
pub fn restore(conn: &Connection, repo: &Path, snapshot_id: &str) -> AppResult<String> {
    let snap = store::get_snapshot(conn, snapshot_id)?;
    let mut report = Vec::new();

    if let Some(head_ref) = &snap.head_ref {
        cli::git_ok(repo, &["reset", "--hard", head_ref])?;
        report.push("HEAD and tracked files restored".to_string());
    }
    if let Some(stash_ref) = &snap.stash_ref {
        cli::git_ok(repo, &["stash", "apply", stash_ref])?;
        report.push("staged/unstaged changes re-applied".to_string());
    }
    if let Some(dir) = &snap.untracked_dir {
        let dir = PathBuf::from(dir);
        if dir.exists() {
            let (restored, failed) = snapshot_fs::restore_untracked(&dir, repo);
            report.push(format!("{restored} untracked file(s) restored, {failed} failed"));
        }
    }
    if report.is_empty() {
        return Err(AppError::msg("Snapshot contains no recoverable data"));
    }
    Ok(report.join("; "))
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {:?}: {}", args, String::from_utf8_lossy(&out.stderr));
    }

    #[test]
    fn snapshot_preserves_tracked_and_untracked_work() {
        let dir = std::env::temp_dir().join(format!(
            "gitmgr-snap-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let repo = dir.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        std::fs::write(repo.join("a.txt"), "v1\n").unwrap();
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-q", "-m", "initial"]);

        // Dirty state: tracked modification + untracked file.
        std::fs::write(repo.join("a.txt"), "v2\n").unwrap();
        std::fs::write(repo.join("untracked.txt"), "keep me\n").unwrap();

        let db = crate::store::open_db(&dir.join("app.db")).unwrap();
        let snap = create(&db, "test-repo", &repo, "unit", &dir).unwrap();
        assert!(snap.head_ref.is_some());
        assert!(snap.stash_ref.is_some());
        assert!(snap.untracked_dir.is_some());

        // Destroy the dirty state.
        std::fs::write(repo.join("a.txt"), "v3\n").unwrap();
        std::fs::remove_file(repo.join("untracked.txt")).unwrap();

        let report = restore(&db, &repo, &snap.id).unwrap();
        assert!(report.contains("restored"));
        let a = std::fs::read_to_string(repo.join("a.txt")).unwrap().replace("\r\n", "\n");
        assert_eq!(a, "v2\n");
        assert_eq!(std::fs::read_to_string(repo.join("untracked.txt")).unwrap(), "keep me\n");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
