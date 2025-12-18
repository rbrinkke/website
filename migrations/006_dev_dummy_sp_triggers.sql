-- DEV ONLY: allow local UI testing before the Rust-registered SQLite UDFs exist.
--
-- Problem: triggers that call missing UDFs fail at prepare time and block inserts.
-- Solution: replace apply-triggers with "dummy" triggers that log the call and allow commit.
--
-- When the real UDFs exist, drop these dummy triggers and recreate the apply triggers.

CREATE TABLE IF NOT EXISTS sp_call_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    sp_name TEXT NOT NULL,
    command_table TEXT NOT NULL,
    command_id TEXT NOT NULL
);

DROP TRIGGER IF EXISTS trg_activity_signup_commands_apply;
DROP TRIGGER IF EXISTS trg_activity_waitlist_commands_apply;

CREATE TRIGGER IF NOT EXISTS trg_activity_signup_commands_apply
AFTER INSERT ON activity_signup_commands
BEGIN
    INSERT INTO sp_call_log (sp_name, command_table, command_id)
    VALUES ('sp_apply_activity_signup_command', 'activity_signup_commands', NEW.id);
END;

CREATE TRIGGER IF NOT EXISTS trg_activity_waitlist_commands_apply
AFTER INSERT ON activity_waitlist_commands
BEGIN
    INSERT INTO sp_call_log (sp_name, command_table, command_id)
    VALUES ('sp_apply_activity_waitlist_command', 'activity_waitlist_commands', NEW.id);
END;

