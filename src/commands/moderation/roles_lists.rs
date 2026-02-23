use crate::models::Message;
use crate::rest::RestClient;
use crate::db::Database;
use crate::constants::{colors, emojis};
use std::sync::Arc;
use serde_json::json;

pub async fn handle_roles_lists(
    rest: &RestClient,
    msg: &Message,
    db: Arc<Database>,
    cmd: &str,
    args: &str,
) -> anyhow::Result<()> {
    let guild_id = msg.guild_id.as_deref().unwrap_or("");
    if guild_id.is_empty() { return Ok(()); }

    let required_perm: u64 = 1 << 28;
    let has_perm = rest.has_permission(guild_id, &msg.author.id, required_perm).await.unwrap_or(false);
    let is_bot_admin = db.is_admin(&msg.author.id).await.unwrap_or(false);

    if !has_perm && !is_bot_admin {
        rest.send_message(&msg.channel_id, &format!("{} Permission Denied: You need the `Manage Roles` permission to use this command.", emojis::ERROR)).await?;
        return Ok(());
    }

    let parts: Vec<&str> = args.split_whitespace().collect();
    let sub_cmd = parts.first().unwrap_or(&"");

    match cmd {
        "role" | "r" => {
            if sub_cmd.is_empty() {
                let help = format!(
                    "{} **Role Command Usage:**\n\
                    `!role user @user <@role>` - Toggle a role for a user\n\
                    `!role all <@role>` - Give a role to everyone\n\
                    `!role humans <@role>` - Give a role to all humans\n\
                    `!role bots <@role>` - Give a role to all bots\n\
                    `!role cancel` - Cancel ongoing mass role assignment",
                    emojis::INFO
                );
                rest.send_message(&msg.channel_id, &help).await?;
                return Ok(());
            }

            match *sub_cmd {
                "user" => {
                    if parts.len() < 3 {
                        rest.send_message(&msg.channel_id, &format!("{} Usage: `!role user @user <@role>`", emojis::ERROR)).await?;
                        return Ok(());
                    }
                    let user_id = parts[1].trim_matches(&['<', '@', '!', '>'][..]);
                    let role_id = parts[2].trim_matches(&['<', '@', '&', '>'][..]);

                    if let Err(e) = rest.add_member_role(guild_id, user_id, role_id).await {
                        rest.send_message(&msg.channel_id, &format!("{} Failed to add role (does the bot have permissions? is the role higher?): {:?}", emojis::ERROR, e)).await?;
                    } else {
                        rest.send_message(&msg.channel_id, &format!("{} Successfully added <@&{}> to <@{}>", emojis::SUCCESS, role_id, user_id)).await?;
                    }
                }
                "all" | "humans" | "bots" => {
                    rest.send_message(&msg.channel_id, &format!("{} Mass role assignment (`!role {}`) is temporarily restricted to prevent rate limits.", emojis::WARNING, sub_cmd)).await?;
                }
                _ => {
                    rest.send_message(&msg.channel_id, &format!("{} Unknown `!role` subcommand.", emojis::ERROR)).await?;
                }
            }
        }
        "list" | "l" => {
            if sub_cmd.is_empty() {
                let help = format!(
                    "{} **List Command Usage:**\n\
                    `!list bots`\n`!list admin`\n`!list muted`\n`!list roles`\n`!list bans`\n`!list channels`",
                    emojis::INFO
                );
                rest.send_message(&msg.channel_id, &help).await?;
                return Ok(());
            }

            match *sub_cmd {
                "roles" => {
                    if let Ok(roles) = rest.get_guild_roles(guild_id).await {
                        let count = roles.len();
                        let mut role_list = String::new();
                        for r in roles.iter().take(20) {
                            if let (Some(id), Some(name)) = (r.get("id").and_then(|v| v.as_str()), r.get("name").and_then(|v| v.as_str())) {
                                role_list.push_str(&format!("<@&{}> - `{}`\n", id, name));
                            }
                        }
                        if count > 20 { role_list.push_str(&format!("\n*...and {} more*", count - 20)); }
                        if role_list.is_empty() { role_list = "No roles found.".to_string(); }

                        let embed = json!({
                            "title": format!("{} Server Roles ({})", emojis::SHIELD, count),
                            "description": role_list,
                            "color": colors::MAIN
                        });
                        rest.send_embed(&msg.channel_id, embed).await?;
                    }
                }
                "bans" => {
                    if let Ok(bans) = rest.get_guild_bans(guild_id).await {
                        let count = bans.len();
                        let mut ban_list = String::new();
                        for b in bans.iter().take(20) {
                            if let Some(user) = b.get("user") {
                                if let (Some(id), Some(name)) = (user.get("id").and_then(|v| v.as_str()), user.get("username").and_then(|v| v.as_str())) {
                                    ban_list.push_str(&format!("`{}` - <@{}>\n", name, id));
                                }
                            }
                        }
                        if count > 20 { ban_list.push_str(&format!("\n*...and {} more*", count - 20)); }
                        if ban_list.is_empty() { ban_list = "No bans found.".to_string(); }

                        let embed = json!({
                            "title": format!("{} Server Bans ({})", emojis::HAMMER, count),
                            "description": ban_list,
                            "color": colors::MAIN
                        });
                        rest.send_embed(&msg.channel_id, embed).await?;
                    }
                }
                "channels" => {
                    if let Ok(Some(channels_array)) = rest.get_guild_channels(guild_id).await.map(|v| v.as_array().cloned()) {
                        let count = channels_array.len();
                        let mut channel_list = String::new();
                        for c in channels_array.iter().take(20) {
                            if let (Some(id), Some(kind)) = (c.get("id").and_then(|v| v.as_str()), c.get("type").and_then(|v| v.as_u64())) {

                                if kind == 0 || kind == 2 {
                                    channel_list.push_str(&format!("<#{}>\n", id));
                                }
                            }
                        }
                        if count > 20 { channel_list.push_str(&format!("\n*...and {} more*", count - 20)); }
                        if channel_list.is_empty() { channel_list = "No text/voice channels found.".to_string(); }

                        let embed = json!({
                            "title": format!("📝 Server Channels ({})", count),
                            "description": channel_list,
                            "color": colors::MAIN
                        });
                        rest.send_embed(&msg.channel_id, embed).await?;
                    } else {
                        rest.send_message(&msg.channel_id, &format!("{} Failed to fetch channels.", emojis::ERROR)).await?;
                    }
                }
                "bots" | "admin" | "admins" | "muted" => {
                    rest.send_message(&msg.channel_id, &format!("{} Fetching member list... this may take a moment on large servers.", emojis::LOADING)).await?;

                    let mut admin_role_ids = std::collections::HashSet::new();
                    if *sub_cmd == "admin" || *sub_cmd == "admins" {
                        if let Ok(roles) = rest.get_guild_roles(guild_id).await {
                            for r in roles.iter() {
                                if let (Some(id), Some(perms_str)) = (r.get("id").and_then(|v| v.as_str()), r.get("permissions").and_then(|v| v.as_str())) {
                                    if let Ok(perms) = perms_str.parse::<u64>() {

                                        if (perms & 8) == 8 {
                                            admin_role_ids.insert(id.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Ok(Some(members_array)) = rest.get_guild_members(guild_id).await.map(|v| v.as_array().cloned()) {
                        let mut filtered_list = String::new();
                        let mut count = 0;

                        for m in members_array.iter() {
                            let is_bot = m.get("user").and_then(|u| u.get("bot")).and_then(|b| b.as_bool()).unwrap_or(false);
                            let user_id = m.get("user").and_then(|u| u.get("id")).and_then(|id| id.as_str()).unwrap_or("");
                            let username = m.get("user").and_then(|u| u.get("username")).and_then(|id| id.as_str()).unwrap_or("");
                            let timeout = m.get("communication_disabled_until").and_then(|t| t.as_str());

                            let mut is_admin = false;
                            if (*sub_cmd == "admin" || *sub_cmd == "admins") && !is_bot {

                                if let Some(roles) = m.get("roles").and_then(|r| r.as_array()) {
                                    for r in roles {
                                        if let Some(r_str) = r.as_str() {
                                            if admin_role_ids.contains(r_str) {
                                                is_admin = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }

                            let mut include = false;
                            match *sub_cmd {
                                "bots" => include = is_bot,
                                "muted" => include = timeout.is_some() && !timeout.unwrap().is_empty(),
                                "admin" | "admins" => include = is_admin,
                                _ => {}
                            }

                            if include {
                                count += 1;
                                if count <= 20 {
                                    filtered_list.push_str(&format!("`{}` - <@{}>\n", username, user_id));
                                }
                            }
                        }

                        if count > 20 { filtered_list.push_str(&format!("\n*...and {} more*", count - 20)); }
                        if filtered_list.is_empty() { filtered_list = format!("No {} found.", sub_cmd); }

                        let title = match *sub_cmd {
                            "bots" => format!("🤖 Server Bots ({})", count),
                            "muted" => format!("{} Muted Members ({})", emojis::LOCK, count),
                            "admin" | "admins" => format!("👑 Server Admins ({})", count),
                            _ => format!("List ({})", count)
                        };

                        let embed = json!({
                            "title": title,
                            "description": filtered_list,
                            "color": colors::MAIN
                        });
                        rest.send_embed(&msg.channel_id, embed).await?;
                    } else {
                        rest.send_message(&msg.channel_id, &format!("{} Failed to fetch members. Missing `GUILD_MEMBERS` intent?", emojis::ERROR)).await?;
                    }
                }
                "ignore" | "ignores" => {
                    let channels = db.get_ignored_items(guild_id, "channel").await.unwrap_or_default();
                    let roles = db.get_ignored_items(guild_id, "role").await.unwrap_or_default();
                    let users = db.get_ignored_items(guild_id, "bypass").await.unwrap_or_default();

                    let mut desc = String::new();

                    if !channels.is_empty() {
                        desc.push_str("**Ignored Channels:**\n");
                        let formatted: Vec<String> = channels.into_iter().map(|id| format!("<#{}>", id)).collect();
                        desc.push_str(&formatted.join(", "));
                        desc.push_str("\n\n");
                    }

                    if !roles.is_empty() {
                        desc.push_str("**Ignored Roles:**\n");
                        let formatted: Vec<String> = roles.into_iter().map(|id| format!("<@&{}>", id)).collect();
                        desc.push_str(&formatted.join(", "));
                        desc.push_str("\n\n");
                    }

                    if !users.is_empty() {
                        desc.push_str("**Bypassed Users:**\n");
                        let formatted: Vec<String> = users.into_iter().map(|id| format!("<@{}>", id)).collect();
                        desc.push_str(&formatted.join(", "));
                        desc.push_str("\n\n");
                    }

                    if desc.is_empty() {
                        desc = "There are no ignored channels, roles, or users configured for Antinuke bypasses.".to_string();
                    }

                    let embed = json!({
                        "title": format!("{} Ignore Configuration", emojis::SHIELD),
                        "description": desc,
                        "color": colors::MAIN
                    });
                    rest.send_embed(&msg.channel_id, embed).await?;
                }
                _ => {
                    rest.send_message(&msg.channel_id, &format!("{} That list type is currently not available via the raw REST API without fetching all members. (Supported: `roles`, `bans`, `channels`, `ignore`)", emojis::WARNING)).await?;
                }
            }
        }
        _ => {}
    }

    Ok(())
}
