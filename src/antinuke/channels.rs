use crate::rest::RestClient;
use crate::db::Database;
use crate::constants::emojis;
use std::sync::Arc;
use serde_json::{json, Value};
use tracing::{info, warn, error};

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

    match event_type {
        "CHANNEL_DELETE" => {
            if *settings.get("anti_channel_delete").unwrap_or(&false) {
                detect_mass_channel_action(guild_id, rest, db).await;
            }
        }
        "THREAD_CREATE" => {
            if *settings.get("thread_lock_enabled").unwrap_or(&true) {
                handle_thread_lock(data, &rest, db).await;
            }
        }
        "CHANNEL_CREATE" => {
            if *settings.get("anti_channel_create").unwrap_or(&false) {
                warn!("Antinuke: Unauthorized channel creation detected in {}.", guild_id);
                crate::antinuke::punish_offender(guild_id, 10, rest.clone(), db.clone()).await;
            }
        }
        _ => {}
    }
}

async fn detect_mass_channel_action(guild_id: &str, rest: Arc<RestClient>, db: Arc<Database>) {
    let now = tokio::time::Instant::now();
    let mut map = db.channel_tracker.write().await;
    let entries = map.entry(guild_id.to_string()).or_insert(std::collections::VecDeque::new());

    while let Some(&t) = entries.front() {
        if now.duration_since(t) >= tokio::time::Duration::from_secs(10) {
            entries.pop_front();
        } else {
            break;
        }
    }
    entries.push_back(now);

    if entries.len() >= 1 {
        warn!("CRITICAL: CHANNEL DELETE detected in server {}.", guild_id);
        let _alert = json!({
            "title": format!("{} SECURITY ALERT: CHANNEL DELETION DETECTED", emojis::WARNING),
            "description": "An unauthorized channel deletion has been detected and intercepted.",
            "color": 0xff0000,
            "footer": { "text": "Rimuru Advanced Security | Cog Protection Active" }
        });
        info!("ANTINUKE ALERT: Server {} | Action: CHANNEL_DELETE", guild_id);

        crate::antinuke::punish_offender(guild_id, 12, rest, db.clone()).await;
    }
}

async fn handle_thread_lock(data: Value, rest: &RestClient, db: Arc<Database>) {
    let guild_id = data.get("guild_id").and_then(|v| v.as_str()).unwrap_or("");
    let thread_id = data.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let owner_id = data.get("owner_id").and_then(|v| v.as_str()).unwrap_or("");

    if db.is_whitelisted(owner_id).await.unwrap_or(false) || db.is_admin(owner_id).await.unwrap_or(false) {
        return;
    }

    if let Ok(bot) = rest.validate_token().await {
        if bot.id == owner_id {
            return;
        }
    }

    if let Ok(threads) = rest.get_active_threads(guild_id).await {
        let count = threads.as_array().map(|a| a.len()).unwrap_or(0);

        if count > 49 {
            warn!("THREAD LOCK: Limit reached in {}. Deleting unauthorized thread {}.", guild_id, thread_id);
            if let Err(e) = rest.delete_channel(thread_id).await {
                error!("Failed to execute THREAD LOCK via delete_channel: {:?}", e);
            }
        }
    }
}
