mod commands;
mod antinuke;
mod constants;
mod db;
mod gateway;
mod handler;
mod models;
mod rest;

use db::Database;
use dotenv::dotenv;
use rest::RestClient;
use std::env;
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() {

    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("rimuru_bot=info")),
        )
        .init();

    let token = env::var("DISCORD_TOKEN")
        .expect("❌ DISCORD_TOKEN not set in .env")
        .trim()
        .to_string();

    let prefix = env::var("PREFIX")
        .unwrap_or_else(|_| "!".to_string())
        .chars()
        .next()
        .expect("PREFIX must be at least one character");

    let db = match Database::new("rimuru.db").await {
        Ok(d) => Arc::new(d),
        Err(e) => {
            error!("❌ Failed to initialize database: {:?}", e);
            return;
        }
    };

    info!("🦀 rimuru-bot starting (prefix='{}') — raw WebSocket + HTTP, no wrapper", prefix);

    let rest = Arc::new(RestClient::new(&token));

    if let Err(e) = rest.validate_token().await {
        error!("❌ Token validation failed: {:?}", e);
        return;
    }

    gateway::run(token, rest, prefix, db).await;
}
