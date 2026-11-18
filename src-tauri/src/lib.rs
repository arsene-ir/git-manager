mod commands;
mod dto;
mod error;
mod git;
mod security;
mod store;

use tauri::Manager;

pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "git_manager_lib=info".into()),
        )
        .try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let conn = store::open_db(&data_dir.join("git-manager.db"))?;
            app.manage(commands::AppState::new(conn, data_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system_info,
            commands::list_repos,
            commands::add_repo,
            commands::remove_repo,
            commands::activate_repo,
            commands::repo_summary,
            commands::refresh_repo,
            commands::clone_repo,
            commands::list_jobs,
            commands::graph_window,
            commands::commit_detail,
            commands::commit_files,
            commands::commit_file_diff,
            commands::diff_between,
            commands::working_status,
            commands::file_diff,
            commands::stage_paths,
            commands::unstage_paths,
            commands::commit_changes,
            commands::branches,
            commands::create_branch,
            commands::switch_branch,
            commands::delete_branch,
            commands::merge_preview,
            commands::merge_branch,
            commands::resolve_conflict,
            commands::mark_resolved,
            commands::continue_operation,
            commands::abort_operation,
            commands::reset_to,
            commands::revert_commit,
            commands::cherry_pick,
            commands::stash_list,
            commands::stash_push,
            commands::stash_apply,
            commands::stash_drop,
            commands::tags_list,
            commands::tag_create,
            commands::tag_delete,
            commands::remotes_list,
            commands::remote_add,
            commands::remote_remove,
            commands::fetch_remote,
            commands::pull_remote,
            commands::push_remote,
            commands::search_repo,
            commands::ops_log,
            commands::snapshots_list,
            commands::snapshot_restore,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
