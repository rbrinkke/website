-- GoAmet Database Schema (DDL)
-- Generated from: sqlite3 goamet.db .schema

CREATE TABLE users (
    user_id TEXT PRIMARY KEY,
    name TEXT,
    profile_description TEXT,
    date_of_birth TEXT,  -- ISO date string
    gender TEXT,
    city TEXT,
    country TEXT,
    postal_code TEXT,
    main_photo_url TEXT,

    -- JSON array of extra photo URLs: ["url1", "url2", ...]
    profile_photos_extra TEXT DEFAULT '[]',

    subscription_level TEXT DEFAULT 'free' CHECK (subscription_level IN ('free', 'club', 'premium')),
    subscription_expires_at TEXT,  -- ISO timestamp

    status TEXT DEFAULT 'active' CHECK (status IN ('active', 'temporary_ban', 'banned', 'pending_onboarding')),

    is_captain INTEGER DEFAULT 0,  -- boolean
    captain_since TEXT,

    -- Counters
    activities_created_count INTEGER DEFAULT 0,
    activities_attended_count INTEGER DEFAULT 0,
    verification_count INTEGER DEFAULT 0,
    no_show_count INTEGER DEFAULT 0,

    last_seen_at TEXT,
    last_login_at TEXT,

    -- Embedded from user_settings as JSON:
    -- {language, timezone, ghost_mode, search_radius_km, profile_visibility,
    --  email_notifications, push_notifications, activity_reminders,
    --  community_updates, friend_requests, marketing_emails, chat_requests}
    settings TEXT DEFAULT '{}',

    -- Embedded from user_interests as JSON array:
    -- [{interest_id, name, emoji, category_name}...]
    interests TEXT DEFAULT '[]',

    -- Embedded from user_badges as JSON array:
    -- [{badge_type, badge_category, title, description, icon_url, earned_at}...]
    badges TEXT DEFAULT '[]',

    -- Embedded from notification_preferences as JSON:
    -- {email_enabled, push_enabled, in_app_enabled, enabled_types[], quiet_hours_start, quiet_hours_end}
    notification_prefs TEXT DEFAULT '{}',

    -- Calculated field (per requesting user via PostGIS)
    distance_km REAL,

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL,  -- ISO timestamp
    is_deleted INTEGER DEFAULT 0,

    created_at TEXT DEFAULT (datetime('now')),
    age INTEGER,
    is_verified INTEGER DEFAULT 0,
    latitude REAL,
    longitude REAL
);
CREATE INDEX idx_users_city ON users(city);
CREATE INDEX idx_users_distance ON users(distance_km);
CREATE INDEX idx_users_changed_at ON users(changed_at);
CREATE INDEX idx_users_subscription ON users(subscription_level);
CREATE INDEX idx_users_lat_lon ON users(latitude, longitude);

CREATE TABLE activities (
    activity_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,

    activity_type TEXT DEFAULT 'standard' CHECK (activity_type IN ('standard', 'xxl', 'womens_only', 'mens_only')),
    privacy_level TEXT DEFAULT 'public' CHECK (privacy_level IN ('public', 'friends_only', 'invite_only')),
    status TEXT DEFAULT 'published' CHECK (status IN ('draft', 'published', 'cancelled', 'completed')),

    scheduled_at TEXT NOT NULL,  -- ISO timestamp
    duration_minutes INTEGER,
    joinable_at_free TEXT,  -- ISO timestamp (when free users can join)

    max_participants INTEGER NOT NULL,
    current_participants_count INTEGER DEFAULT 0,
    waitlist_count INTEGER DEFAULT 0,

    language TEXT DEFAULT 'en',
    city TEXT,

    -- Distance from current user (calculated, for sorting/filtering)
    distance_km REAL,

    -- Embedded from activity_locations as JSON:
    -- {venue_name, address_line1, address_line2, city, state_province,
    --  postal_code, country, latitude, longitude, place_id}
    location TEXT DEFAULT '{}',
    latitude REAL,
    longitude REAL,

    -- Embedded organizer info as JSON:
    -- {user_id, name, photo_url, is_captain, subscription_level}
    organizer TEXT NOT NULL DEFAULT '{}',

    -- Primary organizer (flattened for feed rendering)
    primary_organizer_user_id TEXT,
    primary_organizer_name TEXT,
    primary_organizer_photo_asset_id TEXT,

	    -- Embedded participants as JSON array:
	    -- [{user_id, name, photo_url, role, participation_status, attendance_status, joined_at}...]
	    -- Embedded from activity_tags as JSON array: ["tag1", "tag2", ...]
	    tags TEXT DEFAULT '[]',

    -- Embedded category info as JSON:
    -- {category_id, name, slug, icon_url}
    category TEXT DEFAULT '{}',

    -- My personal status for this activity (per requesting user)
    my_role TEXT,  -- 'organizer', 'co_organizer', 'member', null
    my_participation_status TEXT,  -- 'registered', 'waitlisted', 'declined', 'cancelled', null
    my_attendance_status TEXT,  -- 'registered', 'attended', 'no_show', null
    am_on_waitlist INTEGER DEFAULT 0,
    my_waitlist_position INTEGER,

    -- Review stats (calculated by server, embedded here)
    review_count INTEGER DEFAULT 0,
    avg_rating REAL,  -- 1.0 to 5.0

    -- Timestamps
    created_at TEXT,
    updated_at TEXT,
    completed_at TEXT,
    cancelled_at TEXT,

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    is_deleted INTEGER DEFAULT 0,

    main_photo_asset_id TEXT,
    is_joined INTEGER DEFAULT 0,
    can_manage_activity INTEGER NOT NULL DEFAULT 0,
    can_manage_attendance INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_activities_scheduled ON activities(scheduled_at);
CREATE INDEX idx_activities_status ON activities(status);
CREATE INDEX idx_activities_city ON activities(city);
CREATE INDEX idx_activities_distance ON activities(distance_km);
CREATE INDEX idx_activities_changed_at ON activities(changed_at);
CREATE INDEX idx_activities_organizer ON activities(json_extract(organizer, '$.user_id'));
CREATE INDEX idx_activities_lat_lon ON activities(latitude, longitude);

CREATE TABLE activity_participants (
    activity_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    name TEXT,
    photo_url TEXT,
    role TEXT,
    participation_status TEXT,
    attendance_status TEXT,
    joined_at TEXT,
    updated_at TEXT DEFAULT (datetime('now')),
    is_deleted INTEGER DEFAULT 0,
    PRIMARY KEY (activity_id, user_id)
);
CREATE INDEX idx_activity_participants_activity ON activity_participants(activity_id) WHERE is_deleted = 0;
CREATE INDEX idx_activity_participants_user ON activity_participants(user_id) WHERE is_deleted = 0;
CREATE INDEX idx_activity_participants_status ON activity_participants(activity_id, participation_status, joined_at) WHERE is_deleted = 0;

-- Write-path (transactional): activity signup commands (join/leave).
-- Requires a Rust-registered SQLite UDF:
--   sp_apply_activity_signup_command(command_id TEXT) -> INTEGER
CREATE TABLE activity_signup_commands (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    actor_user_id TEXT NOT NULL,
    activity_id TEXT NOT NULL,
    subject_user_id TEXT NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('join', 'leave')),
    note TEXT
);
CREATE INDEX idx_activity_signup_commands_activity_created ON activity_signup_commands(activity_id, created_at);
CREATE INDEX idx_activity_signup_commands_subject_created ON activity_signup_commands(subject_user_id, created_at);
CREATE TRIGGER trg_activity_signup_commands_apply
AFTER INSERT ON activity_signup_commands
BEGIN
    SELECT
        CASE
            WHEN sp_apply_activity_signup_command(NEW.id) = 1 THEN 1
            ELSE RAISE(ROLLBACK, 'sp_apply_activity_signup_command failed')
        END;
END;

-- Write-path (transactional): activity waitlist commands (owner/mod control).
-- Requires a Rust-registered SQLite UDF:
--   sp_apply_activity_waitlist_command(command_id TEXT) -> INTEGER
CREATE TABLE activity_waitlist_commands (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    actor_user_id TEXT NOT NULL,
    activity_id TEXT NOT NULL,
    subject_user_id TEXT NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('set_waitlisted', 'remove_waitlist', 'set_priority')),
    priority INTEGER,
    note TEXT,
    CHECK (action != 'set_priority' OR priority IS NOT NULL)
);
CREATE INDEX idx_activity_waitlist_commands_activity_created ON activity_waitlist_commands(activity_id, created_at);
CREATE INDEX idx_activity_waitlist_commands_subject_created ON activity_waitlist_commands(subject_user_id, created_at);
CREATE TRIGGER trg_activity_waitlist_commands_apply
AFTER INSERT ON activity_waitlist_commands
BEGIN
    SELECT
        CASE
            WHEN sp_apply_activity_waitlist_command(NEW.id) = 1 THEN 1
            ELSE RAISE(ROLLBACK, 'sp_apply_activity_waitlist_command failed')
        END;
END;

CREATE TABLE communities (
    community_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    description TEXT,

    community_type TEXT DEFAULT 'open' CHECK (community_type IN ('open', 'closed', 'secret')),
    status TEXT DEFAULT 'active' CHECK (status IN ('active', 'archived', 'suspended')),

    member_count INTEGER DEFAULT 0,
    max_members INTEGER,
    is_featured INTEGER DEFAULT 0,

    cover_image_url TEXT,
    icon_url TEXT,

    -- Embedded creator as JSON:
    -- {user_id, name, photo_url}
    creator TEXT DEFAULT '{}',

    -- Embedded tags as JSON array: ["tag1", "tag2", ...]
    tags TEXT DEFAULT '[]',

    -- My membership info (per requesting user)
    my_role TEXT,  -- 'organizer', 'co_organizer', 'member', null
    my_status TEXT,  -- 'pending', 'active', 'banned', 'left', null
    my_joined_at TEXT,

    -- Timestamps
    created_at TEXT,
    updated_at TEXT,

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    is_deleted INTEGER DEFAULT 0
);
CREATE INDEX idx_communities_slug ON communities(slug);
CREATE INDEX idx_communities_type ON communities(community_type);
CREATE INDEX idx_communities_changed_at ON communities(changed_at);

CREATE TABLE conversations (
    conversation_id TEXT PRIMARY KEY,

    conversation_type TEXT NOT NULL CHECK (conversation_type IN ('private', 'activity')),
    status TEXT DEFAULT 'accepted' CHECK (status IN ('pending', 'accepted', 'rejected')),

    -- For private chats - the other user as JSON:
    -- {user_id, name, photo_url, is_captain}
    other_user TEXT,

    -- For activity chats - the activity reference as JSON:
    -- {activity_id, title, scheduled_at}
    activity TEXT,

    -- External reference (e.g., MongoDB chat_id)
    external_chat_id TEXT,

    last_message_at TEXT,
    last_message_preview TEXT,
    last_sender_id TEXT,

    -- Per-user state
    unread_count INTEGER DEFAULT 0,
    initiated_by_me INTEGER DEFAULT 0,
    is_muted INTEGER DEFAULT 0,

    -- Timestamps
    created_at TEXT,

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    is_deleted INTEGER DEFAULT 0
);
CREATE INDEX idx_conversations_type ON conversations(conversation_type);
CREATE INDEX idx_conversations_last_message ON conversations(last_message_at DESC);
CREATE INDEX idx_conversations_changed_at ON conversations(changed_at);

CREATE TABLE messages (
    message_id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,

    -- Embedded sender as JSON:
    -- {user_id, name, photo_url}
    sender TEXT NOT NULL DEFAULT '{}',

    content TEXT NOT NULL,
    message_type TEXT DEFAULT 'text' CHECK (message_type IN ('text', 'image', 'system', 'activity_update')),

    -- Convenience flag
    is_mine INTEGER DEFAULT 0,

    -- Timestamps
    created_at TEXT NOT NULL,
    edited_at TEXT,

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    is_deleted INTEGER DEFAULT 0,

    FOREIGN KEY (conversation_id) REFERENCES conversations(conversation_id) ON DELETE CASCADE
);
CREATE INDEX idx_messages_conversation ON messages(conversation_id, created_at DESC);
CREATE INDEX idx_messages_changed_at ON messages(changed_at);

CREATE TABLE friends (
    friendship_id TEXT PRIMARY KEY,

    -- Embedded friend profile as JSON:
    -- {user_id, name, photo_url, city, is_captain, subscription_level, last_seen_at}
    friend TEXT NOT NULL DEFAULT '{}',

    status TEXT DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'blocked')),
    initiated_by_me INTEGER DEFAULT 0,

    -- Timestamps
    created_at TEXT,
    accepted_at TEXT,

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    is_deleted INTEGER DEFAULT 0
);
CREATE INDEX idx_friends_status ON friends(status);
CREATE INDEX idx_friends_changed_at ON friends(changed_at);

CREATE TABLE notifications (
    notification_id TEXT PRIMARY KEY,

    notification_type TEXT NOT NULL,
    -- Types: activity_invite, activity_reminder, activity_update, community_invite,
    -- new_member, new_post, comment, reaction, mention, profile_view,
    -- new_favorite, system, friend_request, community_announcement, friend_accepted, chat_request

    title TEXT NOT NULL,
    message TEXT,

    status TEXT DEFAULT 'unread' CHECK (status IN ('unread', 'read', 'archived')),
    priority TEXT DEFAULT 'normal' CHECK (priority IN ('low', 'normal', 'high', 'critical')),

    -- Embedded actor (who triggered notification) as JSON:
    -- {user_id, name, photo_url}
    actor TEXT,

    -- Target reference
    target_type TEXT,  -- 'activity', 'community', 'post', 'user', 'chat', etc.
    target_id TEXT,

    -- Rich notification fields
    deep_link TEXT,
    media_url TEXT,
    media_type TEXT,  -- 'image', 'video', 'avatar'

    -- Action buttons as JSON array:
    -- [{id, label, url, method, style, confirm}...]
    actions TEXT DEFAULT '[]',

    -- Grouping
    collapse_key TEXT,
    group_id TEXT,
    grouped_count INTEGER DEFAULT 1,
    is_group_head INTEGER DEFAULT 1,

    -- Timestamps
    created_at TEXT NOT NULL,
    read_at TEXT,
    expires_at TEXT,

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    is_deleted INTEGER DEFAULT 0
);
CREATE INDEX idx_notifications_status ON notifications(status);
CREATE INDEX idx_notifications_type ON notifications(notification_type);
CREATE INDEX idx_notifications_created ON notifications(created_at DESC);
CREATE INDEX idx_notifications_changed_at ON notifications(changed_at);

CREATE TABLE favorites (
    favorite_id TEXT PRIMARY KEY,

    -- Embedded user profile as JSON:
    -- {user_id, name, photo_url, city, is_captain, last_seen_at}
    user TEXT NOT NULL DEFAULT '{}',

    created_at TEXT,

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    is_deleted INTEGER DEFAULT 0
);
CREATE INDEX idx_favorites_changed_at ON favorites(changed_at);

CREATE TABLE blocks (
    block_id TEXT PRIMARY KEY,
    blocked_user_id TEXT NOT NULL,
    reason TEXT,
    created_at TEXT,

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    is_deleted INTEGER DEFAULT 0
);
CREATE INDEX idx_blocks_user ON blocks(blocked_user_id);
CREATE INDEX idx_blocks_changed_at ON blocks(changed_at);

CREATE TABLE mutes (
    mute_id TEXT PRIMARY KEY,
    muted_user_id TEXT NOT NULL,
    reason TEXT,
    created_at TEXT,

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    is_deleted INTEGER DEFAULT 0
);
CREATE INDEX idx_mutes_user ON mutes(muted_user_id);
CREATE INDEX idx_mutes_changed_at ON mutes(changed_at);

CREATE TABLE categories (
    category_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT,
    icon_url TEXT,
    display_order INTEGER DEFAULT 0,
    is_active INTEGER DEFAULT 1,

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    is_deleted INTEGER DEFAULT 0
);
CREATE INDEX idx_categories_slug ON categories(slug);
CREATE INDEX idx_categories_order ON categories(display_order);

CREATE TABLE interests (
    interest_id TEXT PRIMARY KEY,
    category_id TEXT NOT NULL,
    page_id TEXT NOT NULL,

    name TEXT NOT NULL,
    emoji TEXT,
    sort_order INTEGER DEFAULT 0,
    is_active INTEGER DEFAULT 1,

    -- Denormalized category info
    category_name TEXT,
    category_theme_color TEXT,

    -- Denormalized page info
    page_slug TEXT,
    page_title TEXT,

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    is_deleted INTEGER DEFAULT 0
);
CREATE INDEX idx_interests_category ON interests(category_id);
CREATE INDEX idx_interests_page ON interests(page_id);
CREATE INDEX idx_interests_active ON interests(is_active);

CREATE TABLE interests_catalog (
    id INTEGER PRIMARY KEY DEFAULT 1,  -- Single row

    -- Complete onboarding structure as JSON:
    -- {totalPages, pages: [{id, slug, title, subtitle, maxSelections, sortOrder,
    --   categories: [{id, name, subtitle, themeColor, sortOrder,
    --     interests: [{id, name, emoji, sortOrder}...]}...]}...]}
    onboarding_data TEXT NOT NULL DEFAULT '{}',

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL
);

CREATE TABLE activity_invitations (
    invitation_id TEXT PRIMARY KEY,
    activity_id TEXT NOT NULL,

    -- Embedded activity summary as JSON:
    -- {title, scheduled_at, city, organizer_name}
    activity_summary TEXT DEFAULT '{}',

    -- Embedded inviter as JSON:
    -- {user_id, name, photo_url}
    invited_by TEXT DEFAULT '{}',

    status TEXT DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'declined', 'expired')),
    message TEXT,

    invited_at TEXT,
    responded_at TEXT,
    expires_at TEXT,

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    is_deleted INTEGER DEFAULT 0,

    FOREIGN KEY (activity_id) REFERENCES activities(activity_id) ON DELETE CASCADE
);
CREATE INDEX idx_invitations_status ON activity_invitations(status);
CREATE INDEX idx_invitations_activity ON activity_invitations(activity_id);
CREATE INDEX idx_invitations_changed_at ON activity_invitations(changed_at);

CREATE TABLE community_posts (
    post_id TEXT PRIMARY KEY,
    community_id TEXT NOT NULL,

    -- Embedded author as JSON:
    -- {user_id, name, photo_url}
    author TEXT DEFAULT '{}',

    -- Optional linked activity
    activity_id TEXT,

    title TEXT,
    content TEXT NOT NULL,
    content_type TEXT DEFAULT 'post' CHECK (content_type IN ('post', 'photo', 'video', 'poll', 'event_announcement')),
    status TEXT DEFAULT 'published',

    -- Counters
    view_count INTEGER DEFAULT 0,
    comment_count INTEGER DEFAULT 0,
    reaction_count INTEGER DEFAULT 0,

    is_pinned INTEGER DEFAULT 0,

    -- My reaction (per requesting user)
    my_reaction TEXT,  -- 'like', 'love', 'celebrate', 'support', 'insightful', null

    -- Timestamps
    created_at TEXT,
    updated_at TEXT,
    published_at TEXT,

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    is_deleted INTEGER DEFAULT 0,

    FOREIGN KEY (community_id) REFERENCES communities(community_id) ON DELETE CASCADE
);
CREATE INDEX idx_posts_community ON community_posts(community_id, created_at DESC);
CREATE INDEX idx_posts_pinned ON community_posts(community_id, is_pinned DESC, created_at DESC);
CREATE INDEX idx_posts_changed_at ON community_posts(changed_at);

CREATE TABLE profile_views (
    view_id TEXT PRIMARY KEY,

    -- Embedded viewer as JSON:
    -- {user_id, name, photo_url, city, is_captain, subscription_level}
    viewer TEXT NOT NULL DEFAULT '{}',

    -- Timestamp when viewed
    viewed_at TEXT NOT NULL,

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    is_deleted INTEGER DEFAULT 0
);
CREATE INDEX idx_profile_views_date ON profile_views(viewed_at DESC);
CREATE INDEX idx_profile_views_changed_at ON profile_views(changed_at);

CREATE TABLE activity_reviews (
    review_id TEXT PRIMARY KEY,
    activity_id TEXT NOT NULL,

    -- Embedded reviewer as JSON:
    -- {user_id, name, photo_url, is_captain}
    reviewer TEXT NOT NULL DEFAULT '{}',

    -- Review data
    rating INTEGER NOT NULL CHECK (rating >= 1 AND rating <= 5),
    review_text TEXT,

    -- Helpful votes
    helpful_count INTEGER DEFAULT 0,

    -- Convenience flag (is this my review?)
    is_mine INTEGER DEFAULT 0,

    -- Timestamps
    created_at TEXT NOT NULL,
    updated_at TEXT,

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    is_deleted INTEGER DEFAULT 0,

    FOREIGN KEY (activity_id) REFERENCES activities(activity_id) ON DELETE CASCADE
);
CREATE INDEX idx_reviews_activity ON activity_reviews(activity_id, created_at DESC);
CREATE INDEX idx_reviews_rating ON activity_reviews(rating);
CREATE INDEX idx_reviews_changed_at ON activity_reviews(changed_at);

CREATE TABLE comments (
    comment_id TEXT PRIMARY KEY,
    post_id TEXT NOT NULL,
    parent_comment_id TEXT,  -- For nested/threaded replies

    -- Embedded author as JSON:
    -- {user_id, name, photo_url, is_captain}
    author TEXT NOT NULL DEFAULT '{}',

    content TEXT NOT NULL,

    -- Counters
    reply_count INTEGER DEFAULT 0,
    reaction_count INTEGER DEFAULT 0,

    -- Per-user state
    my_reaction TEXT,  -- 'like', 'love', etc. or null
    is_mine INTEGER DEFAULT 0,

    -- Timestamps
    created_at TEXT NOT NULL,
    updated_at TEXT,

    -- Sync tracking
    row_hash TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    is_deleted INTEGER DEFAULT 0,

    FOREIGN KEY (post_id) REFERENCES community_posts(post_id) ON DELETE CASCADE,
    FOREIGN KEY (parent_comment_id) REFERENCES comments(comment_id) ON DELETE CASCADE
);
CREATE INDEX idx_comments_post ON comments(post_id, created_at ASC);
CREATE INDEX idx_comments_parent ON comments(parent_comment_id);
CREATE INDEX idx_comments_changed_at ON comments(changed_at);

CREATE TABLE sync_state (
    entity_type TEXT PRIMARY KEY,
    -- Entity types: users, activities, communities, conversations, messages,
    -- friends, notifications, favorites, blocks, mutes, categories, interests,
    -- activity_invitations, community_posts, profile_views, activity_reviews, comments

    last_sync_at TEXT NOT NULL DEFAULT '1970-01-01T00:00:00Z',  -- ISO timestamp
    last_range_hash TEXT,  -- For integrity verification
    record_count INTEGER DEFAULT 0,

    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TRIGGER trg_sync_state_updated
AFTER UPDATE ON sync_state
BEGIN
    UPDATE sync_state SET updated_at = datetime('now') WHERE entity_type = NEW.entity_type;
END;

CREATE TABLE current_user (
    id INTEGER PRIMARY KEY DEFAULT 1,
    user_id TEXT NOT NULL,

    -- Full user data as JSON (same structure as users table but all in one)
    user_data TEXT NOT NULL DEFAULT '{}',

    -- Auth tokens (encrypted by SQLCipher)
    access_token TEXT,
    refresh_token TEXT,
    token_expires_at TEXT,

    -- Device info
    device_id TEXT,
    fcm_token TEXT,

    -- Location (user's own location, not synced to server for privacy)
    latitude REAL,
    longitude REAL,
    location_updated_at TEXT,

    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE user_preferences (
    user_id TEXT NOT NULL PRIMARY KEY,
    search_radius INTEGER DEFAULT 25 NOT NULL,
    search_postcode TEXT,
    search_latitude REAL,
    search_longitude REAL,
    filter_min_age INTEGER,
    filter_max_age INTEGER,
    filter_gender TEXT,
    is_ghost_mode INTEGER DEFAULT 0 NOT NULL,
    notify_messages INTEGER DEFAULT 1 NOT NULL,
    notify_activities INTEGER DEFAULT 1 NOT NULL,
    notify_marketing INTEGER DEFAULT 0 NOT NULL,
    row_hash TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE user_profiles (
    user_id TEXT NOT NULL PRIMARY KEY,
    search_radius INTEGER DEFAULT 25 NOT NULL,
    postcode TEXT,
    latitude REAL,
    longitude REAL,
    filter_min_age INTEGER,
    filter_max_age INTEGER,
    filter_gender TEXT,
    subscription_plan TEXT DEFAULT 'free' NOT NULL,
    subscription_expires_at TEXT,
    is_auto_renew INTEGER DEFAULT 0 NOT NULL,
    is_ghost_mode INTEGER DEFAULT 0 NOT NULL,
    notify_messages INTEGER DEFAULT 1 NOT NULL,
    notify_activities INTEGER DEFAULT 1 NOT NULL,
    notify_marketing INTEGER DEFAULT 0 NOT NULL,
    row_hash TEXT,
    updated_at TEXT NOT NULL
);

-- GEO Indexing
CREATE VIRTUAL TABLE users_geo_index USING rtree(
   user_rowid,
   min_lat, max_lat,
   min_lng, max_lng
);

CREATE TRIGGER users_geo_insert AFTER INSERT ON users
BEGIN
  INSERT INTO users_geo_index (user_rowid, min_lat, max_lat, min_lng, max_lng)
  VALUES (NEW.rowid, NEW.latitude, NEW.latitude, NEW.longitude, NEW.longitude);
END;

CREATE TRIGGER users_geo_update AFTER UPDATE OF latitude, longitude ON users
BEGIN
  UPDATE users_geo_index SET
    min_lat = NEW.latitude, max_lat = NEW.latitude,
    min_lng = NEW.longitude, max_lng = NEW.longitude
  WHERE user_rowid = OLD.rowid;
END;

CREATE TRIGGER users_geo_delete AFTER DELETE ON users
BEGIN
  DELETE FROM users_geo_index WHERE user_rowid = OLD.rowid;
END;

-- VIEWS
CREATE VIEW v_unread_notification_count AS
SELECT COUNT(*) as count
FROM notifications
WHERE status = 'unread' AND is_deleted = 0;

CREATE VIEW v_pending_friend_requests AS
SELECT * FROM friends
WHERE status = 'pending' AND initiated_by_me = 0 AND is_deleted = 0;

CREATE VIEW v_upcoming_activities AS
SELECT * FROM activities
WHERE status = 'published'
  AND is_deleted = 0
  AND datetime(scheduled_at) > datetime('now')
  AND datetime(scheduled_at) < datetime('now', '+7 days')
ORDER BY scheduled_at ASC;

CREATE VIEW v_my_activities AS
SELECT * FROM activities
WHERE (my_role IS NOT NULL OR my_participation_status IS NOT NULL)
  AND is_deleted = 0
ORDER BY scheduled_at ASC;

CREATE VIEW v_active_conversations AS
SELECT c.*,
       json_extract(c.other_user, '$.name') as other_user_name,
       json_extract(c.activity, '$.title') as activity_title
FROM conversations c
WHERE c.is_deleted = 0 AND c.status = 'accepted'
ORDER BY c.last_message_at DESC;

CREATE VIEW v_blocked_user_ids AS
SELECT blocked_user_id FROM blocks WHERE is_deleted = 0;

CREATE VIEW v_recent_profile_views AS
SELECT pv.*,
       json_extract(pv.viewer, '$.name') as viewer_name,
       json_extract(pv.viewer, '$.photo_url') as viewer_photo
FROM profile_views pv
WHERE pv.is_deleted = 0
  AND datetime(pv.viewed_at) > datetime('now', '-30 days')
ORDER BY pv.viewed_at DESC;

CREATE VIEW v_activity_ratings AS
SELECT activity_id,
       COUNT(*) as review_count,
       ROUND(AVG(rating), 1) as avg_rating
FROM activity_reviews
WHERE is_deleted = 0
GROUP BY activity_id;

CREATE VIEW v_top_level_comments AS
SELECT c.*,
       json_extract(c.author, '$.name') as author_name,
       json_extract(c.author, '$.photo_url') as author_photo
FROM comments c
WHERE c.parent_comment_id IS NULL AND c.is_deleted = 0
ORDER BY c.created_at ASC;
