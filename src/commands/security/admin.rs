use crate::models::{Interaction, Message};
use crate::rest::RestClient;
use crate::db::Database;
use crate::constants::emojis;
use crate::commands::security::whitelist::extract_id;
use std::sync::Arc;

pub async fn handle_admin(
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
            if !is_owner {
                rest.send_message(&msg.channel_id, &format!("{} This command is strictly reserved for the **Server Owner**.", emojis::ERROR)).await?;
                return Ok(());
            }
            let target = parts.get(1).unwrap_or(&"");
            let user_id = extract_id(target);
            if user_id.is_empty() {
                rest.send_message(&msg.channel_id, &format!("{} Please mention a valid user to add as admin.", emojis::ERROR)).await?;
                return Ok(());
            }
            db.add_admin(user_id, target).await?;
            rest.send_message(&msg.channel_id, &format!("{} Added <@{}> to the admin list.", emojis::SUCCESS, user_id)).await?;
        }
        "remove" => {
            if !is_owner {
                rest.send_message(&msg.channel_id, &format!("{} This command is strictly reserved for the **Server Owner**.", emojis::ERROR)).await?;
                return Ok(());
            }
            let target = parts.get(1).unwrap_or(&"");
            let user_id = extract_id(target);
            if user_id.is_empty() {
                rest.send_message(&msg.channel_id, &format!("{} Please mention a valid user to remove.", emojis::ERROR)).await?;
                return Ok(());
            }
            if user_id == msg.author.id {
                rest.send_message(&msg.channel_id, &format!("{} You cannot remove yourself from the admin list.", emojis::ERROR)).await?;
                return Ok(());
            }
            db.remove_admin(user_id).await?;
            rest.send_message(&msg.channel_id, &format!("{} Removed <@{}> from the admin list.", emojis::SUCCESS, user_id)).await?;
        }
        "list" => {
            let list = db.list_admins().await?;
            if list.is_empty() {
                rest.send_message(&msg.channel_id, &format!("{} No admins configured.", emojis::INFO)).await?;
            } else {
                let mut content = format!("{} **Admin Users:**\n", emojis::WRENCH);
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
    _rest: &RestClient,
    _interaction: Interaction,
    _db: Arc<Database>,
) -> anyhow::Result<()> {
    Ok(())
}
