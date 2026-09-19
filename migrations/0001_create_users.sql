CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY NOT NULL,
    full_name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    hashed_password TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('admin', 'staff')) DEFAULT 'staff',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
