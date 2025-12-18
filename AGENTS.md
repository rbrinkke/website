# Repository Guidelines

## Project Structure & Module Organization
Source lives in `src/`:
- `main.rs` wires the Axum app and module roots.
- `models/` contains `sqlx::FromRow` structs (table + view models; future Dart blueprints).
- `database/` contains repository functions (raw SQL `const` strings + small fetch/exec helpers).
- `services/` contains orchestration/business logic (transaction-style “stored procedure” flows).
- `web/` contains HTTP middleware + route handlers (thin; delegates to `services/`).

Templated pages are in `templates/` (Askama), and static assets (CSS, JS, images) are under `assets/`.
SQLite artifacts (`goamet.db*`) sit at the repo root; keep migrations in `migrations/` when added.
Build artifacts land in `target/`; logs such as `server.log` stay in the root.
Environment variables are loaded from `.env` (see `DATABASE_URL` usage in `main.rs`).

## Build, Test, and Development Commands
Use Rust 2021 with Cargo:
```
DATABASE_URL=sqlite://goamet.db cargo run        # start dev server on :3000
DATABASE_URL=sqlite://goamet.db cargo test       # run unit/integration tests
cargo fmt                                        # format code with rustfmt
cargo clippy -- -D warnings                      # lint; keep output clean
```
If you add migrations, prefer `cargo sqlx migrate run` and commit the new files under `migrations/`.

## Coding Style & Naming Conventions
Follow idiomatic Rust: snake_case for files/modules, UpperCamelCase for types, and keep functions small with explicit returns for web handlers. Match existing tracing/logging tone (human-readable, emoji is acceptable) but avoid logging secrets. Run `cargo fmt` before committing. Favor `Result`-returning functions in web layers so errors surface clearly. Template names match the file names in `templates/` and are referenced via `#[template(path = "...")]`.

## Testing Guidelines
Add `#[cfg(test)] mod tests` blocks beside the code they cover; for route handlers, prefer request/response tests using `axum::Router` with a test pool. Name tests `<function>_behaves_as_expected` or similar, and ensure new logic comes with coverage. Use `cargo test -- --nocapture` while debugging to see stdout. If you introduce database queries, add fixtures or in-memory SQLite setup per test for isolation.

## Commit & Pull Request Guidelines
Use present-tense, concise commit messages (e.g., `Add dashboard auth check`, `Fix image proxy headers`). In PRs, include: a short summary, key screenshots of UI changes (login/dashboard), linked issue IDs, and notes on testing performed (`cargo test`, manual login flow). Call out any schema or env var changes (`DATABASE_URL`, auth/image service hosts) so deployers can adjust configs.

## Security & Configuration Tips
Keep `DATABASE_URL`, auth-service, and image-service endpoints in `.env`; never hardcode secrets or tokens. Cookies are HTTP-only and SameSite=Lax—preserve those flags when changing auth flows. If you add new external calls, validate status codes and propagate `StatusCode` responses rather than panicking to avoid leaking internals.

## Data Access Conventions (SQLx + Flutter/Drift Parity)
- Use **SQLx in Rust** for all database access.
- Write **all SQL by hand** (no query builders / ORMs). Keep SQL strings copy/pasteable.
- For every table you touch, add a small `struct` that implements `sqlx::FromRow` (this becomes the blueprint for the later Dart/Drift model).
- Prefer keeping SQL in `const` strings near the handler/repository code so we can later copy the exact SQL into Drift `.drift` files or `customSelect` calls in Flutter.
- Use bound parameters (`?1`, `?2`, … or `.bind(...)`) and never interpolate user input into SQL strings.

## Transactional Trigger + Sync (Write Path)
SQLite in this repo is a **read snapshot**; the source of truth is the **central DB** (Postgres). We do not mutate snapshot tables directly.

**Canonical flow (single transaction, no queue)**
1. UI action → web handler → `services/` → `database/`: `INSERT` a *command row* into a write table (e.g. `activity_participation_commands`).
2. A SQLite trigger fires immediately and calls a Rust-registered SQLite function (UDF) that applies the change to the central DB/service.
3. If the UDF returns an error/false, the trigger raises `ROLLBACK` so the insert fails and the app gets a synchronous error.
4. On success, a sync pulls the derived truth back into the local SQLite snapshot tables.

**Rules**
- Snapshot/read tables (`activities`, `users`, `activity_participants`, …) are read-only from app code.
- Write tables store only what the central apply needs (ids + minimal payload) plus an idempotency key (`client_request_id`).
- Keep all write/trigger SQL hand-written as `const` strings in `src/database/*_repo.rs` (Flutter/Drift copy-paste parity).
