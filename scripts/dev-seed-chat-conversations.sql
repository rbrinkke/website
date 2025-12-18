-- Dev helper: seed a few chat_conversations rows into the local goamet.db.
-- This is only for UI development until sync is ready.
--
-- Usage:
--   sqlite3 goamet.db < scripts/dev-seed-chat-conversations.sql

BEGIN;

-- Use existing local users/activities if present.
-- Abbas is typically present in the snapshot.

INSERT OR REPLACE INTO chat_conversations (
  conversation_id,
  chat_context,
  relationship_status,
  title,
  subtitle,
  image_asset_id,
  target_id,
  effective_mask,
  chat_status,
  is_initiator,
  participant_role,
  other_user_id,
  other_user_name,
  other_user_photo_asset_id,
  row_hash,
  changed_at,
  is_deleted,
  other_user_is_verified
)
SELECT
  a.activity_id AS conversation_id,
  'activity' AS chat_context,
  'active' AS relationship_status,
  a.title AS title,
  'Activiteit chat' AS subtitle,
  a.main_photo_asset_id AS image_asset_id,
  a.activity_id AS target_id,
  15 AS effective_mask,
  'accepted' AS chat_status,
  0 AS is_initiator,
  COALESCE(a.my_role, 'member') AS participant_role,
  NULL AS other_user_id,
  NULL AS other_user_name,
  NULL AS other_user_photo_asset_id,
  hex(randomblob(16)) AS row_hash,
  datetime('now') AS changed_at,
  0 AS is_deleted,
  NULL AS other_user_is_verified
FROM activities a
WHERE a.is_deleted = 0
ORDER BY a.scheduled_at
LIMIT 1;

INSERT OR REPLACE INTO chat_conversations (
  conversation_id,
  chat_context,
  relationship_status,
  title,
  subtitle,
  image_asset_id,
  target_id,
  effective_mask,
  chat_status,
  is_initiator,
  other_user_id,
  other_user_name,
  other_user_photo_asset_id,
  row_hash,
  changed_at,
  is_deleted,
  other_user_is_verified
)
SELECT
  lower(hex(randomblob(16))) || '-' || lower(hex(randomblob(2))) || '-' || lower(hex(randomblob(2))) || '-' || lower(hex(randomblob(2))) || '-' || lower(hex(randomblob(6))) AS conversation_id,
  'private' AS chat_context,
  'accepted' AS relationship_status,
  u.name AS title,
  'Privé chat' AS subtitle,
  u.main_photo_url AS image_asset_id,
  u.user_id AS target_id,
  15 AS effective_mask,
  'accepted' AS chat_status,
  1 AS is_initiator,
  u.user_id AS other_user_id,
  u.name AS other_user_name,
  u.main_photo_url AS other_user_photo_asset_id,
  hex(randomblob(16)) AS row_hash,
  datetime('now', '-1 minute') AS changed_at,
  0 AS is_deleted,
  u.is_verified AS other_user_is_verified
FROM users u
WHERE u.is_deleted = 0
  AND lower(u.name) <> 'abbas'
  AND u.name IS NOT NULL
ORDER BY random()
LIMIT 2;

COMMIT;

