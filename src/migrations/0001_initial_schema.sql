PRAGMA foreign_keys = ON;

CREATE TABLE projects (
    project_id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE directory_links (
    path TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects (project_id) ON DELETE CASCADE
);

CREATE TABLE environments (
    env_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    slug TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects (project_id) ON DELETE CASCADE,
    UNIQUE (project_id, slug)
);

CREATE TABLE secrets (
    secret_id TEXT PRIMARY KEY,
    env_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    FOREIGN KEY (env_id) REFERENCES environments (env_id) ON DELETE CASCADE,
    UNIQUE (env_id, key)
);

CREATE TABLE active_states (
    project_id TEXT PRIMARY KEY,
    active_slug TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects (project_id) ON DELETE CASCADE
);
