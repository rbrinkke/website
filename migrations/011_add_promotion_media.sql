-- Promotion media (images/videos) that can be used as UI CTAs/fillers.
-- This is local configuration content (not a snapshot/derived table).

CREATE TABLE IF NOT EXISTS promotion_media (
  promotion_media_id TEXT PRIMARY KEY, -- UUID
  promotion_key TEXT NOT NULL,         -- e.g. 'activities_participants_fill'
  promo_group TEXT NOT NULL DEFAULT 'default',
  media_kind TEXT NOT NULL CHECK (media_kind IN ('image', 'video')),
  media_asset_id TEXT NOT NULL,        -- UUID of the image/video asset
  title TEXT,
  cta_label TEXT,
  cta_href TEXT,
  sort_order INTEGER NOT NULL DEFAULT 0,
  is_active INTEGER NOT NULL DEFAULT 1,
  starts_at TEXT,                      -- ISO datetime, optional
  ends_at TEXT,                        -- ISO datetime, optional
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_promotion_media_key_group
  ON promotion_media (promotion_key, promo_group, sort_order);

CREATE INDEX IF NOT EXISTS idx_promotion_media_active
  ON promotion_media (is_active, starts_at, ends_at);

