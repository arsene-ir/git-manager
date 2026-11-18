# PRODUCT_VISION.md — Git Manager SSOT

## 1. Goal
A premium Tauri 2 desktop Git manager that lets a developer do their entire Git workflow without VS Code, terminal, or another Git client. Visual, safe, fast, all-day resident.

## 2. Non-goals
- Not a code editor, not a CI system, not a hosting platform.
- No re-implementation of `git` semantics. System `git` is source of truth for mutations.
- AI optional only. Core works offline with no API key.

## 3. Target users
- Daily Git users who know basic branch/commit/push but avoid rebase/conflict CLI.
- Leads reviewing history across repos/worktrees.
- Windows/macOS/Linux parity required.

## 4. Core promise
> "I can manage my entire Git workflow here. I don't need to open VS Code or remember Git commands."

Measured by: common flows (commit, switch, push, pull, merge, stash, conflict resolve) completable in ≤3 clicks/dialogs with explicit destination + preview + undo.

## 5. Workflows in scope (MVP first)
1. Open/clone/add/remove repos, multi-repo workspace, status at a glance.
2. Visual graph + history search + commit detail.
3. Stage (file/hunk) → commit/amend with signing status.
4. Branch create/switch/track/merge with safe preview.
5. Push/pull/fetch with explicit `src -> dst` confirmation.
6. Stash, tags basic, blame, diff (working/HEAD, commit/commit, branch/branch).
7. Conflict resolution (text add/ours/theirs/both) + continue/abort.
8. Undo & Recovery Center + Ops History + Job Center.

Advanced (post-MVP): interactive rebase/squash/reorder, cherry-pick batch, reflog time-machine full, bisect wizard, worktrees advanced, repo health full, GPG/SSH full verify, commit templates reusable, multi-account UI, integrated terminal, GitHub/GitLab module, image diff polish.

## 6. Differentiation (SSOT)
- Preview-before-execute everywhere merge/reset/rebase/push use.
- Human-readable consequences (Keep staged / Keep unstaged / Discard).
- SnapshotManager: every destructive op records snapshot ID before running.
- Merge preview via `merge-tree --write-tree` (never trial-merge in worktree).
- Rust lane layout + Canvas graph, virtualized everywhere.
- Strict budgets: idle ≤180MB Win/mac, 8h growth <10%.

## 7. Competitor gaps exploited
- GitKraken/Fork/SourceTree: heavy, unclear push destination, weak undo. We fix with explicit wizard + Undo Center.
- GitHub Desktop: too simple, no graph power. We keep simplicity but add lane graph + preview.
- LazyGit: powerful but TUI. We keep keyboard-first + palette in GUI.

## 8. Acceptance
MVP ships only if: all MVP flows meet §4 click budget, pass destructive-op gate (explanation + preview + snapshot + confirm + failure tests), and meet `PERFORMANCE_ARCHITECTURE.md` §5 budgets.
