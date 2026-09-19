# AI Usage

## Tools Used

- **Claude (Anthropic)** — Used to scaffold the initial project structure, suggest crate choices, and generate boilerplate for handlers and models.

## What AI Generated

- Initial `Cargo.toml` dependency list and crate selections
- Skeleton handler signatures and routing setup in `main.rs`
- SQL migration DDL
- `models.rs` struct layouts based on the assignment spec

## What Was Manually Written / Reviewed / Changed

- All business logic inside handlers (`auth.rs`, `tasks.rs`) — particularly the 2FA flow, code hashing, challenge lifecycle (used/expired checks), and cache invalidation trigger
- `cache.rs` — DashMap wrapper design and the decision to use per-user string keys rather than composite keys
- `middleware/auth.rs` — The `FromRequest` impl and `require_admin` guard
- `services/two_fa.rs` — Decision to use SHA-256 (not bcrypt) for OTP hashing since OTPs are short-lived and we need fast comparison, not slow hashing
- `services/email.rs` — Console + DB dual logging approach
- Integration test structure and which scenarios to cover
- README validation workflow

## Design Decisions Made Independently

- **SQLite over PostgreSQL** for local-first developer experience (no Postgres installation required)
- **DashMap cache** documented explicitly as an in-memory substitute for Redis, with the migration path noted in README and cache.rs
- **lib.rs + bin split** to allow integration tests to import internal modules without duplicating the app setup
- **SHA-256 for OTP hash** — Argon2 is correct for passwords but unnecessarily slow for short-lived OTPs; SHA-256 is appropriate here
- **Idempotent seed endpoint** — Uses `INSERT ... WHERE NOT EXISTS` logic so it can be called multiple times safely
- **Errors fixed for taks models dependent on each other