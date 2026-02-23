use crate::models::{Interaction, Message};
use crate::rest::RestClient;
use crate::db::Database;
use crate::constants::{emojis, colors};
use std::sync::Arc;
use serde_json::json;

pub async fn handle_antinuke(
    rest: &RestClient,
    msg: &Message,
    db: Arc<Database>,
    _cmd: &str,
    args: &str,
) -> anyhow::Result<()> {

    let is_bot_admin = db.is_admin(&msg.author.id).await.unwrap_or(false);
    let mut is_owner = false;

    if let Some(guild_id) = &msg.guild_id {
        if let Ok(guild) = rest.get_guild(guild_id).await {
            if let Some(owner_id) = guild["owner_id"].as_str() {
                is_owner = owner_id == msg.author.id;
            }
        }
    }

    if !is_bot_admin && !is_owner {
        rest.send_message(&msg.channel_id, &format!("{} This command is restricted to **Server Owners** and **Bot Admins** only.", emojis::ERROR)).await?;
        return Ok(());
    }

    let parts: Vec<&str> = args.split_whitespace().collect();
    let sub_cmd = parts.first().map(|s| s.to_lowercase()).unwrap_or_default();

    let bot_user = rest.validate_token().await?;
    let bot_avatar = bot_user.avatar_url();

    match sub_cmd.as_str() {
        "config" | "manage" | "setup" => {
            show_config_menu(rest, msg, &bot_avatar).await
        }
        "enable" | "activate" | "on" => {
            show_enable_sequence(rest, msg, db, &bot_avatar).await
        }
        "disable" | "deactivate" | "off" => {
            show_disable_sequence(rest, msg, db, &bot_avatar).await
        }
        "settings" | "status" | "info" => {
            show_settings(rest, msg, db, &bot_avatar).await
        }
        "" => {
            show_dashboard(rest, msg, &bot_avatar).await
        }
        _ => {
            rest.send_message(&msg.channel_id, &format!("{} Unknown subcommand. Try `config`, `enable`, `settings`.", emojis::ERROR)).await?;
            Ok(())
        }
    }
}

async fn show_enable_sequence(rest: &RestClient, msg: &Message, db: Arc<Database>, bot_avatar: &str) -> anyhow::Result<()> {
    let guild_id = msg.guild_id.as_deref().unwrap_or("");

    let mut lines = vec!["✅ | Initializing Quick Setup!"];
    let mut embed = json!({
        "description": lines.join("\n"),
        "color": colors::MAIN
    });
    let initial_msg = rest.send_complex_message(&msg.channel_id, "", vec![embed], vec![]).await?;
    let msg_id = initial_msg["id"].as_str().unwrap_or("");

    tokio::time::sleep(std::time::Duration::from_millis(800)).await;

    lines.push("✅ | **INITIALIZING** Permission Verification Protocol...");
    embed = json!({ "description": lines.join("\n"), "color": colors::MAIN });
    rest.edit_message(&msg.channel_id, msg_id, "", vec![embed], vec![]).await?;

    tokio::time::sleep(std::time::Duration::from_millis(800)).await;

    lines.push("✅ | **ANALYZING** Role Hierarchy Configuration...");
    embed = json!({ "description": lines.join("\n"), "color": colors::MAIN });
    rest.edit_message(&msg.channel_id, msg_id, "", vec![embed], vec![]).await?;

    tokio::time::sleep(std::time::Duration::from_millis(800)).await;

    lines.push("✅ | **ENGINEERING** Rimuru Impenetrable Power Role...");
    embed = json!({ "description": lines.join("\n"), "color": colors::MAIN });
    rest.edit_message(&msg.channel_id, msg_id, "", vec![embed], vec![]).await?;

    if let Ok(role) = rest.create_role(guild_id, "Rimuru Absolute Authority", 0x57F287, true, "8").await {
        if let Some(role_id) = role.get("id").and_then(|v| v.as_str()) {
            if let Ok(bot) = rest.validate_token().await {

                let _ = rest.add_member_role(guild_id, &bot.id, role_id).await;

                if let Ok(roles) = rest.get_guild_roles(guild_id).await {
                    if let Ok(bot_member) = rest.get_guild_member(guild_id, &bot.id).await {
                        let bot_role_ids: Vec<&str> = bot_member["roles"]
                            .as_array()
                            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                            .unwrap_or_default();

                        let mut max_bot_pos = 0;
                        for r in &roles {
                            if let (Some(id), Some(pos)) = (r["id"].as_str(), r["position"].as_u64()) {
                                if bot_role_ids.contains(&id) && pos > max_bot_pos {
                                    max_bot_pos = pos;
                                }
                            }
                        }

                        let target_pos = if max_bot_pos > 0 { max_bot_pos - 1 } else { 1 };
                        let _ = rest.modify_role_positions(guild_id, role_id, target_pos).await;
                    }
                }
            }
        }
    }

    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;

    db.bulk_update_antinuke(guild_id, true).await?;

    let guild_name = "Whiskey's server"; 
    let final_embed = json!({
        "title": "🛡️ RIMURU Security",
        "description": format!(
            "**Security Settings For {} 🛡️**\n\n\
            Note: For maximum security efficiency, ensure my role maintains the highest position in the role hierarchy.\n\n\
            **Modules Enabled 🛡️**\n\
            Anti Ban: ✅\n\
            Anti Unban: ✅\n\
            Anti Kick: ✅\n\
            Anti Bot: ✅\n\
            Anti Channel Create: ✅\n\
            Anti Channel Delete: ✅\n\
            Anti Channel Update: ✅\n\
            Anti Emoji/Sticker Create: ✅\n\
            Anti Emoji/Sticker Delete: ✅\n\
            Anti Emoji/Sticker Update: ✅\n\
            Anti Everyone/Here Ping: ✅\n\
            Anti Link Role: ✅\n\
            Anti Role Create: ✅\n\
            Anti Role Delete: ✅\n\
            Anti Role Update: ✅\n\
            Anti Role Ping: ✅\n\
            Anti Member Update: ✅\n\
            Anti Integration: ✅\n\
            Anti Server Update: ✅\n\
            Anti Automod Rule Create: ✅\n\
            Anti Automod Rule Update: ✅\n\
            Anti Automod Rule Delete: ✅\n\
            Anti Guild Event Create: ✅\n\
            Anti Guild Event Update: ✅\n\
            Anti Guild Event Delete: ✅\n\
            Anti Webhook: ✅\n\n\
            Anti Prune: ✅",
            guild_name
        ),
        "color": colors::MAIN,
        "thumbnail": { "url": bot_avatar },
        "footer": { "text": "Punishment Type: Ban" }
    });

    rest.edit_message(&msg.channel_id, msg_id, "", vec![final_embed], vec![]).await?;
    Ok(())
}

async fn show_disable_sequence(rest: &RestClient, msg: &Message, db: Arc<Database>, bot_avatar: &str) -> anyhow::Result<()> {
    let guild_id = msg.guild_id.as_deref().unwrap_or("");

    db.bulk_update_antinuke(guild_id, false).await?;

    let footer_text = format!("Security System ID: {} • Today", msg.author.id);
    let avatar_url = msg.author.avatar_url();

    let embed = json!({
        "title": format!("{} Security System Deactivated", emojis::SUCCESS),
        "description": format!(
            "**Status: Shutdown Complete**\n\
            **Protection Level: Minimum**\n\
            **System State: Disabled**\n\n\
            *The security system has been successfully deactivated. All protection protocols are now in standby mode.*\n\n\
            Type `!antinuke enable` to reactivate the security system."
        ),
        "color": colors::MAIN,
        "thumbnail": { "url": bot_avatar },
        "footer": {
            "text": footer_text,
            "icon_url": avatar_url
        }
    });

    rest.send_complex_message(&msg.channel_id, "", vec![embed], vec![]).await?;
    Ok(())
}

async fn show_settings(rest: &RestClient, msg: &Message, db: Arc<Database>, bot_avatar: &str) -> anyhow::Result<()> {
    let guild_id = msg.guild_id.as_deref().unwrap_or("");
    let settings = db.get_antinuke_settings(guild_id).await?;

    let mut enabled_list = String::new();
    let mut active_count = 0;

    let mut keys: Vec<&String> = settings.keys().collect();
    keys.sort();

    for key in keys {
        if key == "auto_recovery" { continue; } 
        if settings.get(key).cloned().unwrap_or(false) {
            let label = key.replace("anti_", "").replace("_", " ");
            let capitalized = label.split_whitespace()
                .map(|word| {
                    let mut c = word.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    }
                })
                .collect::<Vec<String>>()
                .join(" ");

            enabled_list.push_str(&format!("{} Anti {}\n", emojis::SUCCESS, capitalized));
            active_count += 1;
        }
    }

    if enabled_list.is_empty() {
        enabled_list = "No modules currently enabled.".to_string();
    }

    let embed = json!({
        "title": "RIMURU Security System",
        "description": format!(
            "**Advanced Security Configuration**\n\
            Security Status: {} Active\n\n\
            **System Overview**\n\
            • Protection Level: {}\n\
            • Active Modules: {}\n\n\
            **Enabled Security Modules:**\n\
            {}",
            if active_count > 0 { emojis::SUCCESS } else { emojis::ERROR },
            if active_count > 20 { "Maximum" } else if active_count > 10 { "Standard" } else { "Minimum" },
            active_count,
            enabled_list
        ),
        "color": colors::MAIN,
        "thumbnail": { "url": bot_avatar },
        "footer": { "text": format!("Security System ID: {} • Today", msg.id) }
    });

    rest.send_complex_message(&msg.channel_id, "", vec![embed], vec![]).await?;
    Ok(())
}

pub async fn handle_interaction(
    rest: &RestClient,
    interaction: Interaction,
    db: Arc<Database>,
) -> anyhow::Result<()> {

    let user_id = interaction.member.as_ref().and_then(|m| m.user.as_ref()).map(|u| u.id.clone()).unwrap_or_default();
    let is_bot_admin = db.is_admin(&user_id).await.unwrap_or(false);
    let mut is_owner = false;

    let guild_id = interaction.guild_id.as_deref().unwrap_or("");
    if !guild_id.is_empty() {
        if let Ok(guild) = rest.get_guild(guild_id).await {
            if let Some(owner_id) = guild["owner_id"].as_str() {
                is_owner = owner_id == user_id;
            }
        }
    }

    if !is_bot_admin && !is_owner {

        rest.interaction_callback(&interaction.id, &interaction.token, json!({
            "type": 4,
            "data": {
                "content": format!("{} Interaction Denied: You must be the **Server Owner** or a **Bot Admin** to use the Antinuke Control Panel.", emojis::ERROR),
                "flags": 64
            }
        })).await?;
        return Ok(());
    }

    let custom_id = interaction.data.as_ref().and_then(|d| d.custom_id.as_deref()).unwrap_or("");

    match custom_id {
        id if id.starts_with("toggle_") => {
            let setting = &id[7..]; 
            toggle_setting(rest, &interaction, db, setting).await?;
        }
        "antinuke_multi_select" => {
            if let Some(values) = interaction.data.as_ref().and_then(|d| d.values.as_ref()) {
                if let Some(setting) = values.first() {
                    toggle_setting_and_refresh_select(rest, &interaction, db, setting).await?;
                }
            }
        }
        "antinuke_config_menu" => {
            show_full_config(rest, &interaction, db).await?;
        }
        "antinuke_sel_menu" => {
            show_select_config(rest, &interaction, db).await?;
        }
        "antinuke_features" => {
            let bot_user = rest.validate_token().await?;
            let bot_avatar = bot_user.avatar_url();
            show_features(rest, &interaction, &bot_avatar).await?;
        }
        _ => {}
    }
    Ok(())
}

async fn toggle_setting(rest: &RestClient, interaction: &Interaction, db: Arc<Database>, setting: &str) -> anyhow::Result<()> {
    let guild_id = interaction.guild_id.as_deref().unwrap_or("");
    let settings = db.get_antinuke_settings(guild_id).await?;
    let current = settings.get(setting).cloned().unwrap_or(false);

    db.update_antinuke_setting(guild_id, setting, !current).await?;

    show_full_config(rest, interaction, db).await?;
    Ok(())
}

async fn toggle_setting_and_refresh_select(rest: &RestClient, interaction: &Interaction, db: Arc<Database>, setting: &str) -> anyhow::Result<()> {
    let guild_id = interaction.guild_id.as_deref().unwrap_or("");
    let settings = db.get_antinuke_settings(guild_id).await?;
    let current = settings.get(setting).cloned().unwrap_or(false);

    db.update_antinuke_setting(guild_id, setting, !current).await?;

    show_select_config(rest, interaction, db).await?;
    Ok(())
}

async fn show_dashboard(rest: &RestClient, msg: &Message, bot_avatar: &str) -> anyhow::Result<()> {
    let embed = json!({
        "title": "RIMURU ADVANCED SECURITY",
        "description": "Welcome to the Rimuru Advanced Security System - A military-grade solution engineered to protect your server against sophisticated nuking attempts. Deploy and manage elite security protocols through our advanced command interface.\n\n**`!antinuke enable`**\nDeploy the advanced security system with maximum protection protocols.\n\n**`!antinuke disable`**\nDeactivate the security system and its protection modules.\n\n**`!antinuke config`**\nConfigure elite security modules and their operational parameters.\n\n**`!antinuke settings`**\nAccess current security status and module operational data.",
        "color": colors::MAIN,
        "thumbnail": { "url": bot_avatar },
        "footer": { "text": "Use '!antinuke enable' to activate the security system • Today" }
    });

    let components = json!([
        {
            "type": 1,
            "components": [
                { "type": 2, "style": 5, "label": "Support Server", "url": "https://discord.gg/rimuru" },
                { "type": 2, "style": 5, "label": "Vote", "url": "https://top.gg/bot/rimuru" },
                { "type": 2, "style": 2, "label": "View Features", "custom_id": "antinuke_features" }
            ]
        }
    ]);

    rest.send_complex_message(&msg.channel_id, "", vec![embed], components.as_array().unwrap().to_vec()).await?;
    Ok(())
}

async fn show_config_menu(rest: &RestClient, msg: &Message, bot_avatar: &str) -> anyhow::Result<()> {
    let embed = json!({
        "title": "RIMURU - CONFIGURATION",
        "description": "**Select your preferred configuration method:**\n\
                        • **Button Menu**: Directly toggle events via buttons.\n\
                        • **Select Menu**: Choose multiple events from a dropdown.",
        "color": colors::MAIN,
        "thumbnail": { "url": bot_avatar }
    });

    let components = json!([
        {
            "type": 1,
            "components": [
                { "type": 2, "style": 1, "label": "Button Menu", "custom_id": "antinuke_config_menu" },
                { "type": 2, "style": 2, "label": "Select Menu", "custom_id": "antinuke_sel_menu" }
            ]
        }
    ]);

    rest.send_complex_message(&msg.channel_id, "", vec![embed], components.as_array().unwrap().to_vec()).await?;
    Ok(())
}

async fn show_full_config(rest: &RestClient, interaction: &Interaction, db: Arc<Database>) -> anyhow::Result<()> {
    let guild_id = interaction.guild_id.as_deref().unwrap_or("");
    let settings = db.get_antinuke_settings(guild_id).await?;

    let button_modules = [
        ("BAN", "anti_ban"), ("UNBAN", "anti_unban"), ("KICK", "anti_kick"), ("BOT", "anti_bot"), ("PRUNE", "anti_prune"),
        ("CH-ADD", "anti_channel_create"), ("CH-UP", "anti_channel_update"), ("CH-DEL", "anti_channel_delete"), ("ROLE-ADD", "anti_role_create"), ("ROLE-UP", "anti_role_update"),
        ("ROLE-DEL", "anti_role_delete"), ("JOIN-R", "anti_member_role_update"), ("PING", "anti_everyone_ping"), ("SRV-UP", "anti_server_update"), ("LOCK", "thread_lock_enabled"),
        ("EMO-ADD", "anti_emoji_create"), ("STK-ADD", "anti_sticker_create"), ("WB-ADD", "anti_webhook_create"), ("WB-UP", "anti_webhook_update"), ("WB-DEL", "anti_webhook_delete")
    ];

    let mut components = Vec::new();

    let mut row = Vec::new();
    for (label, key) in button_modules {
        let enabled = settings.get(key).cloned().unwrap_or(false);
        row.push(json!({
            "type": 2,
            "style": if enabled { 3 } else { 4 },
            "label": format!("{}: {}", label, if enabled { "ON" } else { "OFF" }),
            "custom_id": format!("toggle_{}", key)
        }));

        if row.len() == 5 {
            components.push(json!({ "type": 1, "components": row.clone() }));
            row.clear();
            if components.len() == 5 { break; } 
        }
    }

    rest.interaction_callback(&interaction.id, &interaction.token, json!({
        "type": 7, 
        "data": {
            "content": "**⚡ Antinuke Mission Control**\nManage all protections via buttons or the menu below.",
            "components": components,
            "flags": 64
        }
    })).await?;

    Ok(())
}

async fn show_select_config(rest: &RestClient, interaction: &Interaction, db: Arc<Database>) -> anyhow::Result<()> {
    let guild_id = interaction.guild_id.as_deref().unwrap_or("");
    let settings = db.get_antinuke_settings(guild_id).await?;

    let mut options = Vec::new();
    let mut keys: Vec<&String> = settings.keys().collect();
    keys.sort();

    for key in keys {
        let enabled = settings.get(key).cloned().unwrap_or(false);
        options.push(json!({
            "label": key.replace("anti_", "").replace("_", " ").to_uppercase(),
            "value": key,
            "description": format!("Status: {}", if enabled { "ENABLED" } else { "DISABLED" }),
            "emoji": { "name": if enabled { "🛡️" } else { "⚙️" } }
        }));
    }

    let components = json!([
        {
            "type": 1,
            "components": [
                {
                    "type": 3,
                    "custom_id": "antinuke_multi_select",
                    "options": &options[0..options.len().min(25)],
                    "placeholder": "Select a module to toggle...",
                    "min_values": 1,
                    "max_values": 1
                }
            ]
        }
    ]);

    rest.interaction_callback(&interaction.id, &interaction.token, json!({
        "type": 7,
        "data": {
            "content": "**⚙️ Antinuke Select Menu**\nChoose a module to toggle from the list.",
            "components": components,
            "flags": 64
        }
    })).await?;

    Ok(())
}

async fn show_features(rest: &RestClient, interaction: &Interaction, bot_avatar: &str) -> anyhow::Result<()> {

    let embed_mod = json!({
        "title": "🛡️ RIMURU SECURITY - CORE MODERATION",
        "description": "Elite protection protocols designed to intercept and reverse malicious moderation actions.",
        "color": colors::MAIN,
        "thumbnail": { "url": bot_avatar },
        "fields": [
            { "name": "Anti Ban", "value": "Advanced ban attempt detection system", "inline": true },
            { "name": "Anti Unban", "value": "Unauthorized unban interception protocol", "inline": true },
            { "name": "Anti Kick", "value": "Elite kick attempt detection system", "inline": true },
            { "name": "Anti Prune", "value": "Mass member pruning prevention system", "inline": true }
        ]
    });

    let embed_struct = json!({
        "title": "🏗️ RIMURU SECURITY - SERVER STRUCTURES",
        "description": "Protects your server's architectural integrity, including channels and roles.",
        "color": colors::MAIN,
        "thumbnail": { "url": bot_avatar },
        "fields": [
            { "name": "Anti Channel", "value": "Create/Delete/Update detection & reversal", "inline": true },
            { "name": "Anti Role", "value": "Role modification and deletion prevention", "inline": true },
            { "name": "Anti Member", "value": "Member role and permission protection", "inline": true },
            { "name": "Anti Server", "value": "Server settings and vanity modification security", "inline": true }
        ]
    });

    let embed_assets = json!({
        "title": "🎨 RIMURU SECURITY - ASSET PROTECTION",
        "description": "Monitors and secures your server's visual and interactive assets.",
        "color": colors::MAIN,
        "thumbnail": { "url": bot_avatar },
        "fields": [
            { "name": "Anti Emoji", "value": "Emoji creation, deletion, and update monitoring", "inline": true },
            { "name": "Anti Sticker", "value": "Sticker creation, deletion, and update monitoring", "inline": true },
            { "name": "Anti Webhook", "value": "Webhook creation and modification interception", "inline": true }
        ]
    });

    let embed_adv = json!({
        "title": "⚙️ RIMURU SECURITY - ADVANCED COGS",
        "description": "High-performance modules for detecting sophisticated automation and raids.",
        "color": colors::MAIN,
        "thumbnail": { "url": bot_avatar },
        "fields": [
            { "name": "Anti Bot", "value": "Elite bot addition prevention system", "inline": true },
            { "name": "Anti Ping", "value": "Mass @everyone and @here ping prevention", "inline": true },
            { "name": "Anti Automod", "value": "Automod rule creation and modification security", "inline": true },
            { "name": "Anti Events", "value": "Guild event creation and modification monitoring", "inline": true },
            { "name": "Thread Lock", "value": "Active thread locking to stop raid propagation", "inline": true },
            { "name": "Auto Recovery", "value": "Advanced attack recovery and restoration protocol", "inline": true }
        ],
        "description": "🔴 **Each protocol is designed to provide maximum security against unauthorized actions**",
        "footer": { "text": "Rimuru Advanced Security • !antinuke config" }
    });

    rest.interaction_callback(&interaction.id, &interaction.token, json!({
        "type": 4, 
        "data": {
            "embeds": [embed_mod, embed_struct, embed_assets, embed_adv],
            "flags": 64
        }
    })).await?;

    Ok(())
}
