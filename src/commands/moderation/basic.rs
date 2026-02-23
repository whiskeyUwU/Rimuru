use crate::models::Message;
use crate::rest::RestClient;
use crate::db::Database;
use crate::constants::{colors, emojis};
use std::sync::Arc;
use serde_json::json;

pub async fn handle_basic(
    rest: &RestClient,
    msg: &Message,
    _db: Arc<Database>,
    cmd: &str,
    args: &str,
) -> anyhow::Result<()> {
    let guild_id = msg.guild_id.as_deref().unwrap_or("");
    if guild_id.is_empty() { return Ok(()); }

    let required_perm: u64 = match cmd {
        "ban" | "softban" | "unban" | "unbanall" => 1 << 2, 
        "kick" => 1 << 1, 
        "mute" | "unmute" => 1 << 40, 
        "nick" => 1 << 27, 
        "slowmode" => 1 << 4, 
        _ => 0,
    };

    if required_perm > 0 {
        let has_perm = rest.has_permission(guild_id, &msg.author.id, required_perm).await.unwrap_or(false);
        let is_bot_admin = _db.is_admin(&msg.author.id).await.unwrap_or(false);

        if !has_perm && !is_bot_admin {
            rest.send_message(&msg.channel_id, &format!("{} Permission Denied: You do not have the required Discord server permissions to run this command.", emojis::ERROR)).await?;
            return Ok(());
        }
    }

    let parts: Vec<&str> = args.split_whitespace().collect();
    let target = parts.first().unwrap_or(&"");
    let target_id = target.trim_matches(&['<', '@', '>', '!'][..]);
    let reason = if parts.len() > 1 { parts[1..].join(" ") } else { "No reason provided".to_string() };

    match cmd {
        "ban" => {
            if target_id.is_empty() {
                rest.send_message(&msg.channel_id, &format!("{} Usage: `!ban @user [reason]`", emojis::ERROR)).await?;
                return Ok(());
            }
            if let Err(e) = rest.ban_user(guild_id, target_id, &reason).await {
                let err_str = e.to_string();
                if err_str.contains("403") {
                    rest.send_message(&msg.channel_id, &format!("{} **Failed to ban:** I do not have permission to ban this user. Ensure my role is higher than theirs and that I have the `Ban Members` permission.", emojis::ERROR)).await?;
                } else {
                    rest.send_message(&msg.channel_id, &format!("{} Failed to ban user: {:?}", emojis::ERROR, e)).await?;
                }
            } else {
                let embed = json!({
                    "title": format!("{} Member Banned", emojis::HAMMER),
                    "description": format!("**Target:** <@{}>\n**Reason:** {}\n**Moderator:** <@{}>", target_id, reason, msg.author.id),
                    "color": colors::MAIN
                });
                rest.send_embed(&msg.channel_id, embed).await?;
            }
        }
        "kick" => {
            if target_id.is_empty() {
                rest.send_message(&msg.channel_id, &format!("{} Usage: `!kick @user [reason]`", emojis::ERROR)).await?;
                return Ok(());
            }
            if let Err(e) = rest.kick_user(guild_id, target_id, &reason).await {
                let err_str = e.to_string();
                if err_str.contains("403") {
                    rest.send_message(&msg.channel_id, &format!("{} **Failed to kick:** I do not have permission to kick this user. Ensure my role is higher than theirs.", emojis::ERROR)).await?;
                } else {
                    rest.send_message(&msg.channel_id, &format!("{} Failed to kick user: {:?}", emojis::ERROR, e)).await?;
                }
            } else {
                let embed = json!({
                    "title": format!("{} Member Kicked", emojis::SHIELD),
                    "description": format!("**Target:** <@{}>\n**Reason:** {}\n**Moderator:** <@{}>", target_id, reason, msg.author.id),
                    "color": colors::MAIN
                });
                rest.send_embed(&msg.channel_id, embed).await?;
            }
        }
        "softban" => {
            if target_id.is_empty() {
                rest.send_message(&msg.channel_id, &format!("{} Usage: `!softban @user [reason]`", emojis::ERROR)).await?;
                return Ok(());
            }
            if let Err(e) = rest.ban_user(guild_id, target_id, &reason).await {
                let err_str = e.to_string();
                if err_str.contains("403") {
                    rest.send_message(&msg.channel_id, &format!("{} **Failed to softban:** I do not have permission to ban this user. Ensure my role is higher than theirs.", emojis::ERROR)).await?;
                } else {
                    rest.send_message(&msg.channel_id, &format!("{} Failed to softban user: {:?}", emojis::ERROR, e)).await?;
                }
            } else {
                let _ = rest.remove_guild_ban(guild_id, target_id, "Softban Unban").await;
                let embed = json!({
                    "title": format!("{} Member Softbanned", emojis::HAMMER),
                    "description": format!("**Target:** <@{}>\n**Reason:** {}\n**Moderator:** <@{}>", target_id, reason, msg.author.id),
                    "color": colors::MAIN
                });
                rest.send_embed(&msg.channel_id, embed).await?;
            }
        }
        "unban" => {
            if target_id.is_empty() {
                rest.send_message(&msg.channel_id, &format!("{} Usage: `!unban <user_id>`", emojis::ERROR)).await?;
                return Ok(());
            }
            if let Err(e) = rest.remove_guild_ban(guild_id, target_id, &reason).await {
                rest.send_message(&msg.channel_id, &format!("{} Failed to unban user: {:?}", emojis::ERROR, e)).await?;
            } else {
                rest.send_message(&msg.channel_id, &format!("{} Successfully unbanned <@{}>", emojis::SUCCESS, target_id)).await?;
            }
        }
        "unbanall" => {
            let bans = rest.get_guild_bans(guild_id).await.unwrap_or_default();
            if bans.is_empty() {
                rest.send_message(&msg.channel_id, &format!("{} No banned users found in this server.", emojis::INFO)).await?;
                return Ok(());
            }
            let mut count = 0;
            let initial_msg = rest.send_message(&msg.channel_id, &format!("{} Unbanning {} users...", emojis::CLOCK, bans.len())).await?;
            for ban in bans {
                if let Some(user) = ban.get("user") {
                    if let Some(id) = user.get("id").and_then(|id| id.as_str()) {
                        if rest.remove_guild_ban(guild_id, id, "Mass Unban").await.is_ok() {
                            count += 1;
                        }
                    }
                }
            }
            rest.send_message(&msg.channel_id, &format!("{} Unbanned **{}** users.", emojis::SUCCESS, count)).await?;
        }
        "mute" => {
            if parts.len() < 2 {
                rest.send_message(&msg.channel_id, &format!("{} Usage: `!mute @user <duration> [reason]` (e.g., 10m, 1h, 1d)", emojis::ERROR)).await?;
                return Ok(());
            }

            let duration_str = parts[1].to_lowercase();
            let mut multiplier = 1;
            let val_str = if duration_str.ends_with('m') {
                &duration_str[..duration_str.len()-1]
            } else if duration_str.ends_with('h') {
                multiplier = 60;
                &duration_str[..duration_str.len()-1]
            } else if duration_str.ends_with('d') {
                multiplier = 1440;
                &duration_str[..duration_str.len()-1]
            } else {
                &duration_str
            };

            let mins: i64 = val_str.parse::<i64>().unwrap_or(0) * multiplier;

            if mins <= 0 {
                rest.send_message(&msg.channel_id, &format!("{} Duration must be at least 1 minute.", emojis::ERROR)).await?;
                return Ok(());
            }
            let mute_reason = if parts.len() > 2 { parts[2..].join(" ") } else { "No reason provided".to_string() };

            let timestamp = chrono::Utc::now() + chrono::Duration::minutes(mins);
            let iso8601 = timestamp.to_rfc3339();

            if let Err(e) = rest.timeout_member(guild_id, target_id, Some(&iso8601), &mute_reason).await {
                let err_str = e.to_string();
                if err_str.contains("403") {
                    rest.send_message(&msg.channel_id, &format!("{} **Failed to mute:** I do not have permission to timeout this user. Ensure my role is higher than theirs.", emojis::ERROR)).await?;
                } else {
                    rest.send_message(&msg.channel_id, &format!("{} Failed to mute user: {:?}", emojis::ERROR, e)).await?;
                }
            } else {
                let embed = json!({
                    "title": format!("{} Member Muted", emojis::LOCK),
                    "description": format!("**Target:** <@{}>\n**Duration:** {} minutes\n**Reason:** {}\n**Moderator:** <@{}>", target_id, mins, mute_reason, msg.author.id),
                    "color": colors::MAIN
                });
                rest.send_embed(&msg.channel_id, embed).await?;
            }
        }
        "unmute" => {
            if target_id.is_empty() {
                rest.send_message(&msg.channel_id, &format!("{} Usage: `!unmute @user`", emojis::ERROR)).await?;
                return Ok(());
            }
            if let Err(e) = rest.timeout_member(guild_id, target_id, None, "Manual Unmute").await {
                rest.send_message(&msg.channel_id, &format!("{} Failed to unmute user: {:?}", emojis::ERROR, e)).await?;
            } else {
                rest.send_message(&msg.channel_id, &format!("{} Successfully unmuted <@{}>", emojis::SUCCESS, target_id)).await?;
            }
        }
        "nick" => {
            if target_id.is_empty() {
                rest.send_message(&msg.channel_id, &format!("{} Usage: `!nick @user <new_nickname>` (leave empty to reset)", emojis::ERROR)).await?;
                return Ok(());
            }
            let new_nick = if parts.len() > 1 { Some(parts[1..].join(" ")) } else { None };
            if let Err(e) = rest.modify_member(guild_id, target_id, new_nick.as_deref(), &format!("Requested by {}", msg.author.username)).await {
                rest.send_message(&msg.channel_id, &format!("{} Failed to modify nickname: {:?}", emojis::ERROR, e)).await?;
            } else {
                rest.send_message(&msg.channel_id, &format!("{} Successfully updated `<@{}>`'s nickname.", emojis::SUCCESS, target_id)).await?;
            }
        }
        "slowmode" => {
            let limit: u16 = target_id.parse().unwrap_or(0); 
            if let Err(e) = rest.modify_channel(&msg.channel_id, limit).await {
                rest.send_message(&msg.channel_id, &format!("{} Failed to set slowmode: {:?}", emojis::ERROR, e)).await?;
            } else {
                if limit == 0 {
                    rest.send_message(&msg.channel_id, &format!("{} Slowmode disabled.", emojis::SUCCESS)).await?;
                } else {
                    rest.send_message(&msg.channel_id, &format!("{} Slowmode set to **{} seconds**.", emojis::SUCCESS, limit)).await?;
                }
            }
        }
        _ => {}
    }

    Ok(())
}
