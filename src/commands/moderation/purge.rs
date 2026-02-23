use crate::models::Message;
use crate::rest::RestClient;
use crate::db::Database;
use crate::constants::emojis;
use std::sync::Arc;

pub async fn handle_purge(
    rest: &RestClient,
    msg: &Message,
    _db: Arc<Database>,
    _cmd: &str,
    args: &str,
) -> anyhow::Result<()> {
    let parts: Vec<&str> = args.split_whitespace().collect();

    let guild_id = msg.guild_id.as_deref().unwrap_or("");
    if guild_id.is_empty() { return Ok(()); }

    let required_perm: u64 = 1 << 13;
    let has_perm = rest.has_permission(guild_id, &msg.author.id, required_perm).await.unwrap_or(false);
    let is_bot_admin = _db.is_admin(&msg.author.id).await.unwrap_or(false);

    if !has_perm && !is_bot_admin {
        rest.send_message(&msg.channel_id, &format!("{} Permission Denied: You need the `Manage Messages` permission to use this command.", emojis::ERROR)).await?;
        return Ok(());
    }
    if parts.is_empty() {
        let help = format!(
            "{} **Purge Usage:**\n\
            `!purge <1-100>` - Delete messages\n\
            `!purge @user <1-100>` - Delete user's messages\n\
            `!purge bots <1-100>` - Delete bot messages\n\
            `!purge humans <1-100>` - Delete human messages\n\
            `!purge links <1-100>` - Delete messages with links\n\
            `!purge attachments <1-100>` - Delete messages with files\n\
            `!purge mentions <1-100>` - Delete messages with mentions\n\
            `!purge contains <text>` - Delete messages with specific text",
            emojis::INFO
        );
        rest.send_message(&msg.channel_id, &help).await?;
        return Ok(());
    }

    let filter_type = parts[0].to_lowercase();
    let mut amount: u8 = 50; 
    let mut target_id: Option<String> = None;
    let mut filter_text: Option<String> = None;

    if parts.len() > 1 {
        amount = parts[1].parse().unwrap_or(50).clamp(1, 100);
        if filter_type == "contains" {
            filter_text = Some(parts[1..].join(" ").to_lowercase());
            amount = 100; 
        }
    } else if let Ok(n) = parts[0].parse::<u8>() {
        amount = n.clamp(1, 100);
    } else if parts[0].starts_with("<@") {
        target_id = Some(parts[0].trim_matches(&['<', '@', '!', '>'][..]).to_string());
    }

    let msgs = rest.get_channel_messages(&msg.channel_id, 100).await.unwrap_or_default();
    let mut to_delete = Vec::new();

    for m in msgs.iter() {
        if to_delete.len() >= amount as usize { break; }

        let msg_id = m.get("id").and_then(|id| id.as_str()).unwrap_or("");
        let content = m.get("content").and_then(|c| c.as_str()).unwrap_or("").to_lowercase();
        let author_id = m.get("author").and_then(|a| a.get("id")).and_then(|id| id.as_str()).unwrap_or("");
        let is_bot = m.get("author").and_then(|a| a.get("bot")).and_then(|b| b.as_bool()).unwrap_or(false);
        let has_attachments = m.get("attachments").and_then(|a| a.as_array()).map_or(false, |a| !a.is_empty());
        let has_mentions = m.get("mentions").and_then(|a| a.as_array()).map_or(false, |a| !a.is_empty());

        let keep = match filter_type.as_str() {
            "bots" | "bot" => is_bot,
            "humans" | "human" => !is_bot,
            "links" | "link" => content.contains("http://") || content.contains("https://"),
            "attachments" | "attachment" | "files" => has_attachments,
            "mentions" | "mention" | "pings" => has_mentions,
            "contains" | "text" => filter_text.as_ref().map_or(false, |t| content.contains(t)),
            _ => {
                if let Some(target) = &target_id {
                    author_id == target
                } else {
                    true 
                }
            }
        };

        if keep {
            to_delete.push(msg_id.to_string());
        }
    }

    if to_delete.is_empty() {
        rest.send_message(&msg.channel_id, &format!("{} No messages found matching that filter.", emojis::INFO)).await?;
        return Ok(());
    }

    let deleted_count = to_delete.len();

    if to_delete.len() == 1 {

        rest.send_message(&msg.channel_id, &format!("{} Found 1 message, skipping bulk delete.", emojis::INFO)).await?;
    } else {
        if let Err(e) = rest.bulk_delete_messages(&msg.channel_id, to_delete).await {
            rest.send_message(&msg.channel_id, &format!("{} Failed to purge messages: {:?}", emojis::ERROR, e)).await?;
        } else {
            let _ = rest.send_message(&msg.channel_id, &format!("{} Successfully purged **{}** messages.", emojis::SUCCESS, deleted_count)).await?;

        }
    }

    Ok(())
}
