use reqwest::{Client, header};
use serde_json::json;
use tracing::{info, error};

const BASE: &str = "https://discord.com/api/v10";

#[derive(Clone)]
pub struct RestClient {
    client: Client,
    token: String,
}

impl RestClient {
    pub fn new(token: &str) -> Self {
        let mut headers = header::HeaderMap::new();
        let auth_val = format!("Bot {}", token.trim());
        let mut auth_header = header::HeaderValue::from_str(&auth_val).expect("invalid token header");
        auth_header.set_sensitive(true);

        headers.insert(header::AUTHORIZATION, auth_header);

        let client = Client::builder()
            .default_headers(headers)
            .user_agent("DiscordBot (https://github.com/rimuru, 1.0)")
            .build()
            .expect("failed to build HTTP client");

        Self {
            client,
            token: token.to_string(),
        }
    }

    pub async fn get_gateway_url(&self) -> anyhow::Result<String> {
        let resp = self
            .client
            .get(format!("{}/gateway/bot", BASE))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("get_gateway_url failed {}: {}", status, text);
            anyhow::bail!("Discord Gateway API error {}: {}", status, text);
        }

        let body: serde_json::Value = resp.json().await?;
        let url = body["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'url' in GET /gateway/bot response: {:?}", body))?
            .to_string();

        Ok(url)
    }

    pub async fn get_guild(&self, guild_id: &str) -> anyhow::Result<serde_json::Value> {
        let resp = self
            .client
            .get(format!("{}/guilds/{}", BASE, guild_id))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("get_guild failed for channel {} with code {}: {}", guild_id, status, text);
            anyhow::bail!("API Error: {}", status);
        }

        Ok(resp.json().await?)
    }

    pub async fn validate_token(&self) -> anyhow::Result<crate::models::User> {
        let resp = self
            .client
            .get(format!("{}/users/@me", BASE))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("Token validation failed {}: {}", status, text);
            anyhow::bail!("Invalid Discord Token ({}): {}", status, text);
        }

        let body: crate::models::User = resp.json().await?;
        info!("✅ Token validated — logged in as {}#{}", body.username, body.discriminator.as_deref().unwrap_or(""));
        Ok(body)
    }

    pub async fn send_message(&self, channel_id: &str, content: &str) -> anyhow::Result<()> {

        let body = json!({ 
            "embeds": [{
                "description": content,
                "color": 0x2B2D31
            }]
        });
        let url = format!("{}/channels/{}/messages", BASE, channel_id);
        info!("POST {} (auto-embedded)", url);

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?;

        info!("Response: {}", resp.status());
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("send_message failed {}: {}", status, text);
            anyhow::bail!("Discord API error {}: {}", status, text);
        }

        Ok(())
    }

    pub async fn send_embed(
        &self,
        channel_id: &str,
        embed: serde_json::Value,
    ) -> anyhow::Result<()> {
        let body = json!({ "embeds": [embed] });
        let url = format!("{}/channels/{}/messages", BASE, channel_id);
        info!("POST {} (with embed)", url);

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?;

        info!("Response: {}", resp.status());
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("send_embed failed {}: {}", status, text);
            anyhow::bail!("Discord API error {}: {}", status, text);
        }

        Ok(())
    }

    pub async fn send_complex_message(
        &self,
        channel_id: &str,
        content: &str,
        embeds: Vec<serde_json::Value>,
        components: Vec<serde_json::Value>,
    ) -> anyhow::Result<serde_json::Value> {
        let body = json!({
            "content": content,
            "embeds": embeds,
            "components": components
        });
        let url = format!("{}/channels/{}/messages", BASE, channel_id);
        info!("POST {} (complex)", url);

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?;

        info!("Response: {}", resp.status());
        let status = resp.status();
        let body_json: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            error!("send_complex_message failed {}: {}", status, body_json);
            anyhow::bail!("Discord API error {}: {}", status, body_json);
        }

        Ok(body_json)
    }

    pub async fn edit_message(
        &self,
        channel_id: &str,
        message_id: &str,
        content: &str,
        embeds: Vec<serde_json::Value>,
        components: Vec<serde_json::Value>,
    ) -> anyhow::Result<serde_json::Value> {
        let body = json!({
            "content": content,
            "embeds": embeds,
            "components": components
        });
        let url = format!("{}/channels/{}/messages/{}", BASE, channel_id, message_id);
        info!("PATCH {}", url);

        let resp = self
            .client
            .patch(&url)
            .json(&body)
            .send()
            .await?;

        info!("Response: {}", resp.status());
        let status = resp.status();
        let body_json: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            error!("edit_message failed {}: {}", status, body_json);
            anyhow::bail!("Discord API error {}: {}", status, body_json);
        }

        Ok(body_json)
    }

    pub async fn interaction_callback(
        &self,
        interaction_id: &str,
        interaction_token: &str,
        body: serde_json::Value,
    ) -> anyhow::Result<()> {
        let resp = self
            .client
            .post(format!("{}/interactions/{}/{}/callback", BASE, interaction_id, interaction_token))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("interaction_callback failed {}: {}", status, text);
            anyhow::bail!("Discord API error {}: {}", status, text);
        }

        Ok(())
    }

    pub async fn get_active_threads(&self, guild_id: &str) -> anyhow::Result<serde_json::Value> {
        let resp = self
            .client
            .get(format!("{}/guilds/{}/threads/active", BASE, guild_id))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("get_active_threads failed {}: {}", status, text);
            anyhow::bail!("API error: {}", status);
        }

        let body: serde_json::Value = resp.json().await?;

        Ok(body.get("threads").cloned().unwrap_or(json!([])))
    }

    pub async fn get_guild_channels(&self, guild_id: &str) -> anyhow::Result<serde_json::Value> {
        let resp = self
            .client
            .get(format!("{}/guilds/{}/channels", BASE, guild_id))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("get_guild_channels failed {}: {}", status, text);
            anyhow::bail!("API error: {}", status);
        }

        let body: serde_json::Value = resp.json().await?;
        Ok(body)
    }

    pub async fn get_guild_members(&self, guild_id: &str) -> anyhow::Result<serde_json::Value> {
        let resp = self
            .client
            .get(format!("{}/guilds/{}/members?limit=1000", BASE, guild_id))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("get_guild_members failed {}: {}", status, text);
            anyhow::bail!("API error: {}", status);
        }

        let body: serde_json::Value = resp.json().await?;
        Ok(body)
    }

    pub async fn get_guild_member(&self, guild_id: &str, user_id: &str) -> anyhow::Result<serde_json::Value> {
        let resp = self
            .client
            .get(format!("{}/guilds/{}/members/{}", BASE, guild_id, user_id))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("get_guild_member failed {}: {}", status, text);
            anyhow::bail!("API error: {}", status);
        }

        Ok(resp.json().await?)
    }

    pub async fn has_permission(&self, guild_id: &str, user_id: &str, required_perm: u64) -> anyhow::Result<bool> {

        let guild = self.get_guild(guild_id).await?;
        if guild["owner_id"].as_str() == Some(user_id) {
            return Ok(true);
        }

        let member = self.get_guild_member(guild_id, user_id).await?;
        let guild_roles = self.get_guild_roles(guild_id).await?;

        let role_ids: Vec<&str> = member["roles"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();

        let mut permissions: u64 = 0;

        if let Some(everyone) = guild_roles.iter().find(|r| r["id"].as_str() == Some(guild_id)) {
            if let Some(perms_str) = everyone["permissions"].as_str() {
                if let Ok(p) = perms_str.parse::<u64>() {
                    permissions |= p;
                }
            }
        }

        for role_id in role_ids {
            if let Some(role) = guild_roles.iter().find(|r| r["id"].as_str() == Some(role_id)) {
                if let Some(perms_str) = role["permissions"].as_str() {
                    if let Ok(p) = perms_str.parse::<u64>() {
                        permissions |= p;
                    }
                }
            }
        }

        let is_admin = (permissions & (1 << 3)) != 0;
        let has_specific = (permissions & required_perm) == required_perm;

        Ok(is_admin || has_specific)
    }

    pub async fn delete_channel(&self, channel_id: &str) -> anyhow::Result<()> {
        let resp = self
            .client
            .delete(format!("{}/channels/{}", BASE, channel_id))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("delete_channel failed {}: {}", status, text);
            anyhow::bail!("API error: {}", status);
        }

        Ok(())
    }

    pub async fn get_audit_logs(&self, guild_id: &str, action_type: u8, limit: u8) -> anyhow::Result<serde_json::Value> {
        let url = format!("{}/guilds/{}/audit-logs?action_type={}&limit={}", BASE, guild_id, action_type, limit);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("get_audit_logs failed {}: {}", status, text);
            anyhow::bail!("API error: {}", status);
        }

        let body: serde_json::Value = resp.json().await?;
        Ok(body.get("audit_log_entries").cloned().unwrap_or(json!([])))
    }

    pub async fn ban_user(&self, guild_id: &str, user_id: &str, reason: &str) -> anyhow::Result<()> {
        let url = format!("{}/guilds/{}/bans/{}", BASE, guild_id, user_id);
        let resp = self.client.put(&url)
            .header("X-Audit-Log-Reason", reason)
            .json(&json!({ "delete_message_seconds": 604800 })) 
            .send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("ban_user failed {}: {}", status, text);
            anyhow::bail!("API error: {} - {}", status, text);
        }

        info!("SUCCESS: Banned user {} from guild {}", user_id, guild_id);
        Ok(())
    }

    pub async fn kick_user(&self, guild_id: &str, user_id: &str, reason: &str) -> anyhow::Result<()> {
        let url = format!("{}/guilds/{}/members/{}", BASE, guild_id, user_id);
        let resp = self.client.delete(&url)
            .header("X-Audit-Log-Reason", reason)
            .send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("kick_user failed {}: {}", status, text);
            anyhow::bail!("API error: {} - {}", status, text);
        }

        info!("SUCCESS: Kicked user {} from guild {}", user_id, guild_id);
        Ok(())
    }

    pub async fn create_role(&self, guild_id: &str, name: &str, color: u32, hoist: bool, permissions: &str) -> anyhow::Result<serde_json::Value> {
        let url = format!("{}/guilds/{}/roles", BASE, guild_id);
        let body = json!({
            "name": name,
            "color": color,
            "hoist": hoist,
            "permissions": permissions
        });
        let resp = self.client.post(&url).json(&body).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("create_role failed {}: {}", status, text);
            anyhow::bail!("API error: {}", status);
        }

        let body_json: serde_json::Value = resp.json().await?;
        Ok(body_json)
    }

    pub async fn get_guild_roles(&self, guild_id: &str) -> anyhow::Result<Vec<serde_json::Value>> {
        let url = format!("{}/guilds/{}/roles", BASE, guild_id);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("get_guild_roles failed {}: {}", status, text);
            anyhow::bail!("API error: {}", status);
        }

        let roles: Vec<serde_json::Value> = resp.json().await?;
        Ok(roles)
    }

    pub async fn modify_role_positions(&self, guild_id: &str, role_id: &str, position: u64) -> anyhow::Result<()> {
        let url = format!("{}/guilds/{}/roles", BASE, guild_id);
        let body = json!([
            { "id": role_id, "position": position }
        ]);
        let resp = self.client.patch(&url).json(&body).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("modify_role_positions failed {}: {}", status, text);
            anyhow::bail!("API error: {}", status);
        }

        Ok(())
    }

    pub async fn add_member_role(&self, guild_id: &str, user_id: &str, role_id: &str) -> anyhow::Result<()> {
        let url = format!("{}/guilds/{}/members/{}/roles/{}", BASE, guild_id, user_id, role_id);
        let resp = self.client.put(&url).header("X-Audit-Log-Reason", "Rimuru Antinuke Auto-Role Setup").send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("add_member_role failed {}: {}", status, text);
            anyhow::bail!("API error: {}", status);
        }

        Ok(())
    }

    pub async fn timeout_member(&self, guild_id: &str, user_id: &str, until: Option<&str>, reason: &str) -> anyhow::Result<()> {
        let url = format!("{}/guilds/{}/members/{}", BASE, guild_id, user_id);
        let body = json!({ "communication_disabled_until": until });
        let resp = self.client.patch(&url).header("X-Audit-Log-Reason", reason).json(&body).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("timeout_member failed {}: {}", status, text);
            anyhow::bail!("API error: {} - {}", status, text);
        }

        Ok(())
    }

    pub async fn modify_channel_permissions(&self, channel_id: &str, overwrite_id: &str, allow: &str, deny: &str, type_: u8) -> anyhow::Result<()> {
        let url = format!("{}/channels/{}/permissions/{}", BASE, channel_id, overwrite_id);
        let body = json!({ "allow": allow, "deny": deny, "type": type_ });
        let resp = self.client.put(&url).json(&body).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("modify_channel_permissions failed {}: {}", status, text);
            anyhow::bail!("API error: {} - {}", status, text);
        }

        Ok(())
    }

    pub async fn bulk_delete_messages(&self, channel_id: &str, messages: Vec<String>) -> anyhow::Result<()> {
        let url = format!("{}/channels/{}/messages/bulk-delete", BASE, channel_id);
        let body = json!({ "messages": messages });
        let resp = self.client.post(&url).json(&body).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("bulk_delete_messages failed {}: {}", status, text);
            anyhow::bail!("API error: {}", status);
        }

        Ok(())
    }

    pub async fn get_channel_messages(&self, channel_id: &str, limit: u8) -> anyhow::Result<Vec<serde_json::Value>> {
        let url = format!("{}/channels/{}/messages?limit={}", BASE, channel_id, limit);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("get_channel_messages failed {}: {}", status, text);
            anyhow::bail!("API error: {}", status);
        }

        let messages: Vec<serde_json::Value> = resp.json().await?;
        Ok(messages)
    }

    pub async fn modify_channel(&self, channel_id: &str, rate_limit_per_user: u16) -> anyhow::Result<()> {
        let url = format!("{}/channels/{}", BASE, channel_id);
        let body = json!({ "rate_limit_per_user": rate_limit_per_user });
        let resp = self.client.patch(&url).json(&body).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("modify_channel failed {}: {}", status, text);
            anyhow::bail!("API error: {}", status);
        }

        Ok(())
    }

    pub async fn get_guild_bans(&self, guild_id: &str) -> anyhow::Result<Vec<serde_json::Value>> {
        let url = format!("{}/guilds/{}/bans", BASE, guild_id);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("get_guild_bans failed {}: {}", status, text);
            anyhow::bail!("API error: {}", status);
        }

        let bans: Vec<serde_json::Value> = resp.json().await?;
        Ok(bans)
    }

    pub async fn remove_guild_ban(&self, guild_id: &str, user_id: &str, reason: &str) -> anyhow::Result<()> {
        let url = format!("{}/guilds/{}/bans/{}", BASE, guild_id, user_id);
        let resp = self.client.delete(&url).header("X-Audit-Log-Reason", reason).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("remove_guild_ban failed {}: {}", status, text);
            anyhow::bail!("API error: {}", status);
        }

        Ok(())
    }

    pub async fn modify_member(&self, guild_id: &str, user_id: &str, nick: Option<&str>, reason: &str) -> anyhow::Result<()> {
        let url = format!("{}/guilds/{}/members/{}", BASE, guild_id, user_id);
        let body = json!({ "nick": nick });
        let resp = self.client.patch(&url).header("X-Audit-Log-Reason", reason).json(&body).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("modify_member failed {}: {}", status, text);
            anyhow::bail!("API error: {}", status);
        }

        Ok(())
    }
}
