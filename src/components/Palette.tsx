import { useEffect, useMemo, useState } from "react";
import { api, errorText } from "../api";
import type { SearchHit } from "../types";

export interface PaletteAction {
  id: string;
  label: string;
  hint?: string;
  run: () => void;
}

interface Props {
  repoId: string | null;
  actions: PaletteAction[];
  onPickCommit: (oid: string) => void;
  onClose: () => void;
  toast: (kind: "info" | "error" | "success", text: string) => void;
}

export function Palette({ repoId, actions, onPickCommit, onClose, toast }: Props) {
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [index, setIndex] = useState(0);

  const filteredActions = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return actions;
    return actions.filter((a) => a.label.toLowerCase().includes(q));
  }, [actions, query]);

  useEffect(() => {
    if (!repoId || query.trim().length < 2) {
      setHits([]);
      return;
    }
    let alive = true;
    const t = setTimeout(() => {
      api
        .search(repoId, query.trim())
        .then((r) => alive && setHits(r))
        .catch((e) => alive && toast("error", errorText(e)));
    }, 180);
    return () => {
      alive = false;
      clearTimeout(t);
    };
  }, [repoId, query, toast]);

  interface Entry {
    key: string;
    title: string;
    subtitle: string;
    kind: string;
    run: () => void;
  }

  const entries: Entry[] = useMemo(() => {
    const a: Entry[] = filteredActions.map((x) => ({
      key: `a:${x.id}`,
      title: x.label,
      subtitle: x.hint ?? "Action",
      kind: "action",
      run: () => {
        onClose();
        x.run();
      },
    }));
    const h: Entry[] = hits.map((x) => ({
      key: `h:${x.kind}:${x.oid ?? x.path}:${x.title}`,
      title: x.title,
      subtitle: x.subtitle,
      kind: x.kind,
      run: () => {
        if (x.oid) onPickCommit(x.oid);
        onClose();
      },
    }));
    return [...a, ...h];
  }, [filteredActions, hits, onClose, onPickCommit]);

  useEffect(() => {
    setIndex(0);
  }, [query, hits.length]);

  return (
    <div className="palette-backdrop" onPointerDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="palette" role="dialog" aria-modal="true" aria-label="Command palette">
        <input
          autoFocus
          className="palette-input"
          value={query}
          placeholder="Search commits, branches, tags, files or run a command…"
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") onClose();
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setIndex((i) => Math.min(entries.length - 1, i + 1));
            }
            if (e.key === "ArrowUp") {
              e.preventDefault();
              setIndex((i) => Math.max(0, i - 1));
            }
            if (e.key === "Enter") {
              e.preventDefault();
              entries[index]?.run();
            }
          }}
        />
        <div className="palette-results">
          {entries.length === 0 && <div className="section-empty">No matches</div>}
          {entries.slice(0, 30).map((entry, i) => (
            <div
              key={entry.key}
              className={`palette-row ${i === index ? "active" : ""}`}
              onPointerEnter={() => setIndex(i)}
              onClick={entry.run}
            >
              <span className={`palette-kind ${entry.kind}`}>{entry.kind}</span>
              <div className="palette-text">
                <div className="palette-title">{entry.title}</div>
                <div className="palette-sub">{entry.subtitle}</div>
              </div>
            </div>
          ))}
        </div>
        <div className="palette-foot">
          <span>↑↓ navigate</span>
          <span>↵ run</span>
          <span>esc close</span>
        </div>
      </div>
    </div>
  );
}
