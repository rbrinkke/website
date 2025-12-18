-- Snapshot helper: organizer == owner.
-- The central DB should fill this during sync; we backfill from my_role for existing local data.

ALTER TABLE activities
ADD COLUMN is_owner INTEGER NOT NULL DEFAULT 0;

UPDATE activities
SET is_owner = 1
WHERE COALESCE(my_role, '') IN ('organizer', 'co_organizer');

