ALTER TABLE tasks
    ADD COLUMN started_at   DATETIME;
ALTER TABLE tasks
    ADD COLUMN last_retry   DATETIME;
