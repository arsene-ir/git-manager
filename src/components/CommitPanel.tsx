import { useEffect, useState } from "react";
import { api, errorText } from "../api";
import { useStore } from "../store";
import type { CommitDetail, CommitFile, FileDiff } from "../types";
import { DiffView } from "./DiffView";

interface Props {
  repoId: string;
  oid: string;
  onSelect: (oid: string) => void;
  onChanged: () => void;
  onReset: (oid: string) => void;
}

export function CommitPanel({ repoId, oid, onSelect, onChanged, onReset }: Props) {
  const toast = useStore((s) => s.toast);
  const [detail, setDetail] = useState<CommitDetail | null>(null);
  const [files, setFiles] = useState<CommitFile[]>([]);
  const [activeFile, setActiveFile] = useState<string | null>(null);
  const [diff, setDiff] = useState<FileDiff | null>(null);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let alive = true;
    setDetail(null);
    setFiles([]);
    setActiveFile(null);
    setDiff(null);
    Promise.all([api.commitDetail(repoId, oid), api.commitFiles(repoId, oid)])
      .then(([d, f]) => {
        if (!alive) return;
        setDetail(d);
        setFiles(f);
      })
      .catch((e) => alive && toast("error", errorText(e)));
    return () => {
      alive = false;
    };
  }, [repoId, oid, toast]);

  useEffect(() => {
    if (!activeFile || !detail) {
      setDiff(null);
      return;
    }
    let alive = true;
    setLoading(true);
    api
      .commitFileDiff(repoId, detail.oid, activeFile)
      .then((d) => alive && setDiff(d))
      .catch((e) => alive && toast("error", errorText(e)))
      .finally(() => alive && setLoading(false));
    return () => {
      alive = false;
    };
  }, [repoId, activeFile, detail, toast]);

  async function action(label: string, fn: () => Promise<unknown>) {
    setBusy(true);
    try {
      const res = (await fn()) as { message?: string; conflicts?: boolean } | undefined;
      toast("success", res?.message || `${label} complete`);
      onChanged();
      if (res?.conflicts) toast("error", "Operation stopped with conflicts — open the Conflicts tab");
    } catch (e) {
      toast("error", `${label}: ${errorText(e)}`);
    } finally {
      setBusy(false);
    }
  }

  if (!detail) {
    return <div className="panel-scroll"><div className="section-empty">Loading commit…</div></div>;
  }

  return (
    <div className="panel-scroll commit-panel">
      <div className="commit-head">
        <div className="commit-title">{detail.summary || "(no message)"}</div>
        <div className="commit-meta">
          <span>{detail.author}</span>
          <span className="muted">{"<"}{detail.author_email}{">"}</span>
          <span className="muted">{new Date(detail.time * 1000).toLocaleString()}</span>
        </div>
        <div className="commit-oids">
          <code>{detail.oid}</code>
          {detail.refs.map((r) => (
            <span key={r} className="chip">
              {r}
            </span>
          ))}
        </div>
        <pre className="commit-message">{detail.message}</pre>
        <div className="commit-stats">
          <span>{detail.files_changed} files</span>
          <span className="add-text">+{detail.insertions}</span>
          <span className="del-text">−{detail.deletions}</span>
          <span>{detail.parents.length} parent(s)</span>
        </div>
        <div className="commit-actions-row">
          {detail.parents.map((p) => (
            <button key={p} className="btn ghost small" onClick={() => onSelect(p)}>
              parent {p.slice(0, 7)}
            </button>
          ))}
          <button className="btn ghost small" disabled={busy} onClick={() => void action("Revert", () => api.revert(repoId, detail.oid))}>
            Revert
          </button>
          <button className="btn ghost small" disabled={busy} onClick={() => void action("Cherry-pick", () => api.cherryPick(repoId, detail.oid))}>
            Cherry-pick
          </button>
          <button className="btn ghost small danger" disabled={busy} onClick={() => onReset(detail.oid)}>
            Reset…
          </button>
        </div>
      </div>

      <div className="section-head">
        <span>Changed files</span>
        <span className="count">{files.length}</span>
      </div>
      {files.map((f) => (
        <div
          key={f.path}
          className={`file-row ${activeFile === f.path ? "active" : ""}`}
          onClick={() => setActiveFile(f.path)}
        >
          <span className="file-path">{f.path}</span>
          <span className="diffstat">
            {f.binary ? (
              <span className="muted">bin</span>
            ) : (
              <>
                <span className="add-text">+{f.additions}</span> <span className="del-text">−{f.deletions}</span>
              </>
            )}
          </span>
        </div>
      ))}
      <div className="diff-box">
        <div className="diff-title">{activeFile ?? "Diff"}</div>
        <DiffView diff={diff} loading={loading} />
      </div>
    </div>
  );
}
