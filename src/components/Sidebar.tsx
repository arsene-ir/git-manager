import type { RepoInfo, SystemInfo } from "../types";

interface Props {
  repos: RepoInfo[];
  activeId: string | null;
  system: SystemInfo | null;
  onSelect: (id: string) => void;
  onAdd: () => void;
  onClone: () => void;
  onRemove: (id: string) => void;
}

export function Sidebar({ repos, activeId, system, onSelect, onAdd, onClone, onRemove }: Props) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark">
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none">
            <path d="M6 3v12a3 3 0 0 0 3 3h7" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
            <circle cx="6" cy="5" r="2.4" stroke="currentColor" strokeWidth="2" />
            <circle cx="18" cy="18" r="2.4" stroke="currentColor" strokeWidth="2" />
            <circle cx="6" cy="18" r="2.4" stroke="currentColor" strokeWidth="2" />
            <path d="M6 15v.5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
          </svg>
        </div>
        <div>
          <div className="brand-name">Git Manager</div>
          <div className="brand-sub">local-first</div>
        </div>
      </div>

      <div className="sidebar-actions">
        <button className="btn ghost small" onClick={onAdd} title="Open a local repository">
          Open
        </button>
        <button className="btn ghost small" onClick={onClone} title="Clone from a remote URL">
          Clone
        </button>
      </div>

      <div className="sidebar-label">Repositories</div>
      <div className="repo-list">
        {repos.length === 0 && (
          <div className="repo-empty">
            No repositories yet.
            <button className="link" onClick={onAdd}>
              Open one
            </button>
          </div>
        )}
        {repos.map((repo) => (
          <div
            key={repo.id}
            className={`repo-item ${repo.id === activeId ? "active" : ""}`}
            onClick={() => onSelect(repo.id)}
            title={repo.path}
          >
            <span className={`repo-dot ${repo.id === activeId ? "on" : ""}`} />
            <div className="repo-meta">
              <div className="repo-name">{repo.name}</div>
              <div className="repo-path">{compactPath(repo.path)}</div>
            </div>
            <button
              className="repo-remove"
              title="Remove from workspace (does not delete files)"
              onClick={(e) => {
                e.stopPropagation();
                onRemove(repo.id);
              }}
            >
              ✕
            </button>
          </div>
        ))}
      </div>

      <div className="sidebar-foot">
        <div className="system-line">
          <span className={`status-dot ${system?.merge_tree ? "ok" : "warn"}`} />
          {system ? system.git_version.replace("git version ", "git ") : "git not detected"}
        </div>
        {system && !system.merge_tree && <div className="system-warn">merge preview needs git 2.38+</div>}
        <div className="system-line muted">v{system?.app_version ?? "0.1.0"}</div>
      </div>
    </aside>
  );
}

function compactPath(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  if (parts.length <= 2) return path;
  return `…/${parts.slice(-2).join("/")}`;
}
