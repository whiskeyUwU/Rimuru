use crate::models::Message;
use crate::rest::RestClient;
use crate::db::Database;
use crate::constants::emojis;
use std::sync::Arc;

pub async fn handle_channels(
    rest: &RestClient,
    msg: &Message,
    db: Arc<Database>,
    cmd: &str,
    args: &str,
) -> anyhow::Result<()> {
    let guild_id = msg.guild_id.as_deref().unwrap_or("");
    if guild_id.is_empty() { return Ok(()); }

    let required_perm: u64 = 1 << 4;
    let has_perm = rest.has_permission(guild_id, &msg.author.id, required_perm).await.unwrap_or(false);
    let is_bot_admin = db.is_admin(&msg.author.id).await.unwrap_or(false);

    if !has_perm && !is_bot_admin {
        rest.send_message(&msg.channel_id, &format!("{} Permission Denied: You need the `Manage Channels` permission to use this command.", emojis::ERROR)).await?;
        return Ok(());
    }

    let parts: Vec<&str> = args.split_whitespace().collect();
    let target = parts.first().unwrap_or(&"");
    let target_id = target.trim_matches(&['<', '@', '>', '!', '#'][..]); 

    let everyone_id = guild_id;

    let mut resolved_cmd = cmd;
    if target_id == "all" && (cmd == "lock" || cmd == "unlock" || cmd == "hide" || cmd == "unhide") {
        resolved_cmd = match cmd {
            "lock" => "lockall",
            "unlock" => "unlockall",
            "hide" => "hideall",
            "unhide" => "unhideall",
            _ => cmd,
        };
    }

    match resolved_cmd {
        "lock" => {
            let channel_to_lock = if target_id.is_empty() { &msg.channel_id } else { target_id };

            if let Err(e) = rest.modify_channel_permissions(channel_to_lock, everyone_id, "0", "2048", 0).await {
                rest.send_message(&msg.channel_id, &format!("{} Failed to lock channel: {:?}", emojis::ERROR, e)).await?;
            } else {
                rest.send_message(&msg.channel_id, &format!("{} Channel <#{}> locked for @everyone.", emojis::LOCK, channel_to_lock)).await?;
            }
        }
        "unlock" => {
            let channel_to_unlock = if target_id.is_empty() { &msg.channel_id } else { target_id };

            if let Err(e) = rest.modify_channel_permissions(channel_to_unlock, everyone_id, "2048", "0", 0).await {
                rest.send_message(&msg.channel_id, &format!("{} Failed to unlock channel: {:?}", emojis::ERROR, e)).await?;
            } else {
                rest.send_message(&msg.channel_id, &format!("{} Channel <#{}> unlocked for @everyone.", emojis::UNLOCK, channel_to_unlock)).await?;
            }
        }
        "hide" => {
            let channel_to_hide = if target_id.is_empty() { &msg.channel_id } else { target_id };

            if let Err(e) = rest.modify_channel_permissions(channel_to_hide, everyone_id, "0", "1024", 0).await {
                rest.send_message(&msg.channel_id, &format!("{} Failed to hide channel: {:?}", emojis::ERROR, e)).await?;
            } else {
                rest.send_message(&msg.channel_id, &format!("{} Channel <#{}> hidden from @everyone.", emojis::SHIELD, channel_to_hide)).await?;
            }
        }
        "unhide" => {
            let channel_to_unhide = if target_id.is_empty() { &msg.channel_id } else { target_id };

            if let Err(e) = rest.modify_channel_permissions(channel_to_unhide, everyone_id, "1024", "0", 0).await {
                rest.send_message(&msg.channel_id, &format!("{} Failed to unhide channel: {:?}", emojis::ERROR, e)).await?;
            } else {
                rest.send_message(&msg.channel_id, &format!("{} Channel <#{}> is now visible to @everyone.", emojis::EYE, channel_to_unhide)).await?;
            }
        }
        "block" => {
            if target_id.is_empty() {
                rest.send_message(&msg.channel_id, &format!("{} Usage: `!block @user`", emojis::ERROR)).await?;
                return Ok(());
            }

            if let Err(e) = rest.modify_channel_permissions(&msg.channel_id, target_id, "0", "3072", 1).await {
                rest.send_message(&msg.channel_id, &format!("{} Failed to block user: {:?}", emojis::ERROR, e)).await?;
            } else {
                rest.send_message(&msg.channel_id, &format!("{} User <@{}> blocked from this channel.", emojis::HAMMER, target_id)).await?;
            }
        }
        "unblock" => {
            if target_id.is_empty() {
                rest.send_message(&msg.channel_id, &format!("{} Usage: `!unblock @user`", emojis::ERROR)).await?;
                return Ok(());
            }

            if let Err(e) = rest.modify_channel_permissions(&msg.channel_id, target_id, "3072", "0", 1).await {
                rest.send_message(&msg.channel_id, &format!("{} Failed to unblock user: {:?}", emojis::ERROR, e)).await?;
            } else {
                rest.send_message(&msg.channel_id, &format!("{} User <@{}> unblocked in this channel.", emojis::SUCCESS, target_id)).await?;
            }
        }
        "lockall" | "unlockall" | "hideall" | "unhideall" => {
            rest.send_message(&msg.channel_id, &format!("{} Executing `{}` on all channels. This may take a minute...", emojis::LOADING, cmd)).await?;

            if let Ok(Some(channels_array)) = rest.get_guild_channels(guild_id).await.map(|v| v.as_array().cloned()) {
                let mut success_count = 0;
                let mut fail_count = 0;

                for c in channels_array.iter() {
                    if let (Some(id), Some(kind)) = (c.get("id").and_then(|v| v.as_str()), c.get("type").and_then(|v| v.as_u64())) {

                        if kind == 0 || kind == 2 || kind == 5 {
                            let (allow, deny) = match resolved_cmd {
                                "lockall" => ("0", "2048"),   
                                "unlockall" => ("2048", "0"), 
                                "hideall" => ("0", "1024"),   
                                "unhideall" => ("1024", "0"), 
                                _ => break,
                            };

                            if rest.modify_channel_permissions(id, everyone_id, allow, deny, 0).await.is_ok() {
                                success_count += 1;
                            } else {
                                fail_count += 1;
                            }

                            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                        }
                    }
                }

                let icon = match resolved_cmd {
                    "lockall" => emojis::LOCK,
                    "unlockall" => emojis::UNLOCK,
                    "hideall" => emojis::SHIELD,
                    "unhideall" => emojis::EYE,
                    _ => emojis::SUCCESS,
                };

                rest.send_message(&msg.channel_id, &format!("{} `{}` complete! Modified **{}** channels. (Failed: {})", icon, resolved_cmd, success_count, fail_count)).await?;
            } else {
                rest.send_message(&msg.channel_id, &format!("{} Failed to fetch channels.", emojis::ERROR)).await?;
            }
        }
        _ => {}
    }

    Ok(())
}
