use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GatewayPayload {

    pub op: u8,

    pub d: Option<Value>,

    pub s: Option<u64>,

    pub t: Option<String>,
}

pub type Payload = GatewayPayload;

#[allow(dead_code)]
pub mod op {
    pub const DISPATCH: u8 = 0;
    pub const HEARTBEAT: u8 = 1;
    pub const IDENTIFY: u8 = 2;
    pub const RESUME: u8 = 6;
    pub const RECONNECT: u8 = 7;
    pub const INVALID_SESSION: u8 = 9;
    pub const HELLO: u8 = 10;
    pub const HEARTBEAT_ACK: u8 = 11;
}

#[allow(dead_code)]
pub mod intent {
    pub const GUILDS: u32             = 1 << 0;
    pub const GUILD_MESSAGES: u32     = 1 << 9;  
    pub const DIRECT_MESSAGES: u32    = 1 << 12; 
    pub const MESSAGE_CONTENT: u32    = 1 << 15; 
}

#[derive(Debug, Deserialize, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    pub discriminator: Option<String>,
    pub avatar: Option<String>,
    #[serde(default)]
    pub bot: bool,
}

impl User {
    pub fn avatar_url(&self) -> String {
        match &self.avatar {
            Some(hash) => format!("https://cdn.discordapp.com/avatars/{}/{}.png", self.id, hash),
            None => "https://cdn.discordapp.com/embed/avatars/0.png".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Message {
    pub id: String,
    pub guild_id: Option<String>,
    pub channel_id: String,
    pub author: User,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct HelloData {
    pub heartbeat_interval: u64,
}

#[derive(Debug, Deserialize)]
pub struct ReadyData {
    pub session_id: String,
    pub resume_gateway_url: String,
    #[serde(rename = "v")]
    pub version: u8,
    pub user: User,
}

#[derive(Debug, Deserialize)]
pub struct Interaction {
    pub id: String,
    pub application_id: String,
    #[serde(rename = "type")]
    pub kind: u8,
    pub data: Option<InteractionData>,
    pub guild_id: Option<String>,
    pub channel_id: Option<String>,
    pub message: Option<Message>,
    pub member: Option<Member>,
    pub user: Option<User>,
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct Member {
    pub user: Option<User>,
    pub nick: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InteractionData {
    pub custom_id: Option<String>,
    pub component_type: Option<u8>,
    pub values: Option<Vec<String>>,
}

pub mod interaction_type {
    pub const PING: u8 = 1;
    pub const APPLICATION_COMMAND: u8 = 2;
    pub const MESSAGE_COMPONENT: u8 = 3;
}

pub mod component_type {
    pub const ACTION_ROW: u8 = 1;
    pub const BUTTON: u8 = 2;
    pub const STRING_SELECT: u8 = 3;
}
