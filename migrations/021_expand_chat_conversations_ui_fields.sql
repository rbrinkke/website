-- Expand chat_conversations with UI-friendly, activitydb-sourced fields.
-- All columns are nullable so sync can roll out incrementally.

ALTER TABLE chat_conversations ADD COLUMN subtitle TEXT;

-- Activity-chat convenience fields (all derived from activitydb activities snapshot).
ALTER TABLE chat_conversations ADD COLUMN activity_status TEXT;
ALTER TABLE chat_conversations ADD COLUMN activity_scheduled_at TEXT;
ALTER TABLE chat_conversations ADD COLUMN activity_city TEXT;
ALTER TABLE chat_conversations ADD COLUMN activity_location_name TEXT;
ALTER TABLE chat_conversations ADD COLUMN activity_main_photo_asset_id TEXT;

-- Private-chat convenience fields (all derived from activitydb users snapshot).
ALTER TABLE chat_conversations ADD COLUMN other_user_username TEXT;
ALTER TABLE chat_conversations ADD COLUMN other_user_is_verified INTEGER;

CREATE INDEX IF NOT EXISTS idx_chat_conversations_target_id
  ON chat_conversations(target_id);
CREATE INDEX IF NOT EXISTS idx_chat_conversations_other_user_id
  ON chat_conversations(other_user_id);

