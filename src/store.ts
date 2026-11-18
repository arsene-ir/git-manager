import { create } from "zustand";
import type { FileChange, JobUpdate, RepoInfo, RepoSummary } from "./types";

export type Tab = "changes" | "commit" | "conflicts" | "recovery";

interface Toast {
  id: number;
  kind: "info" | "error" | "success";
  text: string;
}

interface State {
  repos: RepoInfo[];
  activeId: string | null;
  summary: RepoSummary | null;
  changes: FileChange[];
  selectedOid: string | null;
  selectedPath: string | null;
  tab: Tab;
  jobs: JobUpdate[];
  toasts: Toast[];
  graphKey: number;
  searching: boolean;
  setRepos: (r: RepoInfo[]) => void;
  setActive: (id: string | null) => void;
  setSummary: (s: RepoSummary | null) => void;
  setChanges: (c: FileChange[]) => void;
  selectCommit: (oid: string | null) => void;
  selectPath: (p: string | null) => void;
  setTab: (t: Tab) => void;
  pushJob: (j: JobUpdate) => void;
  toast: (kind: Toast["kind"], text: string) => void;
  dismissToast: (id: number) => void;
  bumpGraph: () => void;
  setSearching: (v: boolean) => void;
}

let toastSeq = 1;

export const useStore = create<State>((set) => ({
  repos: [],
  activeId: null,
  summary: null,
  changes: [],
  selectedOid: null,
  selectedPath: null,
  tab: "changes",
  jobs: [],
  toasts: [],
  graphKey: 0,
  searching: false,
  setRepos: (repos) => set({ repos }),
  setActive: (activeId) => set({ activeId, selectedOid: null, selectedPath: null }),
  setSummary: (summary) => set({ summary }),
  setChanges: (changes) => set({ changes }),
  selectCommit: (selectedOid) => set({ selectedOid, tab: selectedOid ? "commit" : "changes" }),
  selectPath: (selectedPath) => set({ selectedPath }),
  setTab: (tab) => set({ tab }),
  pushJob: (job) =>
    set((s) => ({
      jobs: [job, ...s.jobs.filter((j) => j.id !== job.id)].slice(0, 8),
      toasts:
        job.status === "failed"
          ? [...s.toasts, { id: toastSeq++, kind: "error" as const, text: job.message }].slice(-4)
          : s.toasts,
    })),
  toast: (kind, text) =>
    set((s) => ({ toasts: [...s.toasts, { id: toastSeq++, kind, text }].slice(-4) })),
  dismissToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
  bumpGraph: () => set((s) => ({ graphKey: s.graphKey + 1 })),
  setSearching: (searching) => set({ searching }),
}));
