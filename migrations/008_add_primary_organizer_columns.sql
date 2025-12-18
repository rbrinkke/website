-- Snapshot helper: store a primary organizer (for fast feed rendering).
-- The full organizer JSON stays as-is for the activity detail page (multiple organizers possible).

ALTER TABLE activities ADD COLUMN primary_organizer_user_id TEXT;
ALTER TABLE activities ADD COLUMN primary_organizer_name TEXT;
ALTER TABLE activities ADD COLUMN primary_organizer_photo_asset_id TEXT;

-- Backfill from existing organizer JSON snapshot.
WITH src AS (
    SELECT
        activity_id,
        json_extract(organizer, '$.user_id') AS uid,
        json_extract(organizer, '$.name') AS name,
        json_extract(organizer, '$.photo_url') AS photo_url,
        instr(json_extract(organizer, '$.photo_url'), '/api/v1/images/') AS p
    FROM activities
)
UPDATE activities
SET
    primary_organizer_user_id = (SELECT uid FROM src WHERE src.activity_id = activities.activity_id),
    primary_organizer_name = (SELECT name FROM src WHERE src.activity_id = activities.activity_id),
    primary_organizer_photo_asset_id = (
        SELECT
            CASE
                WHEN photo_url IS NULL OR trim(photo_url) = '' THEN NULL
                WHEN p > 0 THEN
                    CASE
                        WHEN instr(substr(photo_url, p + length('/api/v1/images/')), '/') > 0 THEN
                            substr(
                                substr(photo_url, p + length('/api/v1/images/')),
                                1,
                                instr(substr(photo_url, p + length('/api/v1/images/')), '/') - 1
                            )
                        ELSE substr(photo_url, p + length('/api/v1/images/'))
                    END
                WHEN instr(photo_url, '/') = 0 THEN photo_url
                ELSE NULL
            END
        FROM src
        WHERE src.activity_id = activities.activity_id
    )
WHERE primary_organizer_user_id IS NULL
   OR primary_organizer_name IS NULL
   OR primary_organizer_photo_asset_id IS NULL;

