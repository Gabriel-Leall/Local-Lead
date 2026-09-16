mod commands;
mod database;
mod dedupe;
mod discovery;
mod domain;
mod enrichment;
mod error;
mod providers;
mod services;

use database::{db_path, init_db, mark_interrupted, DbState};
use rusqlite::Connection;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let path = db_path(app.handle()).expect("db path");
            let conn = Connection::open(&path).expect("open db");
            init_db(&conn).expect("migrate db");
            mark_interrupted(&conn);
            app.manage(DbState(Mutex::new(conn)));
            app.manage(crate::services::scraper_runner::ScraperJobs::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::search::search_leads,
            commands::search::search_osm_cmd,
            commands::search::test_places_key,
            commands::search::autocomplete_city_cmd,
            commands::leads::get_leads,
            commands::leads::update_lead_status,
            commands::dashboard::get_dashboard_stats,
            commands::jobs::start_adaptive_search_cmd,
            commands::jobs::get_search_jobs,
            commands::jobs::get_search_cells,
            commands::jobs::pause_search_job,
            commands::jobs::resume_search_job,
            commands::jobs::cancel_search_job,
            commands::enrichment::enrich_leads_cmd,
            commands::enrichment::enrich_websites_cmd,
            commands::enrichment::enrich_ddg_cmd,
            commands::enrichment::get_leads_filtered,
            commands::qualification::get_score_config,
            commands::qualification::update_score_config,
            commands::qualification::rescore_leads,
            commands::qualification::get_qualification_queue,
            commands::qualification::bulk_set_status,
            commands::qualification::list_filters,
            commands::qualification::save_named_filter,
            commands::qualification::delete_filter,
            commands::messages::list_templates,
            commands::messages::preview_message,
            commands::messages::generate_message,
            commands::messages::get_lead_detail,
            commands::messages::get_lead_messages,
            commands::messages::set_outreach_status,
            commands::scraper::check_scraper_binary,
            commands::scraper::import_scraper_json,
            commands::scraper::start_scraper_search_cmd,
            commands::scraper::poll_scraper_job_cmd,
            commands::scraper::cancel_scraper_search_cmd,
            commands::scraper::get_provider_counts,
            commands::crm::get_activity,
            commands::crm::add_note,
            commands::crm::set_pipeline_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
