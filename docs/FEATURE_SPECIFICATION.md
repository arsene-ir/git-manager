# FEATURE_SPECIFICATION.md — Git Manager SSOT

Conventions: each feature has Stories + Acceptance + Edge cases. Destructive ops must also meet the Destructive Gate (§12).

## A. Repository Management (MVP)
- Open/discover/clone/add/remove, multi-repo workspace, status dot, HEAD/remote/worktree display, worktree list.
- Accept: open 50k repo <2s to first 500; invalid path error without crash; `safe.directory` hint on Win.
- Edge: nested repos, bare repos (read-only notice), missing git binary.

## B. Visual Graph (MVP, perf-critical)
- Lane graph, branches, merges/divergence, HEAD, local/remote, tags, zoom/pan, click→detail, ahead/behind, compare two commits.
- Normative: Rust lane calc + Canvas renderer, paged 500–1000, virtualized. No component-per-commit.
- Accept: first 500 ≤800ms cold; 100k scroll ≥55fps p50; zoom does not re-layout.
- Edge: 500k history windowed; thousands of branches (refs map cap + search).

## C. Branch Management (MVP)
- Create/switch/rename/delete, publish/track, compare, merge (preview), status, merged/unmerged, divergence viz.
- Switch dirty → block + offer Stash&Switch / Discard (confirm) / Cancel. Never silent overwrite.
- Edge: detached HEAD warning; delete unmerged requires snapshot + confirm.

## D. Commit Management (MVP core, advanced later)
- MVP: stage file/hunk, line/hunk via `apply --cached`, commit/amend, history, revert (CLI), reset soft/mixed/hard with human labels.
- Later: interactive rebase/squash/reorder, cherry-pick batch.
- Accept: hooks output shown; amend preserves author unless changed; reset-hard needs typed confirm + snapshot.
- Edge: empty commit blocked; GPG fail surfaces stderr.

## E. Diff & Changes (MVP)
- Modified/added/deleted/renamed, side-by-side/inline, syntax, hunk/file stage, rename detect, working-vs-HEAD, commit/commit, branch/branch.
- Normative: libgit2 stats + CLI `diff -U3 -M` content, chunked paging, worker highlight, 10k-line first paint ≤500ms. Truncate + "show more".
- Later: image diff polish.

## F. Remotes (MVP)
- Add/remove, fetch/pull/push, upstream set, multi-remote, tracking status, incoming/outgoing, SSH/HTTPS via system git + ssh-agent. Never silent destination.
- Push wizard: `src -> dst` explicit + force-with-lease only with typed confirm.
- Edge: auth fail → credential helper hint, no token logging.

## G. Stash (MVP)
- Create/apply/pop/delete, named, content preview. Snapshot-backed.

## H. Conflict Resolution (MVP text, extended taxonomy)
- Auto-detect, list files, base/ours/theirs panes + editable result, Accept L/R/Both, manual edit, Mark resolved, continue/abort with `RepositoryState` check.
- Must cover: text, rename/delete, delete/modify, binary (choose side, no markers), mode change, submodule (no merge-base), directory/file. Marker detector post-merge.
- Edge: abort restores snapshot; continue blocked if markers remain (override explicit).

## I. Safety & Recovery (MVP)
- Undo & Recovery Center (snapshots + backup refs + `ORIG_HEAD`), reflog view, restore deleted branch/commit, `fsck --lost-found` (later full).
- Ops History: every op + args (redacted) + outcome + duration + snapshot ID in SQLite.

## J. Added capabilities (from gap review)
- Blame (MVP viewer: libgit2 + CLI `--ignore-rev`); Bisect (later wizard, isolated, warns checkout side-effects).
- Tags (MVP: create/delete/push/fetch/inspect annotated/signed; rename = create+delete).
- Config Manager (MVP identity only; later signing/aliases/credential.helper safe edit).
- `.gitignore` visual editor (MVP basic + template insert).
- Signing (MVP status via `log --show-signature`; later full GPG/SSH setup).
- Patches (MVP export/import `format-patch`/`am`/`apply`).
- Templates (later reusable).
- Worktrees advanced (later: add/remove/lock/prune/repair).
- Repo Health (later full: `fsck`, `count-objects`, large-blob scan, LFS check).
- Multi-account (later, delegated to git helpers + OS keychain).
- Global search (MVP: commits/branches/tags/hash/paths, FTS + window index, ≤150ms).
- Terminal escape hatch (later: `xterm.js` + local shell reusing repo lock + refresh).
- Shortcuts + palette (MVP core map), Job Center statuses (MVP).
- Providers GitHub/GitLab (later optional module, offline core).

## 12. Destructive Gate (all destructive ops)
Must have: explanation, preview if possible, snapshot ID, explicit confirm (typed for hard reset/force-push), failure/interrupt tests (kill -9, hook fail, disk-full). No claim of recoverable without preserved data.
