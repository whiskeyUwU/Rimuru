use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use std::collections::{HashMap, HashSet, VecDeque};
use tokio::time::Instant;
use tracing::{info, error};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
    pub ban_tracker: Arc<RwLock<HashMap<String, VecDeque<Instant>>>>,
    pub channel_tracker: Arc<RwLock<HashMap<String, VecDeque<Instant>>>>,

    pub settings_cache: Arc<RwLock<HashMap<String, Arc<HashMap<String, bool>>>>>,
    pub whitelist_cache: Arc<RwLock<HashSet<String>>>,
    pub admin_cache: Arc<RwLock<HashSet<String>>>,
    pub prefix_cache: Arc<RwLock<HashMap<String, String>>>,
}

impl Database {
    pub async fn new(path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            ban_tracker: Arc::new(RwLock::new(HashMap::new())),
            channel_tracker: Arc::new(RwLock::new(HashMap::new())),
            settings_cache: Arc::new(RwLock::new(HashMap::new())),
            whitelist_cache: Arc::new(RwLock::new(HashSet::new())),
            admin_cache: Arc::new(RwLock::new(HashSet::new())),
            prefix_cache: Arc::new(RwLock::new(HashMap::new())),
        };
        db.init().await?;
        db.preload_caches().await?;
        Ok(db)
    }

    async fn preload_caches(&self) -> anyhow::Result<()> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare("SELECT user_id FROM whitelist")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut wl_cache = self.whitelist_cache.write().await;
        for row in rows {
            if let Ok(id) = row {
                wl_cache.insert(id);
            }
        }

        let mut stmt_admin = conn.prepare("SELECT user_id FROM admins")?;
        let rows_admin = stmt_admin.query_map([], |row| row.get::<_, String>(0))?;
        let mut admin_cache = self.admin_cache.write().await;
        for row in rows_admin {
            if let Ok(id) = row {
                admin_cache.insert(id);
            }
        }
        Ok(())
    }

    async fn init(&self) -> anyhow::Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS whitelist (
                user_id TEXT PRIMARY KEY,
                username TEXT
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS admins (
                user_id TEXT PRIMARY KEY,
                username TEXT
            )",
            [],
        )?;

        conn.execute("CREATE TABLE IF NOT EXISTS warnings (id INTEGER PRIMARY KEY AUTOINCREMENT, guild_id TEXT, user_id TEXT, reason TEXT, moderator TEXT, timestamp DATETIME DEFAULT CURRENT_TIMESTAMP)", [])?;
        conn.execute("CREATE TABLE IF NOT EXISTS ignored_channels (guild_id TEXT, channel_id TEXT, PRIMARY KEY (guild_id, channel_id))", [])?;
        conn.execute("CREATE TABLE IF NOT EXISTS ignored_roles (guild_id TEXT, role_id TEXT, PRIMARY KEY (guild_id, role_id))", [])?;
        conn.execute("CREATE TABLE IF NOT EXISTS bypass_users (guild_id TEXT, user_id TEXT, PRIMARY KEY (guild_id, user_id))", [])?;
        conn.execute("CREATE TABLE IF NOT EXISTS disabled_commands (guild_id TEXT, command_name TEXT, PRIMARY KEY (guild_id, command_name))", [])?;
        conn.execute("CREATE TABLE IF NOT EXISTS prefixes (guild_id TEXT PRIMARY KEY, prefix TEXT)", [])?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS antinuke_config (
                guild_id TEXT PRIMARY KEY,
                anti_ban INTEGER DEFAULT 0,
                anti_unban INTEGER DEFAULT 0,
                anti_kick INTEGER DEFAULT 0,
                anti_bot INTEGER DEFAULT 0,
                anti_prune INTEGER DEFAULT 0,
                anti_channel_create INTEGER DEFAULT 0,
                anti_channel_update INTEGER DEFAULT 0,
                anti_channel_delete INTEGER DEFAULT 0,
                anti_role_create INTEGER DEFAULT 0,
                anti_role_update INTEGER DEFAULT 0,
                anti_role_delete INTEGER DEFAULT 0,
                anti_member_role_update INTEGER DEFAULT 0,
                anti_everyone_ping INTEGER DEFAULT 0,
                anti_server_update INTEGER DEFAULT 0,
                anti_emoji_create INTEGER DEFAULT 0,
                anti_emoji_update INTEGER DEFAULT 0,
                anti_emoji_delete INTEGER DEFAULT 0,
                anti_sticker_create INTEGER DEFAULT 0,
                anti_sticker_update INTEGER DEFAULT 0,
                anti_sticker_delete INTEGER DEFAULT 0,
                anti_webhook_create INTEGER DEFAULT 0,
                anti_webhook_update INTEGER DEFAULT 0,
                anti_webhook_delete INTEGER DEFAULT 0,
                anti_automod_create INTEGER DEFAULT 0,
                anti_automod_update INTEGER DEFAULT 0,
                anti_automod_delete INTEGER DEFAULT 0,
                anti_guild_event_create INTEGER DEFAULT 0,
                anti_guild_event_update INTEGER DEFAULT 0,
                anti_guild_event_delete INTEGER DEFAULT 0,
                auto_recovery INTEGER DEFAULT 0,
                thread_lock_enabled INTEGER DEFAULT 1
            )",
            [],
        )?;

        self.ensure_columns(&conn).await?;

        Ok(())
    }

    async fn ensure_columns(&self, conn: &Connection) -> anyhow::Result<()> {
        let columns = [
            "anti_ban", "anti_unban", "anti_kick", "anti_bot", "anti_prune",
            "anti_channel_create", "anti_channel_update", "anti_channel_delete",
            "anti_role_create", "anti_role_update", "anti_role_delete",
            "anti_member_role_update", "anti_everyone_ping", "anti_server_update",
            "anti_emoji_create", "anti_emoji_update", "anti_emoji_delete",
            "anti_sticker_create", "anti_sticker_update", "anti_sticker_delete",
            "anti_webhook_create", "anti_webhook_update", "anti_webhook_delete",
            "anti_automod_create", "anti_automod_update", "anti_automod_delete",
            "anti_guild_event_create", "anti_guild_event_update", "anti_guild_event_delete",
            "auto_recovery", "thread_lock_enabled"
        ];

        for col in columns {
            let mut stmt = conn.prepare("SELECT count(*) FROM pragma_table_info('antinuke_config') WHERE name = ?")?;
            let exists: i32 = stmt.query_row([col], |row| row.get(0)).unwrap_or(0);

            if exists == 0 {
                let default_val = if col == "thread_lock_enabled" { "1" } else { "0" };
                let sql = format!("ALTER TABLE antinuke_config ADD COLUMN {} INTEGER DEFAULT {}", col, default_val);
                match conn.execute(&sql, []) {
                    Ok(_) => info!("Migration: Added column {} to antinuke_config", col),
                    Err(e) => {

                        if !e.to_string().contains("duplicate column name") {
                             error!("Migration failed for column {}: {:?}", col, e);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn update_antinuke_setting(&self, guild_id: &str, setting: &str, enabled: bool) -> anyhow::Result<()> {
        let val = if enabled { 1 } else { 0 };

        {
            let conn = self.conn.lock().await;
            let sql = format!(
                "INSERT INTO antinuke_config (guild_id, {}) VALUES (?, ?) \
                 ON CONFLICT(guild_id) DO UPDATE SET {} = ?",
                setting, setting
            );
            conn.execute(&sql, [guild_id, &val.to_string(), &val.to_string()])?;
        }

        let mut cache = self.settings_cache.write().await;
        let mut new_settings = cache.get(guild_id).map(|s| (**s).clone()).unwrap_or_default();
        new_settings.insert(setting.to_string(), enabled);
        cache.insert(guild_id.to_string(), Arc::new(new_settings));

        Ok(())
    }

    pub async fn bulk_update_antinuke(&self, guild_id: &str, enabled: bool) -> anyhow::Result<()> {
        let columns = [
            "anti_ban", "anti_unban", "anti_kick", "anti_bot", "anti_prune",
            "anti_channel_create", "anti_channel_update", "anti_channel_delete",
            "anti_role_create", "anti_role_update", "anti_role_delete",
            "anti_member_role_update", "anti_everyone_ping", "anti_server_update",
            "anti_emoji_create", "anti_emoji_update", "anti_emoji_delete",
            "anti_sticker_create", "anti_sticker_update", "anti_sticker_delete",
            "anti_webhook_create", "anti_webhook_update", "anti_webhook_delete",
            "anti_automod_create", "anti_automod_update", "anti_automod_delete",
            "anti_guild_event_create", "anti_guild_event_update", "anti_guild_event_delete",
            "auto_recovery", "thread_lock_enabled"
        ];

        let val = if enabled { 1 } else { 0 };

        {
            let conn = self.conn.lock().await;
            let sets: Vec<String> = columns.iter().map(|c| format!("{} = {}", c, val)).collect();
            let sql = format!(
                "INSERT INTO antinuke_config (guild_id) VALUES (?) \
                 ON CONFLICT(guild_id) DO UPDATE SET {}",
                sets.join(", ")
            );
            conn.execute(&sql, [guild_id])?;
        }

        let mut cache = self.settings_cache.write().await;
        let mut settings = HashMap::new();
        for col in columns {
            settings.insert(col.to_string(), enabled);
        }
        cache.insert(guild_id.to_string(), Arc::new(settings));

        Ok(())
    }

    pub async fn get_antinuke_settings(&self, guild_id: &str) -> anyhow::Result<Arc<HashMap<String, bool>>> {

        {
            let cache = self.settings_cache.read().await;
            if let Some(settings) = cache.get(guild_id) {
                return Ok(settings.clone());
            }
        }

        let columns = [
            "anti_ban", "anti_unban", "anti_kick", "anti_bot", "anti_prune",
            "anti_channel_create", "anti_channel_update", "anti_channel_delete",
            "anti_role_create", "anti_role_update", "anti_role_delete",
            "anti_member_role_update", "anti_everyone_ping", "anti_server_update",
            "anti_emoji_create", "anti_emoji_update", "anti_emoji_delete",
            "anti_sticker_create", "anti_sticker_update", "anti_sticker_delete",
            "anti_webhook_create", "anti_webhook_update", "anti_webhook_delete",
            "anti_automod_create", "anti_automod_update", "anti_automod_delete",
            "anti_guild_event_create", "anti_guild_event_update", "anti_guild_event_delete",
            "auto_recovery", "thread_lock_enabled"
        ];

        let mut settings = HashMap::new();
        {
            let conn = self.conn.lock().await;
            let sql = format!("SELECT {} FROM antinuke_config WHERE guild_id = ?", columns.join(", "));
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query([guild_id])?;

            if let Some(row) = rows.next()? {
                for (i, col) in columns.iter().enumerate() {
                    let val: i32 = row.get(i)?;
                    settings.insert(col.to_string(), val == 1);
                }
            } else {
                for col in columns {
                    let default = if col == "thread_lock_enabled" { true } else { false };
                    settings.insert(col.to_string(), default);
                }
            }
        }

        let arc_settings = Arc::new(settings);

        {
            let mut cache = self.settings_cache.write().await;
            cache.insert(guild_id.to_string(), arc_settings.clone());
        }

        Ok(arc_settings)
    }

    pub async fn add_whitelist(&self, user_id: &str, username: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO whitelist (user_id, username) VALUES (?, ?) ON CONFLICT(user_id) DO UPDATE SET username = ?",
            [user_id, username, username],
        )?;
        drop(conn);
        let mut cache = self.whitelist_cache.write().await;
        cache.insert(user_id.to_string());
        Ok(())
    }

    pub async fn remove_whitelist(&self, user_id: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM whitelist WHERE user_id = ?", [user_id])?;
        drop(conn);
        let mut cache = self.whitelist_cache.write().await;
        cache.remove(user_id);
        Ok(())
    }

    pub async fn is_whitelisted(&self, user_id: &str) -> anyhow::Result<bool> {
        let cache = self.whitelist_cache.read().await;
        Ok(cache.contains(user_id))
    }

    pub async fn list_whitelist(&self) -> anyhow::Result<Vec<(String, String)>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT user_id, username FROM whitelist")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;

        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    }

    pub async fn add_admin(&self, user_id: &str, username: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO admins (user_id, username) VALUES (?, ?) ON CONFLICT(user_id) DO UPDATE SET username = ?",
            [user_id, username, username],
        )?;
        drop(conn);
        let mut cache = self.admin_cache.write().await;
        cache.insert(user_id.to_string());
        Ok(())
    }

    pub async fn remove_admin(&self, user_id: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM admins WHERE user_id = ?", [user_id])?;
        drop(conn);
        let mut cache = self.admin_cache.write().await;
        cache.remove(user_id);
        Ok(())
    }

    pub async fn is_admin(&self, user_id: &str) -> anyhow::Result<bool> {
        let cache = self.admin_cache.read().await;
        Ok(cache.contains(user_id))
    }

    pub async fn list_admins(&self) -> anyhow::Result<Vec<(String, String)>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT user_id, username FROM admins")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;

        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    }

    pub async fn get_prefix(&self, guild_id: &str) -> String {
        {
            let cache = self.prefix_cache.read().await;
            if let Some(p) = cache.get(guild_id) {
                return p.clone();
            }
        }
        let prefix = {
            let conn = self.conn.lock().await;
            let mut stmt = conn.prepare("SELECT prefix FROM prefixes WHERE guild_id = ?").unwrap();
            stmt.query_row([guild_id], |row| row.get(0)).unwrap_or_else(|_| "!".to_string())
        };
        let mut cache = self.prefix_cache.write().await;
        cache.insert(guild_id.to_string(), prefix.clone());
        prefix
    }

    pub async fn set_prefix(&self, guild_id: &str, prefix: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().await;
        conn.execute("INSERT INTO prefixes (guild_id, prefix) VALUES (?, ?) ON CONFLICT(guild_id) DO UPDATE SET prefix = ?", [guild_id, prefix, prefix])?;
        drop(conn);
        let mut cache = self.prefix_cache.write().await;
        cache.insert(guild_id.to_string(), prefix.to_string());
        Ok(())
    }

    pub async fn add_warning(&self, guild_id: &str, user_id: &str, reason: &str, moderator: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().await;
        conn.execute("INSERT INTO warnings (guild_id, user_id, reason, moderator) VALUES (?, ?, ?, ?)", [guild_id, user_id, reason, moderator])?;
        Ok(())
    }

    pub async fn get_warnings(&self, guild_id: &str, user_id: &str) -> anyhow::Result<Vec<(i64, String, String, String)>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT id, reason, moderator, timestamp FROM warnings WHERE guild_id = ? AND user_id = ? ORDER BY id DESC")?;
        let rows = stmt.query_map([guild_id, user_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?;
        let mut list = Vec::new();
        for row in rows { list.push(row?); }
        Ok(list)
    }

    pub async fn remove_warning(&self, guild_id: &str, warning_id: i64) -> anyhow::Result<usize> {
        let conn = self.conn.lock().await;
        let changes = conn.execute("DELETE FROM warnings WHERE guild_id = ? AND id = ?", rusqlite::params![guild_id, warning_id])?;
        Ok(changes)
    }

    pub async fn clear_warnings(&self, guild_id: &str, user_id: &str) -> anyhow::Result<usize> {
        let conn = self.conn.lock().await;
        let changes = conn.execute("DELETE FROM warnings WHERE guild_id = ? AND user_id = ?", [guild_id, user_id])?;
        Ok(changes)
    }

    pub async fn is_command_disabled(&self, guild_id: &str, cmd: &str) -> anyhow::Result<bool> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT 1 FROM disabled_commands WHERE guild_id = ? AND command_name = ?")?;
        let exists: i32 = stmt.query_row([guild_id, cmd], |row| row.get(0)).unwrap_or(0);
        Ok(exists == 1)
    }

    pub async fn toggle_command(&self, guild_id: &str, cmd: &str, disable: bool) -> anyhow::Result<()> {
        let conn = self.conn.lock().await;
        if disable {
            conn.execute("INSERT OR IGNORE INTO disabled_commands (guild_id, command_name) VALUES (?, ?)", [guild_id, cmd])?;
        } else {
            conn.execute("DELETE FROM disabled_commands WHERE guild_id = ? AND command_name = ?", [guild_id, cmd])?;
        }
        Ok(())
    }

    pub async fn is_ignored_channel(&self, guild_id: &str, channel_id: &str) -> anyhow::Result<bool> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT 1 FROM ignored_channels WHERE guild_id = ? AND channel_id = ?")?;
        let exists: i32 = stmt.query_row([guild_id, channel_id], |row| row.get(0)).unwrap_or(0);
        Ok(exists == 1)
    }

    pub async fn is_ignored_role(&self, guild_id: &str, role_id: &str) -> anyhow::Result<bool> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT 1 FROM ignored_roles WHERE guild_id = ? AND role_id = ?")?;
        let exists: i32 = stmt.query_row([guild_id, role_id], |row| row.get(0)).unwrap_or(0);
        Ok(exists == 1)
    }

    pub async fn is_ignored_user(&self, guild_id: &str, user_id: &str) -> anyhow::Result<bool> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT 1 FROM bypass_users WHERE guild_id = ? AND user_id = ?")?;
        let exists: i32 = stmt.query_row([guild_id, user_id], |row| row.get(0)).unwrap_or(0);
        Ok(exists == 1)
    }

    pub async fn ignore_item(&self, guild_id: &str, target_type: &str, target_id: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().await;
        let table = match target_type {
            "channel" => "ignored_channels",
            "role" => "ignored_roles",
            "user" | "bypass" => "bypass_users",
            _ => return Err(anyhow::anyhow!("Invalid ignore type")),
        };
        let col = format!("{}_id", if target_type == "bypass" { "user" } else { target_type });
        let sql = format!("INSERT OR IGNORE INTO {} (guild_id, {}) VALUES (?, ?)", table, col);
        conn.execute(&sql, [guild_id, target_id])?;
        Ok(())
    }

    pub async fn unignore_item(&self, guild_id: &str, target_type: &str, target_id: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().await;
        let table = match target_type {
            "channel" => "ignored_channels",
            "role" => "ignored_roles",
            "user" | "bypass" => "bypass_users",
            _ => return Err(anyhow::anyhow!("Invalid ignore type")),
        };
        let col = format!("{}_id", if target_type == "bypass" { "user" } else { target_type });
        let sql = format!("DELETE FROM {} WHERE guild_id = ? AND {} = ?", table, col);
        conn.execute(&sql, [guild_id, target_id])?;
        Ok(())
    }

    pub async fn get_ignored_items(&self, guild_id: &str, target_type: &str) -> anyhow::Result<Vec<String>> {
        let conn = self.conn.lock().await;
        let table = match target_type {
            "channel" => "ignored_channels",
            "role" => "ignored_roles",
            "user" | "bypass" => "bypass_users",
            _ => return Err(anyhow::anyhow!("Invalid ignore type")),
        };
        let col = format!("{}_id", if target_type == "bypass" { "user" } else { target_type });
        let sql = format!("SELECT {} FROM {} WHERE guild_id = ?", col, table);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([guild_id], |row| row.get(0))?;
        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    }
}
