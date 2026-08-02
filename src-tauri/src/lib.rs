mod commands;

use commands::{
    create_board_cmd, create_column_cmd, create_habit_cmd, create_task_cmd, delete_board_cmd,
    delete_column_cmd, delete_habit_cmd, init_db, list_boards_cmd, list_columns_cmd,
    list_habit_logs_cmd, list_habits_cmd, list_tasks_cmd, log_habit_cmd, move_task_cmd,
    rename_board_cmd, rename_column_cmd, reorder_board_cmd, reorder_column_cmd, unlog_habit_cmd,
    update_habit_cmd, DbState,
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
                init_db(handle, db_state)
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
            create_habit_cmd,
            list_habits_cmd,
            update_habit_cmd,
            delete_habit_cmd,
            log_habit_cmd,
            unlog_habit_cmd,
            list_habit_logs_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
