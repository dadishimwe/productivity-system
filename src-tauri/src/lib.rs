mod commands;
mod google_oauth;

use commands::{
    connect_google_oauth_cmd, create_board_cmd, create_calendar_cmd, create_column_cmd,
    create_event_cmd, create_habit_cmd, create_shopping_item_cmd, create_shopping_list_cmd,
    create_task_cmd, delete_board_cmd, delete_calendar_cmd, delete_column_cmd, delete_event_cmd,
    delete_habit_cmd, delete_occurrence_cmd, delete_shopping_item_cmd, delete_shopping_list_cmd,
    delete_task_cmd, disconnect_google_account_cmd, ensure_default_calendar_cmd,
    get_shopping_list_summary_cmd, init_db, list_boards_cmd, list_calendars_cmd, list_columns_cmd,
    list_events_cmd, list_google_accounts_cmd, list_habit_logs_cmd, list_habits_cmd,
    list_occurrences_cmd, list_shopping_items_cmd, list_shopping_lists_cmd, list_tasks_cmd,
    log_habit_cmd, move_occurrence_cmd, move_task_cmd, rename_board_cmd, rename_calendar_cmd,
    rename_column_cmd, rename_shopping_list_cmd, reorder_board_cmd, reorder_column_cmd,
    set_shopping_budget_cmd, toggle_shopping_item_cmd, unlog_habit_cmd, update_event_cmd,
    update_habit_cmd, update_shopping_item_cmd, update_task_cmd, DbState,
};
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(DbState(Mutex::new(None)))
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                let db_state = handle.state::<DbState>();
                init_db(&handle, db_state)
                    .await
                    .expect("failed to init database");
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_board_cmd,
            list_boards_cmd,
            rename_board_cmd,
            reorder_board_cmd,
            delete_board_cmd,
            create_column_cmd,
            list_columns_cmd,
            rename_column_cmd,
            reorder_column_cmd,
            delete_column_cmd,
            create_task_cmd,
            list_tasks_cmd,
            move_task_cmd,
            update_task_cmd,
            delete_task_cmd,
            create_habit_cmd,
            list_habits_cmd,
            update_habit_cmd,
            delete_habit_cmd,
            log_habit_cmd,
            unlog_habit_cmd,
            list_habit_logs_cmd,
            create_shopping_list_cmd,
            list_shopping_lists_cmd,
            rename_shopping_list_cmd,
            set_shopping_budget_cmd,
            delete_shopping_list_cmd,
            create_shopping_item_cmd,
            list_shopping_items_cmd,
            update_shopping_item_cmd,
            toggle_shopping_item_cmd,
            delete_shopping_item_cmd,
            get_shopping_list_summary_cmd,
            ensure_default_calendar_cmd,
            list_calendars_cmd,
            create_calendar_cmd,
            rename_calendar_cmd,
            delete_calendar_cmd,
            list_events_cmd,
            list_occurrences_cmd,
            create_event_cmd,
            update_event_cmd,
            delete_event_cmd,
            move_occurrence_cmd,
            delete_occurrence_cmd,
            list_google_accounts_cmd,
            connect_google_oauth_cmd,
            disconnect_google_account_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
