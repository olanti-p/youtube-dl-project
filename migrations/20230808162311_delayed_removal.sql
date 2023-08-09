ALTER TABLE tasks
    ADD COLUMN pending_delete   BOOLEAN NOT NULL DEFAULT false;
