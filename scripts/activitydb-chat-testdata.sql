-- Chat permissions test data for activitydb (NO changes to Abbas).
-- Idempotent: uses fixed UUIDs + ON CONFLICT guards where possible.
--
-- Usage:
--   PGPASSWORD=postgres_secure_password_change_in_prod \
--   psql -h localhost -p 5441 -U postgres -d activitydb -f scripts/activitydb-chat-testdata.sql
--
-- What it creates:
-- - 4 test users (Alice/Bob/Carol/Dave)
-- - 1 activity chat membership (Alice organizer; Bob+Carol participants; Bob muted)
-- - 4 private chats:
--     - pending  (Alice -> Dave)
--     - accepted (Alice <-> Bob)
--     - rejected (Bob -> Dave)
--     - blocked  (Carol <-> Dave + Carol blocks Dave)
-- - Verification queries for can_chat() + get_chat_permission_data()

\set ON_ERROR_STOP on

BEGIN;

-- ─────────────────────────────────────────────────────────────────────────────
-- Fixed IDs (safe to share and re-run)
-- ─────────────────────────────────────────────────────────────────────────────
\set alice_id   'cfe8d74f-752e-448f-831a-cb6581e41732'
\set bob_id     'f49e8303-2f96-4fe1-a4aa-7901ce72b4ae'
\set carol_id   'da709fa1-d574-429d-85e1-c72e244d55ca'
\set dave_id    'fd25b9a7-e57b-4b7a-ace3-70f133ee79b9'

\set activity_id      '965b1228-7b33-41e4-ae27-3f468ccc744b'
\set chat_pending_id  'b6cd994b-a2a1-4d66-b46c-d5784525ad56'
\set chat_accepted_id 'b5ecd66c-96b3-4e43-a731-271a3962a4b4'
\set chat_rejected_id 'baa49d20-1c23-487d-84fb-430893025637'
\set chat_blocked_id  '1cd31359-1c3e-42da-90ed-e4410f39a7ca'

-- ─────────────────────────────────────────────────────────────────────────────
-- Users
-- ─────────────────────────────────────────────────────────────────────────────
INSERT INTO activity.users (user_id, name, city, country, status, payload)
VALUES
  (:'alice_id'::uuid, 'Chat Test Alice', 'Enschede', 'NL', 'active', jsonb_build_object('dev_tag','chat_test_2025_12_17')),
  (:'bob_id'::uuid,   'Chat Test Bob',   'Enschede', 'NL', 'active', jsonb_build_object('dev_tag','chat_test_2025_12_17')),
  (:'carol_id'::uuid, 'Chat Test Carol', 'Enschede', 'NL', 'active', jsonb_build_object('dev_tag','chat_test_2025_12_17')),
  (:'dave_id'::uuid,  'Chat Test Dave',  'Enschede', 'NL', 'active', jsonb_build_object('dev_tag','chat_test_2025_12_17'))
ON CONFLICT (user_id) DO UPDATE
  SET name = EXCLUDED.name,
      city = EXCLUDED.city,
      country = EXCLUDED.country,
      status = EXCLUDED.status,
      payload = EXCLUDED.payload,
      updated_at = now();

-- ─────────────────────────────────────────────────────────────────────────────
-- Activity chat: join activity => can chat
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
  :'activity_id'::uuid,
  :'alice_id'::uuid,
  NULL,
  'Chat Test Activity (Permissions)',
  'Test activity to validate chat permissions (participants can chat; non-participants cannot).',
  now() + interval '10 days',
  12,
  'Café De Test',
  'Enschede',
  'nl',
  'published',
  :'activity_id'
)
ON CONFLICT (activity_id) DO UPDATE
  SET title = EXCLUDED.title,
      description = EXCLUDED.description,
      updated_at = now();

-- Participants (Alice organizer; Bob+Carol members)
INSERT INTO activity.participants (activity_id, user_id, role, participation_status)
VALUES
  (:'activity_id'::uuid, :'alice_id'::uuid, 'organizer', 'registered'),
  (:'activity_id'::uuid, :'bob_id'::uuid,   'member',    'registered'),
  (:'activity_id'::uuid, :'carol_id'::uuid, 'member',    'registered')
ON CONFLICT (activity_id, user_id) DO NOTHING;

-- Mute Bob in the activity chat (READ only)
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
  :'bob_id'::uuid,
  'chat',
  'MODIFY',
  1,
  :'activity_id'::uuid,
  :'alice_id'::uuid,
  now() + interval '2 hours',
  'dev mute test (read-only)'
)
ON CONFLICT (user_id, activity_id) WHERE activity_id IS NOT NULL DO UPDATE
  SET override_type = EXCLUDED.override_type,
      permission_mask = EXCLUDED.permission_mask,
      granted_by = EXCLUDED.granted_by,
      expires_at = EXCLUDED.expires_at,
      reason = EXCLUDED.reason,
      updated_at = now();

-- ─────────────────────────────────────────────────────────────────────────────
-- Private chats (pending/accepted/rejected/blocked)
-- ─────────────────────────────────────────────────────────────────────────────
-- Pending: Alice initiates to Dave
INSERT INTO activity.private_chats (
  private_chat_id,
  user_id_1,
  user_id_2,
  external_chat_id,
  status,
  initiated_by
)
VALUES (
  :'chat_pending_id'::uuid,
  LEAST(:'alice_id'::uuid, :'dave_id'::uuid),
  GREATEST(:'alice_id'::uuid, :'dave_id'::uuid),
  'ext_chat_pending_b6cd994b',
  'pending',
  :'alice_id'::uuid
)
ON CONFLICT (private_chat_id) DO NOTHING;

-- Accepted: Alice <-> Bob
INSERT INTO activity.private_chats (
  private_chat_id,
  user_id_1,
  user_id_2,
  external_chat_id,
  status,
  initiated_by
)
VALUES (
  :'chat_accepted_id'::uuid,
  LEAST(:'alice_id'::uuid, :'bob_id'::uuid),
  GREATEST(:'alice_id'::uuid, :'bob_id'::uuid),
  'ext_chat_accepted_b5ecd66c',
  'accepted',
  :'alice_id'::uuid
)
ON CONFLICT (private_chat_id) DO NOTHING;

-- Rejected: Bob initiates to Dave, but rejected
INSERT INTO activity.private_chats (
  private_chat_id,
  user_id_1,
  user_id_2,
  external_chat_id,
  status,
  initiated_by
)
VALUES (
  :'chat_rejected_id'::uuid,
  LEAST(:'bob_id'::uuid, :'dave_id'::uuid),
  GREATEST(:'bob_id'::uuid, :'dave_id'::uuid),
  'ext_chat_rejected_baa49d20',
  'rejected',
  :'bob_id'::uuid
)
ON CONFLICT (private_chat_id) DO NOTHING;

-- Blocked: Carol <-> Dave accepted, but Carol blocks Dave (can_chat should return false)
INSERT INTO activity.private_chats (
  private_chat_id,
  user_id_1,
  user_id_2,
  external_chat_id,
  status,
  initiated_by
)
VALUES (
  :'chat_blocked_id'::uuid,
  LEAST(:'carol_id'::uuid, :'dave_id'::uuid),
  GREATEST(:'carol_id'::uuid, :'dave_id'::uuid),
  'ext_chat_blocked_1cd31359',
  'accepted',
  :'carol_id'::uuid
)
ON CONFLICT (private_chat_id) DO NOTHING;

INSERT INTO activity.user_blocks (blocker_user_id, blocked_user_id, reason)
VALUES (:'carol_id'::uuid, :'dave_id'::uuid, 'dev block test')
ON CONFLICT (blocker_user_id, blocked_user_id) DO NOTHING;

COMMIT;

-- ─────────────────────────────────────────────────────────────────────────────
-- Verification: can_chat (READ=1, SEND=2, EDIT_OWN=4, DELETE_OWN=8)
-- ─────────────────────────────────────────────────────────────────────────────
SELECT 'private_pending_initiator_can_send' AS test,
       activity.can_chat(:'alice_id'::uuid, :'chat_pending_id'::uuid, 2) AS ok;
SELECT 'private_pending_receiver_can_send' AS test,
       activity.can_chat(:'dave_id'::uuid, :'chat_pending_id'::uuid, 2) AS ok;

SELECT 'private_accepted_alice_can_send' AS test,
       activity.can_chat(:'alice_id'::uuid, :'chat_accepted_id'::uuid, 2) AS ok;
SELECT 'private_accepted_bob_can_send' AS test,
       activity.can_chat(:'bob_id'::uuid, :'chat_accepted_id'::uuid, 2) AS ok;

SELECT 'private_rejected_bob_can_send' AS test,
       activity.can_chat(:'bob_id'::uuid, :'chat_rejected_id'::uuid, 2) AS ok;
SELECT 'private_rejected_dave_can_send' AS test,
       activity.can_chat(:'dave_id'::uuid, :'chat_rejected_id'::uuid, 2) AS ok;

SELECT 'private_blocked_carol_can_send' AS test,
       activity.can_chat(:'carol_id'::uuid, :'chat_blocked_id'::uuid, 2) AS ok;
SELECT 'private_blocked_dave_can_send' AS test,
       activity.can_chat(:'dave_id'::uuid, :'chat_blocked_id'::uuid, 2) AS ok;

SELECT 'activity_chat_alice_can_send' AS test,
       activity.can_chat(:'alice_id'::uuid, :'activity_id'::uuid, 2) AS ok;
SELECT 'activity_chat_bob_muted_can_send' AS test,
       activity.can_chat(:'bob_id'::uuid, :'activity_id'::uuid, 2) AS ok;
SELECT 'activity_chat_carol_can_send' AS test,
       activity.can_chat(:'carol_id'::uuid, :'activity_id'::uuid, 2) AS ok;
SELECT 'activity_chat_dave_not_participant_can_send' AS test,
       activity.can_chat(:'dave_id'::uuid, :'activity_id'::uuid, 2) AS ok;

-- Rich context output
SELECT 'permission_data_activity_alice' AS test, * FROM activity.get_chat_permission_data(:'alice_id'::uuid, :'activity_id'::uuid);
SELECT 'permission_data_activity_bob'   AS test, * FROM activity.get_chat_permission_data(:'bob_id'::uuid, :'activity_id'::uuid);
SELECT 'permission_data_activity_dave'  AS test, * FROM activity.get_chat_permission_data(:'dave_id'::uuid, :'activity_id'::uuid);

SELECT 'permission_data_private_pending_alice' AS test, * FROM activity.get_chat_permission_data(:'alice_id'::uuid, :'chat_pending_id'::uuid);
SELECT 'permission_data_private_pending_dave'  AS test, * FROM activity.get_chat_permission_data(:'dave_id'::uuid, :'chat_pending_id'::uuid);
