# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**GoAmet Website** is a server-side rendered web application built with Rust that serves as a high-fidelity replica of the GoAmet mobile app's "Discovery" UI. It connects to a local SQLite database (`goamet.db`) and proxies image requests to the Image API service.

### Tech Stack
- **Language:** Rust (2021 Edition)
- **Web Framework:** Axum 0.7 (async, modular)
- **Templating:** Askama (type-safe, Jinja-like syntax)
- **Database:** SQLx 0.7 (async, type-safe SQL) with SQLite
- **Runtime:** Tokio
- **Logging:** Tracing / Tracing-Subscriber

## Project Structure

```
src/
├── main.rs                          # App entry point, router setup, DB pool initialization
├── models/                          # `sqlx::FromRow` structs (Dart blueprint)
├── database/                        # Repository layer (raw SQL consts + fetch/exec)
├── services/                        # Orchestration layer (transaction-style flows)
└── web/
    ├── mod.rs                       # Web module organization
    ├── middleware/
    │   ├── mod.rs
    │   └── auth.rs                  # Cookie-based auth middleware (require_auth layer)
    └── routes/
        ├── mod.rs
        ├── auth.rs                  # Login page & POST, logout handler
        ├── discovery.rs             # GET /discovery (main Discovery view)
        ├── images.rs                # GET /images/:image_id (Image API proxy)
        ├── location.rs              # GET /api/location/search (location search)
        └── user.rs                  # GET /users/:user_id (profile page)

templates/                           # Askama HTML templates
├── layout.html                      # Base page layout with head/body
├── dashboard.html                   # Discovery/masonry grid layout
└── login.html                       # Login form

assets/                              # Static files served at /assets
├── css/
│   └── mobile.css                   # Mobile discovery view styling
├── js/
└── images/

migrations/                          # SQLx database migrations
└── 001_add_lat_lon_indexes.sql     # Database schema changes

.env                                 # Environment variables (DATABASE_URL, IMAGE_API_URL)
```

## Core Architecture

### Request Flow
1. **Router** (`main.rs:44-55`): Defines public routes (`/login`, `/`) and protected routes under auth middleware
2. **Middleware** (`web/middleware/auth.rs`): Checks for valid auth cookies, rejects requests without them
3. **Handlers** (`web/routes/*.rs`): Process requests, query database, render templates
4. **Database Pool**: Shared SQLite connection pool passed as app state

### Authentication
- Cookie-based (HTTP-only, SameSite=Lax)
- Middleware layer at `web/middleware/auth.rs` implements `require_auth` that wraps protected routes
- Login at `/login` (GET for form, POST for credentials)
- Logout at `/logout` (POST only)

### Protected Routes
- `/discovery` - Discovery view (GET)
- `/images/:image_id` - Image proxy to Image API service (GET)
- `/api/location/search` - Location search endpoint (GET)
- `/logout` - Logout endpoint (POST)

## Common Development Tasks

### Running the Application

```bash
# Development mode (with hot reload via cargo-watch)
cargo run

# Fast build using sccache and mold/lld
./scripts/cargo-fast.sh run

# Release build
cargo build --release
```

**Default server location**: `http://localhost:3000`
**Login page**: `http://localhost:3000/login`

### Database Operations

```bash
# Run new migrations
DATABASE_URL=sqlite:///opt/goamet/website/goamet.db cargo sqlx migrate run

# Add a new migration
cargo sqlx migrate add -r <description>

# Database file location
# /opt/goamet/website/goamet.db (defined in .env as DATABASE_URL)
```

### Data Access Conventions (SQLx + Flutter/Drift Parity)

We intentionally keep DB access in a “portable SQL” style so we can later migrate the exact SQL into Flutter (Drift).

- Use **SQLx in Rust** for all DB access.
- Write **all SQL by hand** (no query builders / ORMs).
- For every table you touch, add a small `struct` that implements `sqlx::FromRow`.
- Prefer putting SQL in `const` strings close to the code using it so it can be copied into Drift `.drift` files or `customSelect()` later.
- Always bind parameters (`?1`, `?2`, … / `.bind(...)`)—never string-interpolate user input into SQL.

### Transactional Trigger + Sync (Write Path)

Local SQLite is a **read snapshot**; the source of truth is the **central DB** (Postgres). Therefore:

- **Never** mutate snapshot tables directly from handlers/services (`activities`, `users`, `activity_participants`, …).
- All mutations are written as *command rows* to dedicated write tables, and processed synchronously via a SQLite trigger.

**Canonical flow (single transaction, no queue)**
1. UI action → handler → `services/` → `database/`: `INSERT` a command row (e.g. `activity_signup_commands` or `activity_waitlist_commands`).
2. SQLite trigger calls a Rust-registered SQLite function (UDF) that applies the change to the central DB/service (e.g. `sp_apply_activity_signup_command(id)`).
3. If the UDF fails, the trigger raises `ROLLBACK`, the insert fails, and the app gets an immediate error.
4. On success, a sync refreshes local snapshot tables from the derived central truth.

**Practical note**
- UI state comes from snapshot columns (`is_joined`, `my_participation_status`, `waitlist_count`, etc.), not from local business rules.

### Code Quality

```bash
# Format code
cargo fmt

# Lint with clippy
cargo clippy -- -D warnings

# Type-check without building
cargo check

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

## Key Implementation Details

### Handler Pattern
All route handlers follow this pattern:
```rust
pub async fn handler_name(
    State(pool): State<SqlitePool>,
    // Optional: Path/Query extractors
) -> Result<impl IntoResponse> {
    // Query database via pool
    // Render template or return response
}
```

The `State` extractor gives access to the shared database pool. Templates use the `#[template(path = "...")]` attribute.

### Template Rendering
Askama templates are type-safe. When rendering a template, pass a struct that matches the template variables:
```rust
DashboardTemplate {
    user_id: 123,
    activities: vec![...],
}
```

Templates are located in `templates/` and referenced by filename (e.g., `dashboard.html` → `path = "dashboard.html"`).

### Image Proxying
The `/images/:image_id` route proxies to the Image API service (configured in `.env` as `IMAGE_API_URL`). It:
1. Receives the image ID from URL parameter
2. Constructs the Image API request
3. Forwards the response (status, headers, body) back to the client

### Database Query Patterns
SQLx provides compile-time checked queries. The pool is accessed via:
```rust
sqlx::query_as::<_, Model>("SELECT * FROM table WHERE id = ?")
    .bind(value)
    .fetch_one(&pool)
    .await
```

## Environment Configuration

**Key variables** (defined in `.env`):
- `DATABASE_URL` - SQLite connection string (required)
- `IMAGE_API_URL` - Image API service base URL (required for image proxying)
- `HOST` - Server bind address (default: `127.0.0.1`)
- `PORT` - Server port (default: `3000`, with fallback to `PORT+1` if unavailable)

## Important Notes

- **SQLite is the only data source** - No external databases or remote services except the Image API for image proxying
- **Static files** are served from the `assets/` directory at the `/assets` path
- **Database migrations** must be committed to `migrations/` directory
- **Logs** like `server.log` are written to the project root during development
- **Fallback port behavior** - If port 3000 is busy, the server automatically tries 3001 (see `main.rs:67-83`)
- **Keep secrets in `.env`** - Never hardcode API keys, auth tokens, or database URLs in code

## Testing Guidelines

Add tests alongside the code they cover:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn handler_behaves_as_expected() {
        // Create test pool
        // Make request to handler
        // Assert response
    }
}
```

For route handlers, use `axum::Router` and test client patterns. For database tests, use in-memory SQLite setup for isolation.

## Coding Conventions

- **File/module names**: `snake_case`
- **Type names**: `UpperCamelCase`
- **Functions/variables**: `snake_case`
- **Format before committing**: `cargo fmt`
- **Keep functions small** and focused on single responsibility
- **Error handling**: Prefer `Result`-returning functions in web layers so errors surface clearly
- **Template names** match the HTML filename and are referenced in handlers

## Database Query Patterns - CRITICAL

### The Golden Rule: **Database-First Filtering**

**NEVER** pre-fetch data into the application and then filter it. This causes:
- Duplicate queries (fetch + filter, then re-query)
- Data blinding (duplicates, self-references hidden until runtime)
- N+1 problems
- Logic scattered between SQL and Rust

### The Right Way: Push ALL Filtering Into SQL

**Pattern 1: Conditional Subqueries**
```rust
// ✅ CORRECT: All filtering stays in database
let mut sql = String::from("SELECT * FROM users WHERE ...");
let mut args = SqliteArguments::default();

// Conditionally add filters - they stay in SQL, never leave DB
if let Some(gender) = query.gender {
    sql.push_str(" AND gender = ?");
    args.add(gender);
}

if query.friends_only {
    // SINGLE inline subquery, NO pre-fetching
    sql.push_str(
        " AND user_id IN (
            SELECT DISTINCT json_extract(friend, '$.user_id')
            FROM friends
            WHERE ... AND friendship_id LIKE ? || ':%' OR ...
        )"
    );
    args.add(&current_user_id);
    args.add(&current_user_id);
}

// Execute once, get clean results
let users = sqlx::query_as_with::<_, User, _>(&sql, args)
    .fetch_all(&pool)
    .await?;
```

**Pattern 2: What NOT To Do**
```rust
// ❌ WRONG: Pre-fetching + in-memory filtering
let friend_ids = fetch_friend_ids(&pool, user_id).await?;  // Query 1
if friend_ids.is_empty() {
    return empty_results();
}
// Later in code...
if friends_only {
    sql.push_str(" AND user_id IN (SELECT ... FROM friends ...)");  // Query 2 again!
}
```

### Why This Matters

**Example: Friends Filter Bug**
- Fetching friends into memory hid the fact that data contained duplicates and self-references
- Dupes weren't visible until runtime—code looked fine but returned nothing
- Two queries meant two chances for different logic
- Debugging took 2 hours because data state was invisible until the second query

**With Database-First**: The subquery either works or doesn't, immediately testable with `sqlite3 goamet.db`

### Checklist for Query Building

Before writing any handler that filters data, ask:

1. **Is filtering in SQL?** ✓ (in WHERE/subquery)
2. **Is it a single execution?** ✓ (one `.fetch_all()` call)
3. **Are parameters bound safely?** ✓ (using `SqliteArguments`, no string concat)
4. **Can I test it directly in SQLite?** ✓ (copy the SQL, run it with `sqlite3 goamet.db`)

If the answer is "no" to any, refactor to push logic into SQL.

### Testing Database Queries

Before writing Rust code, test the query in SQLite:
```bash
sqlite3 goamet.db << 'EOF'
SELECT user_id FROM users
WHERE ... AND user_id IN (
    SELECT DISTINCT json_extract(friend, '$.user_id')
    FROM friends
    WHERE status = 'accepted' AND ...
)
LIMIT 10;
EOF
```

If it returns what you expect, then build it in Rust with `SqliteArguments`. If it doesn't, fix SQL first.

## Discovery Page Architecture

**Pattern:** Fetch-once, filter-many

The Discovery page uses a unique architecture for optimal performance:

- **ONE database query at page load** - Loads up to 500 users within variable radius_km
- **SQL handles**: Bounding box geo-filtering, LEFT JOIN to calculate is_friend boolean, limit 500
- **Rust handles**: Haversine exact distance filtering, distance_km calculation
- **JavaScript handles**: gender, age range, search text, friends_only filters (client-side only)
- **NO page reloads** on filter changes - only CSS `.hidden` class toggling

### Data Flow

1. **Page Load** → SQL query (bounding box + is_friend LEFT JOIN) → Rust haversine filter → Render ALL users with data-* attributes
2. **User Changes Filter** → JavaScript applyFilters() → Check data-* attributes → Toggle .hidden class → Console logs "✅ Zichtbaar: X/Y"
3. **User Changes Location** → Location picker submits form → Page reload with new lat/lon parameters

### What Gets Filtered Where

| Filter | Scope | Implementation |
|--------|-------|-----------------|
| gender, age range, search text, friends_only | JavaScript | Client-side `.hidden` toggle (instant) |
| radius_km | Rust/Haversine | Exact distance check in haversine_km function |
| lat/lon bounding box | SQL | BETWEEN clause on latitude/longitude |

### Important Notes

- **radius_km is variable** (1-200km) - NOT hardcoded, uses effective_filters.radius_km
- **is_friend calculated via LEFT JOIN** - Single SQL query, no N+1 problems, no pre-fetching
- **All 500 users are rendered** - No truncate to 50, allows client-side flexibility
- **User card data attributes** - data-is-friend, data-gender, data-age, data-name, data-city for JavaScript filtering

## Security Reminders

- Auth middleware validates every protected route
- Cookies are HTTP-only and SameSite=Lax—preserve these flags when modifying auth flows
- Validate external API responses (Image API) and propagate status codes rather than panicking
- Avoid logging sensitive data (tokens, passwords)
- Never hardcode secrets; use `.env` for configuration

## Container-per-User

- SQLite database in container met alle user informatie
- JWT token bevat port nummer → routing naar juiste container
- 5 minuten inactiviteit → container stopt
- Container zo klein mogelijk
