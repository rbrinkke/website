-- Rename snapshot helper column to reflect semantics:
-- - can_manage_activity: the signed-in user can manage the activity (organizer)
-- - can_manage_attendance: the signed-in user can help manage attendance (co-organizer or organizer)

-- SQLite updates dependent view definitions on table/column renames. Earlier migrations rebuild the `activities` table,
-- which can leave views pointing at `activities_old` (and thus invalid). Drop them first so this migration is robust
-- when applied to a fresh DB.
DROP VIEW IF EXISTS v_upcoming_activities;
DROP VIEW IF EXISTS v_my_activities;

ALTER TABLE activities RENAME COLUMN is_owner TO can_manage_activity;

ALTER TABLE activities
ADD COLUMN can_manage_attendance INTEGER NOT NULL DEFAULT 0;

-- Backfill semantics from my_role snapshot:
-- - organizer => can_manage_activity=1 and can_manage_attendance=1
-- - co_organizer => can_manage_activity=0 but can_manage_attendance=1
-- - others => 0/0
UPDATE activities
SET
    can_manage_activity = CASE WHEN COALESCE(my_role, '') = 'organizer' THEN 1 ELSE 0 END,
    can_manage_attendance = CASE
        WHEN COALESCE(my_role, '') IN ('organizer', 'co_organizer') THEN 1
        ELSE 0
    END;
