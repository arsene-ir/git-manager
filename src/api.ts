import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  BranchInfo,
  CommitDetail,
  CommitFile,
  FileChange,
  FileDiff,
  GraphWindow,
  JobUpdate,
  MergePreview,
  OpsEntry,
  OpResult,
  RemoteInfo,
  RepoInfo,
  RepoSummary,
  SearchHit,
  SnapshotInfo,
  StashInfo,
  SystemInfo,
  TagInfo,
} from "./types";

export const api = {
  systemInfo: () => invoke<SystemInfo>("system_info"),
  listRepos: () => invoke<RepoInfo[]>("list_repos"),
  addRepo: (path: string) => invoke<RepoInfo>("add_repo", { path }),
  removeRepo: (id: string) => invoke<void>("remove_repo", { id }),
  activateRepo: (id: string) => invoke<RepoSummary>("activate_repo", { id }),
  repoSummary: (id: string) => invoke<RepoSummary>("repo_summary", { id }),
  refreshRepo: (id: string) => invoke<RepoSummary>("refresh_repo", { id }),
  cloneRepo: (url: string, dest: string) => invoke<RepoInfo>("clone_repo", { url, dest }),
  listJobs: () => invoke<JobUpdate[]>("list_jobs"),

  graphWindow: (id: string, skip: number, limit: number) =>
    invoke<GraphWindow>("graph_window", { id, skip, limit }),
  commitDetail: (id: string, oid: string) => invoke<CommitDetail>("commit_detail", { id, oid }),
  commitFiles: (id: string, oid: string) => invoke<CommitFile[]>("commit_files", { id, oid }),
  commitFileDiff: (id: string, oid: string, path: string) =>
    invoke<FileDiff>("commit_file_diff", { id, oid, path }),
  diffBetween: (id: string, from: string, to: string) =>
    invoke<CommitFile[]>("diff_between", { id, from, to }),

  status: (id: string) => invoke<FileChange[]>("working_status", { id }),
  fileDiff: (id: string, path: string, staged: boolean) =>
    invoke<FileDiff>("file_diff", { id, path, staged }),
  stage: (id: string, paths: string[]) => invoke<void>("stage_paths", { id, paths }),
  unstage: (id: string, paths: string[]) => invoke<void>("unstage_paths", { id, paths }),
  commit: (id: string, message: string, amend: boolean) =>
    invoke<OpResult>("commit_changes", { id, message, amend }),

  branches: (id: string) => invoke<BranchInfo[]>("branches", { id }),
  createBranch: (id: string, name: string, startPoint?: string) =>
    invoke<void>("create_branch", { id, name, startPoint: startPoint ?? null }),
  switchBranch: (id: string, name: string, stashFirst: boolean) =>
    invoke<OpResult>("switch_branch", { id, name, stashFirst }),
  deleteBranch: (id: string, name: string, force: boolean) =>
    invoke<OpResult>("delete_branch", { id, name, force }),

  mergePreview: (id: string, source: string) =>
    invoke<MergePreview>("merge_preview", { id, source }),
  merge: (id: string, source: string) => invoke<OpResult>("merge_branch", { id, source }),
  resolveConflict: (id: string, path: string, side: "ours" | "theirs") =>
    invoke<void>("resolve_conflict", { id, path, side }),
  markResolved: (id: string, path: string) => invoke<void>("mark_resolved", { id, path }),
  continueOp: (id: string) => invoke<OpResult>("continue_operation", { id }),
  abortOp: (id: string) => invoke<OpResult>("abort_operation", { id }),

  resetTo: (id: string, oid: string, mode: "soft" | "mixed" | "hard") =>
    invoke<OpResult>("reset_to", { id, oid, mode }),
  revert: (id: string, oid: string) => invoke<OpResult>("revert_commit", { id, oid }),
  cherryPick: (id: string, oid: string) => invoke<OpResult>("cherry_pick", { id, oid }),

  stashList: (id: string) => invoke<StashInfo[]>("stash_list", { id }),
  stashPush: (id: string, message?: string) =>
    invoke<OpResult>("stash_push", { id, message: message ?? null }),
  stashApply: (id: string, index: number, pop: boolean) =>
    invoke<OpResult>("stash_apply", { id, index, pop }),
  stashDrop: (id: string, index: number) => invoke<OpResult>("stash_drop", { id, index }),

  tags: (id: string) => invoke<TagInfo[]>("tags_list", { id }),
  createTag: (id: string, name: string, message?: string, oid?: string) =>
    invoke<void>("tag_create", { id, name, message: message ?? null, oid: oid ?? null }),
  deleteTag: (id: string, name: string) => invoke<void>("tag_delete", { id, name }),

  remotes: (id: string) => invoke<RemoteInfo[]>("remotes_list", { id }),
  addRemote: (id: string, name: string, url: string) =>
    invoke<void>("remote_add", { id, name, url }),
  removeRemote: (id: string, name: string) => invoke<void>("remote_remove", { id, name }),
  fetch: (id: string) => invoke<OpResult>("fetch_remote", { id }),
  pull: (id: string, mode: "merge" | "rebase" | "ff-only") =>
    invoke<OpResult>("pull_remote", { id, mode }),
  push: (id: string, remote: string, src: string, dst: string, forceWithLease: boolean) =>
    invoke<OpResult>("push_remote", { id, remote, src, dst, forceWithLease }),

  search: (id: string, query: string) => invoke<SearchHit[]>("search_repo", { id, query }),
  opsLog: (id: string) => invoke<OpsEntry[]>("ops_log", { id }),
  snapshots: (id: string) => invoke<SnapshotInfo[]>("snapshots_list", { id }),
  restoreSnapshot: (id: string, snapshotId: string) =>
    invoke<OpResult>("snapshot_restore", { id, snapshotId }),
};

export const events = {
  onRepoChanged: (cb: (repoId: string) => void) =>
    listen<string>("repo://changed", (e) => cb(e.payload)),
  onJob: (cb: (job: JobUpdate) => void) => listen<JobUpdate>("job://update", (e) => cb(e.payload)),
};

export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return JSON.stringify(e);
}
