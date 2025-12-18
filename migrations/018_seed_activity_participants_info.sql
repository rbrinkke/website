-- Seed "info" tiles for the participants strip (always rendered as last tile).
-- We keep these in promotion_units so copy variants can be updated centrally.

INSERT INTO promotion_units (
  promotion_unit_id,
  placement,
  promo_group,
  locale,
  priority,
  weight,
  is_active,
  starts_at,
  ends_at,
  title,
  body,
  emoji,
  layout_kind,
  background_color,
  background_gradient,
  media_kind,
  media_asset_id,
  poster_asset_id,
  video_autoplay,
  video_muted,
  video_loop,
  video_controls,
  actions_json,
  created_at,
  updated_at
)
VALUES
  (lower(hex(randomblob(16))), 'activities_participants_info', 'default', 'nl', 0, 10, 1, NULL, NULL, 'Info', 'Meer info', 'ℹ️', 'tile', '#1E88E5', NULL, NULL, NULL, NULL, 0, 1, 1, 0, '[{"kind":"info","label":"Info","icon":"info"}]', datetime('now'), datetime('now')),
  (lower(hex(randomblob(16))), 'activities_participants_info', 'default', 'nl', 0, 10, 1, NULL, NULL, 'Info', 'Details', 'ℹ️', 'tile', '#1E88E5', NULL, NULL, NULL, NULL, 0, 1, 1, 0, '[{"kind":"info","label":"Info","icon":"info"}]', datetime('now'), datetime('now')),
  (lower(hex(randomblob(16))), 'activities_participants_info', 'default', 'nl', 0, 10, 1, NULL, NULL, 'Info', 'Lees meer', 'ℹ️', 'tile', '#1E88E5', NULL, NULL, NULL, NULL, 0, 1, 1, 0, '[{"kind":"info","label":"Info","icon":"info"}]', datetime('now'), datetime('now')),
  (lower(hex(randomblob(16))), 'activities_participants_info', 'default', 'nl', 0, 8, 1, NULL, NULL, 'Info', 'Wat gaan we doen?', 'ℹ️', 'tile', '#1E88E5', NULL, NULL, NULL, NULL, 0, 1, 1, 0, '[{"kind":"info","label":"Info","icon":"info"}]', datetime('now'), datetime('now')),
  (lower(hex(randomblob(16))), 'activities_participants_info', 'default', 'nl', 0, 8, 1, NULL, NULL, 'Info', 'Planning', 'ℹ️', 'tile', '#1E88E5', NULL, NULL, NULL, NULL, 0, 1, 1, 0, '[{"kind":"info","label":"Info","icon":"info"}]', datetime('now'), datetime('now')),
  (lower(hex(randomblob(16))), 'activities_participants_info', 'default', 'nl', 0, 8, 1, NULL, NULL, 'Info', 'Waar & wanneer', 'ℹ️', 'tile', '#1E88E5', NULL, NULL, NULL, NULL, 0, 1, 1, 0, '[{"kind":"info","label":"Info","icon":"info"}]', datetime('now'), datetime('now')),
  (lower(hex(randomblob(16))), 'activities_participants_info', 'default', 'nl', 0, 6, 1, NULL, NULL, 'Info', 'Praktisch', 'ℹ️', 'tile', '#1E88E5', NULL, NULL, NULL, NULL, 0, 1, 1, 0, '[{"kind":"info","label":"Info","icon":"info"}]', datetime('now'), datetime('now')),
  (lower(hex(randomblob(16))), 'activities_participants_info', 'default', 'nl', 0, 6, 1, NULL, NULL, 'Info', 'Bekijk info', 'ℹ️', 'tile', '#1E88E5', NULL, NULL, NULL, NULL, 0, 1, 1, 0, '[{"kind":"info","label":"Info","icon":"info"}]', datetime('now'), datetime('now')),
  (lower(hex(randomblob(16))), 'activities_participants_info', 'default', 'nl', 0, 6, 1, NULL, NULL, 'Info', 'Over deze activiteit', 'ℹ️', 'tile', '#1E88E5', NULL, NULL, NULL, NULL, 0, 1, 1, 0, '[{"kind":"info","label":"Info","icon":"info"}]', datetime('now'), datetime('now')),
  (lower(hex(randomblob(16))), 'activities_participants_info', 'default', 'nl', 0, 4, 1, NULL, NULL, 'Info', 'Info', 'ℹ️', 'tile', '#1E88E5', NULL, NULL, NULL, NULL, 0, 1, 1, 0, '[{"kind":"info","label":"Info","icon":"info"}]', datetime('now'), datetime('now'));
