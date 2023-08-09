ALTER TABLE tasks
    ADD COLUMN pending_cleanup   BOOLEAN NOT NULL DEFAULT false;
