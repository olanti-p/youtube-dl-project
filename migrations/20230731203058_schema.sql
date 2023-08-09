CREATE TABLE jobs
(
    job_id      BLOB NOT NULL PRIMARY KEY,
    thumbnail   TEXT NOT NULL,
    url         TEXT NOT NULL,
    format      TEXT NOT NULL,
    created_at  DATETIME NOT NULL,
    finished_at DATETIME,
    prioritized BOOLEAN NOT NULL DEFAULT false,
    title       TEXT NOT NULL
);

CREATE TABLE tasks
(
    task_id      BLOB NOT NULL PRIMARY KEY,
    status       TEXT NOT NULL,
    thumbnail    TEXT NOT NULL,
    owner_job_id BLOB REFERENCES jobs (job_id) ON DELETE CASCADE,
    url          TEXT NOT NULL,
    format       TEXT NOT NULL,
    created_at   DATETIME NOT NULL,
    finished_at  DATETIME,
    prioritized  BOOLEAN DEFAULT false NOT NULL,
    task_index   INTEGER NOT NULL,
    title        TEXT NOT NULL,
    is_resumed   BOOLEAN DEFAULT false NOT NULL,
    num_retries  INTEGER DEFAULT 0 NOT NULL,
    kind         TEXT NOT NULL
);

CREATE TABLE sessions
(
    session_token BLOB PRIMARY KEY,
    user_id       BLOB REFERENCES users (user_id) ON DELETE CASCADE
);

CREATE TABLE users
(
    user_id   BLOB NOT NULL PRIMARY KEY,
    name      TEXT NOT NULL UNIQUE,
    api_token TEXT NOT NULL UNIQUE
);
