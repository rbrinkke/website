#!/usr/bin/env bash
set -euo pipefail

# Dev helper: set an activity's max/current participants for UI testing.
# Usage:
#   ./scripts/dev-set-activity-capacity.sh <activity_id> <max> [current]

activity_id="${1:-}"
max="${2:-}"
current="${3:-}"

if [[ -z "$activity_id" || -z "$max" ]]; then
  echo "Usage: $0 <activity_id> <max> [current]" >&2
  exit 2
fi

if [[ -z "$current" ]]; then
  current="$max"
fi

sqlite3 goamet.db <<SQL
UPDATE activities
SET max_participants = CAST('$max' AS INTEGER),
    current_participants_count = CAST('$current' AS INTEGER)
WHERE activity_id = '$activity_id';

SELECT activity_id, title, current_participants_count, max_participants
FROM activities
WHERE activity_id = '$activity_id';
SQL

