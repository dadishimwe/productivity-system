CREATE UNIQUE INDEX idx_habit_logs_habit_date ON habit_logs(habit_id, date);

CREATE INDEX idx_columns_board_position ON columns(board_id, position);
CREATE INDEX idx_tasks_column_position ON tasks(column_id, position);
