use crate::rest::RestClient;
use crate::db::Database;
use std::sync::Arc;
use serde_json::Value;
use tracing::{warn, error};

pub async fn handle_event(
    event_type: &str,
    data: Value,
    rest: Arc<RestClient>,
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

    let executor_id = data.get("executor_id").and_then(|v| v.as_str()).unwrap_or("");

    match event_type {
        "GUILD_ROLE_CREATE" => {
            if *settings.get("anti_role_create").unwrap_or(&false) {
                if !executor_id.is_empty() && (db.is_whitelisted(executor_id).await.unwrap_or(false) || db.is_admin(executor_id).await.unwrap_or(false)) {
                    return;
                }
                warn!("ANTINUKE: Unauthorized role creation in server {}.", guild_id);
                crate::antinuke::punish_offender(guild_id, 30, rest.clone(), db.clone()).await;
            }
        }
        "GUILD_ROLE_DELETE" => {
            if *settings.get("anti_role_delete").unwrap_or(&false) {
                if !executor_id.is_empty() && (db.is_whitelisted(executor_id).await.unwrap_or(false) || db.is_admin(executor_id).await.unwrap_or(false)) {
                    return;
                }
                warn!("ANTINUKE: Role deletion detected in server {}.", guild_id);
                crate::antinuke::punish_offender(guild_id, 32, rest.clone(), db.clone()).await;
            }
        }
        "GUILD_MEMBER_UPDATE" => {
            if *settings.get("anti_member_role_update").unwrap_or(&false) {

            }
        }
        _ => {}
    }
}
