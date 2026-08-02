use productivity_core::{
    boards, columns, habit_logs, habits, create_task, init_pool, list_tasks, move_task, AppState,
};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

pub struct DbState(pub Mutex<Option<AppState>>);

fn map_err(e: productivity_core::error::CoreError) -> tauri::Error {
    tauri::Error::Io(std::io::Error::other(e.to_string()))
}

fn app_state(db_state: &State<'_, DbState>) -> tauri::Result<AppState> {
    let guard = db_state
        .0
        .lock()
        .map_err(|_| tauri::Error::Io(std::io::Error::other("database lock poisoned")))?;
    guard
        .clone()
        .ok_or_else(|| tauri::Error::Io(std::io::Error::other("database not initialized")))
}

fn db_path(app: &AppHandle) -> tauri::Result<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| tauri::Error::Io(std::io::Error::other(e.to_string())))?;
    Ok(dir.join("productivity.db"))
}

pub async fn init_db(app: AppHandle, db_state: State<'_, DbState>) -> tauri::Result<()> {
    let path = db_path(&app)?;
    let state = init_pool(&path).await.map_err(map_err)?;
    *db_state.0.lock().unwrap() = Some(state);
    Ok(())
}

#[derive(Serialize)]
pub struct BoardDto {
    pub id: String,
    pub name: String,
    pub position: f64,
}

#[derive(Serialize)]
pub struct ColumnDto {
    pub id: String,
    pub board_id: String,
    pub name: String,
    pub position: f64,
}

#[derive(Serialize)]
pub struct TaskDto {
    pub id: String,
    pub column_id: String,
    pub title: String,
    pub position: f64,
    pub status: String,
}

#[derive(Serialize)]
pub struct HabitDto {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub target_frequency: Option<String>,
}

#[derive(Serialize)]
pub struct HabitLogDto {
    pub id: String,
    pub habit_id: String,
    pub date: String,
    pub value: i64,
}

impl From<productivity_core::boards::Board> for BoardDto {
    fn from(b: productivity_core::boards::Board) -> Self {
        Self {
            id: b.id,
            name: b.name,
            position: b.position,
        }
    }
}

impl From<productivity_core::columns::Column> for ColumnDto {
    fn from(c: productivity_core::columns::Column) -> Self {
        Self {
            id: c.id,
            board_id: c.board_id,
            name: c.name,
            position: c.position,
        }
    }
}

impl From<productivity_core::Task> for TaskDto {
    fn from(t: productivity_core::Task) -> Self {
        Self {
            id: t.id,
            column_id: t.column_id,
            title: t.title,
            position: t.position,
            status: t.status,
        }
    }
}

impl From<productivity_core::habits::Habit> for HabitDto {
    fn from(h: productivity_core::habits::Habit) -> Self {
        Self {
            id: h.id,
            name: h.name,
            color: h.color,
            target_frequency: h.target_frequency,
        }
    }
}

impl From<productivity_core::habit_logs::HabitLog> for HabitLogDto {
    fn from(l: productivity_core::habit_logs::HabitLog) -> Self {
        Self {
            id: l.id,
            habit_id: l.habit_id,
            date: l.date,
            value: l.value,
        }
    }
}

#[tauri::command]
pub async fn create_board_cmd(db_state: State<'_, DbState>, name: String) -> tauri::Result<BoardDto> {
    let state = app_state(&db_state)?;
    Ok(boards::create_board(&state, &name).await.map_err(map_err)?.into())
}

#[tauri::command]
pub async fn list_boards_cmd(db_state: State<'_, DbState>) -> tauri::Result<Vec<BoardDto>> {
    let state = app_state(&db_state)?;
    Ok(boards::list_boards(&state)
        .await
        .map_err(map_err)?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
pub async fn rename_board_cmd(
    db_state: State<'_, DbState>,
    id: String,
    name: String,
) -> tauri::Result<BoardDto> {
    let state = app_state(&db_state)?;
    Ok(boards::rename_board(&state, &id, &name)
        .await
        .map_err(map_err)?
        .into())
}

#[tauri::command]
pub async fn reorder_board_cmd(
    db_state: State<'_, DbState>,
    id: String,
    new_position: f64,
) -> tauri::Result<BoardDto> {
    let state = app_state(&db_state)?;
    Ok(boards::reorder_board(&state, &id, new_position)
        .await
        .map_err(map_err)?
        .into())
}

#[tauri::command]
pub async fn delete_board_cmd(db_state: State<'_, DbState>, id: String) -> tauri::Result<()> {
    let state = app_state(&db_state)?;
    boards::delete_board(&state, &id).await.map_err(map_err)
}

#[tauri::command]
pub async fn create_column_cmd(
    db_state: State<'_, DbState>,
    board_id: String,
    name: String,
) -> tauri::Result<ColumnDto> {
    let state = app_state(&db_state)?;
    Ok(columns::create_column(&state, &board_id, &name)
        .await
        .map_err(map_err)?
        .into())
}

#[tauri::command]
pub async fn list_columns_cmd(
    db_state: State<'_, DbState>,
    board_id: String,
) -> tauri::Result<Vec<ColumnDto>> {
    let state = app_state(&db_state)?;
    Ok(columns::list_columns(&state, &board_id)
        .await
        .map_err(map_err)?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
pub async fn rename_column_cmd(
    db_state: State<'_, DbState>,
    id: String,
    name: String,
) -> tauri::Result<ColumnDto> {
    let state = app_state(&db_state)?;
    Ok(columns::rename_column(&state, &id, &name)
        .await
        .map_err(map_err)?
        .into())
}

#[tauri::command]
pub async fn reorder_column_cmd(
    db_state: State<'_, DbState>,
    id: String,
    new_position: f64,
) -> tauri::Result<ColumnDto> {
    let state = app_state(&db_state)?;
    Ok(columns::reorder_column(&state, &id, new_position)
        .await
        .map_err(map_err)?
        .into())
}

#[tauri::command]
pub async fn delete_column_cmd(db_state: State<'_, DbState>, id: String) -> tauri::Result<()> {
    let state = app_state(&db_state)?;
    columns::delete_column(&state, &id).await.map_err(map_err)
}

#[tauri::command]
pub async fn create_task_cmd(
    db_state: State<'_, DbState>,
    column_id: String,
    title: String,
) -> tauri::Result<TaskDto> {
    let state = app_state(&db_state)?;
    Ok(create_task(&state, &column_id, &title)
        .await
        .map_err(map_err)?
        .into())
}

#[tauri::command]
pub async fn list_tasks_cmd(
    db_state: State<'_, DbState>,
    column_id: String,
) -> tauri::Result<Vec<TaskDto>> {
    let state = app_state(&db_state)?;
    Ok(list_tasks(&state, &column_id)
        .await
        .map_err(map_err)?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
pub async fn move_task_cmd(
    db_state: State<'_, DbState>,
    task_id: String,
    new_column_id: String,
    new_position: f64,
) -> tauri::Result<TaskDto> {
    let state = app_state(&db_state)?;
    Ok(move_task(&state, &task_id, &new_column_id, new_position)
        .await
        .map_err(map_err)?
        .into())
}

#[tauri::command]
pub async fn create_habit_cmd(
    db_state: State<'_, DbState>,
    name: String,
    color: Option<String>,
    target_frequency: Option<String>,
) -> tauri::Result<HabitDto> {
    let state = app_state(&db_state)?;
    Ok(habits::create_habit(
        &state,
        &name,
        color.as_deref(),
        target_frequency.as_deref(),
    )
    .await
    .map_err(map_err)?
    .into())
}

#[tauri::command]
pub async fn list_habits_cmd(db_state: State<'_, DbState>) -> tauri::Result<Vec<HabitDto>> {
    let state = app_state(&db_state)?;
    Ok(habits::list_habits(&state)
        .await
        .map_err(map_err)?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
pub async fn update_habit_cmd(
    db_state: State<'_, DbState>,
    id: String,
    name: String,
    color: Option<String>,
    target_frequency: Option<String>,
) -> tauri::Result<HabitDto> {
    let state = app_state(&db_state)?;
    Ok(habits::update_habit(
        &state,
        &id,
        &name,
        color.as_deref(),
        target_frequency.as_deref(),
    )
    .await
    .map_err(map_err)?
    .into())
}

#[tauri::command]
pub async fn delete_habit_cmd(db_state: State<'_, DbState>, id: String) -> tauri::Result<()> {
    let state = app_state(&db_state)?;
    habits::delete_habit(&state, &id).await.map_err(map_err)
}

#[tauri::command]
pub async fn log_habit_cmd(
    db_state: State<'_, DbState>,
    habit_id: String,
    date: String,
    value: i64,
) -> tauri::Result<HabitLogDto> {
    let state = app_state(&db_state)?;
    Ok(habit_logs::log_habit(&state, &habit_id, &date, value)
        .await
        .map_err(map_err)?
        .into())
}

#[tauri::command]
pub async fn unlog_habit_cmd(
    db_state: State<'_, DbState>,
    habit_id: String,
    date: String,
) -> tauri::Result<()> {
    let state = app_state(&db_state)?;
    habit_logs::unlog_habit(&state, &habit_id, &date)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn list_habit_logs_cmd(
    db_state: State<'_, DbState>,
    habit_id: String,
    from_date: String,
    to_date: String,
) -> tauri::Result<Vec<HabitLogDto>> {
    let state = app_state(&db_state)?;
    Ok(habit_logs::list_habit_logs(&state, &habit_id, &from_date, &to_date)
        .await
        .map_err(map_err)?
        .into_iter()
        .map(Into::into)
        .collect())
}
