export interface RepoInfo {
  id: string;
  name: string;
  path: string;
  last_opened: number;
}

export interface RepoSummary {
  head_oid: string | null;
  short_oid: string | null;
  branch: string | null;
  detached: boolean;
  upstream: string | null;
  ahead: number;
  behind: number;
  staged: number;
  unstaged: number;
  untracked: number;
  conflicts: number;
  operation: string;
  remotes: string[];
}

export interface CommitNode {
  oid: string;
  short: string;
  summary: string;
  author: string;
  email: string;
  time: number;
  parents: string[];
  refs: string[];
  lane: number;
}

export interface GraphEdge {
  from_row: number;
  from_lane: number;
  to_row: number;
  to_lane: number;
}

export interface GraphWindow {
  commits: CommitNode[];
  edges: GraphEdge[];
  max_lane: number;
  total: number;
  next_skip: number | null;
}

export interface CommitDetail {
  oid: string;
  short: string;
  message: string;
  summary: string;
  author: string;
  author_email: string;
  committer: string;
  time: number;
  parents: string[];
  refs: string[];
  files_changed: number;
  insertions: number;
  deletions: number;
}

export interface FileChange {
  path: string;
  old_path: string | null;
  status: string;
  staged: boolean;
  conflict: boolean;
}

export interface CommitFile {
  path: string;
  additions: number;
  deletions: number;
  binary: boolean;
}

export interface DiffLine {
  kind: "ctx" | "add" | "del" | string;
  text: string;
  old_lineno: number | null;
  new_lineno: number | null;
}

export interface DiffHunk {
  header: string;
  lines: DiffLine[];
}

export interface FileDiff {
  path: string;
  old_path: string | null;
  binary: boolean;
  hunks: DiffHunk[];
}

export interface BranchInfo {
  name: string;
  is_head: boolean;
  is_remote: boolean;
  upstream: string | null;
  ahead: number;
  behind: number;
  tip: string;
  tip_summary: string;
}

export interface TagInfo {
  name: string;
  oid: string;
  annotated: boolean;
  message: string | null;
}

export interface RemoteInfo {
  name: string;
  url: string;
  push_url: string | null;
}

export interface MergePreview {
  clean: boolean;
  tree: string | null;
  conflicts: string[];
  messages: string[];
  merge_base: string | null;
  supported: boolean;
}

export interface StashInfo {
  index: number;
  oid: string;
  message: string;
  time: number;
}

export interface SnapshotInfo {
  id: string;
  repo_id: string;
  op: string;
  head_ref: string | null;
  stash_ref: string | null;
  untracked_dir: string | null;
  created_at: number;
}

export interface OpsEntry {
  id: number;
  repo_id: string;
  action: string;
  detail: string;
  status: string;
  snapshot_id: string | null;
  duration_ms: number;
  created_at: number;
}

export interface JobUpdate {
  id: string;
  kind: string;
  status: string;
  message: string;
}

export interface SearchHit {
  kind: string;
  title: string;
  subtitle: string;
  oid: string | null;
  path: string | null;
}

export interface SystemInfo {
  git_version: string;
  merge_tree: boolean;
  platform: string;
  app_version: string;
}

export interface OpResult {
  ok: boolean;
  message: string;
  conflicts: boolean;
}
