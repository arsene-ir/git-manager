# GIT_OPERATION_MATRIX.md — Git Manager SSOT

Engine: `L` = libgit2, `C` = CLI. Preview via `merge-tree` only. Snapshot: `B`=backup ref, `S`=stash/write-tree snapshot, `U`=untracked copy, `N/A`=read-only.

| Operation | Engine | Preview | Snapshot | Confirm | Notes / errors |
|---|---|---|---|---|---|
| open/discover/status/revwalk/log | L | N/A | N/A | none | cancellable; large repo windowed |
| diff stats | L | N/A | N/A | none | cap files/size, lazy |
| diff content | C `diff -U3 -M` | N/A | N/A | none | paged hunks |
| stage file | L/C | diff | S | none | never stage markers silently |
| stage hunk/line | C `apply --cached` | hunk | S | none | validate hunk offsets |
| commit/amend | C | staged diff | B+S | normal | show hooks; amend snapshots original |
| branch create/switch/rename/delete | L+C(track) / C(switch) | divergence | B | typed if unmerged delete | dirty switch offers stash |
| merge | C | `merge-tree --write-tree` + merge-base/diffstat | B+S+U | normal; abort available | FORBIDDEN preview via `--no-commit`; fallback temp-worktree trial |
| rebase/interactive | C | step list | B+S+U | explicit | backup tag; never auto-skip |
| cherry-pick/revert | C | commit diff | B+S | normal | sequencer state preserved |
| reset soft/mixed/hard | C | affected files | B+S+U (hard) | typed for hard | human labels |
| fetch/pull/push | C | incoming/outgoing | B (pull/push) | explicit dst; typed force | force-with-lease only; redact creds |
| stash push/apply/pop | C | stash diff | S | normal | named supported |
| tag create/delete/push/inspect | L+C(push) | list | B | normal | rename=create+delete; signed verify via log |
| blame | L+C(ignore-rev) | N/A | N/A | none | paged lines |
| bisect | C | N/A (later wizard) | B+S | explicit | warns checkout side-effects |
| remotes add/rm | C | N/A | N/A | normal | validate URL |
| submodules/LFS/hooks | C | N/A | B+S | explicit | delegate; hooks untrusted |
| worktrees list/add/remove/lock | L(list)+C(write) | N/A | S | explicit | per-worktree status |
| patches export/import | C `format-patch/am/apply` | file list | S | normal | check whitespace errors |
| config read/write | L(read)+C(write) | N/A | N/A | normal | allow-list keys; mask secrets |
| gitignore edit | FS+C `check-ignore` | match test | S | none | template insert |
| signing status | C `log --show-signature` | N/A | N/A | none | GPG/SSH delegated |
| reflog/fsck/recovery | C | N/A | restore dry-run | explicit | `ORIG_HEAD`, lost-found |
| Garbage: full history in SQLite | FORBIDDEN | — | — | — | SQLite holds metadata/FTS/ops/snapshots only |
