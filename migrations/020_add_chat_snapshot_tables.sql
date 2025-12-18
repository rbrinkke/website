-- Chat snapshot tables for webapp UI (sourced from activitydb only).
-- These are read-only snapshot tables; writes happen via *_commands tables.

-- A flat, UI-friendly chat conversation row (private chats + activity chats).
CREATE TABLE IF NOT EXISTS chat_conversations (
  conversation_id TEXT PRIMARY KEY, -- UUID (private_chat_id or activity_id)
  chat_context TEXT NOT NULL CHECK (chat_context IN ('private', 'activity')),

  -- Relationship / request state (activity chats are always "active")
  -- private: pending|accepted|rejected
  -- activity: active
  relationship_status TEXT NOT NULL DEFAULT 'active'
    CHECK (relationship_status IN ('active', 'pending', 'accepted', 'rejected')),

  -- UI fields (fully denormalized so the frontend never has to join)
  title TEXT,
  image_asset_id TEXT, -- UUID of avatar/cover image if available
  target_id TEXT, -- UUID: other_user_id (private) or activity_id (activity)

  -- Permission context (output modeled after activity.get_chat_permission_data)
  effective_mask INTEGER,
  chat_status TEXT, -- private: pending|accepted|rejected, activity: active (optional)
  is_initiator INTEGER, -- 0/1 (private only)
  block_direction TEXT, -- blocked_by|blocking|null (private only)
  mute_expires_at TEXT,
  participant_role TEXT, -- organizer|co_organizer|member (activity only)
  other_user_id TEXT, -- private only

  -- Optional UX helpers (still activitydb-derived; keep nullable)
  other_user_name TEXT,
  other_user_photo_asset_id TEXT,

  -- Sync tracking
  row_hash TEXT NOT NULL,
  changed_at TEXT NOT NULL,
  is_deleted INTEGER DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_chat_conversations_context
  ON chat_conversations(chat_context);
CREATE INDEX IF NOT EXISTS idx_chat_conversations_status
  ON chat_conversations(relationship_status);
CREATE INDEX IF NOT EXISTS idx_chat_conversations_changed_at
  ON chat_conversations(changed_at);

-- Write path (SQLite -> trigger -> apply -> rollback on error).
-- Receiver accepts/rejects a pending private chat request.
CREATE TABLE IF NOT EXISTS private_chat_request_commands (
  id TEXT PRIMARY KEY,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),

  actor_user_id TEXT NOT NULL,
  private_chat_id TEXT NOT NULL,

  action TEXT NOT NULL CHECK (action IN ('accept', 'reject')),

  note TEXT
);

CREATE INDEX IF NOT EXISTS idx_private_chat_request_commands_created_at
  ON private_chat_request_commands(created_at);
CREATE INDEX IF NOT EXISTS idx_private_chat_request_commands_private_chat_created_at
  ON private_chat_request_commands(private_chat_id, created_at);
CREATE INDEX IF NOT EXISTS idx_private_chat_request_commands_actor_created_at
  ON private_chat_request_commands(actor_user_id, created_at);

CREATE TRIGGER IF NOT EXISTS trg_private_chat_request_commands_apply
AFTER INSERT ON private_chat_request_commands
BEGIN
  INSERT INTO sp_call_log (sp_name, command_table, command_id)
  VALUES ('sp_apply_private_chat_request_command', 'private_chat_request_commands', NEW.id);
END;
