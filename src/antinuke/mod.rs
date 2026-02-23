pub mod moderation;
pub mod channels;
pub mod roles;
pub mod server;
pub mod assets;

use crate::rest::RestClient;
use crate::db::Database;
use std::sync::Arc;
use serde_json::Value;
use tracing::{warn, error};

pub async fn punish_offender(guild_id: &str, action_type: u8, rest: Arc<RestClient>, db: Arc<Database>) {

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    if let Ok(logs) = rest.get_audit_logs(guild_id, action_type, 1).await {
        if let Some(entry) = logs.as_array().and_then(|arr| arr.first()) {
            let executor_id = entry.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
            if executor_id.is_empty() { return; }

            if db.is_whitelisted(executor_id).await.unwrap_or(false) || db.is_admin(executor_id).await.unwrap_or(false) {
                return;
            }

            if let Ok(bot) = rest.validate_token().await {
                if bot.id == executor_id { return; }
            }

            warn!("PUNISHING: Banning user {} for unauthorized action.", executor_id);
            if let Err(e) = rest.ban_user(guild_id, executor_id, "Rimuru Antinuke: Unauthorized Action").await {
                error!("Failed to punish offender {}: {:?}", executor_id, e);
            }
        }
    }
}

pub async fn handle_event(
    event_type: &str,
    data: Value,
    rest: Arc<RestClient>,
    db: Arc<Database>,
) {
    match event_type {
        "GUILD_BAN_ADD" | "GUILD_BAN_REMOVE" | "GUILD_MEMBER_REMOVE" => {
            moderation::handle_event(event_type, data, rest, db).await;
        }
        "CHANNEL_CREATE" | "CHANNEL_UPDATE" | "CHANNEL_DELETE" | "THREAD_CREATE" => {
            channels::handle_event(event_type, data, rest, db).await;
        }
        "GUILD_ROLE_CREATE" | "GUILD_ROLE_UPDATE" | "GUILD_ROLE_DELETE" | "GUILD_MEMBER_UPDATE" => {
            roles::handle_event(event_type, data, rest, db).await;
        }
        "GUILD_UPDATE" | "MESSAGE_CREATE" => {
            server::handle_event(event_type, data, rest, db).await;
        }
        "GUILD_EMOJIS_UPDATE" | "GUILD_STICKERS_UPDATE" | "WEBHOOKS_UPDATE" => {
            assets::handle_event(event_type, data, rest, db).await;
        }
        _ => {}
    }
}
