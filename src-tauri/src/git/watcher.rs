use std::path::Path;
use std::time::Duration;

use notify::{RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter};

/// Per-repository filesystem watcher. Debounces bursts and emits `repo://changed`.
pub struct RepoWatcher {
    _inner: notify::RecommendedWatcher,
}

pub fn start(repo_id: String, repo: &Path, app: AppHandle) -> crate::error::AppResult<RepoWatcher> {
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            let relevant = event.paths.iter().any(|p| {
                let s = p.to_string_lossy();
                !s.contains("node_modules")
                    && !s.contains("\\target\\")
                    && !s.contains("/target/")
                    && !s.contains("\\.git\\objects\\")
                    && !s.contains("/.git/objects/")
            });
            if relevant {
                let _ = tx.send(());
            }
        }
    })
    .map_err(|e| crate::error::AppError::msg(format!("Watcher error: {e}")))?;

    watcher
        .watch(repo, RecursiveMode::Recursive)
        .map_err(|e| crate::error::AppError::msg(format!("Watcher error: {e}")))?;

    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            std::thread::sleep(Duration::from_millis(300));
            while rx.try_recv().is_ok() {}
            let _ = app.emit("repo://changed", repo_id.clone());
        }
    });

    Ok(RepoWatcher { _inner: watcher })
}
