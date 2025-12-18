-- Friendship write path (command table + apply trigger).
-- Matches the app's "command row → trigger → UDF" pattern.

CREATE TABLE IF NOT EXISTS friendship_commands (
  id TEXT PRIMARY KEY,
  actor_user_id TEXT NOT NULL,
  target_user_id TEXT NOT NULL,
  action TEXT NOT NULL CHECK (action IN ('request', 'cancel', 'accept', 'decline')),
  note TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_friendship_commands_created_at
  ON friendship_commands (created_at);

CREATE INDEX IF NOT EXISTS idx_friendship_commands_actor_created_at
  ON friendship_commands (actor_user_id, created_at);

CREATE INDEX IF NOT EXISTS idx_friendship_commands_target_created_at
  ON friendship_commands (target_user_id, created_at);

DROP TRIGGER IF EXISTS trg_friendship_commands_apply;

CREATE TRIGGER IF NOT EXISTS trg_friendship_commands_apply
AFTER INSERT ON friendship_commands
BEGIN
  INSERT INTO sp_call_log (sp_name, command_table, command_id)
  VALUES ('sp_apply_friendship_command', 'friendship_commands', NEW.id);
END;

