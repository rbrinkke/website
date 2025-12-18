#!/usr/bin/env bash
set -euo pipefail

# Dev helper: force an activity to be "full" in the local SQLite snapshot so waitlist UI appears.
# Usage:
#   ./scripts/dev-force-activity-full.sh <activity_id>
#
# This only touches the local `goamet.db` used for UI testing.

activity_id="${1:-}"
if [[ -z "$activity_id" ]]; then
  echo "Usage: $0 <activity_id>" >&2
  echo "Tip: copy an id from /activities links, e.g. /activities/<id>" >&2
  exit 2
fi

db_file="goamet.db"
if [[ ! -f "$db_file" ]]; then
  echo "DB not found: $db_file" >&2
  exit 2
fi

sqlite3 "$db_file" <<SQL
UPDATE activities
SET
  current_participants_count = CASE
    WHEN COALESCE(current_participants_count, 0) <= 0 THEN 1
    ELSE current_participants_count
  END,
  max_participants = CASE
    WHEN COALESCE(max_participants, 0) <= 0 THEN 1
    ELSE max_participants
  END;

UPDATE activities
SET
  current_participants_count = max_participants
WHERE activity_id = '$activity_id';

SELECT activity_id, title, current_participants_count, max_participants
FROM activities
WHERE activity_id = '$activity_id';
SQL

echo "✅ Forced activity to full: $activity_id"
