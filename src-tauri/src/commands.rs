use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rusqlite::Connection;
use tauri::{AppHandle, Emitter, State};

use crate::dto::*;
use crate::error::{AppError, AppResult};
use crate::git::{cli, graph::RepoGraph, read, snapshots, watcher::RepoWatcher};
use crate::{security, store};

pub struct AppStateInner {
    pub db: Mutex<Connection>,
    pub graphs: Mutex<HashMap<String, RepoGraph>>,
    pub watchers: Mutex<HashMap<String, RepoWatcher>>,
    pub jobs: Mutex<Vec<JobUpdate>>,
    pub data_dir: PathBuf,
}

#[derive(Clone)]
pub struct AppState(pub Arc<AppStateInner>);

impl AppState {
    pub fn new(conn: Connection, data_dir: PathBuf) -> Self {
        Self(Arc::new(AppStateInner {
            db: Mutex::new(conn),
            graphs: Mutex::new(HashMap::new()),
            watchers: Mutex::new(HashMap::new()),
            jobs: Mutex::new(Vec::new()),
            data_dir,
        }))
    }

    fn repo_path(&self, id: &str) -> AppResult<PathBuf> {
        let db = self.0.db.lock().map_err(|_| AppError::msg("State lock poisoned"))?;
        let path = store::repo_path(&db, id)?;
        security::canonical_dir(&path)
    }
}

async fn blocking<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> AppResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| format!("Background task failed: {e}"))?
        .map_err(String::from)
}

fn invalidate(st: &AppState, repo_id: &str) {
    if let Ok(mut graphs) = st.0.graphs.lock() {
        graphs.remove(repo_id);
    }
}

fn record(
    st: &AppState,
    repo_id: &str,
    action: &str,
    detail: &str,
    status: &str,
    snapshot_id: Option<&str>,
    start: Instant,
) {
    if let Ok(db) = st.0.db.lock() {
        let _ = store::log_op(
            &db,
            repo_id,
            action,
            detail,
            status,
            snapshot_id,
            start.elapsed().as_millis() as i64,
        );
    }
}

fn make_snapshot(st: &AppState, repo_id: &str, repo: &Path, op: &str) -> AppResult<Option<SnapshotInfo>> {
    let db = st.0.db.lock().map_err(|_| AppError::msg("State lock poisoned"))?;
    snapshots::create(&db, repo_id, repo, op, &st.0.data_dir).map(Some)
}

// ───────────────────────── Repositories ─────────────────────────

#[tauri::command]
pub fn system_info() -> SystemInfo {
    SystemInfo {
        git_version: cli::git_version().unwrap_or_else(|| "not found".into()),
        merge_tree: cli::supports_merge_tree(),
        platform: std::env::consts::OS.to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[tauri::command]
pub async fn list_repos(state: State<'_, AppState>) -> Result<Vec<RepoInfo>, String> {
    let st = state.inner().clone();
    blocking(move || {
        let db = st.0.db.lock().map_err(|_| AppError::msg("State lock poisoned"))?;
        store::list_repos(&db)
    })
    .await
}

#[tauri::command]
pub async fn add_repo(state: State<'_, AppState>, path: String) -> Result<RepoInfo, String> {
    let st = state.inner().clone();
    blocking(move || {
        let canonical = security::canonical_dir(&path)?;
        // Verify it is a git repository before registering.
        read::open(&canonical)?;
        let name = canonical
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| canonical.to_string_lossy().to_string());
        let id = canonical.to_string_lossy().to_string();
        let db = st.0.db.lock().map_err(|_| AppError::msg("State lock poisoned"))?;
        store::upsert_repo(&db, &id, &name, &id)
    })
    .await
}

#[tauri::command]
pub async fn remove_repo(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let st = state.inner().clone();
    blocking(move || {
        st.0.watchers.lock().ok().map(|mut w| w.remove(&id));
        invalidate(&st, &id);
        let db = st.0.db.lock().map_err(|_| AppError::msg("State lock poisoned"))?;
        store::remove_repo(&db, &id)
    })
    .await
}

#[tauri::command]
pub async fn activate_repo(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<RepoSummary, String> {
    let st = state.inner().clone();
    blocking(move || {
        let path = st.repo_path(&id)?;
        {
            let db = st.0.db.lock().map_err(|_| AppError::msg("State lock poisoned"))?;
            store::touch_repo(&db, &id)?;
        }
        // Only one active watcher: release the rest to stay idle-cheap.
        if let Ok(mut watchers) = st.0.watchers.lock() {
            watchers.clear();
            match crate::git::watcher::start(id.clone(), &path, app) {
                Ok(w) => {
                    watchers.insert(id.clone(), w);
                }
                Err(e) => tracing::warn!("watcher not started: {e}"),
            }
        }
        invalidate(&st, &id);
        read::summary(&path)
    })
    .await
}

#[tauri::command]
pub async fn repo_summary(state: State<'_, AppState>, id: String) -> Result<RepoSummary, String> {
    let st = state.inner().clone();
    blocking(move || {
        let path = st.repo_path(&id)?;
        read::summary(&path)
    })
    .await
}

#[tauri::command]
pub async fn refresh_repo(state: State<'_, AppState>, id: String) -> Result<RepoSummary, String> {
    let st = state.inner().clone();
    blocking(move || {
        invalidate(&st, &id);
        let path = st.repo_path(&id)?;
        read::summary(&path)
    })
    .await
}

#[tauri::command]
pub async fn clone_repo(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    dest: String,
) -> Result<RepoInfo, String> {
    security::validate_url(&url).map_err(String::from)?;
    let st = state.inner().clone();
    blocking(move || {
        let dest_path = PathBuf::from(&dest);
        if dest_path.exists() {
            return Err(AppError::msg("Destination already exists"));
        }
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let job_id = format!("clone-{}", store::chrono_now());
        let emit = |status: &str, message: &str| {
            let update = JobUpdate {
                id: job_id.clone(),
                kind: "clone".into(),
                status: status.into(),
                message: message.into(),
            };
            if let Ok(mut jobs) = st.0.jobs.lock() {
                jobs.insert(0, update.clone());
                jobs.truncate(30);
            }
            let _ = app.emit("job://update", update);
        };
        emit("running", "Cloning repository");

        let mut child = Command::new("git")
            .args(["clone", "--progress", &url, &dest])
            .env("GIT_TERMINAL_PROMPT", "0")
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AppError::msg(format!("Could not run git: {e}")))?;

        let stderr = child.stderr.take().ok_or_else(|| AppError::msg("No stderr"))?;
        let app2 = app.clone();
        let job2 = job_id.clone();
        let reader = std::thread::spawn(move || {
            let mut last = String::new();
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let trimmed = line.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                last = trimmed.clone();
                let _ = app2.emit(
                    "job://update",
                    JobUpdate {
                        id: job2.clone(),
                        kind: "clone".into(),
                        status: "running".into(),
                        message: trimmed,
                    },
                );
            }
            last
        });

        let status = child.wait()?;
        let last = reader.join().unwrap_or_default();
        if !status.success() {
            emit("failed", &last);
            return Err(AppError::Git(last));
        }
        emit("done", "Clone complete");

        let canonical = security::canonical_dir(&dest)?;
        let name = canonical
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| canonical.to_string_lossy().to_string());
        let id = canonical.to_string_lossy().to_string();
        let db = st.0.db.lock().map_err(|_| AppError::msg("State lock poisoned"))?;
        store::upsert_repo(&db, &id, &name, &id)
    })
    .await
}

#[tauri::command]
pub async fn list_jobs(state: State<'_, AppState>) -> Result<Vec<JobUpdate>, String> {
    let st = state.inner().clone();
    blocking(move || {
        let jobs = st.0.jobs.lock().map_err(|_| AppError::msg("State lock poisoned"))?;
        Ok(jobs.clone())
    })
    .await
}

// ───────────────────────── Graph & history ─────────────────────────

#[tauri::command]
pub async fn graph_window(
    state: State<'_, AppState>,
    id: String,
    skip: usize,
    limit: usize,
) -> Result<GraphWindow, String> {
    let st = state.inner().clone();
    blocking(move || {
        let path = st.repo_path(&id)?;
        let mut graphs = st.0.graphs.lock().map_err(|_| AppError::msg("State lock poisoned"))?;
        if !graphs.contains_key(&id) {
            graphs.insert(id.clone(), RepoGraph::build(&path)?);
        }
        let graph = graphs.get_mut(&id).ok_or_else(|| AppError::msg("Graph cache missing"))?;
        graph.window(&path, skip, limit.min(1000))
    })
    .await
}

#[tauri::command]
pub async fn commit_detail(
    state: State<'_, AppState>,
    id: String,
    oid: String,
) -> Result<CommitDetail, String> {
    let st = state.inner().clone();
    blocking(move || {
        security::validate_ref(&oid).ok();
        let path = st.repo_path(&id)?;
        read::commit_detail(&path, &oid)
    })
    .await
}

#[tauri::command]
pub async fn commit_files(
    state: State<'_, AppState>,
    id: String,
    oid: String,
) -> Result<Vec<CommitFile>, String> {
    let st = state.inner().clone();
    blocking(move || {
        let path = st.repo_path(&id)?;
        read::commit_files(&path, &oid)
    })
    .await
}

#[tauri::command]
pub async fn commit_file_diff(
    state: State<'_, AppState>,
    id: String,
    oid: String,
    path: String,
) -> Result<FileDiff, String> {
    let st = state.inner().clone();
    blocking(move || {
        let repo = st.repo_path(&id)?;
        security::validate_ref(&oid).ok();
        // Merge commits must be compared against their first parent; `git show`
        // renders an empty combined diff for most merges.
        let parent = format!("{oid}^");
        let has_parent = cli::git(&repo, &["rev-parse", "--verify", "--quiet", &parent])?.status == 0;
        let out = if has_parent {
            cli::git_ok(
                &repo,
                &["diff", "--no-color", "-U3", "-M", &parent, &oid, "--", &path],
            )?
        } else {
            cli::git_ok(&repo, &["show", "--format=", "--no-color", "-U3", "-M", &oid, "--", &path])?
        };
        Ok(crate::git::diff::parse_unified(&out, &path))
    })
    .await
}

#[tauri::command]
pub async fn diff_between(
    state: State<'_, AppState>,
    id: String,
    from: String,
    to: String,
) -> Result<Vec<CommitFile>, String> {
    let st = state.inner().clone();
    blocking(move || {
        let repo = st.repo_path(&id)?;
        security::validate_ref(&from).ok();
        security::validate_ref(&to).ok();
        let out = cli::git_ok(&repo, &["diff", "--numstat", "--find-renames", &from, &to])?;
        let mut files = Vec::new();
        for line in out.lines() {
            let mut parts = line.splitn(3, '\t');
            let (Some(add), Some(del), Some(p)) = (parts.next(), parts.next(), parts.next()) else { continue };
            files.push(CommitFile {
                path: p.to_string(),
                additions: add.parse().unwrap_or(0),
                deletions: del.parse().unwrap_or(0),
                binary: add == "-" && del == "-",
            });
        }
        Ok(files)
    })
    .await
}

// ───────────────────────── Working tree ─────────────────────────

#[tauri::command]
pub async fn working_status(state: State<'_, AppState>, id: String) -> Result<Vec<FileChange>, String> {
    let st = state.inner().clone();
    blocking(move || {
        let path = st.repo_path(&id)?;
        read::status(&path)
    })
    .await
}

#[tauri::command]
pub async fn file_diff(
    state: State<'_, AppState>,
    id: String,
    path: String,
    staged: bool,
) -> Result<FileDiff, String> {
    let st = state.inner().clone();
    blocking(move || {
        let repo = st.repo_path(&id)?;
        let mut args = vec!["diff", "--no-color", "-U3", "-M"];
        if staged {
            args.push("--cached");
        }
        args.push("--");
        args.push(&path);
        let out = cli::git_ok(&repo, &args)?;
        Ok(crate::git::diff::parse_unified(&out, &path))
    })
    .await
}

#[tauri::command]
pub async fn stage_paths(
    state: State<'_, AppState>,
    id: String,
    paths: Vec<String>,
) -> Result<(), String> {
    let st = state.inner().clone();
    blocking(move || {
        let repo = st.repo_path(&id)?;
        let mut args = vec!["add", "--"];
        args.extend(paths.iter().map(|s| s.as_str()));
        cli::git_ok(&repo, &args)?;
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn unstage_paths(
    state: State<'_, AppState>,
    id: String,
    paths: Vec<String>,
) -> Result<(), String> {
    let st = state.inner().clone();
    blocking(move || {
        let repo = st.repo_path(&id)?;
        let has_head = cli::git(&repo, &["rev-parse", "--verify", "--quiet", "HEAD"])?.status == 0;
        let mut args = if has_head {
            vec!["reset", "-q", "HEAD", "--"]
        } else {
            vec!["rm", "--cached", "-r", "-q", "--"]
        };
        args.extend(paths.iter().map(|s| s.as_str()));
        cli::git_ok(&repo, &args)?;
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn commit_changes(
    state: State<'_, AppState>,
    id: String,
    message: String,
    amend: bool,
) -> Result<OpResult, String> {
    let st = state.inner().clone();
    blocking(move || {
        let start = Instant::now();
        let repo = st.repo_path(&id)?;
        if message.trim().is_empty() {
            return Err(AppError::msg("Commit message is required"));
        }
        let snap = make_snapshot(&st, &id, &repo, if amend { "amend" } else { "commit" })?;
        let mut args = vec!["commit", "-m", message.trim()];
        if amend {
            args.push("--amend");
        }
        let out = cli::git_env(&repo, &args, &[("GIT_EDITOR", "true")])?;
        let snapshot_id = snap.as_ref().map(|s| s.id.as_str());
        if out.status != 0 {
            let msg = format!("{}\n{}", out.stdout, out.stderr);
            record(&st, &id, "commit", &msg, "failed", snapshot_id, start);
            return Err(AppError::Git(msg));
        }
        record(&st, &id, if amend { "amend" } else { "commit" }, message.trim(), "ok", snapshot_id, start);
        invalidate(&st, &id);
        Ok(OpResult { ok: true, message: out.stdout.trim().to_string(), conflicts: false })
    })
    .await
}

// ───────────────────────── Branches ─────────────────────────

#[tauri::command]
pub async fn branches(state: State<'_, AppState>, id: String) -> Result<Vec<BranchInfo>, String> {
    let st = state.inner().clone();
    blocking(move || {
        let path = st.repo_path(&id)?;
        read::branches(&path)
    })
    .await
}

#[tauri::command]
pub async fn create_branch(
    state: State<'_, AppState>,
    id: String,
    name: String,
    start_point: Option<String>,
) -> Result<(), String> {
    let st = state.inner().clone();
    blocking(move || {
        security::validate_ref(&name)?;
        let repo = st.repo_path(&id)?;
        let mut args = vec!["branch", &name];
        if let Some(sp) = &start_point {
            security::validate_ref(sp)?;
            args.push(sp);
        }
        cli::git_ok(&repo, &args)?;
        invalidate(&st, &id);
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn switch_branch(
    state: State<'_, AppState>,
    id: String,
    name: String,
    stash_first: bool,
) -> Result<OpResult, String> {
    let st = state.inner().clone();
    blocking(move || {
        let start = Instant::now();
        security::validate_ref(&name)?;
        let repo = st.repo_path(&id)?;
        let snap = make_snapshot(&st, &id, &repo, "switch")?;
        if stash_first {
            let _ = cli::git_ok(&repo, &["stash", "push", "-u", "-m", &format!("auto: switch to {name}")]);
        }
        let out = cli::git_env(&repo, &["switch", &name], &[("GIT_EDITOR", "true")])?;
        let snapshot_id = snap.as_ref().map(|s| s.id.as_str());
        if out.status != 0 {
            let msg = format!("{}\n{}", out.stdout, out.stderr);
            record(&st, &id, "switch", &msg, "failed", snapshot_id, start);
            return Ok(OpResult { ok: false, message: msg, conflicts: true });
        }
        record(&st, &id, "switch", &name, "ok", snapshot_id, start);
        invalidate(&st, &id);
        Ok(OpResult { ok: true, message: format!("Switched to {name}"), conflicts: false })
    })
    .await
}

#[tauri::command]
pub async fn delete_branch(
    state: State<'_, AppState>,
    id: String,
    name: String,
    force: bool,
) -> Result<OpResult, String> {
    let st = state.inner().clone();
    blocking(move || {
        let start = Instant::now();
        security::validate_ref(&name)?;
        let repo = st.repo_path(&id)?;
        let snap = make_snapshot(&st, &id, &repo, "branch-delete")?;
        let out = cli::git(&repo, &["branch", if force { "-D" } else { "-d" }, &name])?;
        let snapshot_id = snap.as_ref().map(|s| s.id.as_str());
        if out.status != 0 {
            record(&st, &id, "branch-delete", &out.stderr, "failed", snapshot_id, start);
            return Ok(OpResult { ok: false, message: out.stderr.trim().to_string(), conflicts: false });
        }
        record(&st, &id, "branch-delete", &name, "ok", snapshot_id, start);
        invalidate(&st, &id);
        Ok(OpResult { ok: true, message: out.stdout.trim().to_string(), conflicts: false })
    })
    .await
}

// ───────────────────────── Merge / conflicts ─────────────────────────

#[tauri::command]
pub async fn merge_preview(
    state: State<'_, AppState>,
    id: String,
    source: String,
) -> Result<MergePreview, String> {
    let st = state.inner().clone();
    blocking(move || {
        security::validate_ref(&source)?;
        let repo = st.repo_path(&id)?;
        if !cli::supports_merge_tree() {
            return Ok(MergePreview {
                clean: false,
                tree: None,
                conflicts: vec![],
                messages: vec!["git >= 2.38 required for isolated merge preview".into()],
                merge_base: None,
                supported: false,
            });
        }
        let base = cli::git(&repo, &["merge-base", "HEAD", &source])?;
        let merge_base = if base.status == 0 { Some(base.stdout.trim().to_string()) } else { None };

        // Isolated preview: never touches the working tree or index.
        let out = cli::git(
            &repo,
            &["merge-tree", "--write-tree", "--name-only", "HEAD", &source],
        )?;

        if out.status == 0 {
            return Ok(MergePreview {
                clean: true,
                tree: out.stdout.lines().next().map(|s| s.trim().to_string()),
                conflicts: vec![],
                messages: vec!["Merge would apply cleanly".into()],
                merge_base,
                supported: true,
            });
        }
        if out.status == 1 {
            let mut lines = out.stdout.lines();
            let tree = lines.next().map(|s| s.trim().to_string());
            let mut conflicts = Vec::new();
            let mut messages = Vec::new();
            let mut in_messages = false;
            for line in lines {
                if line.trim().is_empty() {
                    in_messages = true;
                    continue;
                }
                if in_messages {
                    messages.push(line.trim().to_string());
                } else {
                    conflicts.push(line.trim().to_string());
                }
            }
            return Ok(MergePreview { clean: false, tree, conflicts, messages, merge_base, supported: true });
        }
        Err(AppError::Git(format!("{}\n{}", out.stdout, out.stderr)))
    })
    .await
}

#[tauri::command]
pub async fn merge_branch(
    state: State<'_, AppState>,
    id: String,
    source: String,
) -> Result<OpResult, String> {
    let st = state.inner().clone();
    blocking(move || {
        let start = Instant::now();
        security::validate_ref(&source)?;
        let repo = st.repo_path(&id)?;
        let snap = make_snapshot(&st, &id, &repo, "merge")?;
        let message = format!("Merge branch '{source}'");
        let out = cli::git_env(
            &repo,
            &["merge", "--no-ff", &source, "-m", &message],
            &[("GIT_EDITOR", "true")],
        )?;
        let snapshot_id = snap.as_ref().map(|s| s.id.as_str());
        let conflicts = out.status != 0;
        let detail = if conflicts { out.stderr.clone() } else { message.clone() };
        record(&st, &id, "merge", &detail.trim(), if conflicts { "conflicted" } else { "ok" }, snapshot_id, start);
        invalidate(&st, &id);
        Ok(OpResult {
            ok: !conflicts,
            message: if conflicts { "Merge stopped with conflicts".into() } else { out.stdout.trim().into() },
            conflicts,
        })
    })
    .await
}

#[tauri::command]
pub async fn resolve_conflict(
    state: State<'_, AppState>,
    id: String,
    path: String,
    side: String,
) -> Result<(), String> {
    let st = state.inner().clone();
    blocking(move || {
        let repo = st.repo_path(&id)?;
        let flag = match side.as_str() {
            "ours" => "--ours",
            "theirs" => "--theirs",
            _ => return Err(AppError::msg("side must be ours or theirs")),
        };
        cli::git_ok(&repo, &["checkout", flag, "--", &path])?;
        cli::git_ok(&repo, &["add", "--", &path])?;
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn mark_resolved(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<(), String> {
    let st = state.inner().clone();
    blocking(move || {
        let repo = st.repo_path(&id)?;
        cli::git_ok(&repo, &["add", "--", &path])?;
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn continue_operation(state: State<'_, AppState>, id: String) -> Result<OpResult, String> {
    let st = state.inner().clone();
    blocking(move || {
        let start = Instant::now();
        let repo = st.repo_path(&id)?;
        let summary = read::summary(&repo)?;
        let args: Vec<&str> = match summary.operation.as_str() {
            "merge" => vec!["merge", "--continue"],
            "rebase" => vec!["rebase", "--continue"],
            "cherry-pick" => vec!["cherry-pick", "--continue"],
            "revert" => vec!["revert", "--continue"],
            "am" => vec!["am", "--continue"],
            "bisect" => vec!["bisect", "--continue"],
            other => return Err(AppError::msg(format!("No operation to continue ({other})"))),
        };
        let out = cli::git_env(&repo, &args, &[("GIT_EDITOR", "true")])?;
        let ok = out.status == 0;
        record(&st, &id, "continue", &summary.operation, if ok { "ok" } else { "failed" }, None, start);
        invalidate(&st, &id);
        Ok(OpResult {
            ok,
            message: if ok { out.stdout.trim().into() } else { format!("{}\n{}", out.stdout, out.stderr).trim().into() },
            conflicts: !ok,
        })
    })
    .await
}

#[tauri::command]
pub async fn abort_operation(state: State<'_, AppState>, id: String) -> Result<OpResult, String> {
    let st = state.inner().clone();
    blocking(move || {
        let start = Instant::now();
        let repo = st.repo_path(&id)?;
        let summary = read::summary(&repo)?;
        let args: Vec<&str> = match summary.operation.as_str() {
            "merge" => vec!["merge", "--abort"],
            "rebase" => vec!["rebase", "--abort"],
            "cherry-pick" => vec!["cherry-pick", "--abort"],
            "revert" => vec!["revert", "--abort"],
            "am" => vec!["am", "--abort"],
            "bisect" => vec!["bisect", "reset"],
            other => return Err(AppError::msg(format!("No operation to abort ({other})"))),
        };
        let out = cli::git(&repo, &args)?;
        let ok = out.status == 0;
        record(&st, &id, "abort", &summary.operation, if ok { "ok" } else { "failed" }, None, start);
        invalidate(&st, &id);
        Ok(OpResult {
            ok,
            message: if ok { "Operation aborted".into() } else { out.stderr.trim().into() },
            conflicts: false,
        })
    })
    .await
}

// ───────────────────────── Reset / revert / cherry-pick ─────────────────────────

#[tauri::command]
pub async fn reset_to(
    state: State<'_, AppState>,
    id: String,
    oid: String,
    mode: String,
) -> Result<OpResult, String> {
    let st = state.inner().clone();
    blocking(move || {
        let start = Instant::now();
        security::validate_ref(&oid)?;
        let flag = match mode.as_str() {
            "soft" => "--soft",
            "mixed" => "--mixed",
            "hard" => "--hard",
            _ => return Err(AppError::msg("mode must be soft, mixed or hard")),
        };
        let repo = st.repo_path(&id)?;
        let snap = make_snapshot(&st, &id, &repo, &format!("reset-{mode}"))?;
        let out = cli::git(&repo, &["reset", flag, &oid])?;
        let ok = out.status == 0;
        record(&st, &id, "reset", &format!("{mode} {oid}"), if ok { "ok" } else { "failed" }, snap.as_ref().map(|s| s.id.as_str()), start);
        invalidate(&st, &id);
        Ok(OpResult {
            ok,
            message: if ok { "Reset complete".into() } else { out.stderr.trim().into() },
            conflicts: false,
        })
    })
    .await
}

#[tauri::command]
pub async fn revert_commit(
    state: State<'_, AppState>,
    id: String,
    oid: String,
) -> Result<OpResult, String> {
    let st = state.inner().clone();
    blocking(move || {
        let start = Instant::now();
        security::validate_ref(&oid)?;
        let repo = st.repo_path(&id)?;
        let snap = make_snapshot(&st, &id, &repo, "revert")?;
        let out = cli::git_env(&repo, &["revert", "--no-edit", &oid], &[("GIT_EDITOR", "true")])?;
        let ok = out.status == 0;
        record(&st, &id, "revert", &oid, if ok { "ok" } else { "conflicted" }, snap.as_ref().map(|s| s.id.as_str()), start);
        invalidate(&st, &id);
        Ok(OpResult {
            ok,
            message: if ok { "Revert complete".into() } else { "Revert stopped with conflicts".into() },
            conflicts: !ok,
        })
    })
    .await
}

#[tauri::command]
pub async fn cherry_pick(
    state: State<'_, AppState>,
    id: String,
    oid: String,
) -> Result<OpResult, String> {
    let st = state.inner().clone();
    blocking(move || {
        let start = Instant::now();
        security::validate_ref(&oid)?;
        let repo = st.repo_path(&id)?;
        let snap = make_snapshot(&st, &id, &repo, "cherry-pick")?;
        let out = cli::git_env(&repo, &["cherry-pick", &oid], &[("GIT_EDITOR", "true")])?;
        let ok = out.status == 0;
        record(&st, &id, "cherry-pick", &oid, if ok { "ok" } else { "conflicted" }, snap.as_ref().map(|s| s.id.as_str()), start);
        invalidate(&st, &id);
        Ok(OpResult {
            ok,
            message: if ok { "Cherry-pick complete".into() } else { "Cherry-pick stopped with conflicts".into() },
            conflicts: !ok,
        })
    })
    .await
}

// ───────────────────────── Stash ─────────────────────────

#[tauri::command]
pub async fn stash_list(state: State<'_, AppState>, id: String) -> Result<Vec<StashInfo>, String> {
    let st = state.inner().clone();
    blocking(move || {
        let repo = st.repo_path(&id)?;
        let out = cli::git_ok(&repo, &["stash", "list", "--format=%gd%x1f%H%x1f%gs%x1f%ct"])?;
        let mut list = Vec::new();
        for line in out.lines() {
            let parts: Vec<&str> = line.split('\u{1f}').collect();
            if parts.len() < 4 {
                continue;
            }
            let index = parts[0]
                .trim_start_matches("stash@{")
                .trim_end_matches('}')
                .parse()
                .unwrap_or(0);
            list.push(StashInfo {
                index,
                oid: parts[1].into(),
                message: parts[2].into(),
                time: parts[3].parse().unwrap_or(0),
            });
        }
        Ok(list)
    })
    .await
}

#[tauri::command]
pub async fn stash_push(
    state: State<'_, AppState>,
    id: String,
    message: Option<String>,
) -> Result<OpResult, String> {
    let st = state.inner().clone();
    blocking(move || {
        let start = Instant::now();
        let repo = st.repo_path(&id)?;
        let msg = message.unwrap_or_else(|| "WIP".into());
        let out = cli::git(&repo, &["stash", "push", "-u", "-m", &msg])?;
        let ok = out.status == 0;
        record(&st, &id, "stash", &msg, if ok { "ok" } else { "failed" }, None, start);
        invalidate(&st, &id);
        Ok(OpResult { ok, message: out.stdout.trim().into(), conflicts: false })
    })
    .await
}

#[tauri::command]
pub async fn stash_apply(
    state: State<'_, AppState>,
    id: String,
    index: usize,
    pop: bool,
) -> Result<OpResult, String> {
    let st = state.inner().clone();
    blocking(move || {
        let start = Instant::now();
        let repo = st.repo_path(&id)?;
        let snap = make_snapshot(&st, &id, &repo, if pop { "stash-pop" } else { "stash-apply" })?;
        let reference = format!("stash@{{{index}}}");
        let out = cli::git(&repo, &["stash", if pop { "pop" } else { "apply" }, &reference])?;
        let ok = out.status == 0;
        record(&st, &id, if pop { "stash-pop" } else { "stash-apply" }, &reference, if ok { "ok" } else { "failed" }, snap.as_ref().map(|s| s.id.as_str()), start);
        invalidate(&st, &id);
        Ok(OpResult { ok, message: out.stdout.trim().into(), conflicts: !ok })
    })
    .await
}

#[tauri::command]
pub async fn stash_drop(state: State<'_, AppState>, id: String, index: usize) -> Result<OpResult, String> {
    let st = state.inner().clone();
    blocking(move || {
        let start = Instant::now();
        let repo = st.repo_path(&id)?;
        let reference = format!("stash@{{{index}}}");
        let out = cli::git(&repo, &["stash", "drop", &reference])?;
        let ok = out.status == 0;
        record(&st, &id, "stash-drop", &reference, if ok { "ok" } else { "failed" }, None, start);
        Ok(OpResult { ok, message: out.stdout.trim().into(), conflicts: false })
    })
    .await
}

// ───────────────────────── Tags / remotes ─────────────────────────

#[tauri::command]
pub async fn tags_list(state: State<'_, AppState>, id: String) -> Result<Vec<TagInfo>, String> {
    let st = state.inner().clone();
    blocking(move || {
        let repo = st.repo_path(&id)?;
        read::tags(&repo)
    })
    .await
}

#[tauri::command]
pub async fn tag_create(
    state: State<'_, AppState>,
    id: String,
    name: String,
    message: Option<String>,
    oid: Option<String>,
) -> Result<(), String> {
    let st = state.inner().clone();
    blocking(move || {
        security::validate_ref(&name)?;
        let repo = st.repo_path(&id)?;
        let mut args = vec!["tag"];
        if let Some(msg) = message.as_deref().filter(|m| !m.trim().is_empty()) {
            args.push("-a");
            args.push(&name);
            args.push("-m");
            args.push(msg);
        } else {
            args.push(&name);
        }
        if let Some(target) = &oid {
            security::validate_ref(target).ok();
            args.push(target);
        }
        cli::git_ok(&repo, &args)?;
        invalidate(&st, &id);
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn tag_delete(state: State<'_, AppState>, id: String, name: String) -> Result<(), String> {
    let st = state.inner().clone();
    blocking(move || {
        security::validate_ref(&name)?;
        let repo = st.repo_path(&id)?;
        cli::git_ok(&repo, &["tag", "-d", &name])?;
        invalidate(&st, &id);
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn remotes_list(state: State<'_, AppState>, id: String) -> Result<Vec<RemoteInfo>, String> {
    let st = state.inner().clone();
    blocking(move || {
        let repo = st.repo_path(&id)?;
        read::remotes(&repo)
    })
    .await
}

#[tauri::command]
pub async fn remote_add(
    state: State<'_, AppState>,
    id: String,
    name: String,
    url: String,
) -> Result<(), String> {
    let st = state.inner().clone();
    blocking(move || {
        security::validate_ref(&name)?;
        security::validate_url(&url)?;
        let repo = st.repo_path(&id)?;
        cli::git_ok(&repo, &["remote", "add", &name, &url])?;
        invalidate(&st, &id);
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn remote_remove(state: State<'_, AppState>, id: String, name: String) -> Result<(), String> {
    let st = state.inner().clone();
    blocking(move || {
        security::validate_ref(&name)?;
        let repo = st.repo_path(&id)?;
        cli::git_ok(&repo, &["remote", "remove", &name])?;
        invalidate(&st, &id);
        Ok(())
    })
    .await
}

// ───────────────────────── Remote ops ─────────────────────────

fn finish_network_op(
    st: &AppState,
    id: &str,
    action: &str,
    detail: &str,
    out: cli::GitOutput,
    snapshot_id: Option<&str>,
    start: Instant,
) -> OpResult {
    let ok = out.status == 0;
    let text = if ok {
        out.stdout.trim().to_string()
    } else {
        format!("{}\n{}", out.stdout, out.stderr).trim().to_string()
    };
    record(st, id, action, detail, if ok { "ok" } else { "failed" }, snapshot_id, start);
    invalidate(st, id);
    OpResult {
        ok,
        message: crate::security::redact(&text),
        conflicts: !ok && text.contains("CONFLICT"),
    }
}

#[tauri::command]
pub async fn fetch_remote(state: State<'_, AppState>, id: String) -> Result<OpResult, String> {
    let st = state.inner().clone();
    blocking(move || {
        let start = Instant::now();
        let repo = st.repo_path(&id)?;
        let out = cli::git(&repo, &["fetch", "--all", "--prune"])?;
        Ok(finish_network_op(&st, &id, "fetch", "all", out, None, start))
    })
    .await
}

#[tauri::command]
pub async fn pull_remote(
    state: State<'_, AppState>,
    id: String,
    mode: String,
) -> Result<OpResult, String> {
    let st = state.inner().clone();
    blocking(move || {
        let start = Instant::now();
        let flag = match mode.as_str() {
            "merge" => "--no-rebase",
            "rebase" => "--rebase",
            "ff-only" => "--ff-only",
            _ => return Err(AppError::msg("mode must be merge, rebase or ff-only")),
        };
        let repo = st.repo_path(&id)?;
        let snap = make_snapshot(&st, &id, &repo, "pull")?;
        let out = cli::git(&repo, &["pull", flag])?;
        Ok(finish_network_op(&st, &id, "pull", mode.as_str(), out, snap.as_ref().map(|s| s.id.as_str()), start))
    })
    .await
}

#[tauri::command]
pub async fn push_remote(
    state: State<'_, AppState>,
    id: String,
    remote: String,
    src: String,
    dst: String,
    force_with_lease: bool,
) -> Result<OpResult, String> {
    let st = state.inner().clone();
    blocking(move || {
        let start = Instant::now();
        security::validate_ref(&remote)?;
        security::validate_ref(&src)?;
        security::validate_ref(&dst)?;
        let repo = st.repo_path(&id)?;
        let snap = make_snapshot(&st, &id, &repo, "push")?;
        let refspec = format!("{src}:{dst}");
        let mut args = vec!["push"];
        if force_with_lease {
            args.push("--force-with-lease");
        }
        args.push(&remote);
        args.push(&refspec);
        let out = cli::git(&repo, &args)?;
        if out.status == 0 {
            // Set upstream so the next push is one click with a known destination.
            let _ = cli::git(&repo, &["branch", "--set-upstream-to", &format!("{remote}/{dst}"), &src]);
        }
        Ok(finish_network_op(&st, &id, "push", &format!("{refspec} -> {remote}"), out, snap.as_ref().map(|s| s.id.as_str()), start))
    })
    .await
}

// ───────────────────────── Search ─────────────────────────

#[tauri::command]
pub async fn search_repo(
    state: State<'_, AppState>,
    id: String,
    query: String,
) -> Result<Vec<SearchHit>, String> {
    let st = state.inner().clone();
    blocking(move || {
        let q = query.trim().to_lowercase();
        if q.len() < 2 {
            return Ok(Vec::new());
        }
        let repo_path = st.repo_path(&id)?;
        let mut hits = Vec::new();

        let graph = {
            let mut graphs = st.0.graphs.lock().map_err(|_| AppError::msg("State lock poisoned"))?;
            if !graphs.contains_key(&id) {
                graphs.insert(id.clone(), RepoGraph::build(&repo_path)?);
            }
            graphs.remove(&id)
        };
        if let Some(graph) = &graph {
            for (oid, summary, author, time) in graph.search_commits(&repo_path, &q, 40) {
                hits.push(SearchHit {
                    kind: "commit".into(),
                    title: summary,
                    subtitle: format!("{author} · {oid:.7} · {}", fmt_time(time)),
                    oid: Some(oid),
                    path: None,
                });
            }
        }
        if let Some(graph) = graph {
            if let Ok(mut graphs) = st.0.graphs.lock() {
                graphs.insert(id.clone(), graph);
            }
        }

        for b in read::branches(&repo_path).unwrap_or_default() {
            if b.name.to_lowercase().contains(&q) {
                hits.push(SearchHit {
                    kind: if b.is_remote { "remote branch".into() } else { "branch".into() },
                    title: b.name.clone(),
                    subtitle: format!("{} · {}", b.tip, b.tip_summary),
                    oid: None,
                    path: None,
                });
            }
        }
        for t in read::tags(&repo_path).unwrap_or_default() {
            if t.name.to_lowercase().contains(&q) {
                hits.push(SearchHit {
                    kind: "tag".into(),
                    title: t.name.clone(),
                    subtitle: t.oid.chars().take(7).collect(),
                    oid: Some(t.oid),
                    path: None,
                });
            }
        }
        let files = cli::git(&repo_path, &["ls-files"]).map(|o| o.stdout).unwrap_or_default();
        for f in files.lines().filter(|f| f.to_lowercase().contains(&q)).take(20) {
            hits.push(SearchHit {
                kind: "file".into(),
                title: f.to_string(),
                subtitle: "tracked file".into(),
                oid: None,
                path: Some(f.to_string()),
            });
        }
        Ok(hits)
    })
    .await
}

fn fmt_time(secs: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let diff = (now - secs).max(0);
    if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86_400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86_400)
    }
}

// ───────────────────────── Ops history & snapshots ─────────────────────────

#[tauri::command]
pub async fn ops_log(state: State<'_, AppState>, id: String) -> Result<Vec<OpsEntry>, String> {
    let st = state.inner().clone();
    blocking(move || {
        let db = st.0.db.lock().map_err(|_| AppError::msg("State lock poisoned"))?;
        store::list_ops(&db, &id, 200)
    })
    .await
}

#[tauri::command]
pub async fn snapshots_list(state: State<'_, AppState>, id: String) -> Result<Vec<SnapshotInfo>, String> {
    let st = state.inner().clone();
    blocking(move || {
        let db = st.0.db.lock().map_err(|_| AppError::msg("State lock poisoned"))?;
        store::list_snapshots(&db, &id)
    })
    .await
}

#[tauri::command]
pub async fn snapshot_restore(
    state: State<'_, AppState>,
    id: String,
    snapshot_id: String,
) -> Result<OpResult, String> {
    let st = state.inner().clone();
    blocking(move || {
        let start = Instant::now();
        let repo = st.repo_path(&id)?;
        let db = st.0.db.lock().map_err(|_| AppError::msg("State lock poisoned"))?;
        let result = snapshots::restore(&db, &repo, &snapshot_id);
        drop(db);
        match result {
            Ok(report) => {
                record(&st, &id, "snapshot-restore", &report, "ok", Some(&snapshot_id), start);
                invalidate(&st, &id);
                Ok(OpResult { ok: true, message: report, conflicts: false })
            }
            Err(e) => {
                record(&st, &id, "snapshot-restore", &e.to_string(), "failed", Some(&snapshot_id), start);
                Err(e)
            }
        }
    })
    .await
}
