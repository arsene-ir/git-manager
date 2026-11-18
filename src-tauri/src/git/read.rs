use std::collections::HashMap;
use std::path::Path;

use git2::{BranchType, Oid, Repository, Sort, Status};

use crate::dto::*;
use crate::error::{AppError, AppResult};

pub fn open(path: &Path) -> AppResult<Repository> {
    Repository::discover(path).map_err(AppError::from)
}

pub fn refs_map(repo: &Repository) -> HashMap<Oid, Vec<String>> {
    let mut map: HashMap<Oid, Vec<String>> = HashMap::new();
    let head_branch = repo
        .head()
        .ok()
        .filter(|h| h.is_branch())
        .and_then(|h| h.shorthand().ok().map(|s| s.to_string()));

    if let Ok(references) = repo.references() {
        for r in references.flatten() {
            let Ok(name) = r.name() else { continue };
            if name.starts_with("refs/backups/") || name == "HEAD" {
                continue;
            }
            let Ok(commit) = r.peel_to_commit() else { continue };
            let label = if let Some(b) = name.strip_prefix("refs/heads/") {
                match &head_branch {
                    Some(h) if h == b => format!("HEAD → {b}"),
                    _ => b.to_string(),
                }
            } else if let Some(b) = name.strip_prefix("refs/remotes/") {
                b.to_string()
            } else if let Some(t) = name.strip_prefix("refs/tags/") {
                format!("tag: {t}")
            } else {
                continue;
            };
            let entry = map.entry(commit.id()).or_default();
            if !entry.contains(&label) {
                entry.push(label);
            }
        }
    }

    for labels in map.values_mut() {
        labels.sort_by_key(|l| {
            let rank = if l.starts_with("HEAD") {
                0
            } else if l.starts_with("tag:") {
                2
            } else if l.contains('/') {
                3
            } else {
                1
            };
            (rank, l.clone())
        });
    }
    map
}

pub fn summary(path: &Path) -> AppResult<RepoSummary> {
    let repo = open(path)?;
    let mut s = RepoSummary::default();

    if let Ok(head) = repo.head() {
        s.head_oid = head.target().map(|o| o.to_string());
        s.short_oid = s.head_oid.as_ref().map(|o| o[..7.min(o.len())].to_string());
        if head.is_branch() {
            s.branch = head.shorthand().ok().map(|x| x.to_string());
        } else {
            s.detached = true;
            s.branch = Some(head.shorthand().ok().unwrap_or("HEAD").to_string());
        }
    }

    if let Some(branch_name) = &s.branch {
        if !s.detached {
            if let Ok(branch) = repo.find_branch(branch_name, BranchType::Local) {
                if let Ok(upstream) = branch.upstream() {
                    s.upstream = upstream.name().ok().flatten().map(|x| x.to_string());                    if let (Some(local), Some(up)) = (branch.get().target(), upstream.get().target()) {
                        if let Ok((a, b)) = repo.graph_ahead_behind(local, up) {
                            s.ahead = a;
                            s.behind = b;
                        }
                    }
                }
            }
        }
    }

    if let Ok(statuses) = repo.statuses(Some(
        git2::StatusOptions::new()
            .include_untracked(true)
            .recurse_untracked_dirs(true)
            .renames_head_to_index(true)
            .exclude_submodules(false),
    )) {
        for e in statuses.iter() {
            let st = e.status();
            if st.contains(Status::CONFLICTED) {
                s.conflicts += 1;
                continue;
            }
            if st.intersects(
                Status::INDEX_NEW
                    | Status::INDEX_MODIFIED
                    | Status::INDEX_DELETED
                    | Status::INDEX_RENAMED
                    | Status::INDEX_TYPECHANGE,
            ) {
                s.staged += 1;
            }
            if st.contains(Status::WT_NEW) {
                s.untracked += 1;
            } else if st.intersects(
                Status::WT_MODIFIED | Status::WT_DELETED | Status::WT_RENAMED | Status::WT_TYPECHANGE,
            ) {
                s.unstaged += 1;
            }
        }
    }

    s.operation = match repo.state() {
        git2::RepositoryState::Clean => "clean",
        git2::RepositoryState::Merge => "merge",
        git2::RepositoryState::Revert | git2::RepositoryState::RevertSequence => "revert",
        git2::RepositoryState::CherryPick | git2::RepositoryState::CherryPickSequence => "cherry-pick",
        git2::RepositoryState::Bisect => "bisect",
        git2::RepositoryState::Rebase | git2::RepositoryState::RebaseInteractive | git2::RepositoryState::RebaseMerge => "rebase",
        git2::RepositoryState::ApplyMailbox | git2::RepositoryState::ApplyMailboxOrRebase => "am",
    }
    .to_string();

    if let Ok(remotes) = repo.remotes() {
        s.remotes = remotes.iter().flatten().flatten().map(|x| x.to_string()).collect();
    }
    Ok(s)
}

pub fn status(path: &Path) -> AppResult<Vec<FileChange>> {
    let repo = open(path)?;
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .renames_head_to_index(true);
    let statuses = repo.statuses(Some(&mut opts))?;
    let mut out = Vec::new();
    for e in statuses.iter() {
        let st = e.status();
        let path = e.path().unwrap_or("").to_string();
        let conflict = st.contains(Status::CONFLICTED);
        let mut staged_made = false;
        if st.intersects(
            Status::INDEX_NEW | Status::INDEX_MODIFIED | Status::INDEX_DELETED | Status::INDEX_RENAMED | Status::INDEX_TYPECHANGE,
        ) {
            let kind = if st.contains(Status::INDEX_NEW) {
                "added"
            } else if st.contains(Status::INDEX_DELETED) {
                "deleted"
            } else if st.contains(Status::INDEX_RENAMED) {
                "renamed"
            } else if st.contains(Status::INDEX_TYPECHANGE) {
                "typechange"
            } else {
                "modified"
            };
            out.push(FileChange { path: path.clone(), old_path: None, status: kind.into(), staged: true, conflict });
            staged_made = true;
        }
        if conflict {
            if !staged_made {
                out.push(FileChange { path: path.clone(), old_path: None, status: "conflicted".into(), staged: false, conflict: true });
            }
            continue;
        }
        let wt_kind = if st.contains(Status::WT_NEW) {
            Some("untracked")
        } else if st.contains(Status::WT_DELETED) {
            Some("deleted")
        } else if st.contains(Status::WT_RENAMED) {
            Some("renamed")
        } else if st.contains(Status::WT_TYPECHANGE) {
            Some("typechange")
        } else if st.contains(Status::WT_MODIFIED) {
            Some("modified")
        } else {
            None
        };
        if let Some(kind) = wt_kind {
            out.push(FileChange { path, old_path: None, status: kind.into(), staged: false, conflict: false });
        }
    }
    out.sort_by(|a, b| (a.staged, &a.path).cmp(&(b.staged, &b.path)));
    Ok(out)
}

pub fn commit_detail(path: &Path, oid: &str) -> AppResult<CommitDetail> {
    let repo = open(path)?;
    let oid = Oid::from_str(oid).map_err(|_| AppError::msg("Invalid commit id"))?;
    let commit = repo.find_commit(oid)?;
    let author = commit.author();
    let committer = commit.committer();
    let parents: Vec<String> = commit.parent_ids().map(|p| p.to_string()).collect();
    let refs = refs_map(&repo).get(&oid).cloned().unwrap_or_default();

    let tree = commit.tree()?;
    let parent_tree = if commit.parent_count() > 0 {
        Some(commit.parent(0)?.tree()?)
    } else {
        None
    };
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
    let stats = diff.stats()?;

    let message = commit.message().unwrap_or("").to_string();
    let summary = message.lines().next().unwrap_or("").to_string();
    Ok(CommitDetail {
        oid: oid.to_string(),
        short: oid.to_string()[..7].to_string(),
        message,
        summary,
        author: author.name().unwrap_or("").to_string(),
        author_email: author.email().unwrap_or("").to_string(),
        committer: committer.name().unwrap_or("").to_string(),
        time: commit.time().seconds(),
        parents,
        refs,
        files_changed: stats.files_changed(),
        insertions: stats.insertions(),
        deletions: stats.deletions(),
    })
}

pub fn commit_files(path: &Path, oid: &str) -> AppResult<Vec<CommitFile>> {
    let out = crate::git::cli::git_ok(
        path,
        &["show", "--numstat", "--format=", "--find-renames", oid, "--"],
    )?;
    let mut files = Vec::new();
    for line in out.lines() {
        let mut parts = line.splitn(3, '\t');
        let (Some(add), Some(del), Some(p)) = (parts.next(), parts.next(), parts.next()) else { continue };
        let binary = add == "-" && del == "-";
        files.push(CommitFile {
            path: p.to_string(),
            additions: add.parse().unwrap_or(0),
            deletions: del.parse().unwrap_or(0),
            binary,
        });
    }
    Ok(files)
}

pub fn branches(path: &Path) -> AppResult<Vec<BranchInfo>> {
    let repo = open(path)?;
    let head = repo.head().ok().and_then(|h| h.shorthand().ok().map(|s| s.to_string()));
    let mut out = Vec::new();
    for bt in [BranchType::Local, BranchType::Remote] {
        let iter = repo.branches(Some(bt))?;
        for item in iter {
            let (branch, _) = item?;
            let Some(name) = branch.name().ok().flatten() else { continue };
            if bt == BranchType::Remote && name.ends_with("/HEAD") {
                continue;
            }
            let is_remote = bt == BranchType::Remote;
            let is_head = !is_remote && head.as_deref() == Some(name);
            let upstream = branch
                .upstream()
                .ok()
                .and_then(|u| u.name().ok().flatten().map(|s| s.to_string()));
            let (ahead, behind) = if let (Some(local), Some(up)) = (branch.get().target(), branch.upstream().ok().and_then(|u| u.get().target())) {
                repo.graph_ahead_behind(local, up).unwrap_or((0, 0))
            } else {
                (0, 0)
            };
            let tip = branch.get().peel_to_commit().ok();
            out.push(BranchInfo {
                name: name.to_string(),
                is_head,
                is_remote,
                upstream,
                ahead,
                behind,
                tip: tip.as_ref().map(|c| c.id().to_string()[..7].to_string()).unwrap_or_default(),
                tip_summary: tip.and_then(|c| c.summary().ok().flatten().map(|s| s.to_string())).unwrap_or_default(),
            });
        }
    }
    out.sort_by(|a, b| (a.is_remote, &a.name).cmp(&(b.is_remote, &b.name)));
    Ok(out)
}

pub fn tags(path: &Path) -> AppResult<Vec<TagInfo>> {
    let repo = open(path)?;
    let names = repo.tag_names(None)?;
    let mut out = Vec::new();
    for name in names.iter().flatten().flatten() {
        let Ok(r) = repo.find_reference(&format!("refs/tags/{name}")) else { continue };
        let annotated = r.peel(git2::ObjectType::Tag).is_ok();
        let message = if annotated {
            r.target()
                .and_then(|target| repo.find_tag(target).ok())
                .and_then(|t| t.message().ok().flatten().map(|m| m.to_string()))
        } else {
            None
        };
        let oid = r.peel_to_commit().map(|c| c.id().to_string()).unwrap_or_default();
        out.push(TagInfo { name: name.to_string(), oid, annotated, message });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub fn remotes(path: &Path) -> AppResult<Vec<RemoteInfo>> {
    let repo = open(path)?;
    let names = repo.remotes()?;
    let mut out = Vec::new();
    for name in names.iter().flatten().flatten() {
        if let Ok(r) = repo.find_remote(name) {
            out.push(RemoteInfo {
                name: name.to_string(),
                url: r.url().unwrap_or("").to_string(),
                push_url: r.pushurl().ok().flatten().map(|s| s.to_string()),
            });
        }
    }
    Ok(out)
}

#[allow(dead_code)]
pub fn head_oid(path: &Path) -> Option<Oid> {
    open(path).ok()?.head().ok()?.target()
}/// Full commit ordering (newest first) used by the graph cache.
pub fn all_commit_oids(repo: &Repository) -> AppResult<Vec<Oid>> {
    let mut walk = repo.revwalk()?;
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
    let mut pushed = false;
    if walk.push_head().is_ok() {
        pushed = true;
    }
    for glob in ["refs/heads/*", "refs/remotes/*", "refs/tags/*"] {
        if walk.push_glob(glob).is_ok() {
            pushed = true;
        }
    }
    if !pushed {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for oid in walk.flatten() {
        out.push(oid);
    }
    Ok(out)
}
