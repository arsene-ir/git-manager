import { useCallback, useEffect, useRef, useState } from "react";
import { api, errorText } from "../api";
import type { CommitNode, GraphEdge, GraphWindow } from "../types";

const ROW_H = 30;
const LANE_W = 16;
const PAD_X = 16;
// Memory cap: never retain an unbounded history in the renderer (SSOT: 5k commit window).
const MAX_LOADED = 5000;
const LANE_COLORS = [
  "#58a6ff",
  "#7ee787",
  "#bc8cff",
  "#ffa657",
  "#79c0ff",
  "#56d4dd",
  "#ff7b72",
  "#d2a8ff",
  "#f2cc60",
  "#8b949e",
];

interface Props {
  repoId: string;
  selected: string | null;
  onSelect: (oid: string) => void;
  refreshKey: number;
}

function timeAgo(secs: number): string {
  const diff = Math.max(0, Date.now() / 1000 - secs);
  if (diff < 60) return "now";
  if (diff < 3600) return `${Math.floor(diff / 60)}m`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h`;
  if (diff < 86400 * 30) return `${Math.floor(diff / 86400)}d`;
  return new Date(secs * 1000).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
}

function refColor(ref: string): { bg: string; fg: string } {
  if (ref.startsWith("HEAD")) return { bg: "#1f6f3f", fg: "#d7ffe6" };
  if (ref.startsWith("tag:")) return { bg: "#5c4210", fg: "#ffd580" };
  if (ref.includes("/")) return { bg: "#3b2d55", fg: "#e2ccff" };
  return { bg: "#123a5c", fg: "#bfe2ff" };
}

function truncate(ctx: CanvasRenderingContext2D, text: string, max: number): string {
  if (max <= 0) return "";
  if (ctx.measureText(text).width <= max) return text;
  let lo = 0;
  let hi = text.length;
  while (lo < hi) {
    const mid = Math.ceil((lo + hi) / 2);
    if (ctx.measureText(text.slice(0, mid) + "…").width <= max) lo = mid;
    else hi = mid - 1;
  }
  return text.slice(0, lo) + "…";
}

export function GraphCanvas({ repoId, selected, onSelect, refreshKey }: Props) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const wrapRef = useRef<HTMLDivElement | null>(null);
  const [commits, setCommits] = useState<CommitNode[]>([]);
  const [edges, setEdges] = useState<GraphEdge[]>([]);
  const [total, setTotal] = useState(0);
  const [maxLane, setMaxLane] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [size, setSize] = useState({ w: 800, h: 600 });
  const [scrollTop, setScrollTop] = useState(0);
  const scrollRef = useRef(0);
  const loadingRef = useRef(false);
  const commitsRef = useRef<CommitNode[]>([]);
  const edgesRef = useRef<GraphEdge[]>([]);
  const maxLaneRef = useRef(0);
  const totalRef = useRef(0);
  const selectedRef = useRef<string | null>(null);
  const dragRef = useRef<{ startY: number; startOffset: number } | null>(null);

  selectedRef.current = selected;

  const paint = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    const w = size.w;
    const h = size.h;
    if (canvas.width !== Math.floor(w * dpr) || canvas.height !== Math.floor(h * dpr)) {
      canvas.width = Math.floor(w * dpr);
      canvas.height = Math.floor(h * dpr);
    }
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);

    const all = commitsRef.current;
    const ed = edgesRef.current;
    const top = scrollRef.current;
    const startRow = Math.max(0, Math.floor(top / ROW_H) - 1);
    const endRow = Math.min(all.length - 1, Math.ceil((top + h) / ROW_H) + 1);
    const laneArea = Math.min(220, PAD_X * 2 + (maxLaneRef.current + 1) * LANE_W);

    // lane guides
    const yOf = (row: number) => row * ROW_H - top + ROW_H / 2;

    ctx.lineWidth = 1.6;
    for (const edge of ed) {
      if (edge.from_row < startRow - 3 && edge.to_row < startRow - 3) continue;
      if (edge.from_row > endRow + 3 && edge.to_row > endRow + 3) continue;
      if (edge.to_row < 0) continue;
      const x1 = PAD_X + edge.from_lane * LANE_W;
      const y1 = yOf(edge.from_row);
      const x2 = PAD_X + edge.to_lane * LANE_W;
      const y2 = yOf(edge.to_row);
      ctx.strokeStyle = LANE_COLORS[edge.from_lane % LANE_COLORS.length];
      ctx.globalAlpha = 0.85;
      ctx.beginPath();
      if (x1 === x2) {
        ctx.moveTo(x1, y1);
        ctx.lineTo(x2, y2);
      } else {
        const my = (y1 + y2) / 2;
        ctx.moveTo(x1, y1);
        ctx.bezierCurveTo(x1, my, x2, my, x2, y2);
      }
      ctx.stroke();
    }
    ctx.globalAlpha = 1;

    ctx.font = '12.5px "Segoe UI Variable", "Inter", system-ui, sans-serif';
    ctx.textBaseline = "middle";

    for (let row = startRow; row <= endRow; row++) {
      const c = all[row];
      if (!c) continue;
      const y = row * ROW_H - top;
      if (y > h || y + ROW_H < 0) continue;
      const cy = y + ROW_H / 2;

      if (c.oid === selectedRef.current) {
        ctx.fillStyle = "rgba(88,166,255,0.13)";
        ctx.fillRect(0, y, w, ROW_H);
        ctx.fillStyle = "#58a6ff";
        ctx.fillRect(0, y, 2, ROW_H);
      } else if (row % 2 === 1) {
        ctx.fillStyle = "rgba(255,255,255,0.012)";
        ctx.fillRect(0, y, w, ROW_H);
      }

      // commit dot
      const x = PAD_X + c.lane * LANE_W;
      ctx.beginPath();
      ctx.fillStyle = c.refs.some((r) => r.startsWith("HEAD")) ? "#7ee787" : LANE_COLORS[c.lane % LANE_COLORS.length];
      ctx.arc(x, cy, c.parents.length > 1 ? 4.4 : 3.6, 0, Math.PI * 2);
      ctx.fill();
      if (c.refs.some((r) => r.startsWith("HEAD"))) {
        ctx.strokeStyle = "rgba(126,231,135,0.45)";
        ctx.lineWidth = 3;
        ctx.beginPath();
        ctx.arc(x, cy, 8, 0, Math.PI * 2);
        ctx.stroke();
      }

      // right meta
      const meta = `${timeAgo(c.time)}  ${c.author}`;
      ctx.font = '11px "Segoe UI Variable", "Inter", system-ui, sans-serif';
      const metaW = Math.min(230, ctx.measureText(meta).width);
      ctx.fillStyle = "#6e7681";
      ctx.textAlign = "right";
      ctx.fillText(truncate(ctx, meta, 230), w - 18, cy);
      ctx.textAlign = "left";

      // refs chips
      let cx = laneArea + 8;
      ctx.font = '10.5px "Segoe UI Variable", "Inter", system-ui, sans-serif';
      for (const ref of c.refs.slice(0, 2)) {
        const label = ref.replace("HEAD → ", "");
        const tw = ctx.measureText(label).width;
        if (cx + tw + 14 > w - metaW - 60) break;
        const colors = refColor(ref);
        ctx.fillStyle = colors.bg;
        const chipW = tw + 12;
        ctx.beginPath();
        ctx.roundRect(cx, cy - 9, chipW, 18, 5);
        ctx.fill();
        ctx.fillStyle = colors.fg;
        ctx.fillText(label, cx + 6, cy + 0.5);
        cx += chipW + 6;
      }
      if (c.refs.length > 2) {
        ctx.fillStyle = "#8b949e";
        ctx.fillText(`+${c.refs.length - 2}`, cx, cy);
        cx += 22;
      }

      // summary + hash
      ctx.font = '12.5px "Segoe UI Variable", "Inter", system-ui, sans-serif';
      const hashW = 58;
      const maxText = w - cx - metaW - 36 - hashW;
      ctx.fillStyle = c.summary ? "#e6edf3" : "#6e7681";
      ctx.fillText(truncate(ctx, c.summary || "(no message)", maxText), cx, cy);
      ctx.font = '10.5px "Cascadia Code", "JetBrains Mono", monospace';
      ctx.fillStyle = "#484f58";
      ctx.fillText(c.short, w - metaW - 26 - hashW, cy);
    }

    // scrollbar
    const totalRows = totalRef.current;
    if (totalRows > 0) {
      const contentH = totalRows * ROW_H;
      if (contentH > h) {
        const trackX = w - 7;
        ctx.fillStyle = "rgba(255,255,255,0.04)";
        ctx.fillRect(trackX, 0, 4, h);
        const thumbH = Math.max(36, (h / contentH) * h);
        const maxTop = Math.max(1, contentH - h);
        const thumbY = (top / maxTop) * (h - thumbH);
        ctx.fillStyle = "rgba(139,148,158,0.45)";
        ctx.beginPath();
        ctx.roundRect(trackX, thumbY, 4, thumbH, 2);
        ctx.fill();
      }
    }
  }, [size.w, size.h]);

  const loadPage = useCallback(
    async (skip: number) => {
      if (loadingRef.current) return;
      if (skip > 0 && skip >= MAX_LOADED) return;
      loadingRef.current = true;
      setLoading(true);
      try {
        const page: GraphWindow = await api.graphWindow(repoId, skip, 500);        setError(null);
        commitsRef.current = skip === 0 ? page.commits : [...commitsRef.current, ...page.commits];
        const shifted = page.edges.map((e) => ({
          ...e,
          from_row: e.from_row + skip,
          to_row: e.to_row >= 0 ? e.to_row + skip : -1,
        }));
        edgesRef.current = skip === 0 ? shifted : [...edgesRef.current, ...shifted];
        maxLaneRef.current = Math.max(maxLaneRef.current, page.max_lane);
        totalRef.current = page.total;
        setCommits(commitsRef.current);
        setEdges(edgesRef.current);
        setMaxLane(maxLaneRef.current);
        setTotal(page.total);
      } catch (e) {
        setError(errorText(e));
      } finally {
        loadingRef.current = false;
        setLoading(false);
      }
    },
    [repoId],
  );

  useEffect(() => {
    commitsRef.current = [];
    edgesRef.current = [];
    maxLaneRef.current = 0;
    totalRef.current = 0;
    scrollRef.current = 0;
    setScrollTop(0);
    setCommits([]);
    setEdges([]);
    setTotal(0);
    setMaxLane(0);
    void loadPage(0);
  }, [repoId, refreshKey, loadPage]);

  useEffect(() => {
    paint();
  }, [commits, edges, selected, size, scrollTop, paint]);

  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      const r = entries[0].contentRect;
      setSize({ w: Math.max(320, r.width), h: Math.max(240, r.height) });
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const contentH = totalRef.current * ROW_H;
      const max = Math.max(0, contentH - size.h);
      const next = Math.max(0, Math.min(max, scrollRef.current + e.deltaY));
      scrollRef.current = next;
      setScrollTop(next);
      const loadedH = commitsRef.current.length * ROW_H;
      if (next + size.h > loadedH - ROW_H * 120) {
        void loadPage(commitsRef.current.length);
      }
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [loadPage, size.h]);

  const rowAt = useCallback((clientY: number): number => {
    const rect = wrapRef.current?.getBoundingClientRect();
    if (!rect) return -1;
    return Math.floor((clientY - rect.top + scrollRef.current) / ROW_H);
  }, []);

  return (
    <div
      ref={wrapRef}
      className="graph-wrap"
      onPointerDown={(e) => {
        const rect = wrapRef.current?.getBoundingClientRect();
        if (!rect) return;
        const x = e.clientX - rect.left;
        if (x > rect.width - 14) {
          dragRef.current = { startY: e.clientY, startOffset: scrollRef.current };
          (e.target as HTMLElement).setPointerCapture(e.pointerId);
          return;
        }
        const row = rowAt(e.clientY);
        const c = commitsRef.current[row];
        if (c) onSelect(c.oid);
      }}
      onPointerMove={(e) => {
        const drag = dragRef.current;
        if (!drag) return;
        const contentH = totalRef.current * ROW_H;
        const max = Math.max(0, contentH - size.h);
        const delta = (e.clientY - drag.startY) * (contentH / Math.max(1, size.h));
        const next = Math.max(0, Math.min(max, drag.startOffset + delta));
        scrollRef.current = next;
        setScrollTop(next);
      }}
      onPointerUp={() => {
        dragRef.current = null;
      }}
    >
      <canvas ref={canvasRef} style={{ width: "100%", height: "100%", display: "block" }} />
      {loading && commits.length === 0 && <div className="graph-overlay">Loading history…</div>}
      {error && <div className="graph-overlay error">{error}</div>}
      {!loading && !error && commits.length === 0 && (
        <div className="graph-overlay">
          <div className="empty-title">No commits yet</div>
          <div className="empty-sub">Stage files and make your first commit.</div>
        </div>
      )}
      {total > 0 && (
        <div className="graph-count">
          {commits.length.toLocaleString()} of {total.toLocaleString()} commits
          {commits.length >= MAX_LOADED && total > MAX_LOADED ? " · window capped (search to jump)" : ""}
        </div>
      )}
    </div>
  );
}
