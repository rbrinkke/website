use sqlx::SqlitePool;

use crate::models::PromotionUnitRow;

const SQL_LIST_ACTIVE_PROMOTION_UNITS: &str = r#"
SELECT
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
FROM promotion_units
WHERE is_active = 1
  AND placement = ?
  AND promo_group = ?
  AND locale = ?
  AND (starts_at IS NULL OR datetime(starts_at) <= datetime('now'))
  AND (ends_at IS NULL OR datetime(ends_at) >= datetime('now'))
ORDER BY priority DESC, weight DESC, created_at DESC
"#;

pub async fn list_active_for_placement(
    pool: &SqlitePool,
    placement: &str,
    promo_group: &str,
    locale: &str,
) -> sqlx::Result<Vec<PromotionUnitRow>> {
    sqlx::query_as::<_, PromotionUnitRow>(SQL_LIST_ACTIVE_PROMOTION_UNITS)
        .bind(placement)
        .bind(promo_group)
        .bind(locale)
        .fetch_all(pool)
        .await
}
