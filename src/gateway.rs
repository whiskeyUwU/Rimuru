use crate::{handler, models, antinuke};
use crate::rest::RestClient;
use crate::db::Database;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tracing::{info, error, debug};
use serde_json::json;

pub async fn run(token: String, rest: Arc<RestClient>, prefix: char, db: Arc<Database>) {
    loop {
        if let Err(e) = connect_and_run(&token, Arc::clone(&rest), prefix, Arc::clone(&db)).await {
            error!("Gateway error: {:?}. Reconnecting in 5s…", e);
            sleep(Duration::from_secs(5)).await;
        }
    }
}

async fn connect_and_run(token: &str, rest: Arc<RestClient>, prefix: char, db: Arc<Database>) -> anyhow::Result<()> {

    let mut gw_url = rest.get_gateway_url().await?;

    if !gw_url.ends_with('/') {
        gw_url.push('/');
    }
    let ws_url = format!("{}?v=10&encoding=json", gw_url);
    info!("Connecting to Discord Gateway: {}...", ws_url);

    let mut request = ws_url.into_client_request()?;
    request.headers_mut().insert(
        "User-Agent",
        "DiscordBot (https://github.com/rimuru, 1.0)".parse()?
    );

    let (ws_stream, _) = connect_async(request).await?;
    info!("✅ WebSocket handshake complete");

    let (mut ws_sink, mut ws_stream) = ws_stream.split();

    let hello_msg = ws_stream
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("Stream closed before HELLO"))??;

    let hello_payload: models::Payload = serde_json::from_str(hello_msg.to_text()?)?;
    let heartbeat_interval = hello_payload.d
        .and_then(|d| d.get("heartbeat_interval").and_then(|v| v.as_u64()))
        .ok_or_else(|| anyhow::anyhow!("Missing heartbeat_interval"))?;

    info!("HELLO received — heartbeat interval: {}ms", heartbeat_interval);

    let mut interval_timer = tokio::time::interval(Duration::from_millis(heartbeat_interval));

    let (hb_tx, mut hb_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    tokio::spawn(async move {
        loop {
            interval_timer.tick().await;
            let hb = json!({ "op": 1, "d": null });
            if let Err(_) = hb_tx.send(Message::Text(hb.to_string().into())) { break; }
        }
    });

    let intents = 37377 | 4 | 32768 | 2 | 8; 
    let identify = json!({
        "op": 2,
        "d": {
            "token": token,
            "intents": intents,
            "properties": { "os": "windows", "browser": "rimuru-bot", "device": "rimuru-bot" }
        }
    });
    ws_sink.send(Message::Text(identify.to_string().into())).await?;
    info!("IDENTIFY sent (intents={})", intents);

    loop {
        tokio::select! {
            Some(hb_msg) = hb_rx.recv() => {
                ws_sink.send(hb_msg).await?;
            }
            Some(res) = ws_stream.next() => {
                match res {
                    Ok(msg) => {
                        if msg.is_text() {
                            let payload: models::Payload = match serde_json::from_str(msg.to_text()?) {
                                Ok(p) => p,
                                Err(e) => { error!("Parse error: {:?}", e); continue; }
                            };

                            if let Some(t) = payload.t.as_deref() {
                                let d = payload.d.clone().unwrap_or(json!({}));

                                match t {
                                    "READY" => {
                                        let user = d["user"]["username"].as_str().unwrap_or("Unknown");
                                        let session = d["session_id"].as_str().unwrap_or("");
                                        info!("✅ READY — logged in as {} (session: {})", user, session);
                                    }
                                    "MESSAGE_CREATE" => {
                                        let msg_data: models::Message = serde_json::from_value(d.clone())?;
                                        let rest_clone_1 = rest.clone();
                                        let db_clone_1 = Arc::clone(&db);
                                        tokio::spawn(async move { handler::handle_message(msg_data, rest_clone_1, prefix, db_clone_1).await; });

                                        let rest_clone_2 = rest.clone();
                                        let db_clone_2 = Arc::clone(&db);
                                        tokio::spawn(async move { antinuke::handle_event("MESSAGE_CREATE", d, rest_clone_2, db_clone_2).await; });
                                    }
                                    "INTERACTION_CREATE" => {
                                        let int_data: models::Interaction = serde_json::from_value(d)?;
                                        let rest_clone = rest.clone();
                                        let db_clone = Arc::clone(&db);
                                        tokio::spawn(async move { handler::handle_interaction(int_data, rest_clone, db_clone).await; });
                                    }

                                    nuke_event if nuke_event.starts_with("GUILD_") || nuke_event.starts_with("CHANNEL_") || nuke_event.contains("UPDATE") => {
                                        let rest_clone = rest.clone();
                                        let db_clone = Arc::clone(&db);
                                        let event_name = nuke_event.to_string();
                                        tokio::spawn(async move { antinuke::handle_event(&event_name, d, rest_clone, db_clone).await; });
                                    }
                                    _ => { debug!("Dispatching ignored event: {}", t); }
                                }
                            }
                        }
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }
    }
}
