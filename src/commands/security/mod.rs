pub mod admin;
pub mod whitelist;
pub mod antinuke;

use crate::models::{Interaction, Message};
use crate::rest::RestClient;
use crate::db::Database;
use std::sync::Arc;

pub async fn security_cmd(
    rest: &RestClient,
    msg: &Message,
    db: Arc<Database>,
    cmd: &str,
    args: &str,
) -> anyhow::Result<()> {
    match cmd {
        "whitelist" | "wl" | "wlist" | "unwhitelist" | "uwl" => {
            whitelist::handle_whitelist(rest, msg, db, cmd, args).await
        }
        "admin" | "adm" | "extraowner" | "eo" => {
            admin::handle_admin(rest, msg, db, cmd, args).await
        }
        "security" | "s" | "sec" | "antinuke" | "nightmode" | "nm" => {
            antinuke::handle_antinuke(rest, msg, db, cmd, args).await
        }
        _ => Ok(()),
    }
}

pub async fn handle_interaction(
    rest: &RestClient,
    interaction: Interaction,
    db: Arc<Database>,
) -> anyhow::Result<()> {
    let custom_id = interaction.data.as_ref().and_then(|d| d.custom_id.as_deref()).unwrap_or("");

    if custom_id.starts_with("antinuke_") || custom_id.starts_with("toggle_") || custom_id.contains("mass_") {
        antinuke::handle_interaction(rest, interaction, db).await
    } else if custom_id.starts_with("whitelist_") {
        whitelist::handle_interaction(rest, interaction, db).await
    } else if custom_id.starts_with("admin_") {
        admin::handle_interaction(rest, interaction, db).await
    } else {

        Ok(())
    }
}
