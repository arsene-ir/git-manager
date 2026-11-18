import { useState } from "react";
import { api, errorText } from "../api";
import { Modal } from "./Modal";
import type { RepoSummary } from "../types";

interface Props {
  repoId: string;
  summary: RepoSummary;
  onClose: () => void;
  onDone: () => void;
  toast: (kind: "info" | "error" | "success", text: string) => void;
}

export function PullModal({ repoId, summary, onClose, onDone, toast }: Props) {
  const [mode, setMode] = useState<"merge" | "rebase" | "ff-only">("ff-only");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function pull() {
    setBusy(true);
    setError(null);
    try {
      const res = await api.pull(repoId, mode);
      if (res.ok) {
        toast("success", res.message || "Pull complete");
        onDone();
        onClose();
      } else {
        setError(res.message || "Pull failed");
        onDone();
      }
    } catch (e) {
      setError(errorText(e));
      onDone();
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal
      title={`Pull ${summary.branch ?? ""}`}
      subtitle={summary.upstream ? `From ${summary.upstream}` : "No upstream configured — set one by pushing first."}
      onClose={onClose}
      footer={
        <>
          <button className="btn ghost" onClick={onClose}>
            Cancel
          </button>
          <button className="btn primary" disabled={busy || !summary.upstream} onClick={() => void pull()}>
            {busy ? "Pulling…" : "Pull"}
          </button>
        </>
      }
    >
      <div className="radio-list">
        <label className={`radio-card ${mode === "ff-only" ? "selected" : ""}`}>
          <input type="radio" checked={mode === "ff-only"} onChange={() => setMode("ff-only")} />
          <div>
            <div className="radio-title">Fast-forward only</div>
            <div className="radio-detail">Safest. Refuses when history has diverged instead of creating a merge.</div>
          </div>
        </label>
        <label className={`radio-card ${mode === "merge" ? "selected" : ""}`}>
          <input type="radio" checked={mode === "merge"} onChange={() => setMode("merge")} />
          <div>
            <div className="radio-title">Merge</div>
            <div className="radio-detail">Creates a merge commit when histories diverged. Conflicts are handled in the Conflicts tab.</div>
          </div>
        </label>
        <label className={`radio-card ${mode === "rebase" ? "selected" : ""}`}>
          <input type="radio" checked={mode === "rebase"} onChange={() => setMode("rebase")} />
          <div>
            <div className="radio-title">Rebase</div>
            <div className="radio-detail">Replays your commits on top of the remote. History is rewritten; a recovery snapshot is created first.</div>
          </div>
        </label>
      </div>
      <div className="push-meta">
        <span className="chip down">↓{summary.behind} incoming</span>
        <span className="chip up">↑{summary.ahead} outgoing</span>
      </div>
      {error && <div className="inline-error">{error}</div>}
    </Modal>
  );
}
