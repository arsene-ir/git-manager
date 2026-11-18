import { useEffect, useState } from "react";
import { api, errorText } from "../api";
import { useStore } from "../store";
import type { OpsEntry, SnapshotInfo } from "../types";
import { Modal } from "./Modal";

interface Props {
  repoId: string;
  onChanged: () => void;
}

export function RecoveryPanel({ repoId, onChanged }: Props) {
  const toast = useStore((s) => s.toast);
  const [snapshots, setSnapshots] = useState<SnapshotInfo[]>([]);
  const [ops, setOps] = useState<OpsEntry[]>([]);
  const [confirm, setConfirm] = useState<SnapshotInfo | null>(null);
  const [busy, setBusy] = useState(false);

  function reload() {
    void api.snapshots(repoId).then(setSnapshots).catch(() => setSnapshots([]));
    void api.opsLog(repoId).then(setOps).catch(() => setOps([]));
  }

  useEffect(reload, [repoId]);

  async function restore(snap: SnapshotInfo) {
    setBusy(true);
    try {
      const res = await api.restoreSnapshot(repoId, snap.id);
      toast("success", res.message);
      onChanged();
    } catch (e) {
      toast("error", errorText(e));
    } finally {
      setBusy(false);
      setConfirm(null);
      reload();
    }
  }

  return (
    <div className="panel-scroll recovery-panel">
      <div className="op-card neutral">
        <div className="op-title">Undo & Recovery</div>
        <p className="muted">
          Every destructive operation stores a recovery snapshot first: a backup ref for committed history, a stash
          commit for staged/unstaged changes, and a copy of untracked files (capped).
        </p>
      </div>

      <div className="section-head">
        <span>Snapshots</span>
        <span className="count">{snapshots.length}</span>
      </div>
      {snapshots.length === 0 && <div className="section-empty">No snapshots yet.</div>}
      {snapshots.map((s) => (
        <div key={s.id} className="snapshot-row">
          <div className="snapshot-meta">
            <div className="snapshot-op">{s.op}</div>
            <div className="muted tiny">{new Date(s.created_at).toLocaleString()}</div>
            <div className="muted tiny">
              {s.head_ref ? "history " : ""}
              {s.stash_ref ? "worktree " : ""}
              {s.untracked_dir ? "untracked" : ""}
            </div>
          </div>
          <button className="btn ghost small" disabled={busy} onClick={() => setConfirm(s)}>
            Restore
          </button>
        </div>
      ))}

      <div className="section-head">
        <span>Operation history</span>
        <span className="count">{ops.length}</span>
      </div>
      {ops.length === 0 && <div className="section-empty">No operations recorded.</div>}
      {ops.map((o) => (
        <div key={o.id} className="op-row">
          <span className={`job-dot ${o.status === "ok" ? "ok" : o.status === "conflicted" ? "warn" : "err"}`} />
          <div className="op-row-main">
            <div>
              <span className="op-action">{o.action}</span>
              <span className="muted"> · {o.detail.slice(0, 120)}</span>
            </div>
            <div className="muted tiny">
              {new Date(o.created_at).toLocaleString()} · {o.duration_ms}ms · {o.status}
            </div>
          </div>
        </div>
      ))}

      {confirm && (
        <Modal
          title={`Restore snapshot “${confirm.op}”?`}
          subtitle="This rewrites the working tree and branch position to the captured state. A new snapshot is not created automatically."
          onClose={() => setConfirm(null)}
          footer={
            <>
              <button className="btn ghost" onClick={() => setConfirm(null)}>
                Cancel
              </button>
              <button className="btn danger" disabled={busy} onClick={() => void restore(confirm)}>
                Restore
              </button>
            </>
          }
        >
          <div className="muted">
            Captured {new Date(confirm.created_at).toLocaleString()}.
            {confirm.head_ref && <div>Committed history: backup ref available.</div>}
            {confirm.stash_ref && <div>Staged/unstaged changes: stash commit available.</div>}
            {confirm.untracked_dir && <div>Untracked files: on-disk copies available.</div>}
          </div>
        </Modal>
      )}
    </div>
  );
}
