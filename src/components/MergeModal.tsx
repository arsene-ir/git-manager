import { useEffect, useState } from "react";
import { api, errorText } from "../api";
import type { BranchInfo, MergePreview, RepoSummary } from "../types";
import { Modal } from "./Modal";

interface Props {
  repoId: string;
  summary: RepoSummary;
  onClose: () => void;
  onDone: () => void;
  toast: (kind: "info" | "error" | "success", text: string) => void;
}

export function MergeModal({ repoId, summary, onClose, onDone, toast }: Props) {
  const [branches, setBranches] = useState<BranchInfo[]>([]);
  const [source, setSource] = useState("");
  const [preview, setPreview] = useState<MergePreview | null>(null);
  const [previewing, setPreviewing] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .branches(repoId)
      .then((b) => {
        const candidates = b.filter((x) => !x.is_head);
        setBranches(candidates);
        setSource(candidates[0]?.name ?? "");
      })
      .catch((e) => setError(errorText(e)));
  }, [repoId]);

  async function runPreview() {
    if (!source) return;
    setPreviewing(true);
    setError(null);
    try {
      setPreview(await api.mergePreview(repoId, source));
    } catch (e) {
      setError(errorText(e));
    } finally {
      setPreviewing(false);
    }
  }

  async function merge() {
    setBusy(true);
    setError(null);
    try {
      const res = await api.merge(repoId, source);
      if (res.conflicts) {
        toast("error", "Merge stopped with conflicts — resolve them in the Conflicts tab");
      } else {
        toast("success", `Merged ${source} into ${summary.branch ?? "HEAD"}`);
      }
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
      title="Merge branch"
      subtitle="Preview runs in isolation (git merge-tree) and never touches your working tree."
      width={620}
      onClose={onClose}
      footer={
        <>
          <button className="btn ghost" onClick={onClose}>
            Cancel
          </button>
          <button className="btn ghost" disabled={!source || previewing} onClick={() => void runPreview()}>
            {previewing ? "Previewing…" : "Preview"}
          </button>
          <button className="btn primary" disabled={!source || busy} onClick={() => void merge()}>
            Merge into {summary.branch ?? "HEAD"}
          </button>
        </>
      }
    >
      <label className="field">
        <span>Source branch</span>
        <select value={source} onChange={(e) => { setSource(e.target.value); setPreview(null); }}>
          {branches.length === 0 && <option value="">No other branches</option>}
          {branches.map((b) => (
            <option key={`${b.is_remote ? "r" : "l"}:${b.name}`} value={b.name}>
              {b.is_remote ? `${b.name} (remote)` : b.name} — {b.tip} {b.tip_summary.slice(0, 50)}
            </option>
          ))}
        </select>
      </label>

      <div className="route-card">
        <div>
          <div className="route-label">Source</div>
          <div className="route-value">{source || "—"}</div>
        </div>
        <div className="route-arrow">→</div>
        <div>
          <div className="route-label">Target</div>
          <div className="route-value">{summary.branch ?? "HEAD"}</div>
        </div>
      </div>

      {preview && (
        <div className={`preview-card ${preview.clean ? "clean" : "conflict"}`}>
          <div className="preview-head">
            {preview.supported ? (preview.clean ? "Clean merge" : "Conflicts detected") : "Preview unavailable"}
          </div>
          {preview.merge_base && <div className="muted tiny">merge-base {preview.merge_base.slice(0, 12)}</div>}
          {preview.conflicts.length > 0 && (
            <ul className="conflict-preview">
              {preview.conflicts.slice(0, 12).map((c) => (
                <li key={c}>{c}</li>
              ))}
              {preview.conflicts.length > 12 && <li>…and {preview.conflicts.length - 12} more</li>}
            </ul>
          )}
          {preview.messages.map((m) => (
            <div key={m} className="muted tiny">
              {m}
            </div>
          ))}
        </div>
      )}
      {error && <div className="inline-error">{error}</div>}
    </Modal>
  );
}
