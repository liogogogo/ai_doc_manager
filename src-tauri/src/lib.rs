mod commands;
pub mod core;
pub mod models;
pub mod services;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter("docguardian=debug")
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            tracing::info!("DocGuardian starting up...");

            // Initialize database
            #[cfg(debug_assertions)]
            let app_data_dir = {
                let current_dir = std::env::current_dir().unwrap();
                // If running from src-tauri (cargo run), place db in project root to avoid watch loop
                if current_dir.ends_with("src-tauri") {
                    current_dir.parent().unwrap().join("local_data")
                } else {
                    current_dir.join("local_data")
                }
            };
            #[cfg(not(debug_assertions))]
            let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
            
            std::fs::create_dir_all(&app_data_dir).ok();
            let db_path = app_data_dir.join("docguardian.db");
            let db = services::db::Database::new(&db_path).expect("failed to init database");
            app.manage(std::sync::Arc::new(db));

            // Initialize conversation state for multi-turn LLM sessions
            app.manage(commands::llm::ConversationState(std::sync::Mutex::new(Vec::new())));

            // Initialize LLM cancellation flag
            app.manage(commands::llm::CancellationFlag(
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            ));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::project::add_project,
            commands::project::remove_project,
            commands::project::list_projects,
            commands::project::get_project_health,
            commands::init::check_governance,
            commands::init::scan_project,
            commands::init::update_init_rules,
            commands::init::confirm_init,
            commands::init::read_governance_file,
            commands::gc::run_memory_gc,
            commands::gc::get_gc_status,
            commands::conflict::scan_conflicts,
            commands::conflict::resolve_conflict,
            commands::rule::extract_rules,
            commands::rule::accept_rule,
            commands::prune::scan_redundancy,
            commands::llm::save_llm_config,
            commands::llm::get_llm_config,
            commands::llm::test_llm_connection,
            commands::llm::cancel_llm_generation,
            commands::llm::generate_agents_md_llm,
            commands::llm::refine_agents_md,
            commands::llm::check_governance_freshness,
            commands::llm::suggest_governance_updates,
            commands::llm::apply_governance_updates,
            commands::compliance::run_compliance_check,
            commands::compliance::list_violations,
            commands::compliance::update_violation_status,
            commands::compliance::setup_git_hooks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DocGuardian");
}
