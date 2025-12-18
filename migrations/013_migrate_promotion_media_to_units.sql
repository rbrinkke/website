-- One-time migration from `promotion_media` (older) to `promotion_units` (unified).
-- Safe to run even if `promotion_media` doesn't exist.

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
SELECT
  pm.promotion_media_id AS promotion_unit_id,
  pm.promotion_key AS placement,
  pm.promo_group AS promo_group,
  'nl' AS locale,
  0 AS priority,
  1 AS weight,
  pm.is_active AS is_active,
  pm.starts_at AS starts_at,
  pm.ends_at AS ends_at,
  pm.title AS title,
  NULL AS body,
  NULL AS emoji,
  NULL AS layout_kind,
  NULL AS background_color,
  NULL AS background_gradient,
  pm.media_kind AS media_kind,
  pm.media_asset_id AS media_asset_id,
  NULL AS poster_asset_id,
  0 AS video_autoplay,
  1 AS video_muted,
  1 AS video_loop,
  0 AS video_controls,
  CASE
    WHEN pm.cta_href IS NOT NULL AND TRIM(pm.cta_href) != '' THEN
      json_array(
        json_object(
          'kind', 'open_url',
          'label', COALESCE(NULLIF(TRIM(pm.cta_label), ''), 'Bekijk'),
          'href', pm.cta_href,
          'method', 'GET',
          'style', 'primary'
        )
      )
    ELSE NULL
  END AS actions_json,
  pm.created_at AS created_at,
  pm.updated_at AS updated_at
FROM promotion_media pm
WHERE EXISTS (SELECT 1 FROM sqlite_master WHERE type='table' AND name='promotion_media')
  AND NOT EXISTS (
    SELECT 1 FROM promotion_units u WHERE u.promotion_unit_id = pm.promotion_media_id
  );

DROP TABLE IF EXISTS promotion_media;

