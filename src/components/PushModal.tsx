import { useEffect, useState } from "react";
import { api, errorText } from "../api";
import type { BranchInfo, RemoteInfo, RepoSummary } from "../types";
import { Modal } from "./Modal";

interface Props {
  repoId: string;
  summary: RepoSummary;
  onClose: () => void;
  onDone: () => void;
  toast: (kind: "info" | "error" | "success", text: string) => void;
}

export function PushModal({ repoId, summary, onClose, onDone, toast }: Props) {
  const [remotes, setRemotes] = useState<RemoteInfo[]>([]);
  const [branches, setBranches] = useState<BranchInfo[]>([]);
  const [remote, setRemote] = useState("");
  const [src, setSrc] = useState(summary.branch ?? "");
  const [dst, setDst] = useState(summary.upstream?.split("/").slice(1).join("/") || (summary.branch ?? ""));
  const [force, setForce] = useState(false);
  const [confirmText, setConfirmText] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([api.remotes(repoId), api.branches(repoId)])
      .then(([r, b]) => {
        setRemotes(r);
        setBranches(b);
        const upstreamRemote = summary.upstream?.split("/")[0];
        setRemote(upstreamRemote && r.some((x) => x.name === upstreamRemote) ? upstreamRemote : r[0]?.name ?? "");
      })
      .catch((e) => setError(errorText(e)));
  }, [repoId, summary.upstream]);

  const locals = branches.filter((b) => !b.is_remote);
  const upstreamBranch = summary.upstream?.split("/").slice(1).join("/");
  const forceAllowed = force && confirmText.trim() === (summary.branch ?? "");
  const canPush = !!remote && !!src && !!dst && (!force || forceAllowed);

  async function push() {
    setBusy(true);
    setError(null);
    try {
      const res = await api.push(repoId, remote, src, dst, force);
      if (!res.ok) {
        setError(res.message || "Push failed");
        return;
      }
      toast("success", res.message || `Pushed ${src} → ${remote}/${dst}`);
      onDone();
      onClose();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal
      title="Push"
      subtitle="Review the exact destination before anything leaves your machine."
      onClose={onClose}
      width={560}
      footer={
        <>
          <button className="btn ghost" onClick={onClose}>
            Cancel
          </button>
          <button className="btn primary" disabled={busy || !canPush} onClick={() => void push()}>
            {busy ? "Pushing…" : `Push ${src || "?"} → ${remote || "?"}/${dst || "?"}`}
          </button>
        </>
      }
    >
      <div className="route-card">
        <div>
          <div className="route-label">Local branch</div>
          <div className="route-value">{summary.branch ?? "detached HEAD"}</div>
        </div>
        <div className="route-arrow">→</div>
        <div>
          <div className="route-label">Remote destination</div>
          <div className="route-value">{remote && dst ? `${remote}/${dst}` : "choose a destination"}</div>
        </div>
      </div>

      <div className="form-grid">
        <label className="field">
          <span>Remote</span>
          <select value={remote} onChange={(e) => setRemote(e.target.value)}>
            {remotes.length === 0 && <option value="">No remotes configured</option>}
            {remotes.map((r) => (
              <option key={r.name} value={r.name}>
                {r.name} — {r.url}
              </option>
            ))}
          </select>
        </label>
        <label className="field">
          <span>Local ref</span>
          <select value={src} onChange={(e) => setSrc(e.target.value)}>
            {locals.length === 0 && <option value={summary.branch ?? ""}>{summary.branch ?? "HEAD"}</option>}
            {locals.map((b) => (
              <option key={b.name} value={b.name}>
                {b.name}
              </option>
            ))}
          </select>
        </label>
        <label className="field">
          <span>Destination branch</span>
          <input
            value={dst}
            onChange={(e) => setDst(e.target.value)}
            placeholder={upstreamBranch ?? "branch name"}
          />
        </label>
      </div>

      <div className="push-meta">
        <span className="chip up">↑{summary.ahead} outgoing</span>
        <span className="chip down">↓{summary.behind} incoming</span>
        {summary.upstream && <span className="muted">upstream {summary.upstream}</span>}
      </div>

      <label className="check force">
        <input
          type="checkbox"
          checked={force}
          onChange={(e) => {
            setForce(e.target.checked);
            setConfirmText("");
          }}
        />
        Force with lease (refuses if the remote moved unexpectedly)
      </label>
      {force && (
        <label className="field">
          <span>Type “{summary.branch}” to confirm the force push</span>
          <input value={confirmText} onChange={(e) => setConfirmText(e.target.value)} autoFocus />
        </label>
      )}
      {error && <div className="inline-error">{error}</div>}
    </Modal>
  );
}
