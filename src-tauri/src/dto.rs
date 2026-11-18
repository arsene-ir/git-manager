use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub last_opened: i64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RepoSummary {
    pub head_oid: Option<String>,
    pub short_oid: Option<String>,
    pub branch: Option<String>,
    pub detached: bool,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub staged: usize,
    pub unstaged: usize,
    pub untracked: usize,
    pub conflicts: usize,
    pub operation: String,
    pub remotes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitNode {
    pub oid: String,
    pub short: String,
    pub summary: String,
    pub author: String,
    pub email: String,
    pub time: i64,
    pub parents: Vec<String>,
    pub refs: Vec<String>,
    pub lane: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub from_row: i32,
    pub from_lane: u16,
    pub to_row: i32,
    pub to_lane: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphWindow {
    pub commits: Vec<CommitNode>,
    pub edges: Vec<GraphEdge>,
    pub max_lane: u16,
    pub total: u32,
    pub next_skip: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitDetail {
    pub oid: String,
    pub short: String,
    pub message: String,
    pub summary: String,
    pub author: String,
    pub author_email: String,
    pub committer: String,
    pub time: i64,
    pub parents: Vec<String>,
    pub refs: Vec<String>,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileChange {
    pub path: String,
    pub old_path: Option<String>,
    pub status: String,
    pub staged: bool,
    pub conflict: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitFile {
    pub path: String,
    pub additions: usize,
    pub deletions: usize,
    pub binary: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffLine {
    pub kind: String,
    pub text: String,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub binary: bool,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub tip: String,
    pub tip_summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TagInfo {
    pub name: String,
    pub oid: String,
    pub annotated: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub push_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MergePreview {
    pub clean: bool,
    pub tree: Option<String>,
    pub conflicts: Vec<String>,
    pub messages: Vec<String>,
    pub merge_base: Option<String>,
    pub supported: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct StashInfo {
    pub index: usize,
    pub oid: String,
    pub message: String,
    pub time: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotInfo {
    pub id: String,
    pub repo_id: String,
    pub op: String,
    pub head_ref: Option<String>,
    pub stash_ref: Option<String>,
    pub untracked_dir: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpsEntry {
    pub id: i64,
    pub repo_id: String,
    pub action: String,
    pub detail: String,
    pub status: String,
    pub snapshot_id: Option<String>,
    pub duration_ms: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobUpdate {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    pub oid: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub git_version: String,
    pub merge_tree: bool,
    pub platform: String,
    pub app_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpResult {
    pub ok: bool,
    pub message: String,
    pub conflicts: bool,
}
