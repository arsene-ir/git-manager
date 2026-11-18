import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import type { BranchInfo, RepoSummary } from "../types";

interface Props {
  repoId: string;
  summary: RepoSummary | null;
  onRefresh: () => void;
  onPush: () => void;
  onPull: () => void;
  onFetch: () => void;
  onMerge: () => void;
  onSwitch: (name: string) => void;
  onPalette: () => void;
  onTab: (tab: "changes" | "commit" | "conflicts" | "recovery") => void;
  busy: boolean;
  refreshKey?: number;
}

export function Toolbar({
  repoId,
  summary,
  onRefresh,
  onPush,
  onPull,
  onFetch,
  onMerge,
  onSwitch,
  onPalette,
  onTab,
  busy,
  refreshKey,
}: Props) {
  const [branches, setBranches] = useState<BranchInfo[]>([]);
  const [open, setOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!repoId) return;
    void api.branches(repoId).then(setBranches).catch(() => setBranches([]));
  }, [repoId, summary?.branch, summary?.head_oid, refreshKey]);

  useEffect(() => {
    const close = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener("mousedown", close);
    return () => window.removeEventListener("mousedown", close);
  }, []);

  const locals = branches.filter((b) => !b.is_remote);

  return (
    <header className="toolbar">
      <div className="branch-picker" ref={menuRef}>
        <button className={`branch-btn ${open ? "open" : ""}`} onClick={() => setOpen((v) => !v)}>
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
            <path d="M4 2v8a2 2 0 0 0 2 2h5" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
            <circle cx="4" cy="3" r="1.7" stroke="currentColor" strokeWidth="1.4" />
            <circle cx="11" cy="13" r="1.7" stroke="currentColor" strokeWidth="1.4" />
          </svg>
          <span className="branch-name">{summary?.branch ?? "no branch"}</span>
          {summary?.detached && <span className="chip warn">detached</span>}
          <span className="chev">▾</span>
        </button>
        {open && (
          <div className="branch-menu">
            <div className="menu-label">Local branches</div>
            {locals.map((b) => (
              <button
                key={b.name}
                className={`menu-item ${b.is_head ? "current" : ""}`}
                onClick={() => {
                  setOpen(false);
                  if (!b.is_head) onSwitch(b.name);
                }}
              >
                <span className="branch-line">{b.name}</span>
                <span className="branch-tip">{b.tip}</span>
              </button>
            ))}
            {locals.length === 0 && <div className="menu-empty">No local branches</div>}
          </div>
        )}
        {summary && (
          <div className="aheadbehind">
            {summary.ahead > 0 && <span className="chip up">↑{summary.ahead}</span>}
            {summary.behind > 0 && <span className="chip down">↓{summary.behind}</span>}
            {summary.upstream && <span className="upstream">{summary.upstream}</span>}
          </div>
        )}
      </div>

      <div className="toolbar-actions">
        <button className="btn ghost" onClick={onFetch} disabled={busy} title="Fetch all remotes">
          Fetch
        </button>
        <button className="btn ghost" onClick={onPull} disabled={busy} title="Pull with an explicit strategy">
          Pull
        </button>
        <button className="btn primary" onClick={onPush} disabled={busy || !summary?.branch} title="Push to an explicit destination">
          Push
          {summary && summary.ahead > 0 && <span className="btn-badge">{summary.ahead}</span>}
        </button>
        <div className="toolbar-sep" />
        <button className="btn ghost" onClick={onMerge} disabled={busy} title="Preview and merge a branch">
          Merge
        </button>
        <button className="icon-btn" onClick={onPalette} title="Command palette (Ctrl+K)">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <circle cx="7" cy="7" r="4.4" stroke="currentColor" strokeWidth="1.5" />
            <path d="m10.4 10.4 3 3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
        </button>
        <button className="icon-btn" onClick={onRefresh} disabled={busy} title="Refresh (Ctrl+R)">
          ↻
        </button>
        <div className="toolbar-sep" />
        <button className="icon-btn" onClick={() => onTab("recovery")} title="Undo & recovery">
          ⤺
        </button>
      </div>
    </header>
  );
}
