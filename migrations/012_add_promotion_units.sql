-- Unified, flat promotions table (media + copy + styling + unlimited actions).
-- Local UI configuration content (not a snapshot/derived table).

CREATE TABLE IF NOT EXISTS promotion_units (
  promotion_unit_id TEXT PRIMARY KEY, -- UUID

  -- Placement & grouping
  placement TEXT NOT NULL,            -- e.g. 'activities_participants_filler'
  promo_group TEXT NOT NULL DEFAULT 'default',
  locale TEXT NOT NULL DEFAULT 'nl',

  -- Selection
  priority INTEGER NOT NULL DEFAULT 0,
  weight INTEGER NOT NULL DEFAULT 1,
  is_active INTEGER NOT NULL DEFAULT 1,
  starts_at TEXT,
  ends_at TEXT,

  -- Copy
  title TEXT,
  body TEXT,
  emoji TEXT,

  -- Styling (flat)
  layout_kind TEXT,                   -- e.g. 'card' | 'tile' | 'banner'
  background_color TEXT,              -- e.g. '#0B1220' or 'rgba(...)'
  background_gradient TEXT,           -- optional raw CSS gradient string

  -- Media
  media_kind TEXT CHECK (media_kind IN ('image', 'video')),
  media_asset_id TEXT,                -- UUID of the image/video asset
  poster_asset_id TEXT,               -- UUID for video poster (optional)

  -- Video options (applies when media_kind='video')
  video_autoplay INTEGER NOT NULL DEFAULT 0,
  video_muted INTEGER NOT NULL DEFAULT 1,
  video_loop INTEGER NOT NULL DEFAULT 1,
  video_controls INTEGER NOT NULL DEFAULT 0,

  -- Unlimited actions (flat via JSON array)
  actions_json TEXT,                  -- JSON array, optional

  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_promotion_units_select
  ON promotion_units (placement, promo_group, locale, is_active, starts_at, ends_at, priority);

CREATE INDEX IF NOT EXISTS idx_promotion_units_sort
  ON promotion_units (placement, promo_group, locale, priority, weight);

