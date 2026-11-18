import { useState } from "react";
import { api, errorText } from "../api";
import { useStore } from "../store";
import type { FileChange } from "../types";

interface Props {
  repoId: string;
  changes: FileChange[];
  onChanged: () => void;
}

export function ConflictsPanel({ repoId, changes, onChanged }: Props) {
  const summary = useStore((s) => s.summary);
  const toast = useStore((s) => s.toast);
  const [busy, setBusy] = useState(false);
  const conflicts = changes.filter((c) => c.conflict);

  async function run(label: string, fn: () => Promise<unknown>) {
    setBusy(true);
    try {
      const res = (await fn()) as { message?: string } | undefined;
      toast("success", res?.message || `${label} complete`);
      onChanged();
    } catch (e) {
      toast("error", `${label}: ${errorText(e)}`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="panel-scroll conflicts-panel">
      {summary && summary.operation !== "clean" ? (
        <div className="op-card">
          <div className="op-title">{summary.operation} in progress</div>
          <p className="muted">
            Resolve each file, mark it resolved, then continue. Aborting restores the pre-operation state.
          </p>
          <div className="op-actions">
            <button className="btn primary small" disabled={busy} onClick={() => void run("Continue", () => api.continueOp(repoId))}>
              Continue
            </button>
            <button className="btn ghost small danger" disabled={busy} onClick={() => void run("Abort", () => api.abortOp(repoId))}>
              Abort
            </button>
          </div>
        </div>
      ) : (
        <div className="section-empty">No merge, rebase or cherry-pick in progress.</div>
      )}

      <div className="section-head">
        <span>Conflicted files</span>
        <span className="count">{conflicts.length}</span>
      </div>
      {conflicts.length === 0 && summary?.operation === "clean" && (
        <div className="section-empty">Nothing to resolve.</div>
      )}
      {conflicts.map((c) => (
        <div key={c.path} className="conflict-card">
          <div className="conflict-path" title={c.path}>
            {c.path}
          </div>
          <div className="conflict-actions">
            <button className="btn ghost small" disabled={busy} onClick={() => void run("Accept ours", () => api.resolveConflict(repoId, c.path, "ours"))}>
              Accept ours
            </button>
            <button className="btn ghost small" disabled={busy} onClick={() => void run("Accept theirs", () => api.resolveConflict(repoId, c.path, "theirs"))}>
              Accept theirs
            </button>
            <button className="btn ghost small" disabled={busy} onClick={() => void run("Mark resolved", () => api.markResolved(repoId, c.path))}>
              Mark resolved
            </button>
          </div>
          <div className="conflict-hint muted tiny">
            Binary, delete/modify and submodule conflicts should be resolved in the file or via terminal, then marked resolved.
          </div>
        </div>
      ))}
    </div>
  );
}
