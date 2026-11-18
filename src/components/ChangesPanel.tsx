import { useEffect, useState } from "react";
import { api, errorText } from "../api";
import { useStore } from "../store";
import type { FileChange, FileDiff, StashInfo } from "../types";
import { DiffView } from "./DiffView";
import { Modal } from "./Modal";

interface Props {
  repoId: string;
  changes: FileChange[];
  onChanged: () => void;
  onConflicts: () => void;
}

const STATUS_LABEL: Record<string, string> = {
  added: "A",
  modified: "M",
  deleted: "D",
  renamed: "R",
  untracked: "U",
  conflicted: "!",
  typechange: "T",
};

export function ChangesPanel({ repoId, changes, onChanged, onConflicts }: Props) {
  const summary = useStore((s) => s.summary);
  const toast = useStore((s) => s.toast);
  const setChanges = useStore((s) => s.setChanges);
  const [selected, setSelected] = useState<{ path: string; staged: boolean } | null>(null);
  const [diff, setDiff] = useState<FileDiff | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);
  const [message, setMessage] = useState("");
  const [amend, setAmend] = useState(false);
  const [busy, setBusy] = useState(false);
  const [stashes, setStashes] = useState<StashInfo[]>([]);
  const [stashOpen, setStashOpen] = useState(false);
  const [stashMessage, setStashMessage] = useState("");

  const staged = changes.filter((c) => c.staged && !c.conflict);
  const unstaged = changes.filter((c) => !c.staged && !c.conflict);
  const conflicts = changes.filter((c) => c.conflict);

  useEffect(() => {
    void api.stashList(repoId).then(setStashes).catch(() => setStashes([]));
  }, [repoId, summary?.head_oid, changes.length]);

  useEffect(() => {
    if (!selected) {
      setDiff(null);
      return;
    }
    let alive = true;
    setDiffLoading(true);
    api
      .fileDiff(repoId, selected.path, selected.staged)
      .then((d) => alive && setDiff(d))
      .catch((e) => alive && toast("error", errorText(e)))
      .finally(() => alive && setDiffLoading(false));
    return () => {
      alive = false;
    };
  }, [repoId, selected, toast, changes]);

  async function run(label: string, fn: () => Promise<unknown>) {
    setBusy(true);
    try {
      await fn();
      onChanged();
    } catch (e) {
      toast("error", `${label}: ${errorText(e)}`);
    } finally {
      setBusy(false);
    }
  }

  async function commit() {
    if (!message.trim()) {
      toast("error", "Write a commit message first");
      return;
    }
    setBusy(true);
    try {
      const res = await api.commit(repoId, message, amend);
      toast("success", res.message || "Commit created");
      setMessage("");
      setAmend(false);
      onChanged();
    } catch (e) {
      toast("error", errorText(e));
    } finally {
      setBusy(false);
    }
  }

  function fileRow(c: FileChange) {
    const key = `${c.staged ? "s" : "w"}:${c.path}`;
    const active = selected?.path === c.path && selected.staged === c.staged;
    return (
      <div key={key} className={`file-row ${active ? "active" : ""}`} onClick={() => setSelected({ path: c.path, staged: c.staged })}>
        <span className={`file-status ${c.status}`}>{STATUS_LABEL[c.status] ?? "?"}</span>
        <span className="file-path" title={c.path}>
          {c.path}
        </span>
        <button
          className="mini-btn"
          title={c.staged ? "Unstage" : "Stage"}
          onClick={(e) => {
            e.stopPropagation();
            void run(c.staged ? "Unstage" : "Stage", () =>
              c.staged ? api.unstage(repoId, [c.path]) : api.stage(repoId, [c.path]),
            );
          }}
        >
          {c.staged ? "−" : "+"}
        </button>
      </div>
    );
  }

  return (
    <div className="changes-panel">
      <div className="panel-scroll">
        {summary?.operation !== "clean" && (
          <div className="op-banner">
            <div>
              <strong>{summary?.operation}</strong> in progress
              {conflicts.length > 0 ? ` · ${conflicts.length} conflicted file(s)` : ""}
            </div>
            <button className="link" onClick={onConflicts}>
              Resolve
            </button>
          </div>
        )}

        <div className="section-head">
          <span>Staged</span>
          <span className="count">{staged.length}</span>
        </div>
        {staged.length === 0 && <div className="section-empty">Nothing staged</div>}
        {staged.map(fileRow)}

        <div className="section-head">
          <span>Changes</span>
          <span className="count">{unstaged.length + conflicts.length}</span>
          {unstaged.length > 0 && (
            <button
              className="link tiny"
              disabled={busy}
              onClick={() => void run("Stage all", () => api.stage(repoId, unstaged.map((c) => c.path)))}
            >
              Stage all
            </button>
          )}
        </div>
        {conflicts.map((c) => (
          <div key={`c:${c.path}`} className="file-row conflict" onClick={onConflicts}>
            <span className="file-status conflicted">!</span>
            <span className="file-path">{c.path}</span>
            <span className="chip warn">conflict</span>
          </div>
        ))}
        {unstaged.length === 0 && conflicts.length === 0 && <div className="section-empty">Working tree clean</div>}
        {unstaged.map(fileRow)}

        <div className="section-head">
          <span>Stashes</span>
          <button className="link tiny" disabled={busy} onClick={() => setStashOpen(true)}>
            Stash all
          </button>
        </div>
        {stashes.length === 0 && <div className="section-empty">No stashes</div>}
        {stashes.map((s) => (
          <div key={s.oid} className="stash-row">
            <span className="stash-msg" title={s.message}>
              {s.message}
            </span>
            <button
              className="mini-btn"
              title="Apply stash"
              disabled={busy}
              onClick={() => void run("Apply stash", () => api.stashApply(repoId, s.index, false))}
            >
              apply
            </button>
            <button
              className="mini-btn"
              title="Pop stash"
              disabled={busy}
              onClick={() => void run("Pop stash", () => api.stashApply(repoId, s.index, true))}
            >
              pop
            </button>
            <button
              className="mini-btn danger"
              title="Drop stash"
              disabled={busy}
              onClick={() => void run("Drop stash", () => api.stashDrop(repoId, s.index))}
            >
              ✕
            </button>
          </div>
        ))}

        <div className="diff-box">
          <div className="diff-title">
            {selected ? selected.path : "Diff"}
            {selected && <span className="muted"> · {selected.staged ? "staged" : "working tree"}</span>}
          </div>
          <DiffView diff={diff} loading={diffLoading} />
        </div>
      </div>

      <div className="commit-box">
        <textarea
          placeholder={amend ? "Reword the previous commit…" : "Commit message — what changed and why?"}
          value={message}
          onChange={(e) => setMessage(e.target.value)}
          onKeyDown={(e) => {
            if ((e.ctrlKey || e.metaKey) && e.key === "Enter") void commit();
          }}
          rows={3}
        />
        <div className="commit-actions">
          <label className="check">
            <input type="checkbox" checked={amend} onChange={(e) => setAmend(e.target.checked)} />
            Amend
          </label>
          <span className="muted tiny">
            {summary ? `${summary.staged} staged` : ""}
            {summary?.conflicts ? ` · ${summary.conflicts} conflicts` : ""}
          </span>
          <button className="btn primary" disabled={busy} onClick={() => void commit()}>
            {amend ? "Amend commit" : "Commit"}
          </button>
        </div>
        {summary && (summary.staged > 0 || summary.unstaged > 0 || summary.untracked > 0) && (
          <button className="link tiny discard" disabled={busy} onClick={onConflicts}>
            Need a safe alternative? Use stash or Undo center.
          </button>
        )}
      </div>

      {stashOpen && (
        <Modal
          title="Stash changes"
          subtitle="Includes untracked files. Restore later from the stash list or Undo center."
          onClose={() => setStashOpen(false)}
          footer={
            <>
              <button className="btn ghost" onClick={() => setStashOpen(false)}>
                Cancel
              </button>
              <button
                className="btn primary"
                disabled={busy}
                onClick={() => {
                  void run("Stash", () => api.stashPush(repoId, stashMessage || undefined));
                  setStashOpen(false);
                  setStashMessage("");
                }}
              >
                Create stash
              </button>
            </>
          }
        >
          <label className="field">
            <span>Message</span>
            <input
              autoFocus
              value={stashMessage}
              onChange={(e) => setStashMessage(e.target.value)}
              placeholder="WIP: refactor parser"
            />
          </label>
        </Modal>
      )}
    </div>
  );
}
