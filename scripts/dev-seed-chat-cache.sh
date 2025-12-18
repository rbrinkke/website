#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHAT_CACHE_DIR="$ROOT_DIR/chat_cache"
CHAT_CACHE_DB="$CHAT_CACHE_DIR/chat_cache.db"
MAIN_DB="$ROOT_DIR/goamet.db"

mkdir -p "$CHAT_CACHE_DIR"

rm -f "$CHAT_CACHE_DB"
sqlite3 "$CHAT_CACHE_DB" < "$CHAT_CACHE_DIR/schema.sql"

# Seed conversations based on current chat_conversations in goamet.db
sqlite3 "$CHAT_CACHE_DB" <<SQL
ATTACH '$MAIN_DB' AS goamet;

INSERT INTO conversations (conversation_id, chat_context, updated_at, last_message_id, last_message_at, last_message_preview)
SELECT
  conversation_id,
  chat_context,
  datetime('now', '-' || (abs(random()) % 180) || ' seconds') AS updated_at,
  lower(hex(randomblob(16))) AS last_message_id,
  datetime('now', '-' || (abs(random()) % 3600) || ' seconds') AS last_message_at,
  CASE
    WHEN chat_context = 'activity' THEN 'Zin om mee te doen?'
    ELSE 'Hey! Alles goed?'
  END AS last_message_preview
FROM goamet.chat_conversations
WHERE is_deleted = 0;

-- Insert a small, nice message history per conversation.
INSERT INTO messages (conversation_id, message_id, created_at, sender_id, message_type, content, metadata_json, status, is_deleted, edited_at)
SELECT
  c.conversation_id,
  lower(hex(randomblob(16))) AS message_id,
  datetime('now', '-40 minutes') AS created_at,
  (SELECT user_id FROM goamet.current_user LIMIT 1) AS sender_id,
  'text' AS message_type,
  CASE
    WHEN c.chat_context = 'activity' THEN 'Hoi! Wie gaan er allemaal mee?'
    ELSE 'Hoi! Zin om te chatten?'
  END AS content,
  '{}' AS metadata_json,
  'sent' AS status,
  0 AS is_deleted,
  NULL AS edited_at
FROM goamet.chat_conversations c
WHERE c.is_deleted = 0;

INSERT INTO messages (conversation_id, message_id, created_at, sender_id, message_type, content, metadata_json, status, is_deleted, edited_at)
SELECT
  c.conversation_id,
  lower(hex(randomblob(16))) AS message_id,
  datetime('now', '-35 minutes') AS created_at,
  COALESCE(c.other_user_id, (SELECT user_id FROM goamet.users WHERE lower(name) <> 'abbas' AND name IS NOT NULL LIMIT 1)) AS sender_id,
  'text' AS message_type,
  CASE
    WHEN c.relationship_status = 'pending' THEN 'Ik zie je verzoek, ik reageer straks 🙂'
    WHEN c.chat_context = 'activity' THEN 'Leuk! Ik ben er ook bij.'
    ELSE 'Ja zeker! Hoe is het?'
  END AS content,
  '{}' AS metadata_json,
  'delivered' AS status,
  0 AS is_deleted,
  NULL AS edited_at
FROM goamet.chat_conversations c
WHERE c.is_deleted = 0;

INSERT OR REPLACE INTO conversation_state (conversation_id, last_seen_message_id, last_read_message_id, draft_text, scroll_anchor, scroll_offset, updated_at)
SELECT
  conversation_id,
  NULL,
  NULL,
  '',
  NULL,
  0,
  datetime('now')
FROM goamet.chat_conversations
WHERE is_deleted = 0;

DETACH goamet;
SQL

echo "Seeded chat cache: $CHAT_CACHE_DB"
