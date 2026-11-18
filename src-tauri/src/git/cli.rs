use std::path::Path;
use std::process::Command;

use crate::error::{AppError, AppResult};
use crate::security::redact;

#[derive(Debug, Clone)]
pub struct GitOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Run git with an explicit argument array. Never invokes a shell.
pub fn git(repo: &Path, args: &[&str]) -> AppResult<GitOutput> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .map_err(|e| AppError::msg(format!("Could not run git (is it installed?): {e}")))?;

    Ok(GitOutput {
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// Run git and require success.
pub fn git_ok(repo: &Path, args: &[&str]) -> AppResult<String> {
    let out = git(repo, args)?;
    if out.status != 0 {
        let detail = if out.stderr.trim().is_empty() {
            out.stdout.trim().to_string()
        } else {
            out.stderr.trim().to_string()
        };
        return Err(AppError::Git(redact(&detail)));
    }
    Ok(out.stdout)
}

/// Run git with extra environment variables (e.g. GIT_EDITOR=true for sequencer continue).
pub fn git_env(repo: &Path, args: &[&str], envs: &[(&str, &str)]) -> AppResult<GitOutput> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo).args(args);
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let output = cmd
        .output()
        .map_err(|e| AppError::msg(format!("Could not run git: {e}")))?;
    Ok(GitOutput {
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

pub fn git_version() -> Option<String> {
    let out = Command::new("git").arg("--version").output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

/// `git merge-tree --write-tree` is supported from git 2.38.
pub fn supports_merge_tree() -> bool {
    git_version()
        .and_then(|v| {
            let core = v.trim_start_matches("git version ").to_string();
            let mut parts = core.split('.');
            let major: u32 = parts.next()?.parse().ok()?;
            let minor: u32 = parts.next()?.parse().ok()?;
            Some(major > 2 || (major == 2 && minor >= 38))
        })
        .unwrap_or(false)
}
