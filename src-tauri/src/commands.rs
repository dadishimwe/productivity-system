use productivity_core::{
    boards, calendars, columns, events, habit_logs, habits, shopping_items, shopping_lists,
    create_task, delete_task, init_pool, list_tasks, move_task, update_task, AppState,
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

pub async fn init_db(app: &AppHandle, db_state: State<'_, DbState>) -> tauri::Result<()> {
    let path = db_path(app)?;
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
    pub description: Option<String>,
    pub position: f64,
    pub due_date: Option<i64>,
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
            description: t.description,
            position: t.position,
            due_date: t.due_date,
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

#[tauri::command]
pub async fn update_task_cmd(
    db_state: State<'_, DbState>,
    id: String,
    title: String,
    description: Option<String>,
    due_date: Option<i64>,
    status: String,
) -> tauri::Result<TaskDto> {
    let state = app_state(&db_state)?;
    Ok(update_task(
        &state,
        &id,
        &title,
        description.as_deref(),
        due_date,
        &status,
    )
    .await
    .map_err(map_err)?
    .into())
}

#[tauri::command]
pub async fn delete_task_cmd(db_state: State<'_, DbState>, task_id: String) -> tauri::Result<()> {
    let state = app_state(&db_state)?;
    delete_task(&state, &task_id).await.map_err(map_err)
}

#[derive(Serialize)]
pub struct ShoppingListDto {
    pub id: String,
    pub name: String,
    pub budget_limit: Option<i64>,
}

#[derive(Serialize)]
pub struct ShoppingItemDto {
    pub id: String,
    pub list_id: String,
    pub name: String,
    pub qty: f64,
    pub unit: Option<String>,
    pub unit_price: Option<i64>,
    pub checked: bool,
    pub category: Option<String>,
}

#[derive(Serialize)]
pub struct ListSummaryDto {
    pub total_cents: i64,
    pub item_count: i32,
    pub checked_count: i32,
}

impl From<productivity_core::shopping_lists::ShoppingList> for ShoppingListDto {
    fn from(l: productivity_core::shopping_lists::ShoppingList) -> Self {
        Self {
            id: l.id,
            name: l.name,
            budget_limit: l.budget_limit,
        }
    }
}

impl From<productivity_core::shopping_items::ShoppingItem> for ShoppingItemDto {
    fn from(i: productivity_core::shopping_items::ShoppingItem) -> Self {
        Self {
            id: i.id,
            list_id: i.list_id,
            name: i.name,
            qty: i.qty,
            unit: i.unit,
            unit_price: i.unit_price,
            checked: i.checked != 0,
            category: i.category,
        }
    }
}

impl From<productivity_core::shopping_items::ListSummary> for ListSummaryDto {
    fn from(s: productivity_core::shopping_items::ListSummary) -> Self {
        Self {
            total_cents: s.total_cents,
            item_count: s.item_count,
            checked_count: s.checked_count,
        }
    }
}

#[tauri::command]
pub async fn create_shopping_list_cmd(
    db_state: State<'_, DbState>,
    name: String,
    budget_limit: Option<i64>,
) -> tauri::Result<ShoppingListDto> {
    let state = app_state(&db_state)?;
    Ok(shopping_lists::create_list(&state, &name, budget_limit)
        .await
        .map_err(map_err)?
        .into())
}

#[tauri::command]
pub async fn list_shopping_lists_cmd(
    db_state: State<'_, DbState>,
) -> tauri::Result<Vec<ShoppingListDto>> {
    let state = app_state(&db_state)?;
    Ok(shopping_lists::list_lists(&state)
        .await
        .map_err(map_err)?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
pub async fn rename_shopping_list_cmd(
    db_state: State<'_, DbState>,
    id: String,
    name: String,
) -> tauri::Result<ShoppingListDto> {
    let state = app_state(&db_state)?;
    Ok(shopping_lists::rename_list(&state, &id, &name)
        .await
        .map_err(map_err)?
        .into())
}

#[tauri::command]
pub async fn set_shopping_budget_cmd(
    db_state: State<'_, DbState>,
    list_id: String,
    budget_limit: Option<i64>,
) -> tauri::Result<ShoppingListDto> {
    let state = app_state(&db_state)?;
    Ok(shopping_lists::set_budget(&state, &list_id, budget_limit)
        .await
        .map_err(map_err)?
        .into())
}

#[tauri::command]
pub async fn delete_shopping_list_cmd(
    db_state: State<'_, DbState>,
    id: String,
) -> tauri::Result<()> {
    let state = app_state(&db_state)?;
    shopping_lists::delete_list(&state, &id).await.map_err(map_err)
}

#[tauri::command]
pub async fn create_shopping_item_cmd(
    db_state: State<'_, DbState>,
    list_id: String,
    name: String,
    qty: f64,
    unit: Option<String>,
    unit_price: Option<i64>,
    category: Option<String>,
) -> tauri::Result<ShoppingItemDto> {
    let state = app_state(&db_state)?;
    Ok(shopping_items::create_item(
        &state,
        &list_id,
        &name,
        qty,
        unit.as_deref(),
        unit_price,
        category.as_deref(),
    )
    .await
    .map_err(map_err)?
    .into())
}

#[tauri::command]
pub async fn list_shopping_items_cmd(
    db_state: State<'_, DbState>,
    list_id: String,
) -> tauri::Result<Vec<ShoppingItemDto>> {
    let state = app_state(&db_state)?;
    Ok(shopping_items::list_items(&state, &list_id)
        .await
        .map_err(map_err)?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
pub async fn update_shopping_item_cmd(
    db_state: State<'_, DbState>,
    id: String,
    name: String,
    qty: f64,
    unit: Option<String>,
    unit_price: Option<i64>,
    category: Option<String>,
) -> tauri::Result<ShoppingItemDto> {
    let state = app_state(&db_state)?;
    Ok(shopping_items::update_item(
        &state,
        &id,
        &name,
        qty,
        unit.as_deref(),
        unit_price,
        category.as_deref(),
    )
    .await
    .map_err(map_err)?
    .into())
}

#[tauri::command]
pub async fn toggle_shopping_item_cmd(
    db_state: State<'_, DbState>,
    item_id: String,
) -> tauri::Result<ShoppingItemDto> {
    let state = app_state(&db_state)?;
    Ok(shopping_items::toggle_checked(&state, &item_id)
        .await
        .map_err(map_err)?
        .into())
}

#[tauri::command]
pub async fn delete_shopping_item_cmd(
    db_state: State<'_, DbState>,
    item_id: String,
) -> tauri::Result<()> {
    let state = app_state(&db_state)?;
    shopping_items::delete_item(&state, &item_id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn get_shopping_list_summary_cmd(
    db_state: State<'_, DbState>,
    list_id: String,
) -> tauri::Result<ListSummaryDto> {
    let state = app_state(&db_state)?;
    Ok(shopping_items::get_list_summary(&state, &list_id)
        .await
        .map_err(map_err)?
        .into())
}

#[derive(Serialize)]
pub struct CalendarDto {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Serialize)]
pub struct OccurrenceDto {
    pub event_id: String,
    pub calendar_id: String,
    pub title: String,
    pub description: Option<String>,
    pub original_start_ms: i64,
    pub start_ms: i64,
    pub end_ms: i64,
    pub all_day: bool,
    pub recurring: bool,
}

impl From<productivity_core::occurrences::EventOccurrence> for OccurrenceDto {
    fn from(o: productivity_core::occurrences::EventOccurrence) -> Self {
        Self {
            event_id: o.event_id,
            calendar_id: o.calendar_id,
            title: o.title,
            description: o.description,
            original_start_ms: o.original_start_ms,
            start_ms: o.start_ms,
            end_ms: o.end_ms,
            all_day: o.all_day,
            recurring: o.recurring,
        }
    }
}

fn parse_occurrence_scope(scope: &str) -> tauri::Result<productivity_core::occurrences::OccurrenceScope> {
    use productivity_core::occurrences::OccurrenceScope;
    match scope {
        "this" => Ok(OccurrenceScope::This),
        "this_and_following" => Ok(OccurrenceScope::ThisAndFollowing),
        "all" => Ok(OccurrenceScope::All),
        _ => Err(tauri::Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid occurrence scope",
        ))),
    }
}

impl From<productivity_core::calendars::Calendar> for CalendarDto {
    fn from(c: productivity_core::calendars::Calendar) -> Self {
        Self {
            id: c.id,
            name: c.name,
            color: c.color,
        }
    }
}

#[derive(Serialize)]
pub struct EventDto {
    pub id: String,
    pub calendar_id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_ms: i64,
    pub end_ms: i64,
    pub all_day: bool,
    pub rrule: Option<String>,
}

impl From<productivity_core::events::Event> for EventDto {
    fn from(e: productivity_core::events::Event) -> Self {
        Self {
            id: e.id,
            calendar_id: e.calendar_id,
            title: e.title,
            description: e.description,
            start_ms: e.start_ms,
            end_ms: e.end_ms,
            all_day: e.all_day != 0,
            rrule: e.rrule,
        }
    }
}

#[tauri::command]
pub async fn ensure_default_calendar_cmd(
    db_state: State<'_, DbState>,
) -> tauri::Result<CalendarDto> {
    let state = app_state(&db_state)?;
    Ok(calendars::ensure_default_calendar(&state)
        .await
        .map_err(map_err)?
        .into())
}

#[tauri::command]
pub async fn list_calendars_cmd(
    db_state: State<'_, DbState>,
) -> tauri::Result<Vec<CalendarDto>> {
    let state = app_state(&db_state)?;
    Ok(calendars::list_calendars(&state)
        .await
        .map_err(map_err)?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
pub async fn create_calendar_cmd(
    db_state: State<'_, DbState>,
    name: String,
    color: Option<String>,
) -> tauri::Result<CalendarDto> {
    let state = app_state(&db_state)?;
    Ok(calendars::create_calendar(&state, &name, color.as_deref())
        .await
        .map_err(map_err)?
        .into())
}

#[tauri::command]
pub async fn rename_calendar_cmd(
    db_state: State<'_, DbState>,
    id: String,
    name: String,
) -> tauri::Result<CalendarDto> {
    let state = app_state(&db_state)?;
    Ok(calendars::rename_calendar(&state, &id, &name)
        .await
        .map_err(map_err)?
        .into())
}

#[tauri::command]
pub async fn delete_calendar_cmd(
    db_state: State<'_, DbState>,
    id: String,
) -> tauri::Result<()> {
    let state = app_state(&db_state)?;
    calendars::delete_calendar(&state, &id).await.map_err(map_err)
}

#[tauri::command]
pub async fn list_occurrences_cmd(
    db_state: State<'_, DbState>,
    calendar_id: String,
    range_start_ms: i64,
    range_end_ms: i64,
) -> tauri::Result<Vec<OccurrenceDto>> {
    let state = app_state(&db_state)?;
    Ok(events::list_occurrences(&state, &calendar_id, range_start_ms, range_end_ms)
        .await
        .map_err(map_err)?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
pub async fn list_events_cmd(
    db_state: State<'_, DbState>,
    calendar_id: String,
    range_start_ms: i64,
    range_end_ms: i64,
) -> tauri::Result<Vec<EventDto>> {
    let state = app_state(&db_state)?;
    Ok(events::list_events_in_range(&state, &calendar_id, range_start_ms, range_end_ms)
        .await
        .map_err(map_err)?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
pub async fn create_event_cmd(
    db_state: State<'_, DbState>,
    calendar_id: String,
    title: String,
    description: Option<String>,
    start_ms: i64,
    end_ms: i64,
    all_day: bool,
    rrule: Option<String>,
) -> tauri::Result<EventDto> {
    let state = app_state(&db_state)?;
    Ok(events::create_event(
        &state,
        &calendar_id,
        &title,
        description.as_deref(),
        start_ms,
        end_ms,
        all_day,
        rrule.as_deref(),
    )
    .await
    .map_err(map_err)?
    .into())
}

#[tauri::command]
pub async fn update_event_cmd(
    db_state: State<'_, DbState>,
    id: String,
    title: String,
    description: Option<String>,
    start_ms: i64,
    end_ms: i64,
    all_day: bool,
    rrule: Option<String>,
) -> tauri::Result<EventDto> {
    let state = app_state(&db_state)?;
    Ok(events::update_event(
        &state,
        &id,
        &title,
        description.as_deref(),
        start_ms,
        end_ms,
        all_day,
        rrule.as_deref(),
    )
    .await
    .map_err(map_err)?
    .into())
}

#[tauri::command]
pub async fn delete_event_cmd(db_state: State<'_, DbState>, id: String) -> tauri::Result<()> {
    let state = app_state(&db_state)?;
    events::delete_event(&state, &id).await.map_err(map_err)
}

#[tauri::command]
pub async fn move_occurrence_cmd(
    db_state: State<'_, DbState>,
    event_id: String,
    original_start_ms: i64,
    new_start_ms: i64,
    new_end_ms: i64,
    scope: String,
) -> tauri::Result<()> {
    let state = app_state(&db_state)?;
    events::move_occurrence(
        &state,
        &event_id,
        original_start_ms,
        new_start_ms,
        new_end_ms,
        parse_occurrence_scope(&scope)?,
    )
    .await
    .map_err(map_err)
}

#[tauri::command]
pub async fn delete_occurrence_cmd(
    db_state: State<'_, DbState>,
    event_id: String,
    original_start_ms: i64,
    scope: String,
) -> tauri::Result<()> {
    let state = app_state(&db_state)?;
    events::delete_occurrence(
        &state,
        &event_id,
        original_start_ms,
        parse_occurrence_scope(&scope)?,
    )
    .await
    .map_err(map_err)
}
