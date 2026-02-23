use crate::models::{Interaction, Message};
use crate::rest::RestClient;
use crate::db::Database;
use crate::constants::emojis;
use std::sync::Arc;

pub async fn handle_whitelist(
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
        rest.send_message(&msg.channel_id, &format!("{} This command is restricted to Server Owners and Bot Admins only.", emojis::ERROR)).await?;
        return Ok(());
    }
    let parts: Vec<&str> = args.split_whitespace().collect();
    let sub_cmd = parts.first().unwrap_or(&"");

    match *sub_cmd {
        "add" => {
            let target = parts.get(1).unwrap_or(&"");
            let user_id = extract_id(target);
            if user_id.is_empty() {
                rest.send_message(&msg.channel_id, &format!("{} Please mention a valid user to whitelist.", emojis::ERROR)).await?;
                return Ok(());
            }
            db.add_whitelist(user_id, target).await?;
            rest.send_message(&msg.channel_id, &format!("{} Added <@{}> to the whitelist.", emojis::SUCCESS, user_id)).await?;
        }
        "remove" => {
            let target = parts.get(1).unwrap_or(&"");
            let user_id = extract_id(target);
            if user_id.is_empty() {
                rest.send_message(&msg.channel_id, &format!("{} Please mention a valid user to remove.", emojis::ERROR)).await?;
                return Ok(());
            }
            db.remove_whitelist(user_id).await?;
            rest.send_message(&msg.channel_id, &format!("{} Removed <@{}> from the whitelist.", emojis::SUCCESS, user_id)).await?;
        }
        "list" => {
            let list = db.list_whitelist().await?;
            if list.is_empty() {
                rest.send_message(&msg.channel_id, &format!("{} The whitelist is currently empty.", emojis::INFO)).await?;
            } else {
                let mut content = format!("{} **Whitelisted Users:**\n", emojis::BOOK);
                for (id, name) in list {
                    content.push_str(&format!("• <@{}> ({})\n", id, name));
                }
                rest.send_message(&msg.channel_id, &content).await?;
            }
        }
        "" => {

            rest.send_message(&msg.channel_id, &format!("{} Use `add`, `remove`, or `list` subcommands.", emojis::SHIELD)).await?;
        }
        _ => {
            rest.send_message(&msg.channel_id, &format!("{} Unknown subcommand. Try `add`, `remove`, `list`.", emojis::ERROR)).await?;
        }
    }
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

        rest.interaction_callback(&interaction.id, &interaction.token, serde_json::json!({
            "type": 4,
            "data": {
                "content": format!("{} Interaction Denied: You must be the Server Owner or a Bot Admin to use the Whitelist Control Panel.", emojis::ERROR),
                "flags": 64
            }
        })).await?;
        return Ok(());
    }

    Ok(())
}

pub fn extract_id(target: &str) -> &str {
    if let Some(id) = target.strip_prefix("<@").and_then(|s| s.strip_suffix(">")) {
        let id = id.strip_prefix("!").unwrap_or(id);
        let id = id.strip_prefix("&").unwrap_or(id);
        return id;
    }
    ""
}
