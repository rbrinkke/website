#!/usr/bin/env bash
set -euo pipefail

# Baseline/repair `_sqlx_migrations` for an existing SQLite DB that already has the schema.
# This does NOT run migrations; it records them as applied so `sqlx migrate run` won't
# try to re-apply old migrations and fail on duplicate columns/tables.

database_url="${DATABASE_URL:-sqlite://goamet.db}"

db_path_from_url() {
  local url="$1"
  url="${url%%\?*}"
  case "$url" in
  sqlite://*)
    printf "%s" "${url#sqlite://}"
    ;;
  sqlite:*)
    local p="${url#sqlite:}"
    p="${p#//}"
    printf "%s" "$p"
    ;;
  *)
    echo "Unsupported DATABASE_URL for this script: $url" >&2
    return 2
    ;;
  esac
}

db_file="$(db_path_from_url "$database_url")"
if [[ -z "$db_file" ]]; then
  echo "Could not resolve SQLite DB file from DATABASE_URL=$database_url" >&2
  exit 2
fi

if [[ ! -f "$db_file" ]]; then
  echo "SQLite DB file not found: $db_file" >&2
  exit 2
fi

sqlite3 "$db_file" <<'SQL'
CREATE TABLE IF NOT EXISTS _sqlx_migrations (
  version BIGINT PRIMARY KEY,
  description TEXT NOT NULL,
  installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  success BOOLEAN NOT NULL,
  checksum BLOB NOT NULL,
  execution_time BIGINT NOT NULL
);
SQL

shopt -s nullglob
migration_files=(migrations/*.sql)
if [[ ${#migration_files[@]} -eq 0 ]]; then
  echo "No migrations found in ./migrations" >&2
  exit 2
fi

for f in "${migration_files[@]}"; do
  base="$(basename "$f")"
  ver="${base%%_*}"
  ver_num=$((10#$ver))
  desc="${base#*_}"
  desc="${desc%.sql}"
  desc="${desc//_/ }"
  desc_sql="${desc//\'/\'\'}"
  if command -v sha384sum >/dev/null 2>&1; then
    checksum_hex="$(sha384sum "$f" | awk '{print $1}')"
  elif command -v shasum >/dev/null 2>&1; then
    checksum_hex="$(shasum -a 384 "$f" | awk '{print $1}')"
  else
    echo "Missing sha384sum/shasum; cannot compute migration checksum." >&2
    exit 2
  fi

  sqlite3 "$db_file" <<SQL
INSERT INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time)
VALUES ($ver_num, '$desc_sql', datetime('now'), 1, X'$checksum_hex', 0)
ON CONFLICT(version) DO UPDATE SET
  description=excluded.description,
  success=1,
  checksum=excluded.checksum;
SQL
done

echo "Baselined _sqlx_migrations for $db_file"
