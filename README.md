# Git Manager

A fast, safe, visual Git desktop manager. Do your entire Git workflow without the terminal, VS Code, or another Git client.

Built with [Tauri 2](https://tauri.app/) (Rust backend) + React + TypeScript (frontend). Local-first, no account required. System `git` is the source of truth for mutations.

## Features

- **Multi-repo workspace** — open, clone, add, remove repos; status at a glance
- **Visual commit graph** — Rust lane layout on canvas, virtualized history
- **Stage & commit** — stage by file/hunk, commit/amend
- **Branches & tags** — create, switch (with stash-on-dirty), track, delete; lightweight/annotated tags
- **Safe merge** — preview via `merge-tree --write-tree` before touching the worktree
- **Push / pull / fetch** — explicit `src -> dst` confirmation, merge / rebase / ff-only pull
- **Conflicts** — ours/theirs resolution, continue/abort in-progress operations
- **Stash, reset, revert, cherry-pick**
- **Undo & Recovery Center** — snapshot before every destructive op + ops history log
- **Command palette & search** (`Ctrl/Cmd+K`) — actions, branches, commits, files
- **Background jobs** — clone/fetch/push run async with progress

See `docs/` for the full spec: `PRODUCT_VISION.md`, `FEATURE_SPECIFICATION.md`, `TECHNICAL_ARCHITECTURE.md`, `GIT_OPERATION_MATRIX.md`, `UX_FLOWS.md`, `PERFORMANCE_ARCHITECTURE.md`, `IMPLEMENTATION_ROADMAP.md`.

## Tech stack

| Layer | Technology |
|---|---|
| Desktop shell | Tauri 2 |
| Backend | Rust (`git2`, `tokio`, `rusqlite`, `notify`) |
| Frontend | React 18, Zustand, `@tanstack/react-virtual`, Vite 5 |
| Git mutations | System `git` CLI; reads via `git2` |
| State | SQLite workspace store + file watcher auto-refresh |

## Prerequisites

- Node.js 18+ and npm
- Rust stable toolchain
- System `git` on `PATH` (`git --version`)
- Tauri system deps per OS ([Tauri prerequisites](https://tauri.app/start/prerequisites/))

## Getting started

```bash
npm install
npm run tauri dev
```

Production build:

```bash
npm run tauri build
```

Frontend-only (no Rust, backend calls will fail):

```bash
npm run dev
npm run build
npm run preview
```

## Project structure

```
src/                 React frontend (App.tsx, api.ts, store.ts, types.ts, components/)
src-tauri/           Rust backend (commands.rs, git/, store.rs, security.rs)
src-tauri/capabilities/  Tauri permission sets
dist/                Vite build output (frontendDist)
release/             Local release artifacts
docs/                Product / architecture / roadmap specs
```

Key entry points:

- `src/App.tsx` — workspace layout, dialogs, palette, toasts
- `src/api.ts` — typed Tauri `invoke` wrappers + `repo://changed` / `job://update` events
- `src-tauri/src/lib.rs`, `commands.rs` — Tauri command surface
- `src-tauri/src/git/` — git operations against system git
- `src-tauri/tauri.conf.json` — window, CSP, bundler config (`dev.gitmanager.app`)

## Safety model

- Destructive ops require explanation + preview + snapshot + confirm.
- `SnapshotManager` records a snapshot ID before running.
- Merge safety checked with trial merge outside the worktree.
- Credentials stay with git (credential helper / SSH agent) — the app never stores them.

## Scripts

| Command | Purpose |
|---|---|
| `npm run dev` | Vite dev server (port 1420) |
| `npm run build` | `tsc && vite build` |
| `npm run preview` | Preview production frontend |
| `npm run tauri` | Tauri CLI (`dev`, `build`) |

## Notes

- App window defaults to 1440×900 (min 1000×640), dark theme.
- Shortcuts: `Ctrl/Cmd+K` palette, `Ctrl/Cmd+R` refresh, `Ctrl/Cmd+1..4` switch Changes/Commit/Conflicts/Recovery tabs.
