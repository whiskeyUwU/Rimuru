pub mod basic;
pub mod channels;
pub mod purge;
pub mod roles_lists;
pub mod system;

use crate::models::Message;
use crate::rest::RestClient;
use crate::db::Database;
use std::sync::Arc;

pub async fn handle_command(
    rest: &RestClient,
    msg: &Message,
    db: Arc<Database>,
    cmd: &str,
    args: &str,
) -> anyhow::Result<()> {
    match cmd {

        "ban" | "kick" | "softban" | "unban" | "unbanall" | "mute" | "unmute" | "unmuteall" | "nick" | "slowmode" => {
            basic::handle_basic(rest, msg, db, cmd, args).await
        }

        "lock" | "unlock" | "lockall" | "unlockall" | "hide" | "unhide" | "hideall" | "unhideall" | "block" | "unblock" => {
            channels::handle_channels(rest, msg, db, cmd, args).await
        }

        "purge" | "clear" | "p" | "c" => {
            purge::handle_purge(rest, msg, db, cmd, args).await
        }

        "list" | "l" | "role" | "r" => {
            roles_lists::handle_roles_lists(rest, msg, db, cmd, args).await
        }

        "warn" | "warning" | "command" | "ignore" | "unignore" | "prefix" => {
            system::handle_system(rest, msg, db, cmd, args).await
        }
        _ => Ok(()) 
    }
}
