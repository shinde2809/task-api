# Task Management API

Rust backend built with **Actix-Web**, **SQLx (SQLite)**, **JWT**, **Argon2**, and an **in-memory DashMap cache**.

---

## Stack

| Layer | Choice | Notes |
|---|---|---|
| Web framework | Actix-Web 4 | |
| Database | SQLite via SQLx | No separate DB server needed locally |
| ORM/query | SQLx (raw queries) | Compile-time checked with `query_as!` |
| Auth | JWT (jsonwebtoken) | HS256 |
| Password hashing | Argon2 | Industry standard |
| 2FA | SHA-256 hashed OTP | Stored hash, not plaintext |
| Cache | DashMap (in-memory) | See cache note below |
| Migrations | SQLx built-in | Auto-applied on startup |

> **Cache note:** Using in-memory `DashMap` instead of Redis. It satisfies all cache semantics (hit/miss, per-user, invalidation on assignment). Does not survive server restarts or scale across instances. To switch to Redis, replace `cache.rs` with a `redis`/`fred` crate implementation — the handler logic is unchanged.

---

## Prerequisites

```bash
# Rust stable (2021 edition or later)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# SQLx CLI (for manual migrations if needed)
cargo install sqlx-cli --no-default-features --features sqlite
```

---

## Setup & Run

```bash
# 1. Clone / unzip the project
cd task-api

# 2. Copy env (already present, no edits needed for local dev)
cp .env.example .env

# 3. Run (migrations apply automatically on first start)
cargo run
```

Server starts at **http://127.0.0.1:8080**

---

## Validation Workflow (curl)

Follow these steps exactly to complete the full assignment flow.

### Step 1 — Create users

```bash
curl -s -X POST http://localhost:8080/seed/users | jq
```

Creates `admin@example.com` (role: admin) and `jamesbond@example.com` (role: staff).

---

### Step 2 — Start Admin login (triggers 2FA)

```bash
curl -s -X POST http://localhost:8080/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@example.com","password":"Admin1234!"}' | jq
```

Returns a `login_challenge_id`. The OTP prints to the **console** and is stored in the DB.

---

### Step 3 — Get the Admin OTP

From the **terminal running cargo run** — look for:

```
┌─────────────────────────────────────────┐
│         📧  VERIFICATION EMAIL           │
│  Code:    123456                         │
└─────────────────────────────────────────┘
```

Or via API:

```bash
curl -s http://localhost:8080/dev/email-logs/latest | jq
```

---

### Step 4 — Verify Admin 2FA → get JWT

```bash
curl -s -X POST http://localhost:8080/auth/verify-2fa \
  -H "Content-Type: application/json" \
  -d '{
    "login_challenge_id": "<CHALLENGE_ID_FROM_STEP_2>",
    "code": "<CODE_FROM_STEP_3>"
  }' | jq
```

Returns `{ "access_token": "eyJ...", "token_type": "Bearer" }`.

Save the token:
```bash
ADMIN_TOKEN="eyJ..."
```

---

### Step 5 — Create 5 tasks as Admin

```bash
for priority in high high medium medium low; do
  curl -s -X POST http://localhost:8080/tasks \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"title\":\"Task $priority\",\"priority\":\"$priority\"}" | jq .id
done
```

Note the 5 task IDs returned.

---

### Step 6 — Assign 3 tasks to James Bond

```bash
curl -s -X POST http://localhost:8080/tasks/assign \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "task_ids": ["<ID1>","<ID2>","<ID3>"],
    "user_email": "jamesbond@example.com"
  }' | jq
```

This also **invalidates** James Bond's task cache.

---

### Step 7 — James Bond login + 2FA

```bash
# Start login
curl -s -X POST http://localhost:8080/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"jamesbond@example.com","password":"Bond007!"}' | jq

# Get code
curl -s http://localhost:8080/dev/email-logs/latest | jq

# Verify 2FA
curl -s -X POST http://localhost:8080/auth/verify-2fa \
  -H "Content-Type: application/json" \
  -d '{
    "login_challenge_id": "<CHALLENGE_ID>",
    "code": "<CODE>"
  }' | jq
```

Save the token:
```bash
BOND_TOKEN="eyJ..."
```

---

### Step 8 — James Bond tries to create a task → 403

```bash
curl -s -X POST http://localhost:8080/tasks \
  -H "Authorization: Bearer $BOND_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"title":"Bond task"}' | jq
```

Expected: `403 Forbidden`

---

### Step 9 — First call: view-my-tasks (cache.hit = false)

```bash
curl -s http://localhost:8080/tasks/view-my-tasks \
  -H "Authorization: Bearer $BOND_TOKEN" | jq
```

---

### Step 10 — Second call: view-my-tasks (cache.hit = true)

```bash
curl -s http://localhost:8080/tasks/view-my-tasks \
  -H "Authorization: Bearer $BOND_TOKEN" | jq
```

---

## Final Validation Response

**First call** (cache.hit = false):

```json
{
  "user": {
    "email": "jamesbond@example.com",
    "role": "staff"
  },
  "tasks": [
    {
      "id": "...",
      "title": "Task high",
      "status": "todo",
      "priority": "high",
      "assigned_to": "jamesbond@example.com"
    },
    {
      "id": "...",
      "title": "Task high",
      "status": "todo",
      "priority": "high",
      "assigned_to": "jamesbond@example.com"
    },
    {
      "id": "...",
      "title": "Task medium",
      "status": "todo",
      "priority": "medium",
      "assigned_to": "jamesbond@example.com"
    }
  ],
  "summary": {
    "total_assigned_tasks": 3
  },
  "cache": {
    "hit": false
  }
}
```

**Second call** — identical except:
```json
"cache": { "hit": true }
```

---

## Tests

```bash
cargo test
```

Tests cover:
- Seed creates 2 users
- Login returns challenge ID, not a JWT
- Wrong password → 401
- Staff role blocked from admin actions (require_admin guard)
- 2FA code generation, hashing, verification
- Cache hit / miss / invalidation logic

---

## Credentials Reference

| User | Email | Password | Role |
|---|---|---|---|
| Admin | admin@example.com | Admin1234! | admin |
| James Bond | jamesbond@example.com | Bond007! | staff |

---

## Project Structure

```
src/
├── main.rs              # Entry point, routing
├── lib.rs               # Re-exports for test access
├── config.rs            # Env-based config
├── db.rs                # AppState (pool + cache)
├── errors.rs            # AppError → HTTP responses
├── models.rs            # All structs, DTOs, DB models
├── cache.rs             # DashMap in-memory cache
├── handlers/
│   ├── auth.rs          # POST /auth/login, /auth/verify-2fa
│   ├── tasks.rs         # POST /tasks, /tasks/assign, GET /tasks/view-my-tasks
│   ├── seed.rs          # POST /seed/users
│   └── dev.rs           # GET /dev/email-logs/latest
├── middleware/
│   └── auth.rs          # AuthUser extractor + require_admin
└── services/
    ├── jwt.rs           # create_jwt / verify_jwt
    ├── password.rs      # Argon2 hash/verify
    ├── two_fa.rs        # OTP generate/hash/verify
    └── email.rs         # Console + DB email logging
migrations/
├── 0001_create_users.sql
├── 0002_create_tasks.sql
└── 0003_create_auth_tables.sql
tests/
└── integration_test.rs
```
