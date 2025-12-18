-- Add query-friendly lat/lon columns for activities (extracted from location JSON)
ALTER TABLE activities ADD COLUMN latitude REAL;
ALTER TABLE activities ADD COLUMN longitude REAL;

-- Backfill from existing location JSON (server snapshot)
UPDATE activities
SET
    latitude = COALESCE(
        json_extract(location, '$.latitude'),
        json_extract(location, '$.lat'),
        json_extract(location, '$._geo.lat')
    ),
    longitude = COALESCE(
        json_extract(location, '$.longitude'),
        json_extract(location, '$.lng'),
        json_extract(location, '$.lon'),
        json_extract(location, '$._geo.lng')
    )
WHERE (latitude IS NULL OR longitude IS NULL)
  AND location IS NOT NULL
  AND location != '';

CREATE INDEX idx_activities_lat_lon ON activities(latitude, longitude);

-- High-churn participant data: store per-user rows (waitlist/registered etc)
CREATE TABLE IF NOT EXISTS activity_participants (
    activity_id TEXT NOT NULL,
    user_id TEXT NOT NULL,

    -- Snapshot fields (as provided by server in participants blob)
    name TEXT,
    photo_url TEXT,

    role TEXT,                    -- organizer/co_organizer/member
    participation_status TEXT,     -- registered/waitlisted/declined/cancelled
    attendance_status TEXT,        -- registered/attended/no_show
    joined_at TEXT,               -- ISO timestamp

    updated_at TEXT DEFAULT (datetime('now')),
    is_deleted INTEGER DEFAULT 0,

    PRIMARY KEY (activity_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_activity_participants_activity
ON activity_participants(activity_id)
WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_activity_participants_user
ON activity_participants(user_id)
WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_activity_participants_status
ON activity_participants(activity_id, participation_status, joined_at)
WHERE is_deleted = 0;

