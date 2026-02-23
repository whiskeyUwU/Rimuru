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
        "GUILD_BAN_ADD" => {
            if *settings.get("anti_ban").unwrap_or(&false) {
                detect_mass_action("ban", guild_id, rest.clone(), db).await;
            }
        }
        "GUILD_BAN_REMOVE" => {
            if *settings.get("anti_unban").unwrap_or(&false) {
                send_alert("UNBAN", guild_id, &rest).await;
            }
        }
        "GUILD_MEMBER_REMOVE" => {
            if *settings.get("anti_kick").unwrap_or(&false) {
                detect_mass_action("kick", guild_id, rest.clone(), db).await;
            }
        }
        _ => {}
    }
}

async fn detect_mass_action(action: &str, guild_id: &str, rest: Arc<RestClient>, db: Arc<Database>) {

    let tracker = if action == "ban" { &db.ban_tracker } else { &db.channel_tracker }; 
    let now = tokio::time::Instant::now();
    let mut map = tracker.write().await;
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
        warn!("CRITICAL: {} detected in server {}.", action.to_uppercase(), guild_id);
        send_alert(&action.to_uppercase(), guild_id, &rest).await;

        let action_type = if action == "ban" { 22 } else { 20 }; 
        crate::antinuke::punish_offender(guild_id, action_type, rest, db.clone()).await;
    }
}

async fn send_alert(action: &str, guild_id: &str, _rest: &Arc<RestClient>) {
    let _alert = json!({
        "title": format!("{} SECURITY ALERT: MASS {} DETECTED", emojis::WARNING, action),
        "description": format!("The Antinuke module has detected rapid-fire **{}** actions in this server. Investigation recommended.", action),
        "color": 0xff0000,
        "footer": { "text": "Rimuru Advanced Security | Cog Protection Active" }
    });

    info!("ANTINUKE ALERT: Server {} | Action: {}", guild_id, action);

}
