-- Fix invalid views that still reference a temporary rebuild table.

DROP VIEW IF EXISTS v_upcoming_activities;
DROP VIEW IF EXISTS v_my_activities;

CREATE VIEW v_upcoming_activities AS
SELECT *
FROM activities
WHERE status = 'published'
  AND is_deleted = 0
  AND datetime(scheduled_at) > datetime('now')
  AND datetime(scheduled_at) < datetime('now', '+7 days')
ORDER BY scheduled_at ASC;

CREATE VIEW v_my_activities AS
SELECT *
FROM activities
WHERE (my_role IS NOT NULL OR my_participation_status IS NOT NULL)
  AND is_deleted = 0
ORDER BY scheduled_at ASC;

