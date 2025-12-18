-- Local per-activity settings (do not modify snapshot tables directly).

CREATE TABLE IF NOT EXISTS activity_settings (
  activity_id TEXT PRIMARY KEY,
  waitlist_enabled INTEGER NOT NULL DEFAULT 1,
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_activity_settings_waitlist
  ON activity_settings (waitlist_enabled);

