use crate::commands::{fun, general, security};
use crate::models::{Interaction, Message};
use crate::rest::RestClient;
use crate::db::Database;
use crate::constants::emojis;
use std::sync::Arc;
use tracing::{info, warn, error};

pub async fn handle_message(msg: Message, rest: Arc<RestClient>, prefix: char, db: Arc<Database>) {
    if msg.author.bot { return; }

    let active_prefix = if let Some(guild_id) = &msg.guild_id {
        db.get_prefix(guild_id).await
    } else {
        prefix.to_string()
    };

    if !msg.content.starts_with(&active_prefix) {
        return;
    }

    let content = &msg.content[active_prefix.len()..];
    let mut parts = content.split_whitespace();
    let cmd = match parts.next() {
        Some(c) => c.to_lowercase(),
        None => return,
    };
    let args = parts.collect::<Vec<&str>>().join(" ");

    if cmd != "ignore" && cmd != "unignore" && cmd != "help" {
        if let Some(guild_id) = &msg.guild_id {

            if db.is_ignored_channel(guild_id, &msg.channel_id).await.unwrap_or(false) {
                return;
            }

            if db.is_ignored_user(guild_id, &msg.author.id).await.unwrap_or(false) {
                return;
            }
        }
    }

    info!("Command \"{}\" from {} (args: \"{}\")", cmd, msg.author.username, args);

    let result = match cmd.as_str() {
        "ping" => general::ping(&rest, &msg).await,
        "info" => general::info(&rest, &msg).await,
        "help" => general::help(&rest, &msg).await,

        "security" | "s" | "sec" | "antinuke" | "admin" | "adm" | "extraowner" | "eo" | 
        "whitelist" | "wl" | "wlist" | "unwhitelist" | "uwl" | "nightmode" | "nm" => {
            security::security_cmd(&rest, &msg, Arc::clone(&db), &cmd, &args).await
        }

        "ban" | "kick" | "softban" | "unban" | "unbanall" | "mute" | "unmute" | "unmuteall" | "nick" | "slowmode" |
        "lock" | "unlock" | "lockall" | "unlockall" | "hide" | "unhide" | "hideall" | "unhideall" | "block" | "unblock" |
        "purge" | "clear" | "p" | "c" |
        "list" | "l" | "role" | "r" |
        "warn" | "warning" | "command" | "ignore" | "unignore" | "prefix" => {
            crate::commands::moderation::handle_command(&rest, &msg, Arc::clone(&db), &cmd, &args).await
        }

        "8ball" | "eightball" => fun::eight_ball(&rest, &msg, &args).await,
        "roll" => fun::roll(&rest, &msg, &args).await,
        "coinflip" => fun::coinflip(&rest, &msg).await,

        _ => {
            warn!("Unknown command: {} (args: {})", cmd, args);
            return;
        }
    };

    if let Err(e) = result {
        error!("Error executing command \"{}\": {:?}", cmd, e);
    }
}

pub async fn handle_interaction(interaction: Interaction, rest: Arc<RestClient>, db: Arc<Database>) {
    let custom_id = interaction.data.as_ref().and_then(|d| d.custom_id.as_deref()).unwrap_or("");

    if custom_id.starts_with("help_") {
        if let Err(e) = general::handle_help_interaction(&rest, &interaction).await {
            error!("Error handling help interaction: {:?}", e);
        }
    } else if let Err(e) = security::handle_interaction(&rest, interaction, db).await {
        error!("Error handling security interaction: {:?}", e);
    }
}
