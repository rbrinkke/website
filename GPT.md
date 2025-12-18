# GPT.md

This file provides guidance for GPT/Codex when working with code in this repository.

## SQLx + Flutter/Drift Parity (Non-Negotiable)
- Use **SQLx** in Rust for all DB access.
- Write **all SQL by hand** (no query builders / ORMs). Keep SQL strings copy/pasteable.
- Put SQL in `const` strings in `src/database/*_repo.rs` so we can later copy the exact SQL into Flutter Drift.
- For every table you touch, add a minimal `sqlx::FromRow` struct in `src/models/` (this becomes the Dart blueprint).
- Always bind parameters (`?1`, `?2`, … / `.bind(...)`)—never interpolate user input into SQL strings.

## Transactional Trigger + Sync (Write Path)
SQLite here is a **read snapshot**; the source of truth is the **central DB** (Postgres). We do not mutate snapshot tables directly.

**Canonical flow (single transaction, no queue)**
1. UI action → handler → `services/` → `database/`: write a *command row* into a write table (e.g. `activity_signup_commands` or `activity_waitlist_commands`).
2. SQLite trigger calls a Rust-registered SQLite function (UDF) that applies the change to the central DB/service (e.g. `sp_apply_activity_signup_command(id)`).
3. If the UDF fails, the trigger raises `ROLLBACK` and the insert fails immediately.
4. Sync pulls derived truth back into local SQLite snapshot tables.

**Rule:** snapshot tables (`activities`, `users`, `activity_participants`, …) are read-only from app code.
