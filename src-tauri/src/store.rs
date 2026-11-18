use std::path::Path;

use rusqlite::{params, Connection};

use crate::dto::{OpsEntry, RepoInfo, SnapshotInfo};
use crate::error::AppResult;

pub fn open_db(path: &Path) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        CREATE TABLE IF NOT EXISTS repos (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL,
            last_opened INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS ops_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            repo_id TEXT NOT NULL,
            action TEXT NOT NULL,
            detail TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL,
            snapshot_id TEXT,
            duration_ms INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_ops_repo ON ops_log(repo_id, created_at DESC);
        CREATE TABLE IF NOT EXISTS snapshots (
            id TEXT PRIMARY KEY,
            repo_id TEXT NOT NULL,
            op TEXT NOT NULL,
            head_ref TEXT,
            stash_ref TEXT,
            untracked_dir TEXT,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_snap_repo ON snapshots(repo_id, created_at DESC);
        "#,
    )?;
    Ok(conn)
}

pub fn list_repos(conn: &Connection) -> AppResult<Vec<RepoInfo>> {
    let mut stmt = conn.prepare("SELECT id, name, path, last_opened FROM repos ORDER BY last_opened DESC, name")?;
    let rows = stmt.query_map([], |r| {
        Ok(RepoInfo {
            id: r.get(0)?,
            name: r.get(1)?,
            path: r.get(2)?,
            last_opened: r.get(3)?,
        })
    })?;
    Ok(rows.flatten().collect())
}

pub fn upsert_repo(conn: &Connection, id: &str, name: &str, path: &str) -> AppResult<RepoInfo> {
    let now = chrono_now();
    conn.execute(
        "INSERT INTO repos(id, name, path, last_opened) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET name=excluded.name, path=excluded.path, last_opened=excluded.last_opened",
        params![id, name, path, now],
    )?;
    Ok(RepoInfo { id: id.into(), name: name.into(), path: path.into(), last_opened: now })
}

pub fn touch_repo(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("UPDATE repos SET last_opened=?1 WHERE id=?2", params![chrono_now(), id])?;
    Ok(())
}

pub fn remove_repo(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM repos WHERE id=?1", params![id])?;
    conn.execute("DELETE FROM ops_log WHERE repo_id=?1", params![id])?;
    conn.execute("DELETE FROM snapshots WHERE repo_id=?1", params![id])?;
    Ok(())
}

pub fn repo_path(conn: &Connection, id: &str) -> AppResult<String> {
    conn.query_row("SELECT path FROM repos WHERE id=?1", params![id], |r| r.get(0))
        .map_err(|_| crate::error::AppError::msg("Repository is not registered"))
}

pub fn log_op(
    conn: &Connection,
    repo_id: &str,
    action: &str,
    detail: &str,
    status: &str,
    snapshot_id: Option<&str>,
    duration_ms: i64,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO ops_log(repo_id, action, detail, status, snapshot_id, duration_ms, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            repo_id,
            action,
            crate::security::redact(detail),
            status,
            snapshot_id,
            duration_ms,
            chrono_now()
        ],
    )?;
    Ok(())
}

pub fn list_ops(conn: &Connection, repo_id: &str, limit: usize) -> AppResult<Vec<OpsEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, repo_id, action, detail, status, snapshot_id, duration_ms, created_at
         FROM ops_log WHERE repo_id=?1 ORDER BY id DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![repo_id, limit as i64], |r| {
        Ok(OpsEntry {
            id: r.get(0)?,
            repo_id: r.get(1)?,
            action: r.get(2)?,
            detail: r.get(3)?,
            status: r.get(4)?,
            snapshot_id: r.get(5)?,
            duration_ms: r.get(6)?,
            created_at: r.get(7)?,
        })
    })?;
    Ok(rows.flatten().collect())
}

pub fn insert_snapshot(conn: &Connection, s: &SnapshotInfo) -> AppResult<()> {
    conn.execute(
        "INSERT INTO snapshots(id, repo_id, op, head_ref, stash_ref, untracked_dir, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![s.id, s.repo_id, s.op, s.head_ref, s.stash_ref, s.untracked_dir, s.created_at],
    )?;
    Ok(())
}

pub fn list_snapshots(conn: &Connection, repo_id: &str) -> AppResult<Vec<SnapshotInfo>> {
    let mut stmt = conn.prepare(
        "SELECT id, repo_id, op, head_ref, stash_ref, untracked_dir, created_at
         FROM snapshots WHERE repo_id=?1 ORDER BY created_at DESC LIMIT 100",
    )?;
    let rows = stmt.query_map(params![repo_id], |r| {
        Ok(SnapshotInfo {
            id: r.get(0)?,
            repo_id: r.get(1)?,
            op: r.get(2)?,
            head_ref: r.get(3)?,
            stash_ref: r.get(4)?,
            untracked_dir: r.get(5)?,
            created_at: r.get(6)?,
        })
    })?;
    Ok(rows.flatten().collect())
}

pub fn get_snapshot(conn: &Connection, id: &str) -> AppResult<SnapshotInfo> {
    conn.query_row(
        "SELECT id, repo_id, op, head_ref, stash_ref, untracked_dir, created_at FROM snapshots WHERE id=?1",
        params![id],
        |r| {
            Ok(SnapshotInfo {
                id: r.get(0)?,
                repo_id: r.get(1)?,
                op: r.get(2)?,
                head_ref: r.get(3)?,
                stash_ref: r.get(4)?,
                untracked_dir: r.get(5)?,
                created_at: r.get(6)?,
            })
        },
    )
    .map_err(|_| crate::error::AppError::msg("Snapshot not found"))
}

#[allow(dead_code)]
pub fn prune_snapshots(conn: &Connection, older_than_days: i64) -> AppResult<usize> {
    let cutoff = chrono_now() - older_than_days * 86_400_000;
    let removed = conn.execute("DELETE FROM snapshots WHERE created_at < ?1", params![cutoff])?;
    Ok(removed)
}

pub fn chrono_now() -> i64 {
    (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()) as i64
}
