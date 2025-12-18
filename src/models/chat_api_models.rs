use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WsTicketResponse {
    pub ticket: String,
    pub expires_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversationList {
    pub conversations: Vec<Conversation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Conversation {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message: Option<Box<Message>>,
    #[serde(default)]
    pub unread_count: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversationCreateResult {
    pub id: String,
    pub org_id: String,
    pub created: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageList {
    pub messages: Vec<Message>,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub sender_id: String,
    pub content: String,
    #[serde(default = "default_message_type")]
    pub message_type: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_message_id: Option<String>,
    #[serde(default)]
    pub reactions: HashMap<String, i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll: Option<Poll>,
    #[serde(default)]
    pub is_pinned: bool,
}

fn default_message_type() -> String {
    "text".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Poll {
    pub id: String,
    pub question: String,
    pub options: Vec<PollOption>,
    pub allows_multiple: bool,
    pub is_anonymous: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PollOption {
    pub id: String,
    pub text: String,
    pub vote_count: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReactionCounts {
    #[serde(flatten)]
    pub counts: HashMap<String, i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThreadResponse {
    pub root_message: Message,
    pub replies: Vec<Message>,
    pub reply_count: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScheduledMessage {
    pub id: String,
    pub conversation_id: String,
    pub content: String,
    pub scheduled_for: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnreadCount {
    pub conversation_id: String,
    pub count: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub matches: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatusConfirmation {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
