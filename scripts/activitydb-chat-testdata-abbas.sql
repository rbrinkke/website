-- Chat permissions test data using EXISTING users (Abbas + existing users).
-- Does NOT modify existing users; only inserts rows in chats/activities/participants/overrides/blocks.
--
-- Usage:
--   PGPASSWORD=postgres_secure_password_change_in_prod \
--   psql -h localhost -p 5441 -U postgres -d activitydb -f scripts/activitydb-chat-testdata-abbas.sql
--
-- This creates:
-- - 1 activity where Abbas is organizer, and 2 participants. (Activity chat is always active.)
-- - 3 private chats for Abbas:
--     - pending: Abbas -> Jelle (initiator can send, receiver read-only)
--     - accepted: Abbas <-> Sam (both can send)
--     - rejected: Abbas -> Eva (nobody can send)
-- - 1 blocked private chat: Abbas <-> Tom accepted, then Tom blocks Abbas (can_chat false)
--
-- IDs are fixed for repeatability.

\set ON_ERROR_STOP on

BEGIN;

-- Fixed IDs
\set abbas_id '5a81825e-9402-4fd7-9ac5-e6ce0420f1d1'
\set jelle_id '08b5f014-684b-4ba3-8d53-80a6a319ca99'
\set sam_id   '37f09f88-9347-48ad-895d-5335676e74ad'
\set eva_id   '7812ab25-fcc9-4614-8f58-9dc0c61d8175'
\set tom_id   '43406d67-8cb3-4fb7-9a00-2ceaa0b1b9d3'

\set abbas_activity_id 'b8a3b1a4-6a09-49c7-9a3a-b6b1ac2f0b2c'
\set abbas_chat_pending_id  'c2d4f2d6-2e2a-4de2-bec8-3b3465f8b2f1'
\set abbas_chat_accepted_id 'd4a9b6d3-bb8f-4d8b-9f63-2a3d0c0a9a11'
\set abbas_chat_rejected_id 'e2f8b4b9-6c6b-4a1b-8d9a-9bcae78f8e21'
\set abbas_chat_blocked_id  'f0b2c9e4-8d1d-4f2b-9c5f-52c07c5d7e10'

-- Guardrails: ensure users exist (fail fast)
WITH required(id) AS (
  VALUES
    (:'abbas_id'::uuid),
    (:'jelle_id'::uuid),
    (:'sam_id'::uuid),
    (:'eva_id'::uuid),
    (:'tom_id'::uuid)
)
SELECT count(*)::int AS missing_user_count
FROM required r
LEFT JOIN activity.users u ON u.user_id = r.id
WHERE u.user_id IS NULL
\gset

\if :missing_user_count
  \echo 'ERROR: missing required users in activity.users; check UUIDs in this script.'
  \quit 1
\endif

-- ─────────────────────────────────────────────────────────────────────────────
-- Activity chat (always active): join activity => can chat
-- ─────────────────────────────────────────────────────────────────────────────
INSERT INTO activity.activities (
  activity_id,
  organizer_user_id,
  category_id,
  title,
  description,
  scheduled_at,
  max_participants,
  location_name,
  city,
  language,
  status,
  external_chat_id
)
VALUES (
  :'abbas_activity_id'::uuid,
  :'abbas_id'::uuid,
  NULL,
  'Abbas Chat Test Activity',
  'Test activity for chat permissions (activity chat always active).',
  now() + interval '7 days',
  10,
  'Café De Tor',
  'Enschede',
  'nl',
  'published',
  :'abbas_activity_id'
)
ON CONFLICT (activity_id) DO UPDATE
  SET title = EXCLUDED.title,
      description = EXCLUDED.description,
      updated_at = now();

INSERT INTO activity.participants (activity_id, user_id, role, participation_status)
VALUES
  (:'abbas_activity_id'::uuid, :'abbas_id'::uuid, 'organizer', 'registered'),
  (:'abbas_activity_id'::uuid, :'sam_id'::uuid,   'member',    'registered'),
  (:'abbas_activity_id'::uuid, :'eva_id'::uuid,   'member',    'registered')
ON CONFLICT (activity_id, user_id) DO NOTHING;

-- Mute Eva in activity chat (READ only)
INSERT INTO activity.access_overrides (
  user_id,
  resource_type,
  override_type,
  permission_mask,
  activity_id,
  granted_by,
  expires_at,
  reason
)
VALUES (
  :'eva_id'::uuid,
  'chat',
  'MODIFY',
  1,
  :'abbas_activity_id'::uuid,
  :'abbas_id'::uuid,
  now() + interval '1 hour',
  'dev mute Eva in activity chat'
)
ON CONFLICT (user_id, activity_id) WHERE activity_id IS NOT NULL DO UPDATE
  SET override_type = EXCLUDED.override_type,
      permission_mask = EXCLUDED.permission_mask,
      granted_by = EXCLUDED.granted_by,
      expires_at = EXCLUDED.expires_at,
      reason = EXCLUDED.reason,
      updated_at = now();

-- ─────────────────────────────────────────────────────────────────────────────
-- Private chats for Abbas
-- ─────────────────────────────────────────────────────────────────────────────
-- Pending: Abbas initiates to Jelle
INSERT INTO activity.private_chats (
  private_chat_id,
  user_id_1,
  user_id_2,
  external_chat_id,
  status,
  initiated_by
)
VALUES (
  :'abbas_chat_pending_id'::uuid,
  LEAST(:'abbas_id'::uuid, :'jelle_id'::uuid),
  GREATEST(:'abbas_id'::uuid, :'jelle_id'::uuid),
  'ext_abbas_pending_c2d4f2d6',
  'pending',
  :'abbas_id'::uuid
)
ON CONFLICT (private_chat_id) DO NOTHING;

-- Accepted: Abbas <-> Sam
INSERT INTO activity.private_chats (
  private_chat_id,
  user_id_1,
  user_id_2,
  external_chat_id,
  status,
  initiated_by
)
VALUES (
  :'abbas_chat_accepted_id'::uuid,
  LEAST(:'abbas_id'::uuid, :'sam_id'::uuid),
  GREATEST(:'abbas_id'::uuid, :'sam_id'::uuid),
  'ext_abbas_accepted_d4a9b6d3',
  'accepted',
  :'abbas_id'::uuid
)
ON CONFLICT (private_chat_id) DO NOTHING;

-- Rejected: Abbas -> Eva (simulates unknown user rejected)
INSERT INTO activity.private_chats (
  private_chat_id,
  user_id_1,
  user_id_2,
  external_chat_id,
  status,
  initiated_by
)
VALUES (
  :'abbas_chat_rejected_id'::uuid,
  LEAST(:'abbas_id'::uuid, :'eva_id'::uuid),
  GREATEST(:'abbas_id'::uuid, :'eva_id'::uuid),
  'ext_abbas_rejected_e2f8b4b9',
  'rejected',
  :'abbas_id'::uuid
)
ON CONFLICT (private_chat_id) DO NOTHING;

-- Blocked: Tom blocks Abbas in an accepted chat
INSERT INTO activity.private_chats (
  private_chat_id,
  user_id_1,
  user_id_2,
  external_chat_id,
  status,
  initiated_by
)
VALUES (
  :'abbas_chat_blocked_id'::uuid,
  LEAST(:'abbas_id'::uuid, :'tom_id'::uuid),
  GREATEST(:'abbas_id'::uuid, :'tom_id'::uuid),
  'ext_abbas_blocked_f0b2c9e4',
  'accepted',
  :'abbas_id'::uuid
)
ON CONFLICT (private_chat_id) DO NOTHING;

INSERT INTO activity.user_blocks (blocker_user_id, blocked_user_id, reason)
VALUES (:'tom_id'::uuid, :'abbas_id'::uuid, 'dev block abbas test')
ON CONFLICT (blocker_user_id, blocked_user_id) DO NOTHING;

COMMIT;

-- ─────────────────────────────────────────────────────────────────────────────
-- Verification queries (SEND bit = 2)
-- ─────────────────────────────────────────────────────────────────────────────
SELECT 'abbas_private_pending_initiator_can_send' AS test,
       activity.can_chat(:'abbas_id'::uuid, :'abbas_chat_pending_id'::uuid, 2) AS ok;
SELECT 'abbas_private_pending_receiver_can_send' AS test,
       activity.can_chat(:'jelle_id'::uuid, :'abbas_chat_pending_id'::uuid, 2) AS ok;

SELECT 'abbas_private_accepted_can_send' AS test,
       activity.can_chat(:'abbas_id'::uuid, :'abbas_chat_accepted_id'::uuid, 2) AS ok;
SELECT 'sam_private_accepted_can_send' AS test,
       activity.can_chat(:'sam_id'::uuid, :'abbas_chat_accepted_id'::uuid, 2) AS ok;

SELECT 'abbas_private_rejected_can_send' AS test,
       activity.can_chat(:'abbas_id'::uuid, :'abbas_chat_rejected_id'::uuid, 2) AS ok;
SELECT 'eva_private_rejected_can_send' AS test,
       activity.can_chat(:'eva_id'::uuid, :'abbas_chat_rejected_id'::uuid, 2) AS ok;

SELECT 'abbas_private_blocked_can_send' AS test,
       activity.can_chat(:'abbas_id'::uuid, :'abbas_chat_blocked_id'::uuid, 2) AS ok;
SELECT 'tom_private_blocked_can_send' AS test,
       activity.can_chat(:'tom_id'::uuid, :'abbas_chat_blocked_id'::uuid, 2) AS ok;

SELECT 'abbas_activity_can_send' AS test,
       activity.can_chat(:'abbas_id'::uuid, :'abbas_activity_id'::uuid, 2) AS ok;
SELECT 'sam_activity_can_send' AS test,
       activity.can_chat(:'sam_id'::uuid, :'abbas_activity_id'::uuid, 2) AS ok;
SELECT 'eva_activity_muted_can_send' AS test,
       activity.can_chat(:'eva_id'::uuid, :'abbas_activity_id'::uuid, 2) AS ok;
SELECT 'jelle_not_participant_activity_can_send' AS test,
       activity.can_chat(:'jelle_id'::uuid, :'abbas_activity_id'::uuid, 2) AS ok;

-- Rich context for Abbas
SELECT 'permission_private_pending_abbas' AS test, * FROM activity.get_chat_permission_data(:'abbas_id'::uuid, :'abbas_chat_pending_id'::uuid);
SELECT 'permission_private_pending_jelle' AS test, * FROM activity.get_chat_permission_data(:'jelle_id'::uuid, :'abbas_chat_pending_id'::uuid);
SELECT 'permission_activity_abbas' AS test, * FROM activity.get_chat_permission_data(:'abbas_id'::uuid, :'abbas_activity_id'::uuid);
SELECT 'permission_activity_eva' AS test, * FROM activity.get_chat_permission_data(:'eva_id'::uuid, :'abbas_activity_id'::uuid);
