# TECHNICAL_ARCHITECTURE.md — Git Manager SSOT (LOCKED)

## 0. Locked stack
Tauri 2 + React + TypeScript + Vite (SPA, no SSR) + Rust + Tokio + `git2@0.21` (reads) + system Git CLI `>=2.40` (writes) + SQLite (`rusqlite`) + `notify` + Zustand + TanStack Query/Virtual + Canvas + Shiki worker. See `PERFORMANCE_ARCHITECTURE.md` for why Tauri locked and GPUI rejected for v1.

## 1. Process model
- `main`: Tauri core, single instance, updater, deep-link.
- `backend`: Tokio runtime, `RepoManager`, `JobManager`, `SnapshotManager`, `GitEngine` trait.
- `frontend`: React SPA served as static dist. IPC via `invoke` + events. No Node sidecar.
- `git`: spawned `git` processes only. No daemon.

## 2. Rust backend crates/modules
- `engine::{LibGitEngine (reads), CliEngine (writes)}` behind `trait GitEngine`.
- `graph::layout` — lane assignment (active-lanes map, merge-lane reuse). Input: topo+time revwalk window. Output: `GraphWindow{commits[], lanes[], lane_events[]}`. Versioned DTO (`v1`).
- `jobs` — `JobManager{DashMap<JobId, CancelToken>, per-repo Mutex<()>}`. One writer per repo, concurrent readers. Progress via `job://progress` events.
- `snapshots` — `SnapshotManager` (§4).
- `watch` — `notify` debounced 250ms, active-repo only, suspend on blur/minimize.
- `store` — SQLite: `repos, settings, search_fts, ops_log, snapshots`. Never per-commit object store.
- `security` — arg-array spawn, path validation, redaction, Tauri ACL minimal.

## 3. Hybrid Git rule (normative)
- libgit2: open/discover, status, revwalk/log, refs/branches/tags lookup, diff stats, blame, ahead/behind, worktree list, config read.
- CLI: clone/fetch/pull/push, merge, rebase (esp. interactive), cherry-pick/revert/sequencer, reset, stash push/apply, submodules, LFS, hooks, worktrees write, reflog/fsck, `merge-tree --write-tree` preview, `credential` flows.
- `gix`: read-only experiments only. No mutations in v1.

## 4. SnapshotManager (normative)
- `committed`: `refs/backups/<op>-<ts>` via `update-ref`.
- `staged+unstaged`: `git stash create` / `write-tree` + `commit-tree` → `refs/backups/snapshot-<id>`. Restore via `read-tree`/`checkout-index` or `stash apply`.
- `untracked`: copy to `<app-data>/snapshots/<id>/untracked/` with caps (500MB, deny `node_modules/target/.git`).
- `sequencer`: preserve `MERGE_HEAD/CHERRY_PICK_HEAD/REBASE_*` + `ORIG_HEAD`.
- Table `snapshots(id, repo, kind, ref, path, created_at, expires_at)`. 30d retention. Every destructive op must log snapshot ID in `ops_log` or claim non-recoverable explicitly.

## 5. IPC / data flow (versioned, renderer-swappable)
- `invoke`: `open_repo, graph_window{repo, cursor, limit}, status, diff_hunks{path, page}, stage_hunk, commit, push{src,dst}, merge_preview{src,dst}, snapshot_restore`.
- Events: `repo://changed, job://progress, job://done`.
- DTOs are `serde` compact, versioned (`v1`). React Canvas is swappable; engine has zero DOM dependency. Future native renderer reuses same DTOs.
- Frontend disposal on repo switch (mandatory): cancel tokens, terminate/reuse workers, clear virtualizer + blob/diff/highlight caches, call `release(repo)`.

## 6. Frontend architecture
- Routes: Sidebar (repos) | Graph Canvas + virtual list | Tabs: Changes / Commit / Conflicts / Search.
- State: Zustand (active repo, selection) + TanStack Query (server cache per repo+cursor). No repo state retained after switch.
- Workers: `highlight.worker (Shiki)`, `search.worker`. Single Canvas for graph, virtualized rows (`tanstack/virtual`). Forbidden: component-per-commit.
- Job Center + Ops History + Undo Center read from `ops_log` events.

## 7. Security (normative)
- `Command::new("git").args(array)` only. No shell/`cmd /C`. No interpolation.
- Validate repo path canonicalized under allowed roots; reject `..`, `--upload-pack=` etc. Allow-list CLI flags.
- Tauri capabilities minimal: `fs:allowAppData`, dialog, opener. No wildcard shell/http unless providers feature enabled.
- Hooks = untrusted: show stdout/stderr, timeout, allow `--no-verify` only if repo config permits.
- Redact tokens/credentials in logs, errors, `ops_log`.

## 8. Perf linkage
All of `PERFORMANCE_ARCHITECTURE.md` §3/§5/§6 normative: Rust lanes, Canvas, virtualization, LRU caps (5k commits, 50 blobs/250MB, 20 diff files, 100 highlights), idle-close handles, no polling. MEASURED/ESTIMATE/TARGET legend applies.
