use crate::constants::{colors, emojis};
use crate::models::Message;
use crate::rest::RestClient;
use serde_json::json;
use std::time::Instant;

pub async fn ping(rest: &RestClient, msg: &Message) -> anyhow::Result<()> {
    let start = Instant::now();

    let _ = rest.validate_token().await;
    let ms = start.elapsed().as_millis();
    rest.send_message(
        &msg.channel_id,
        &format!("{} Pong! REST Latency: **{}ms**", emojis::PING, ms),
    )
    .await?;
    Ok(())
}

pub async fn info(rest: &RestClient, msg: &Message) -> anyhow::Result<()> {
    let embed = json!({
        "title": format!("About rimuru-bot {}", emojis::RUST),
        "description": format!(
            "A Discord bot written in **pure Rust** {} — no wrappers, raw WebSocket + HTTP.",
            emojis::RUST
        ),
        "color": colors::BLURPLE,
        "fields": [
            { "name": "Language",  "value": format!("Rust {}", emojis::RUST),          "inline": true },
            { "name": "Gateway",   "value": "Raw WebSocket (RFC 6455)",                 "inline": true },
            { "name": "REST",      "value": "Raw HTTP (reqwest)",                       "inline": true },
            { "name": "Wrapper",   "value": format!("{} None", emojis::ERROR),          "inline": true }
        ],
        "footer": { "text": "No wrapper. Just raw protocol." }
    });
    rest.send_embed(&msg.channel_id, embed).await?;
    Ok(())
}

pub async fn help(rest: &RestClient, msg: &Message) -> anyhow::Result<()> {
    let embed = json!({
        "title": format!("{} rimuru-bot Help Center", emojis::HELP),
        "description": format!(
            "Welcome to the **rimuru-bot** command dashboard! Use the buttons below or the dropdown menu to navigate between categories.\n\n\
            {} **Quick Navigation:**\n\
            • **Home**: This overview page\n\
            • **Commands List**: All available commands summarized\n\
            • **Buttons Menu**: Interaction demo",
            emojis::BOLT
        ),
        "color": colors::BLURPLE,
        "fields": [
            {
                "name": format!("{} Categories", emojis::FOLDER),
                "value": format!(
                    "{} **General** — System & Info\n\
                    {} **Fun** — Games & RNG\n\
                    {} **Security** — Admin & Protection\n\
                    {} **Moderation** — Server Management",
                    emojis::WRENCH, emojis::DICE, emojis::SHIELD, emojis::HAMMER
                ),
                "inline": false
            }
        ],
        "footer": { "text": "Select a category below to get started!" }
    });

    let components = json!([
        {
            "type": 1,
            "components": [
                {
                    "type": 2,
                    "style": 2,
                    "label": "Home",
                    "emoji": { "name": "🏠" },
                    "custom_id": "help_home"
                },
                {
                    "type": 2,
                    "style": 2,
                    "label": "Commands List",
                    "emoji": { "name": "🗒️" },
                    "custom_id": "help_commands_list"
                }
            ]
        },
        {
            "type": 1,
            "components": [
                {
                    "type": 3,
                    "custom_id": "help_category_select",
                    "options": [
                        { "label": "General", "value": "help_cat_general", "description": "System and Info commands", "emoji": { "name": "🔧" } },
                        { "label": "Fun", "value": "help_cat_fun", "description": "Games and RNG commands", "emoji": { "name": "🎲" } },
                        { "label": "Security", "value": "help_cat_security", "description": "Anti-nuke and Admin commands", "emoji": { "name": "🛡️" } },
                        { "label": "Moderation", "value": "help_cat_moderation", "description": "Server Management commands", "emoji": { "name": "🔨" } }
                    ],
                    "placeholder": "Choose a Category"
                }
            ]
        }
    ]);

    rest.send_complex_message(
        &msg.channel_id,
        &format!("{} My help is here for you, **{}**!", emojis::SPARKLE, msg.author.username),
        vec![embed],
        vec![components[0].clone(), components[1].clone()]
    ).await?;

    Ok(())
}

pub async fn handle_help_interaction(rest: &RestClient, interaction: &crate::models::Interaction) -> anyhow::Result<()> {
    let data = match &interaction.data {
        Some(d) => d,
        None => return Ok(()),
    };

    let custom_id = data.custom_id.as_deref().unwrap_or("");

    match custom_id {
        "help_home" => {

            rest.interaction_callback(&interaction.id, &interaction.token, json!({
                "type": 4,
                "data": { "content": "You are already home!", "flags": 64 }
            })).await?;
        }
        "help_cat_general" | "help_cat_fun" | "help_cat_security" | "help_cat_moderation" | "help_category_select" => {
            let category = if custom_id == "help_category_select" {
                data.values.as_ref().and_then(|v| v.first()).map(|s| s.as_str()).unwrap_or("")
            } else {
                custom_id
            };

            let (title, content) = match category {
                "help_cat_general" => (format!("{} General Commands", emojis::WRENCH), "`!ping`, `!info`, `!help`".to_string()),
                "help_cat_fun" => (format!("{} Fun Commands", emojis::DICE), "`!8ball`, `!roll`, `!coinflip`".to_string()),
                "help_cat_security" => (format!("{} Security Commands", emojis::SHIELD), "`!security`, `!whitelist`, `!admin`".to_string()),
                "help_cat_moderation" => (
                    format!("{} Moderation Commands", emojis::HAMMER),
                    "**Commands:**\n\
                    `!ban` - Ban a user from the server\n\
                    `!block` - Block a user from a channel\n\
                    `!hide` - Hide a channel for @everyone (deny ViewChannel)\n\
                    `!hideall` - Hide all channels in the server\n\
                    `!kick` - Kick a user from the server\n\
                    `!lock` - Lock a channel for @everyone (deny SendMessages)\n\
                    `!mute` (`timeout`, `stfu`) - Timeout a member for a specified duration\n\
                    `!nick` - Change or reset a member's nickname\n\
                    `!prefix` - Set the prefix for the bot\n\
                    `!slowmode` - Set slowmode for a channel\n\
                    `!softban` - Softban a user (ban and instantly unban to delete messages)\n\
                    `!unbanall` - Unban all banned users in the server\n\
                    `!unblock` - Unblock a user from a channel\n\
                    `!unhide` - Unhide a channel for @everyone (allow ViewChannel)\n\
                    `!unhideall` - Unhide all channels in the server\n\
                    `!unlock` - Unlock a channel for @everyone (allow SendMessages)\n\
                    `!unmute` (`untimeout`) - Remove timeout from a member\n\
                    `!unmuteall` - Unmute all muted members in the server\n\
                    `!lockall` - Lock all channels in the server\n\
                    `!unban` - Unban a user from the server\n\
                    `!unlockall` - Unlock all channels in the server\n\n\
                    **🔧 Commands with Subcommands:**\n\
                    `!command` (`cmdsettings`, `config`) - Manage enabled/disabled commands\n\
                    `!ignore` - Manage ignored channels, roles, and users\n\
                    `!list` (`l`) - List various member/role types (`bots`, `admin`, `muted`, `roles`, `bans`, `channels`)\n\
                    `!purge` (`clear`, `p`, `c`) - Bulk delete messages (`@user`, `bots`, `humans`, `links`, `attachments`, `mentions`, `contains`, `0-100`)\n\
                    `!role` (`r`) - Manage Roles (`user`, `all`, `humans`, `bots`)\n\
                    `!warn` (`warning`) - Warn a member or manage warnings (`list`, `clear`, `remove`, `@member`)\n".to_string()
                ),
                _ => ("Unknown".to_string(), "No info available".to_string())
            };

            rest.interaction_callback(&interaction.id, &interaction.token, json!({
                "type": 4,
                "data": {
                    "embeds": [{
                        "title": title,
                        "description": content,
                        "color": colors::BLURPLE
                    }],
                    "flags": 64
                }
            })).await?;
        }
        _ => {}
    }

    Ok(())
}
