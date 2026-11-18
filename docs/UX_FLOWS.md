# UX_FLOWS.md — Git Manager SSOT

## 0. Layout (locked)
Sidebar repos (status dot) | Center graph Canvas + virtual history | Right tabs: Changes / Commit / Conflicts / Search. Top: branch picker + Fetch/Pull/Push + `Ctrl+K` palette. Job Center (bottom/toast) shows pending/running/failed/done. Undo Center + Ops History global.

## 1. Push
Click Push → wizard shows `main -> origin/main (2 ahead, 0 behind)` + files/outgoing list → Confirm. Never auto-push to unexpected dst. Force-with-lease hidden behind typed confirm + snapshot link.

## 2. Pull/Fetch
Fetch (silent refresh) vs Pull = Fetch + Merge/Rebase choice. Shows incoming list + conflict probability via `merge-tree` count. Dirty worktree → offer Stash&Continue.

## 3. Switch branch
If clean: instant switch + refresh. If dirty: modal explains (staged/unstaged/untracked counts) + [Stash & Switch] [Switch Anyway (blocked if would overwrite)] [Cancel]. Detached HEAD needs explicit confirm.

## 4. Merge
Pick source→target visually → Preview panel: `merge-base`, diffstat, `merge-tree --write-tree` result (clean OID or conflict file list + messages) → [Merge] creates snapshot first → on conflict jump to Conflicts tab with continue/abort. Never use `--no-commit` as preview.

## 5. Reset
Radio: Keep staged (`--soft`) / Keep unstaged (`--mixed`) / Discard (`--hard`, typed confirm + snapshot + untracked warning). Explains each. Undo Center restores snapshot.

## 6. Commit
Changes list → stage file/hunk → message (+template later) + signing status → Commit. Amend shows original preserved in snapshot. Hook failure shows stderr + allow `--no-verify` only if permitted.

## 7. Conflict resolution
File list with type badges (text/rename-delete/binary/mode/submodule) → 3 panes + result → Accept L/R/Both or edit → Mark resolved → Continue (blocked if markers) / Abort (restores snapshot). Binary forces side choice.

## 8. Recovery
Undo Center lists snapshots (`op, time, ref/path, restore`) + `ORIG_HEAD`/backup refs + reflog. Restore = dry-run first, then apply + refresh. Never claims recoverable without ID.

## 9. Search / Palette / Shortcuts
`Ctrl+K`: commits/branches/tags/hash/files + all actions. Global search virtualized, debounced, ≤150ms. Shortcuts: commit, push, pull, fetch, switch, stash, palette, undo. Terminal (later) reuses lock.

## 10. Empty/error states
No repo → drop folder/open/clone CTA. Large repo → progress + cancel + windowed notice. Auth fail → helper hint, no secret echo. All errors link to Ops History entry.
