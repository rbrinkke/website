# GEMINI.md - GoAmet Website Context

## Project Overview
**GoAmet Website** is a server-side rendered web application built with **Rust**. Its primary current objective is to serve as a high-fidelity, pixel-perfect web replica of the GoAmet mobile app's "Discovery" UI. It connects to a local SQLite database and proxies image requests to a separate Image API service.

## Tech Stack
*   **Language:** Rust (2021 Edition)
*   **Web Framework:** [Axum 0.7](https://github.com/tokio-rs/axum)
*   **Templating:** [Askama](https://github.com/djc/askama) (Type-safe, Jinja-like syntax)
*   **Database:** [SQLx 0.7](https://github.com/launchbadge/sqlx) (Async, Type-safe SQL) with **SQLite**.
*   **Runtime:** Tokio
*   **Logging:** Tracing / Tracing-Subscriber
*   **Environment:** `dotenvy` for `.env` management.

## Project Structure

```text
/opt/goamet/website/
├── src/
│   ├── main.rs                 # Application entry point (Server setup, DB connect, Routes)
│   ├── models/                 # `sqlx::FromRow` structs (Dart/Drift blueprint)
│   ├── database/               # Repository layer (raw SQL consts + fetch/exec)
│   ├── services/               # Orchestration layer (transaction-style flows)
│   └── web/
│       ├── middleware/         # Custom middleware (e.g., Auth)
│       └── routes/             # Request handlers (Controllers)
│           ├── auth.rs         # Login/Logout logic
│           ├── discovery.rs    # Main "Discovery" UI handler
│           └── images.rs       # Image proxying to Image API
├── templates/                  # HTML templates (Askama)
│   ├── layout.html             # Base layout
│   ├── discovery.html          # Discovery view
│   └── user.html               # User profile view
│   └── login.html              # Login view
├── assets/                     # Static files (CSS, Images, JS)
├── migrations/                 # SQLx database migrations
├── goamet.db                   # Local SQLite database file
├── discovery_plan.json         # UI Design specs for the Discovery view
└── Cargo.toml                  # Project dependencies
```

## Key Features & Goals

1.  **Mobile Discovery Replica:**
    *   The `/discovery` route (handled by `src/web/routes/dashboard.rs`) implements a "pixel-perfect" grid/masonry layout mimicking the mobile app.
    *   UI Specs are defined in `discovery_plan.json`.
    *   Target Viewport: Mobile (max-width 480px, centered).

2.  **Authentication:**
    *   Simple session/token-based auth (Middleware in `src/web/middleware/auth.rs`).
    *   Login available at `/login`.
    *   Protected routes include `/discovery` and `/logout`.

3.  **Image Proxying:**
    *   The application proxies image requests via `/images/:image_id`.
    *   It interacts with an external **Image Processor Service** (documented in `image-api-readme.json`).

## Building and Running

### Prerequisites
*   Rust & Cargo installed.
*   `.env` file configured (must contain `DATABASE_URL`).

### Commands
```bash
# Run the application (Defaults to port 3000)
cargo run

# Build for release
cargo build --release

# Check code without building
cargo check

# ⚡ Optimized Build (Recommended)
# Uses sccache and mold/lld if available for faster incremental builds
./scripts/cargo-fast.sh run
./scripts/cargo-fast.sh build
```

The server will start at `http://localhost:3000`.

## Database
*   **Type:** SQLite
*   **Connection:** Managed via `sqlx::SqlitePool`.
*   **File:** `goamet.db` (in project root). This local SQLite file is the only data source—there is no external database or remote service.
*   **Migrations:** Managed via `sqlx cli` (files in `migrations/`).

## Data Access Conventions (SQLx + Flutter/Drift Parity)
*   Use **SQLx in Rust** for all DB access.
*   Write **all SQL by hand** (no query builders / ORMs).
*   For every table you touch, add a small `struct` that implements `sqlx::FromRow`.
*   Keep SQL strings easy to copy into Flutter later (Drift `.drift` files or `customSelect()`).
*   Always bind parameters (`?1`, `?2`, … / `.bind(...)`)—never interpolate user input into SQL strings.

## Transactional Trigger + Sync (Write Path)
SQLite is a **read snapshot** of the central DB (Postgres). All writes are synchronous and transactional:

1. UI action → handler → `services/` → `database/`: insert a command row into a write table (e.g. `activity_signup_commands` or `activity_waitlist_commands`).
2. SQLite trigger calls a Rust-registered SQLite function (UDF) that applies the change to the central DB/service (e.g. `sp_apply_activity_signup_command(id)`).
3. If the UDF fails, the trigger raises `ROLLBACK` and the app gets an immediate error.
4. Sync pulls the derived truth back into SQLite snapshot tables.

**Rule:** never write domain changes directly to snapshot tables (`activities`, `users`, `activity_participants`, …).

## Development Conventions
*   **Templates:** All HTML resides in `templates/`. Askama structs are defined close to the handlers or in a dedicated models module.
*   **Static Assets:** Served from `assets/` at the `/assets` path.
*   **Styling:** CSS files are in `assets/css/`. `mobile.css` contains the styles for the discovery view.
