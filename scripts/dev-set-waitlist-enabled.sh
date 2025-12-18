#!/usr/bin/env bash
set -euo pipefail

# Dev helper: enable/disable waitlist per activity (local settings table).
# Usage:
#   ./scripts/dev-set-waitlist-enabled.sh <activity_id> <0|1>

activity_id="${1:-}"
enabled="${2:-}"

if [[ -z "$activity_id" || -z "$enabled" ]]; then
  echo "Usage: $0 <activity_id> <0|1>" >&2
  exit 2
fi

if [[ "$enabled" != "0" && "$enabled" != "1" ]]; then
  echo "enabled must be 0 or 1" >&2
  exit 2
fi

sqlite3 goamet.db <<SQL
INSERT INTO activity_settings (activity_id, waitlist_enabled, updated_at)
VALUES ('$activity_id', CAST('$enabled' AS INTEGER), datetime('now'))
ON CONFLICT(activity_id) DO UPDATE SET
  waitlist_enabled = excluded.waitlist_enabled,
  updated_at = datetime('now');

SELECT activity_id, waitlist_enabled, updated_at
FROM activity_settings
WHERE activity_id = '$activity_id';
SQL

