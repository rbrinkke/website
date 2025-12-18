-- 1) Backfill the new activity_participants table from the activities.participants JSON blob.
-- This is a DEV snapshot DB: server is the source of truth; we just want fast reads locally.
INSERT OR REPLACE INTO activity_participants (
    activity_id,
    user_id,
    name,
    photo_url,
    role,
    participation_status,
    attendance_status,
    joined_at
)
SELECT
    a.activity_id,
    json_extract(p.value, '$.user_id') AS user_id,
    json_extract(p.value, '$.name') AS name,
    json_extract(p.value, '$.photo_url') AS photo_url,
    json_extract(p.value, '$.role') AS role,
    json_extract(p.value, '$.participation_status') AS participation_status,
    json_extract(p.value, '$.attendance_status') AS attendance_status,
    json_extract(p.value, '$.joined_at') AS joined_at
FROM activities a, json_each(a.participants) p
WHERE a.participants IS NOT NULL
  AND a.participants != ''
  AND a.participants != '[]'
  AND json_type(p.value) = 'object'
  AND json_extract(p.value, '$.user_id') IS NOT NULL;

-- 2) Drop the old participants blob column by rebuilding the activities table (SQLite-compatible).
BEGIN TRANSACTION;

-- Existing indexes keep their names after the table rename, so drop them first to avoid name collisions.
DROP INDEX IF EXISTS idx_activities_scheduled;
DROP INDEX IF EXISTS idx_activities_status;
DROP INDEX IF EXISTS idx_activities_city;
DROP INDEX IF EXISTS idx_activities_distance;
DROP INDEX IF EXISTS idx_activities_changed_at;
DROP INDEX IF EXISTS idx_activities_organizer;
DROP INDEX IF EXISTS idx_activities_lat_lon;

ALTER TABLE activities RENAME TO activities_old;

CREATE TABLE activities (
    activity_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,

    activity_type TEXT DEFAULT 'standard' CHECK (activity_type IN ('standard', 'xxl', 'womens_only', 'mens_only')),
    privacy_level TEXT DEFAULT 'public' CHECK (privacy_level IN ('public', 'friends_only', 'invite_only')),
    status TEXT DEFAULT 'published' CHECK (status IN ('draft', 'published', 'cancelled', 'completed')),

    scheduled_at TEXT NOT NULL,  -- ISO timestamp
    duration_minutes INTEGER,
    joinable_at_free TEXT,  -- ISO timestamp (when free users can join)

    max_participants INTEGER NOT NULL,
    current_participants_count INTEGER DEFAULT 0,
    waitlist_count INTEGER DEFAULT 0,

    language TEXT DEFAULT 'en',
    city TEXT,

    -- Distance from current user (calculated, for sorting/filtering)
    distance_km REAL,

    -- Embedded from activity_locations as JSON (kept as snapshot)
    location TEXT DEFAULT '{}',
    latitude REAL,
    longitude REAL,

    -- Embedded organizer info as JSON (kept as snapshot)
    organizer TEXT NOT NULL DEFAULT '{}',

    -- Embedded from activity_tags as JSON array: ["tag1", "tag2", ...]
    tags TEXT DEFAULT '[]',

    -- Embedded category info as JSON (kept as snapshot)
    category TEXT DEFAULT '{}',

    -- My personal status for this activity (per requesting user)
    my_role TEXT,  -- 'organizer', 'co_organizer', 'member', null
    my_participation_status TEXT,  -- 'registered', 'waitlisted', 'declined', 'cancelled', null
    my_attendance_status TEXT,  -- 'registered', 'attended', 'no_show', null
    am_on_waitlist INTEGER DEFAULT 0,
    my_waitlist_position INTEGER,

    -- Review stats (calculated by server, embedded here)
    review_count INTEGER DEFAULT 0,
    avg_rating REAL,  -- 1.0 to 5.0

    -- Timestamps
    created_at TEXT,
    updated_at TEXT,
    completed_at TEXT,
    cancelled_at TEXT,

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    is_deleted INTEGER DEFAULT 0,

    main_photo_asset_id TEXT,
    is_joined INTEGER DEFAULT 0
);

INSERT INTO activities (
    activity_id,
    title,
    description,
    activity_type,
    privacy_level,
    status,
    scheduled_at,
    duration_minutes,
    joinable_at_free,
    max_participants,
    current_participants_count,
    waitlist_count,
    language,
    city,
    distance_km,
    location,
    latitude,
    longitude,
    organizer,
    tags,
    category,
    my_role,
    my_participation_status,
    my_attendance_status,
    am_on_waitlist,
    my_waitlist_position,
    review_count,
    avg_rating,
    created_at,
    updated_at,
    completed_at,
    cancelled_at,
    row_hash,
    changed_at,
    is_deleted,
    main_photo_asset_id,
    is_joined
)
SELECT
    activity_id,
    title,
    description,
    activity_type,
    privacy_level,
    status,
    scheduled_at,
    duration_minutes,
    joinable_at_free,
    max_participants,
    current_participants_count,
    waitlist_count,
    language,
    city,
    distance_km,
    location,
    latitude,
    longitude,
    organizer,
    tags,
    category,
    my_role,
    my_participation_status,
    my_attendance_status,
    am_on_waitlist,
    my_waitlist_position,
    review_count,
    avg_rating,
    created_at,
    updated_at,
    completed_at,
    cancelled_at,
    row_hash,
    changed_at,
    is_deleted,
    main_photo_asset_id,
    is_joined
FROM activities_old;

DROP TABLE activities_old;

-- Recreate indexes for the rebuilt table
CREATE INDEX IF NOT EXISTS idx_activities_scheduled ON activities(scheduled_at);
CREATE INDEX IF NOT EXISTS idx_activities_status ON activities(status);
CREATE INDEX IF NOT EXISTS idx_activities_city ON activities(city);
CREATE INDEX IF NOT EXISTS idx_activities_distance ON activities(distance_km);
CREATE INDEX IF NOT EXISTS idx_activities_changed_at ON activities(changed_at);
CREATE INDEX IF NOT EXISTS idx_activities_organizer ON activities(json_extract(organizer, '$.user_id'));
CREATE INDEX IF NOT EXISTS idx_activities_lat_lon ON activities(latitude, longitude);

COMMIT;
