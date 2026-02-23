use crate::models::Message;
use crate::rest::RestClient;
use crate::db::Database;
use crate::constants::emojis;
use std::sync::Arc;
use serde_json::json;

pub async fn handle_system(
    rest: &RestClient,
    msg: &Message,
    db: Arc<Database>,
    cmd: &str,
    args: &str,
) -> anyhow::Result<()> {
    let guild_id = msg.guild_id.as_deref().unwrap_or("");
    if guild_id.is_empty() { return Ok(()); }

    let parts: Vec<&str> = args.split_whitespace().collect();
    let sub_cmd = parts.first().unwrap_or(&"");

    let required_perm: u64 = match cmd {
        "warn" | "warning" => 1 << 40, 
        "command" | "ignore" | "unignore" | "prefix" => 1 << 5, 
        _ => 0,
    };

    if required_perm > 0 {
        let has_perm = rest.has_permission(guild_id, &msg.author.id, required_perm).await.unwrap_or(false);
        let is_bot_admin = db.is_admin(&msg.author.id).await.unwrap_or(false);

        if !has_perm && !is_bot_admin {
            let perm_name = if required_perm == (1 << 40) { "Timeout Members" } else { "Manage Server" };
            rest.send_message(&msg.channel_id, &format!("{} Permission Denied: You need the `{}` permission to use this command.", emojis::ERROR, perm_name)).await?;
            return Ok(());
        }
    }

    match cmd {
        "warn" | "warning" => {
            if sub_cmd.is_empty() {
                let help = format!(
                    "{} **Warning System:**\n\
                    `!warn @user <reason>` - Issue a warning\n\
                    `!warn list @user` - View warnings\n\
                    `!warn remove @user <id>` - Remove a specific warning\n\
                    `!warn clear @user` - Clear all warnings",
                    emojis::INFO
                );
                rest.send_message(&msg.channel_id, &help).await?;
                return Ok(());
            }

            match *sub_cmd {
                "list" => {
                    let target_id = parts.get(1).unwrap_or(&"").trim_matches(&['<', '@', '!', '>'][..]);
                    if target_id.is_empty() { return Ok(()); }
                    let warnings = db.get_warnings(guild_id, target_id).await.unwrap_or_default();
                    if warnings.is_empty() {
                        rest.send_message(&msg.channel_id, &format!("{} User <@{}> has no warnings.", emojis::SUCCESS, target_id)).await?;
                    } else {
                        let mut desc = String::new();
                        for (id, reason, moderator, timestamp) in warnings {
                            desc.push_str(&format!("**ID:** `{}` | **By:** <@{}>\n**Reason:** {}\n**Date:** {}\n\n", id, moderator, reason, timestamp));
                        }
                        let embed = json!({
                            "title": format!("{} Warnings for User", emojis::WARNING),
                            "description": desc,
                            "color": 0xFEE75C 
                        });
                        rest.send_embed(&msg.channel_id, embed).await?;
                    }
                }
                "remove" => {
                    if parts.len() < 3 { return Ok(()); }
                    let target_id = parts[1].trim_matches(&['<', '@', '!', '>'][..]);
                    let warn_id: i64 = parts[2].parse().unwrap_or(0);
                    let removed = db.remove_warning(guild_id, warn_id).await.unwrap_or(0);
                    if removed > 0 {
                        rest.send_message(&msg.channel_id, &format!("{} Removed warning #{} for <@{}>.", emojis::SUCCESS, warn_id, target_id)).await?;
                    } else {
                        rest.send_message(&msg.channel_id, &format!("{} Warning #{} not found.", emojis::ERROR, warn_id)).await?;
                    }
                }
                "clear" => {
                    if parts.len() < 2 { return Ok(()); }
                    let target_id = parts[1].trim_matches(&['<', '@', '!', '>'][..]);
                    let removed = db.clear_warnings(guild_id, target_id).await.unwrap_or(0);
                    rest.send_message(&msg.channel_id, &format!("{} Cleared **{}** warnings for <@{}>.", emojis::SUCCESS, removed, target_id)).await?;
                }
                _ => {

                    let target_id = sub_cmd.trim_matches(&['<', '@', '!', '>'][..]);
                    let reason = if parts.len() > 1 { parts[1..].join(" ") } else { "No reason provided".to_string() };
                    db.add_warning(guild_id, target_id, &reason, &msg.author.id).await?;

                    let embed = json!({
                        "title": format!("{} Member Warned", emojis::WARNING),
                        "description": format!("**Target:** <@{}>\n**Reason:** {}\n**Moderator:** <@{}>", target_id, reason, msg.author.id),
                        "color": 0xFEE75C
                    });
                    rest.send_embed(&msg.channel_id, embed).await?;
                }
            }
        }
        "prefix" => {
            if sub_cmd.is_empty() {
                let current = db.get_prefix(guild_id).await;
                rest.send_embed(&msg.channel_id, json!({
                    "description": format!("{} Current prefix is: `{}`\nUse `!prefix <new>` to change it.", emojis::INFO, current),
                    "color": 0x5865F2
                })).await?;
            } else {
                db.set_prefix(guild_id, sub_cmd).await?;
                rest.send_embed(&msg.channel_id, json!({
                    "description": format!("{} Prefix successfully changed to: `{}`", emojis::SUCCESS, sub_cmd),
                    "color": 0x57F287
                })).await?;
            }
        }
        "ignore" | "unignore" => {
            tracing::info!("Reached ignore/unignore processor with parts len: {}", parts.len());
            if parts.len() < 2 {
                let help = format!(
                    "{} **Ignore Configuration:**\n\
                    `!ignore channel <#id>`\n\
                    `!ignore role <@role>`\n\
                    `!ignore user <@user>`\n\
                    `!unignore <type> <id>` to remove.",
                    if cmd == "ignore" { emojis::SHIELD } else { emojis::EYE }
                );
                rest.send_embed(&msg.channel_id, json!({
                    "description": help,
                    "color": 0x5865F2
                })).await?;
                return Ok(());
            }

            let target_type = sub_cmd.to_lowercase();
            tracing::info!("Target type parsed as: {}", target_type);

            if !["channel", "role", "user", "bypass"].contains(&target_type.as_str()) {
                rest.send_embed(&msg.channel_id, json!({
                    "description": format!("{} Invalid type! Use `channel`, `role`, or `user`.", emojis::ERROR),
                    "color": 0xED4245
                })).await?;
                return Ok(());
            }

            if parts.len() < 2 {
                tracing::info!("Listing currently ignored items due to lack of target mapping...");
                let list = db.get_ignored_items(guild_id, &target_type).await.unwrap_or_default();
                if list.is_empty() {
                    rest.send_embed(&msg.channel_id, json!({
                        "description": format!("{} No {}s are currently ignored.", emojis::INFO, target_type),
                        "color": 0x5865F2
                    })).await?;
                } else {
                    let formatted: Vec<String> = list.into_iter().map(|id| format!("`{}`", id)).collect();
                    rest.send_embed(&msg.channel_id, json!({
                        "description": format!("{} Ignored {}s: {}", emojis::INFO, target_type, formatted.join(", ")),
                        "color": 0x5865F2
                    })).await?;
                }
                return Ok(());
            }

            let target_id = parts.last().unwrap().trim_matches(&['<', '@', '!', '#', '&', '>'][..]);

            tracing::info!("Routing execution to {} against id {}", cmd, target_id);
            if cmd == "ignore" {
                if let Err(e) = db.ignore_item(guild_id, &target_type, target_id).await {
                     tracing::error!("ignore_item DB call failed: {}", e);
                     rest.send_embed(&msg.channel_id, json!({
                         "description": format!("{} Failed to ignore: {}", emojis::ERROR, e),
                         "color": 0xED4245
                     })).await?;
                } else {
                     tracing::info!("ignore_item succeeded");
                     rest.send_embed(&msg.channel_id, json!({
                         "description": format!("{} Successfully ignored {} `<@{}>`.", emojis::SUCCESS, target_type, target_id),
                         "color": 0x57F287
                     })).await?;
                }
            } else {
                tracing::info!("Executing unignore inside Else branch!");
                if let Err(e) = db.unignore_item(guild_id, &target_type, target_id).await {
                     tracing::error!("unignore_item DB call failed: {}", e);
                     rest.send_embed(&msg.channel_id, json!({
                         "description": format!("{} Failed to unignore: {}", emojis::ERROR, e),
                         "color": 0xED4245
                     })).await?;
                } else {
                     tracing::info!("unignore_item DB call succeeded... triggering message embed payload!");
                     rest.send_embed(&msg.channel_id, json!({
                         "description": format!("{} Removed {} `<@{}>` from ignore list.", emojis::SUCCESS, target_type, target_id),
                         "color": 0x57F287
                     })).await?;
                }
            }
        }
        "command" => {
            if parts.len() < 2 {
                let help = format!(
                    "{} **Command Configuration:**\n\
                    `!command disable <cmd>` - Disable a command\n\
                    `!command enable <cmd>` - Re-enable a command",
                    emojis::WRENCH
                );
                rest.send_message(&msg.channel_id, &help).await?;
                return Ok(());
            }
            let action = parts[0].to_lowercase();
            let target_cmd = parts[1].to_lowercase();
            if action == "disable" || action == "off" {
                db.toggle_command(guild_id, &target_cmd, true).await?;
                rest.send_message(&msg.channel_id, &format!("{} Command `{}` is now disabled.", emojis::SUCCESS, target_cmd)).await?;
            } else if action == "enable" || action == "on" {
                db.toggle_command(guild_id, &target_cmd, false).await?;
                rest.send_message(&msg.channel_id, &format!("{} Command `{}` is now enabled.", emojis::SUCCESS, target_cmd)).await?;
            }
        }
        _ => {}
    }

    Ok(())
}
