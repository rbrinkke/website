-- Write-path (transactional): activity waitlist commands (owner/mod control).
-- This table is NOT a snapshot; it exists only to trigger a synchronous apply to the central DB/service.
--
-- Requires a Rust-registered SQLite UDF that maps 1:1 to the central stored procedure:
--   sp_apply_activity_waitlist_command(command_id TEXT) -> INTEGER
-- Return 1 for success; anything else triggers a ROLLBACK.

CREATE TABLE IF NOT EXISTS activity_waitlist_commands (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Actor = who performs the action (owner/mod)
    actor_user_id TEXT NOT NULL,

    activity_id TEXT NOT NULL,
    subject_user_id TEXT NOT NULL,

    action TEXT NOT NULL CHECK (
        action IN (
            'set_waitlisted',
            'remove_waitlist',
            'set_priority'
        )
    ),

    -- Only used for set_priority
    priority INTEGER,

    note TEXT,

    CHECK (action != 'set_priority' OR priority IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS idx_activity_waitlist_commands_activity_created
ON activity_waitlist_commands(activity_id, created_at);

CREATE INDEX IF NOT EXISTS idx_activity_waitlist_commands_subject_created
ON activity_waitlist_commands(subject_user_id, created_at);

-- Transactional apply: if central apply fails, rollback the whole transaction.
CREATE TRIGGER IF NOT EXISTS trg_activity_waitlist_commands_apply
AFTER INSERT ON activity_waitlist_commands
BEGIN
    SELECT
        CASE
            WHEN sp_apply_activity_waitlist_command(NEW.id) = 1 THEN 1
            ELSE RAISE(ROLLBACK, 'sp_apply_activity_waitlist_command failed')
        END;
END;

