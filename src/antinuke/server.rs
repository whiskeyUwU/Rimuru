use crate::rest::RestClient;
use crate::db::Database;
use std::sync::Arc;
use serde_json::Value;
use tracing::{warn, error};

pub async fn handle_event(
    event_type: &str,
    data: Value,
    _rest: Arc<RestClient>,
    db: Arc<Database>,
) {
    let guild_id = data.get("guild_id").and_then(|v| v.as_str()).unwrap_or("");
    let settings = match db.get_antinuke_settings(guild_id).await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to fetch antinuke settings for guild {}: {:?}", guild_id, e);
            return;
        }
    };

    match event_type {
        "GUILD_UPDATE" => {
            if *settings.get("anti_server_update").unwrap_or(&false) {
                warn!("Antinuke: Server settings update detected in {}.", guild_id);
            }
        }
        "MESSAGE_CREATE" => {
            if *settings.get("anti_everyone_ping").unwrap_or(&false) {
                let content = data.get("content").and_then(|v| v.as_str()).unwrap_or("");
                if content.contains("@everyone") || content.contains("@here") {
                    let _channel_id = data.get("channel_id").and_then(|v| v.as_str()).unwrap_or("");
                    let author_id = data.get("author").and_then(|v| v.get("id")).and_then(|v| v.as_str()).unwrap_or("");

                    if !db.is_whitelisted(author_id).await.unwrap_or(false) {
                        warn!("Antinuke: Unauthorized @everyone ping from {}.", author_id);

                    }
                }
            }
        }
        _ => {}
    }
}
