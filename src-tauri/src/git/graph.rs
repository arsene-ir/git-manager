use std::collections::HashMap;
use std::path::Path;

use git2::{Oid, Repository};

use crate::dto::{CommitNode, GraphEdge, GraphWindow};
use crate::error::AppResult;
use crate::git::read;

/// Compute a lane for every commit in topological order.
/// `parents[c]` contains commit indices greater than `c` (topo guarantee).
#[allow(dead_code)]
pub fn assign_lanes(parents: &[Vec<usize>]) -> Vec<u16> {
    let mut state = LaneState::default();
    parents
        .iter()
        .enumerate()
        .map(|(c, ps)| state.step(c, ps))
        .collect()
}

/// Incremental lane state: one lane slot per active branch line.
#[derive(Default)]
pub struct LaneState {
    lanes: Vec<Option<usize>>,
}

impl LaneState {
    pub fn step(&mut self, c: usize, parent_idxs: &[usize]) -> u16 {
        let lane = match self.lanes.iter().position(|x| *x == Some(c)) {
            Some(l) => l,
            None => {
                let l = self.lanes.iter().position(|x| x.is_none()).unwrap_or_else(|| {
                    self.lanes.push(None);
                    self.lanes.len() - 1
                });
                l
            }
        };
        self.lanes[lane] = None;

        if let Some(&p) = parent_idxs.first() {
            if !self.lanes.iter().any(|x| *x == Some(p)) {
                self.lanes[lane] = Some(p);
            }
        }
        for &p in parent_idxs.iter().skip(1) {
            if self.lanes.iter().any(|x| *x == Some(p)) {
                continue;
            }
            let l = self.lanes.iter().position(|x| x.is_none()).unwrap_or_else(|| {
                self.lanes.push(None);
                self.lanes.len() - 1
            });
            self.lanes[l] = Some(p);
        }
        // Drop expectations that point at already-processed commits.
        for slot in self.lanes.iter_mut() {
            if let Some(p) = *slot {
                if p <= c {
                    *slot = None;
                }
            }
        }
        lane as u16
    }
}

/// Full ordered commit list plus lazily computed lanes.
pub struct RepoGraph {
    oids: Vec<Oid>,
    parents: Vec<Vec<usize>>,
    lanes: Vec<u16>,
    lane_state: LaneState,
    refs: HashMap<Oid, Vec<String>>,
}

impl RepoGraph {
    pub fn build(path: &Path) -> AppResult<Self> {
        let repo = Repository::open(path)?;
        let refs = read::refs_map(&repo);
        let oids = read::all_commit_oids(&repo)?;
        let index: HashMap<Oid, usize> = oids.iter().enumerate().map(|(i, o)| (*o, i)).collect();
        let mut parents: Vec<Vec<usize>> = Vec::with_capacity(oids.len());
        for oid in &oids {
            let mut idxs = Vec::new();
            if let Ok(commit) = repo.find_commit(*oid) {
                for p in commit.parent_ids() {
                    if let Some(i) = index.get(&p) {
                        idxs.push(*i);
                    }
                }
            }
            parents.push(idxs);
        }
        Ok(Self {
            oids,
            parents,
            lanes: Vec::new(),
            lane_state: LaneState::default(),
            refs,
        })
    }

    /// Message/author/hash search across the ordered history (bounded scan).
    pub fn search_commits(&self, path: &Path, query: &str, limit: usize) -> Vec<(String, String, String, i64)> {
        let q = query.to_lowercase();
        let Ok(repo) = Repository::open(path) else { return Vec::new() };
        let mut out = Vec::new();
        for oid in self.oids.iter().take(50_000) {
            if out.len() >= limit {
                break;
            }
            let Ok(c) = repo.find_commit(*oid) else { continue };
            let summary = c.summary().ok().flatten().unwrap_or("");
            let author = c.author().name().unwrap_or("").to_string();
            let short = oid.to_string()[..7].to_string();
            if summary.to_lowercase().contains(&q)
                || author.to_lowercase().contains(&q)
                || short.contains(&q)
            {
                out.push((oid.to_string(), summary.to_string(), author, c.time().seconds()));
            }
        }
        out
    }

    fn ensure(&mut self, upto: usize) {
        let upto = upto.min(self.oids.len());
        while self.lanes.len() < upto {
            let c = self.lanes.len();
            let ps = self.parents.get(c).cloned().unwrap_or_default();
            let lane = self.lane_state.step(c, &ps);
            self.lanes.push(lane);
        }
    }

    pub fn window(&mut self, path: &Path, skip: usize, limit: usize) -> AppResult<GraphWindow> {
        let total = self.oids.len();
        let start = skip.min(total);
        let end = (start + limit).min(total);
        self.ensure(end);

        let repo = Repository::open(path)?;
        let mut commits = Vec::with_capacity(end - start);
        for i in start..end {
            let oid = self.oids[i];
            let refs = self.refs.get(&oid).cloned().unwrap_or_default();
            let (summary, author, email, time, parents) = match repo.find_commit(oid) {
                Ok(c) => (
                    c.summary().ok().flatten().unwrap_or("").to_string(),
                    c.author().name().unwrap_or("").to_string(),
                    c.author().email().unwrap_or("").to_string(),
                    c.time().seconds(),
                    c.parent_ids().map(|p| p.to_string()).collect::<Vec<_>>(),
                ),
                Err(_) => (String::new(), String::new(), String::new(), 0, Vec::new()),
            };
            commits.push(CommitNode {
                oid: oid.to_string(),
                short: oid.to_string()[..7].to_string(),
                summary,
                author,
                email,
                time,
                parents,
                refs,
                lane: self.lanes[i],
            });
        }

        let mut edges = Vec::new();
        let mut max_lane = 0u16;
        for (row, node) in commits.iter().enumerate() {
            max_lane = max_lane.max(node.lane);
            let abs = start + row;
            if let Some(parents) = self.parents.get(abs) {
                for p in parents {
                    if *p < start || *p >= end {
                        edges.push(GraphEdge {
                            from_row: row as i32,
                            from_lane: node.lane,
                            to_row: -1,
                            to_lane: node.lane,
                        });
                        continue;
                    }
                    let to_lane = self.lanes[*p];
                    max_lane = max_lane.max(to_lane);
                    edges.push(GraphEdge {
                        from_row: row as i32,
                        from_lane: node.lane,
                        to_row: (*p - start) as i32,
                        to_lane,
                    });
                }
            }
        }

        Ok(GraphWindow {
            commits,
            edges,
            max_lane,
            total: total as u32,
            next_skip: if end < total { Some(end as u32) } else { None },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_history_is_single_lane() {
        let parents = vec![vec![1], vec![2], vec![3], vec![]];
        assert_eq!(assign_lanes(&parents), vec![0, 0, 0, 0]);
    }

    #[test]
    fn merge_allocates_second_lane_and_rejoins() {
        // 0 -> (1, 2); 1 -> 3; 2 -> 3
        let parents = vec![vec![1, 2], vec![3], vec![3], vec![]];
        let lanes = assign_lanes(&parents);
        assert_eq!(lanes[0], 0);
        assert_eq!(lanes[2], 1);
        assert_eq!(lanes[3], 0);
    }

    #[test]
    fn independent_roots_do_not_overlap() {
        let parents = vec![vec![2], vec![3], vec![4], vec![], vec![]];
        let lanes = assign_lanes(&parents);
        assert_eq!(lanes[0], 0);
        assert_eq!(lanes[1], 1);
        assert_eq!(lanes[2], 0);
        assert_eq!(lanes[3], 1);
        assert_eq!(lanes[4], 0);
    }

    #[test]
    fn lane_state_matches_assign_lanes() {
        let parents = vec![vec![1, 2], vec![3], vec![3], vec![]];
        let mut state = LaneState::default();
        let mut got = Vec::new();
        for (c, ps) in parents.iter().enumerate() {
            got.push(state.step(c, ps));
        }
        assert_eq!(got, assign_lanes(&parents));
    }

    #[test]
    fn builds_window_from_real_repository() {
        use std::process::Command;
        let dir = std::env::temp_dir().join(format!(
            "gitmgr-it-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let git = |args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
                .args(args)
                .output()
                .unwrap();
            assert!(out.status.success(), "git {:?} failed: {}", args, String::from_utf8_lossy(&out.stderr));
        };
        git(&["init", "-q", "-b", "main"]);
        std::fs::write(dir.join("a.txt"), "one\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "first"]);
        git(&["switch", "-q", "-c", "feature"]);
        std::fs::write(dir.join("b.txt"), "two\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "feature work"]);
        git(&["switch", "-q", "main"]);
        std::fs::write(dir.join("c.txt"), "three\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "main work"]);
        git(&["merge", "-q", "--no-ff", "feature", "-m", "merge feature"]);

        let mut graph = RepoGraph::build(&dir).unwrap();
        let window = graph.window(&dir, 0, 100).unwrap();
        assert_eq!(window.total, 4);
        assert_eq!(window.commits.len(), 4);
        assert!(window.max_lane >= 1, "merge should use more than one lane");
        assert_eq!(window.commits[0].summary, "merge feature");

        let summary = crate::git::read::summary(&dir).unwrap();
        assert_eq!(summary.branch.as_deref(), Some("main"));
        assert_eq!(summary.operation, "clean");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
