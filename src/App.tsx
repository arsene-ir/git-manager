import { useCallback, useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, errorText, events } from "./api";
import { useStore } from "./store";
import type { RepoInfo, SystemInfo } from "./types";
import { GraphCanvas } from "./components/GraphCanvas";
import { Sidebar } from "./components/Sidebar";
import { Toolbar } from "./components/Toolbar";
import { ChangesPanel } from "./components/ChangesPanel";
import { CommitPanel } from "./components/CommitPanel";
import { ConflictsPanel } from "./components/ConflictsPanel";
import { RecoveryPanel } from "./components/RecoveryPanel";
import { PushModal } from "./components/PushModal";
import { MergeModal } from "./components/MergeModal";
import { PullModal } from "./components/PullModal";
import { ResetModal } from "./components/ResetModal";
import { CloneModal } from "./components/CloneModal";
import { PromptModal } from "./components/PromptModal";
import { Palette } from "./components/Palette";
import type { PaletteAction } from "./components/Palette";
import { Modal } from "./components/Modal";

type Dialog =
  | { kind: "push" }
  | { kind: "pull" }
  | { kind: "merge" }
  | { kind: "clone" }
  | { kind: "reset"; oid: string }
  | { kind: "branch" }
  | { kind: "tag" }
  | { kind: "remote" }
  | { kind: "switch"; name: string }
  | null;

export default function App() {
  const repos = useStore((s) => s.repos);
  const activeId = useStore((s) => s.activeId);
  const summary = useStore((s) => s.summary);
  const changes = useStore((s) => s.changes);
  const selectedOid = useStore((s) => s.selectedOid);
  const tab = useStore((s) => s.tab);
  const graphKey = useStore((s) => s.graphKey);
  const jobs = useStore((s) => s.jobs);
  const toasts = useStore((s) => s.toasts);
  const searching = useStore((s) => s.searching);
  const setRepos = useStore((s) => s.setRepos);
  const setActive = useStore((s) => s.setActive);
  const setSummary = useStore((s) => s.setSummary);
  const setChanges = useStore((s) => s.setChanges);
  const selectCommit = useStore((s) => s.selectCommit);
  const setTab = useStore((s) => s.setTab);
  const pushJob = useStore((s) => s.pushJob);
  const toast = useStore((s) => s.toast);
  const dismissToast = useStore((s) => s.dismissToast);
  const bumpGraph = useStore((s) => s.bumpGraph);
  const setSearching = useStore((s) => s.setSearching);

  const [system, setSystem] = useState<SystemInfo | null>(null);
  const [busy, setBusy] = useState(false);
  const [dialog, setDialog] = useState<Dialog>(null);
  const activeRef = useRef<string | null>(null);
  const summaryRef = useRef(summary);
  const refreshTimer = useRef<number | null>(null);

  activeRef.current = activeId;
  summaryRef.current = summary;

  const refresh = useCallback(async () => {
    const id = activeRef.current;
    if (!id) return;
    try {
      const [s, ch] = await Promise.all([api.repoSummary(id), api.status(id)]);
      if (activeRef.current !== id) return;
      setSummary(s);
      setChanges(ch);
    } catch (e) {
      toast("error", errorText(e));
    }
  }, [setChanges, setSummary, toast]);

  const selectRepo = useCallback(
    async (id: string) => {
      setActive(id);
      activeRef.current = id;
      try {
        const [s, ch] = await Promise.all([api.activateRepo(id), api.status(id)]);
        setSummary(s);
        setChanges(ch);
        bumpGraph();
      } catch (e) {
        toast("error", errorText(e));
      }
    },
    [bumpGraph, setActive, setChanges, setSummary, toast],
  );

  useEffect(() => {
    api.systemInfo().then(setSystem).catch(() => setSystem(null));
    api
      .listRepos()
      .then((list) => {
        setRepos(list);
        if (list[0]) void selectRepo(list[0].id);
      })
      .catch((e) => toast("error", errorText(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    events
      .onRepoChanged((id) => {
        if (id !== activeRef.current) return;
        if (refreshTimer.current) window.clearTimeout(refreshTimer.current);
        refreshTimer.current = window.setTimeout(async () => {
          const before = summaryRef.current?.head_oid;
          await refresh();
          const idNow = activeRef.current;
          if (!idNow) return;
          try {
            const s = await api.repoSummary(idNow);
            if (s.head_oid !== before) bumpGraph();
          } catch {
            /* ignore */
          }
        }, 350);
      })
      .then((u) => (unlisten = u));
    return () => unlisten?.();
  }, [bumpGraph, refresh]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    events.onJob(pushJob).then((u) => (unlisten = u));
    return () => unlisten?.();
  }, [pushJob]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;
      if (mod && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setSearching(true);
      } else if (mod && e.key.toLowerCase() === "r") {
        e.preventDefault();
        void refresh();
      } else if (mod && ["1", "2", "3", "4"].includes(e.key)) {
        e.preventDefault();
        const tabs = ["changes", "commit", "conflicts", "recovery"] as const;
        setTab(tabs[Number(e.key) - 1]);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [refresh, setSearching, setTab]);

  async function addRepo() {
    const picked = await open({ directory: true, multiple: false, title: "Open a Git repository" });
    if (typeof picked !== "string") return;
    try {
      const repo = await api.addRepo(picked);
      const list = await api.listRepos();
      setRepos(list);
      await selectRepo(repo.id);
    } catch (e) {
      toast("error", errorText(e));
    }
  }

  async function removeRepo(id: string) {
    if (!window.confirm("Remove this repository from the workspace? Files on disk are not touched.")) return;
    try {
      await api.removeRepo(id);
      const list = await api.listRepos();
      setRepos(list);
      if (activeRef.current === id) {
        setActive(null);
        setSummary(null);
        setChanges([]);
        if (list[0]) await selectRepo(list[0].id);
      }
    } catch (e) {
      toast("error", errorText(e));
    }
  }

  async function networkOp(label: string, fn: () => Promise<{ ok: boolean; message: string; conflicts: boolean }>) {
    setBusy(true);
    try {
      const res = await fn();
      if (res.ok) toast("success", res.message || `${label} complete`);
      else toast("error", res.message || `${label} failed`);
      await refresh();
      bumpGraph();
    } catch (e) {
      toast("error", `${label}: ${errorText(e)}`);
      await refresh();
    } finally {
      setBusy(false);
    }
  }

  async function switchTo(name: string) {
    const dirty =
      !!summary && (summary.staged > 0 || summary.unstaged > 0 || summary.untracked > 0 || summary.conflicts > 0);
    if (dirty) {
      setDialog({ kind: "switch", name });
      return;
    }
    await doSwitch(name, false);
  }

  async function doSwitch(name: string, stashFirst: boolean) {
    setBusy(true);
    try {
      const res = await api.switchBranch(activeId!, name, stashFirst);
      if (res.ok) toast("success", res.message);
      else toast("error", res.message);
      setDialog(null);
      await refresh();
      bumpGraph();
    } catch (e) {
      toast("error", errorText(e));
    } finally {
      setBusy(false);
    }
  }

  const paletteActions: PaletteAction[] = [
    { id: "fetch", label: "Fetch all remotes", hint: "Network", run: () => void networkOp("Fetch", () => api.fetch(activeId!)) },
    { id: "pull", label: "Pull…", hint: "Choose merge / rebase / ff-only", run: () => setDialog({ kind: "pull" }) },
    { id: "push", label: "Push…", hint: "Explicit destination", run: () => setDialog({ kind: "push" }) },
    { id: "merge", label: "Merge branch…", hint: "Preview with merge-tree", run: () => setDialog({ kind: "merge" }) },
    { id: "branch", label: "Create branch…", hint: "From HEAD", run: () => setDialog({ kind: "branch" }) },
    { id: "tag", label: "Create tag…", hint: "Lightweight or annotated", run: () => setDialog({ kind: "tag" }) },
    { id: "remote", label: "Add remote…", hint: "Name + URL", run: () => setDialog({ kind: "remote" }) },
    { id: "refresh", label: "Refresh repository", hint: "Ctrl+R", run: () => void refresh() },
    { id: "recovery", label: "Open Undo & Recovery", hint: "Snapshots and operation history", run: () => setTab("recovery") },
    { id: "conflicts", label: "Open Conflicts", hint: "Resolve in-progress operations", run: () => setTab("conflicts") },
    { id: "open", label: "Open repository…", hint: "Add a local repo", run: () => void addRepo() },
    { id: "clone", label: "Clone repository…", hint: "From a remote URL", run: () => setDialog({ kind: "clone" }) },
  ].filter(() => !!activeId);

  const running = jobs.filter((j) => j.status === "running").slice(0, 1)[0];

  return (
    <div className="app">
      <Sidebar
        repos={repos}
        activeId={activeId}
        system={system}
        onSelect={(id) => void selectRepo(id)}
        onAdd={() => void addRepo()}
        onClone={() => setDialog({ kind: "clone" })}
        onRemove={(id) => void removeRepo(id)}
      />

      <main className="main">
        {!activeId ? (
          <div className="welcome">
            <div className="welcome-inner">
              <div className="welcome-kicker">Git, without the ceremony</div>
              <h1>Your entire Git workflow, in one calm place.</h1>
              <p>
                Open a repository to see the real commit graph, stage by hunk, merge with a safe preview, and recover
                anything the moment it goes wrong.
              </p>
              <div className="welcome-actions">
                <button className="btn primary lg" onClick={() => void addRepo()}>
                  Open repository
                </button>
                <button className="btn ghost lg" onClick={() => setDialog({ kind: "clone" })}>
                  Clone from URL
                </button>
              </div>
              <div className="welcome-note">
                {system ? `${system.git_version} · local-first · no account required` : "Detecting git…"}
              </div>
            </div>
          </div>
        ) : (
          <>
            <Toolbar
              repoId={activeId}
              summary={summary}
              busy={busy}
              onRefresh={() => void refresh()}
              onPush={() => setDialog({ kind: "push" })}
              onPull={() => setDialog({ kind: "pull" })}
              onFetch={() => void networkOp("Fetch", () => api.fetch(activeId))}
              onMerge={() => setDialog({ kind: "merge" })}
              onSwitch={(name) => void switchTo(name)}
              onPalette={() => setSearching(true)}
              onTab={setTab}
              refreshKey={graphKey}
            />

            <div className="workspace">
              <section className="graph-pane">
                <GraphCanvas
                  repoId={activeId}
                  selected={selectedOid}
                  onSelect={selectCommit}
                  refreshKey={graphKey}
                />
              </section>

              <aside className="right-pane">
                <div className="tabs">
                  <button className={`tab ${tab === "changes" ? "active" : ""}`} onClick={() => setTab("changes")}>
                    Changes
                    {summary && summary.staged + summary.unstaged + summary.untracked > 0 && (
                      <span className="tab-count">{summary.staged + summary.unstaged + summary.untracked}</span>
                    )}
                  </button>
                  <button className={`tab ${tab === "commit" ? "active" : ""}`} onClick={() => setTab("commit")}>
                    Commit
                  </button>
                  <button className={`tab ${tab === "conflicts" ? "active" : ""}`} onClick={() => setTab("conflicts")}>
                    Conflicts
                    {summary && summary.conflicts > 0 && <span className="tab-count warn">{summary.conflicts}</span>}
                  </button>
                  <button className={`tab ${tab === "recovery" ? "active" : ""}`} onClick={() => setTab("recovery")}>
                    Recovery
                  </button>
                </div>

                {tab === "changes" && (
                  <ChangesPanel
                    repoId={activeId}
                    changes={changes}
                    onChanged={async () => {
                      await refresh();
                      bumpGraph();
                    }}
                    onConflicts={() => setTab("conflicts")}
                  />
                )}
                {tab === "commit" &&
                  (selectedOid ? (
                    <CommitPanel
                      repoId={activeId}
                      oid={selectedOid}
                      onSelect={selectCommit}
                      onChanged={async () => {
                        await refresh();
                        bumpGraph();
                      }}
                      onReset={(oid) => setDialog({ kind: "reset", oid })}
                    />
                  ) : (
                    <div className="panel-scroll">
                      <div className="section-empty">Select a commit in the graph to inspect it.</div>
                    </div>
                  ))}
                {tab === "conflicts" && (
                  <ConflictsPanel
                    repoId={activeId}
                    changes={changes}
                    onChanged={async () => {
                      await refresh();
                      bumpGraph();
                    }}
                  />
                )}
                {tab === "recovery" && (
                  <RecoveryPanel
                    repoId={activeId}
                    onChanged={async () => {
                      await refresh();
                      bumpGraph();
                    }}
                  />
                )}
              </aside>
            </div>
          </>
        )}

        {running && (
          <div className="job-pill">
            <span className="spinner" />
            {running.kind} · {running.message.slice(0, 72)}
          </div>
        )}
      </main>

      {dialog?.kind === "push" && activeId && summary && (
        <PushModal
          repoId={activeId}
          summary={summary}
          onClose={() => setDialog(null)}
          onDone={async () => {
            await refresh();
            bumpGraph();
          }}
          toast={toast}
        />
      )}
      {dialog?.kind === "pull" && activeId && summary && (
        <PullModal
          repoId={activeId}
          summary={summary}
          onClose={() => setDialog(null)}
          onDone={async () => {
            await refresh();
            bumpGraph();
          }}
          toast={toast}
        />
      )}
      {dialog?.kind === "merge" && activeId && summary && (
        <MergeModal
          repoId={activeId}
          summary={summary}
          onClose={() => setDialog(null)}
          onDone={async () => {
            await refresh();
            bumpGraph();
          }}
          toast={toast}
        />
      )}
      {dialog?.kind === "reset" && activeId && (
        <ResetModal
          oid={dialog.oid}
          branch={summary?.branch ?? null}
          onClose={() => setDialog(null)}
          onConfirm={async (mode) => {
            setBusy(true);
            try {
              const res = await api.resetTo(activeId, dialog.oid, mode);
              toast(res.ok ? "success" : "error", res.message || (res.ok ? "Reset complete" : "Reset failed"));
              setDialog(null);
              await refresh();
              bumpGraph();
            } catch (e) {
              toast("error", errorText(e));
            } finally {
              setBusy(false);
            }
          }}
        />
      )}
      {dialog?.kind === "clone" && (
        <CloneModal
          onClose={() => setDialog(null)}
          onCloned={async (repo: RepoInfo) => {
            const list = await api.listRepos();
            setRepos(list);
            await selectRepo(repo.id);
          }}
          toast={toast}
        />
      )}
      {dialog?.kind === "branch" && activeId && (
        <PromptModal
          title="Create branch"
          subtitle="The branch starts at the current HEAD and is not switched automatically."
          label="Branch name"
          placeholder="feature/my-change"
          onClose={() => setDialog(null)}
          onSubmit={async (name) => {
            try {
              await api.createBranch(activeId, name);
              toast("success", `Created ${name}`);
              setDialog(null);
              await refresh();
              bumpGraph();
            } catch (e) {
              toast("error", errorText(e));
            }
          }}
        />
      )}
      {dialog?.kind === "tag" && activeId && (
        <PromptModal
          title="Create tag"
          subtitle="Leave the message empty for a lightweight tag."
          label="Tag name"
          placeholder="v1.0.0"
          secondaryLabel="Annotation (optional)"
          secondaryPlaceholder="Release 1.0"
          onClose={() => setDialog(null)}
          onSubmit={async (name, message) => {
            try {
              await api.createTag(activeId, name, message || undefined, selectedOid ?? undefined);
              toast("success", `Tagged ${name}`);
              setDialog(null);
              bumpGraph();
            } catch (e) {
              toast("error", errorText(e));
            }
          }}
        />
      )}
      {dialog?.kind === "remote" && activeId && (
        <PromptModal
          title="Add remote"
          subtitle="Credentials stay with git — configure them through your credential helper or SSH agent."
          label="Remote name"
          placeholder="origin"
          secondaryLabel="URL"
          secondaryPlaceholder="https://github.com/owner/repo.git"
          submitLabel="Add remote"
          onClose={() => setDialog(null)}
          onSubmit={async (name, url) => {
            try {
              await api.addRemote(activeId, name, url);
              toast("success", `Added ${name}`);
              setDialog(null);
              await refresh();
            } catch (e) {
              toast("error", errorText(e));
            }
          }}
        />
      )}
      {dialog?.kind === "switch" && summary && (
        <Modal
          title={`Switch to “${dialog.name}”?`}
          subtitle="You have uncommitted changes. Nothing is discarded."
          onClose={() => setDialog(null)}
          footer={
            <>
              <button className="btn ghost" onClick={() => setDialog(null)}>
                Cancel
              </button>
              <button className="btn primary" disabled={busy} onClick={() => void doSwitch(dialog.name, true)}>
                Stash changes & switch
              </button>
            </>
          }
        >
          <div className="push-meta">
            <span className="chip">{summary.staged} staged</span>
            <span className="chip">{summary.unstaged} modified</span>
            <span className="chip">{summary.untracked} untracked</span>
            {summary.conflicts > 0 && <span className="chip warn">{summary.conflicts} conflicts</span>}
          </div>
          <p className="muted">
            Your changes will be stashed with a message identifying the switch, then restored from the Stashes list or
            Undo center.
          </p>
        </Modal>
      )}

      {searching && (
        <Palette
          repoId={activeId}
          actions={paletteActions}
          onPickCommit={selectCommit}
          onClose={() => setSearching(false)}
          toast={toast}
        />
      )}

      <div className="toast-stack">
        {toasts.map((t) => (
          <div key={t.id} className={`toast ${t.kind}`} onClick={() => dismissToast(t.id)}>
            {t.text}
          </div>
        ))}
      </div>
    </div>
  );
}
