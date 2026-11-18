import { useMemo, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { FileDiff } from "../types";

interface Props {
  diff: FileDiff | null;
  loading?: boolean;
}

type Row =
  | { kind: "hunk"; header: string }
  | { kind: "line"; type: string; text: string; old: number | null; new: number | null };

export function DiffView({ diff, loading }: Props) {
  const rows = useMemo<Row[]>(() => {
    if (!diff) return [];
    const out: Row[] = [];
    for (const hunk of diff.hunks) {
      out.push({ kind: "hunk", header: hunk.header });
      for (const line of hunk.lines) {
        out.push({ kind: "line", type: line.kind, text: line.text, old: line.old_lineno, new: line.new_lineno });
      }
    }
    return out;
  }, [diff]);

  const parentRef = useRef<HTMLDivElement | null>(null);
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 19,
    overscan: 24,
  });

  if (loading) return <div className="diff-empty">Loading diff…</div>;
  if (!diff) return <div className="diff-empty">Select a file to view changes.</div>;
  if (diff.binary) {
    return (
      <div className="diff-empty">
        Binary file changed — side-by-side preview is not available yet.
      </div>
    );
  }
  if (rows.length === 0) return <div className="diff-empty">No textual changes.</div>;

  return (
    <div className="diff-view" ref={parentRef}>
      <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
        {virtualizer.getVirtualItems().map((item) => {
          const row = rows[item.index];
          if (row.kind === "hunk") {
            return (
              <div
                key={item.key}
                className="diff-hunk"
                style={{ position: "absolute", top: 0, transform: `translateY(${item.start}px)`, width: "100%" }}
              >
                {row.header}
              </div>
            );
          }
          return (
            <div
              key={item.key}
              className={`diff-line ${row.type === "add" ? "add" : row.type === "del" ? "del" : ""}`}
              style={{ position: "absolute", top: 0, transform: `translateY(${item.start}px)`, width: "100%" }}
            >
              <span className="gutter">{row.old ?? ""}</span>
              <span className="gutter">{row.new ?? ""}</span>
              <span className="marker">{row.type === "add" ? "+" : row.type === "del" ? "−" : " "}</span>
              <span className="code">{row.text || " "}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
