# IMPLEMENTATION_ROADMAP.md — Git Manager SSOT

## P0 Foundation (2–3 wks) — exit: open/clone/status/watcher/shell
- Tauri2+Vite+React+TS shell, `GitEngine` trait, `RepoManager`, `JobManager`, SQLite schema (`repos, settings, search_fts, ops_log, snapshots`), logging/redaction, CI Win/mac/Linux, `git>=2.40` check.
- Tests: engine trait mocks, path-validation, redaction.

## P1 History + Graph (3 wks) — exit: perf gates pass on B
- revwalk window + Rust lanes + Canvas + virtual list + commit detail + branch/tag list + global search v1.
- Tests: lane snapshots on real repos, `criterion` benches, RSS/time CI asserts.

## P2 Changes + Remotes (3 wks) — exit: daily driver commit/push
- status/diff chunked + stage hunk + commit/amend + push/fetch/pull wizards + stash + tags basic + blame viewer + config identity + gitignore basic + patch basic + Job/Undo/Ops v1.
- Tests: CLI parity (`trycmd`), hook-fail, auth-fail redaction.

## P3 Branches + Merge + Conflicts (3 wks)
- branch CRUD/track, `merge-tree` preview, text conflicts, continue/abort, stash named, remotes multi.
- Tests: conflict taxonomy fixtures (rename/delete, binary, mode, submodule), abort-restore.

## P4 Safety Hardening (2 wks)
- SnapshotManager full (untracked caps, retention), reflog view, reset/cherry-pick/revert with gate, kill-9/interrupt tests, soak 8h.

## Post-MVP (ordered)
P5 rebase-interactive/squash/reorder; P6 worktrees adv + repo health + signing full + templates; P7 bisect + terminal + multi-account; P8 providers (GitHub/GitLab) + AI optional + image diff.

## Dependencies / risks
- `merge-tree --write-tree` needs git≥2.38 else fallback temp-worktree trial.
- WebKitGTK variance → Linux CI gate.
- Credential matrix → delegate to git, per-OS manual QA.
- libgit2/CLI divergence → CLI golden tests block release.

## Test strategy
Rust `cargo test` + `trycmd`, Vitest, Playwright, manual matrix (100k linux, monorepo, LFS, submodules). Destructive gate + perf budgets are release blockers.
