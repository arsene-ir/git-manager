# PERFORMANCE_ARCHITECTURE.md — Git Manager (LOCKED: Tauri 2 + React + TS + Vite + Rust + Tokio + Hybrid Git)

Status: planning only. No code, no scaffolding. Stack locked per decision.

Measurement legend (used everywhere): **MEASURED** = observed on a prototype/real build with env recorded. **ESTIMATE** = inference from public reports, not this app. **TARGET** = budget that must be met, enforced in CI/benchmarks. No MEASURED values exist yet for this product.

## 1. Architecture comparison (GPUI vs Tauri 2 + React + Rust vs Slint vs Iced)

### 1.1 One-line profiles

* **GPUI (Zed's framework):** Hybrid immediate/retained, GPU-accelerated native Rust. Designed for editor-grade 60/120fps. Tied to Zed monorepo (`zed-industries/zed/crates/gpui`). Docs are README + Zed crates, not a stable SDK.
* **Tauri 2 + React + Rust:** Rust backend + OS webview (WebView2 Win / WKWebView macOS / WebKitGTK Linux). UI in HTML/CSS/Canvas. Mature 2.x ACL, updater, plugins.
* **Slint:** Declarative native toolkit (Rust/C++/JS), Skia/software/GPU backends. Claims <300 KiB runtime. Strong embedded story, commercial + GPL/royalty-free licensing.
* **Iced:** Elm-inspired pure-Rust, wgpu renderer. Good ergonomics, but known large-list perf cliffs and binary size cost.

### 1.2 Comparison matrix

| Criterion | GPUI | Tauri 2 + React | Slint | Iced |
|---|---|---|---|---|
| Rendering | Native GPU, minimal DOM overhead. Best theoretical frame consistency. | Webview compositor + Canvas. Good, but pays WebView + JS GC + IPC cost. | Native, efficient property system. Very low overhead for forms/lists. | wgpu, retained. OK for moderate UI, degrades with 10k+ widgets if not virtualized. |
| Idle footprint (ESTIMATE) | 80–150 MB RSS bare app; Zed full editor MEASURED publicly ~180–350 MB with project open (not this app). | 90–200 MB total (15–30 MB Rust + 60–170 MB WebView, OS-dependent). Linux WebKitGTK highest. | 30–80 MB RSS typical for desktop app. Lowest. | 70–130 MB RSS. |
| Startup (ESTIMATE, cold SSD) | <500 ms to window. | 0.6–1.5 s (WebView init dominates). | <300–600 ms. | 0.4–0.9 s. |
| Graph / virtualized history | Must build custom GPU list + lane renderer from scratch. No mature virtual list. | Mature: Canvas + `@tanstack/virtual`, proven in Fork/GitKraken-class apps. | Must build custom `ListView` + canvas; feasible but bespoke. | Must build custom scroll + cache; high effort to hit 60fps at 100k rows. |
| Diff / syntax / conflict 3-way | No Monaco/CodeMirror. Must port `tree-sitter` + custom editor. Largest cost. | Monaco read-only or CodeMirror + Shiki worker, xterm.js for terminal escape hatch. Lowest risk. | Basic `TextEdit`; large diff + per-hunk staging = custom work. | `text_editor` widget insufficient for 10k-line diff; custom work. |
| File explorer / search | Custom. | Trivial with React + worker index. | Built-in tree/list components. Good. | Custom. |
| Win/macOS/Linux parity | macOS best; Windows/Linux improved via Zed ports but still churn. Verify before commit. | Best parity: Tier-1 on all three in Tauri 2. | Good parity, incl. embedded. | Good parity, GPU driver variance. |
| API stability / hiring | Unstable, breaking with Zed; tiny hiring pool; docs = Zed source. | Stable, large pool. | Stable, small pool, license review needed. | Stable-ish, small pool. |
| genuinely better? | Only if team accepts building editor/diff/graph stack + tracking Zed upstream. | Best time-to-premium-UI with acceptable overhead if budgets enforced. | Best if absolute idle RAM <80 MB is hard requirement and graph scope is cut. | No advantage over Slint/GPUI for this workload. |

Do not select GPUI just because it is Rust. For this product (graph + large diff + syntax + 3-way merge + terminal), UI ecosystem outweighs language purity.

## 2. Memory and CPU analysis (estimates, not measurements)

Labels: `[ESTIMATE]` = reasoning from public reports (Zed RSS, Tauri example apps, Slint claims). `[TO MEASURE]` = requires prototype in §4.

* Idle, no repo open: Slint 30–60 MB `[ESTIMATE]`; GPUI 80–130 MB `[ESTIMATE]`; Iced 70–110 MB `[ESTIMATE]`; Tauri 90–160 MB `[ESTIMATE]` (Linux high end).
* 1 repo open (Scenario A: 5k commits, 100 files): +20–60 MB Rust caches (revwalk window + status) in all options; Tauri +15–40 MB JS heap if graph windowed correctly `[ESTIMATE]`.
* 5 repos registered, 1 active: must be ~idle + metadata only (SQLite rows, no open `git2::Repository` handles). Budget: +10 MB total `[TARGET]`. Requires lazy open + idle-close (§5). `[TO MEASURE]`.
* 100k commits (Scenario B): never load all commits. Windowed: 500–2000 commit DTOs (~0.5–2 KB each) + lane assignment + refs map = 5–30 MB Rust `[ESTIMATE]`. Full load would be 200–600 MB and is forbidden. `[TO MEASURE]` with `torvalds/linux` and `chromium`-scale synthetic repo.
* 500k+ commits + large diffs + worktrees (Scenario C): same windowing + blob LRU (e.g. 50 blobs, 5 MB cap each) + diff hunk paging. Must stay <500 MB peak `[TARGET]`. `[TO MEASURE]`.
* Startup: native <0.6 s, Tauri <1.5 s `[TARGET]`. Time to first 500 commits: <800 ms cold (no commit-graph cache), <350 ms warm `[TARGET]`.
* Scroll: 60fps sustained on 100k history with virtualized rows + Canvas lanes; no per-row React/GPUI element per commit beyond viewport + overscan `[TARGET]`.
* Diff large file (10k lines / 1 MB): <500 ms to first paint, chunked, highlight in worker, cap tokenization `[TARGET]`.
* Idle CPU: <1% (timer coalesced, watcher suspended when minimized) `[TARGET]`; watcher active avg <3% `[TARGET]`; GPU mem <150 MB `[TARGET]`; 8h growth <10% vs baseline `[TARGET]`.

Uncertainty: WebView GC, WebKitGTK on Linux, GPU driver, and `libgit2` object cache behavior cannot be judged without a prototype. §4 defines how to measure.

## 3. Rendering strategy (LOCKED for Tauri v1)

Hard constraints:
* Git graph lanes computed in Rust (`GitEngine::graph_window()` returns `{oid, parents, lanes, lane_events, refs}`). Frontend never computes lanes. Renderer is a single Canvas (history + lanes in one pass, viewport + overscan only). **Forbidden: one React component per commit.**
* All large surfaces virtualized + lazy-loaded: commit history, file lists, diff lines/hunks, blame lines, search results, reflog, worktrees. Data paged from Rust in 500–1000 item windows with `CancellationToken`; filter change cancels prior job.

1. **Git graph:** Rust lane assignment over topological+time order. Frontend Canvas draws lanes/dots/edges for visible rows only. Minimap is downsampled bitmap. Zoom = lane-width scale, not re-layout.
2. **Commit history:** `tanstack/virtual` over Rust window. Fixed/measured-cached row height. Search/filter in worker over Rust index, not over rendered rows.
3. **Diff viewer:** Rust returns hunks `{path, old/new ranges, lines[]}` paged per file; virtualized lines, highlight in Web Worker (Shiki) per visible viewport only, cancellable. Large/binary truncated with “show more”. Image diff = canvas compare.
4. **Syntax highlighting:** async worker, per-viewport, cache per blob OID + theme with LRU cap.
5. **3-way conflict:** three read-only virtualized panes (base/ours/theirs) + one editable result pane. Line mapping from Rust.
6. **File explorer / search results:** flat virtual list over `git status` + `ls-files`; memoized icons; 150 ms debounce; SQLite FTS for app-level recall + in-memory window for current rev.

Rule: thousands of commits/files/diff lines must never become thousands of live UI objects.

## 4. Benchmark methodology (reproducible, no invented results)

### 4.1 Environment (pin and record)
* Machines: Win11 WebView2 stable, macOS M-series WKWebView, Ubuntu 24.04 WebKitGTK (for Tauri); same hardware for native builds. Record OS, WebView version, Rust `1.8x`, `git --version`.
* Repos: (a) synthetic A: 5k commits/100 files scripted; (b) B: clone `torvalds/linux` truncated to 100k or `git/git`; (c) C: 500k synthetic + large blobs + 3 worktrees. Record `count-objects -v`, `.git` size.
* Harness: cold (drop OS cache note) vs warm runs, 5 repetitions, report median+p95. Tools: OS RSS (`Get-Process`, `ps`, `heaptrack`), Rust `dhat`/tracing alloc, `cargo instruments`, `perf`, webview devtools heap (Tauri), FPS via `requestAnimationFrame` counter or native frame timer.

### 4.2 Workloads and metrics
* Startup to window; first 500 commits paint; scroll 10k rows FPS + jank; diff 10k-line file paint; `status` after 100-file touch; background watcher CPU over 10 min idle; 8h soak RSS growth.
* Thresholds = §5 budgets. Fail = block P1.

### 4.3 Uncertainty statement
No candidate can be ranked by measurement without building two throwaway prototypes: (1) Tauri graph+diff slice, (2) GPUI or Slint graph+diff slice, same Rust Git engine. Estimate 3–5 days each. Until then all comparisons are estimates.

## 5. Performance budgets + cache eviction (STRICT, enforced in CI)

| State | TARGET RSS |
|---|---|
| Idle, no repo | ≤180 MB Win/mac, ≤220 MB Linux [TARGET] |
| 1 repo Scenario A (5k) | ≤280 MB [TARGET] |
| 5 registered, 1 active | ≤200 MB (others = SQLite rows only) [TARGET] |
| 100k windowed (Scenario B) | ≤400 MB peak [TARGET] |
| 500k+ (Scenario C) windowed | ≤500 MB peak [TARGET] |

* Timings [TARGET]: startup ≤1.2 s; first 500 ≤800 ms cold / ≤350 ms warm; search keystroke→paint ≤150 ms; diff first paint ≤500 ms; scroll ≥55fps p50 on B.
* Runtime [TARGET]: idle CPU <1%; watcher <3% avg; GPU <150 MB; 8h growth <10%.
* Eviction (hard caps): commit window LRU 5k DTOs; blob LRU 50 items / 250 MB total; diff hunk LRU 20 files; highlight cache 100 entries; `libgit2` odb cache cleared on repo idle; temp worktrees/preview trees deleted on job end. Exceeding cap = evict oldest, never grow unbounded.
* Enforcement: `criterion` benches for lane layout + diff chunking; RSS/time assertions on pinned runner; Tauri `removeUnusedCommands` + `opt-level=s`, `lto=true`, `strip=true`.

## 6. Resource management + SQLite scope + repo-switch disposal + engine isolation

* **Single `RepoManager`:** one open handle per path, `DashMap<RepoId, Arc<RwLock<Handle>>>` + per-repo writer mutex. Generation counter prevents duplicate indexing. Event-driven `notify` only, 250 ms debounce, suspend on minimize/unfocus, close handle after idle timeout.
* **SQLite scope (strict):** stores app metadata (repos, settings), FTS search index (refs/paths, capped), `ops_log`, `snapshots` metadata + backup-ref names. **Never duplicates full Git history.** History always re-derived via revwalk; SQLite holds no per-commit object store.
* **Repo-switch disposal (mandatory):** on switch/close frontend must: cancel jobs via `CancellationToken`, terminate/reuse highlight/search workers, clear virtualizer cache, drop blob/diff/highlight window, call `engine.release(repo)` to close handle + clear LRU shard. Verified by soak test (switch A↔B ×50, growth <5%).
* **Lazy + cancel:** all revwalk/diff/blame paginated; Job Center shows pending/running/failed/done.
* **Replaceable renderer:** `GitEngine` exposes versioned DTOs (`GraphWindow, DiffHunks, StatusEntries`) over Tauri IPC/events. React Canvas layer is swappable; engine has no DOM/React dependency. Future GPUI/Slint renderer reuses same engine + DTOs.
* **No polling, no leaks:** compact `serde` DTOs; `Drop` closes repos; soak asserts no growth.

## 7. Technical risks

1. Tauri Linux WebKitGTK variance (mem/CPU higher) — mitigate with strict virtualization + Linux CI gate.
2. JS heap growth from retained diff/graph nodes — mitigate with windowing + dispose audits.
3. `libgit2` large-pack memory maps + index churn — mitigate with CLI fallback for huge ops + cache caps.
4. GPUI API churn + missing editor/diff primitives — 3–6 mo risk, no stable docs.
5. Slint bespoke graph/diff cost — viable but slower to premium UX.
6. GPU driver variance (Iced/wgpu, GPUI Blade) on Intel iGPU.

## 8. Final recommendation (LOCKED)

**Tauri 2 + React + TS + Vite + Rust + Tokio + Hybrid Git Engine for v1.** GPUI/Slint/Iced retained only as future renderer options behind the versioned `GitEngine` DTO boundary; no engine rewrite required to swap renderers.

**GPUI verdict:** not better fit for v1 despite lower theoretical overhead — forces rebuilding virtual list, diff editor, highlight, and terminal on an unstable Zed-tied API with weakest Win/Linux guarantees. Revisit only if MEASURED Tauri fails §5 gates after the throwaway-slice benchmarks.

Next step (no product code): single Tauri throwaway slice to record first MEASURED values into §2, then lock P0.
