use axum::extract::WebSocketUpgrade;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::response::IntoResponse;
use axum::routing::get;
use onebot_api::api::APISender;
use onebot_api::communication::Client;
use onebot_api::communication::ws::WsService;
use onebot_api::message::segment_builder::SegmentBuilder;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::OnceCell;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
	Warn { title: String },
	Reminder { title: String },
}

pub struct OnebotClient {
	pub client: Client,
}
impl Deref for OnebotClient {
	type Target = Client;

	fn deref(&self) -> &Self::Target {
		&self.client
	}
}

impl OnebotClient {
	pub async fn instance() -> anyhow::Result<&'static Self> {
		static INSTANCE: OnceCell<OnebotClient> = OnceCell::const_new();
		INSTANCE
			.get_or_try_init(async || {
				let url = std::env::var("ONEBOT_URL")?;
				let key = std::env::var("ONEBOT_KEY")?;
				let key = if key.is_empty() { None } else { Some(key) };
				let ws_service = WsService::new_with_token(url, key)?;
				let client = Client::with_service(Box::new(ws_service), Some(Duration::from_secs(5)));
				client.start_service().await?;
				Ok(Self { client })
			})
			.await
	}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	OnebotClient::instance().await?;
	let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;

	let router = axum::Router::new().route("/ws", get(ws_handler));

	axum::serve(listener, router).await?;

	Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
	ws.on_upgrade(ws_processer)
}

async fn ws_processer(mut socket: WebSocket) {
	println!("WebSocket client connected");

	let user_id: i64 = std::env::var("MESSAGE_TARGET")
		.expect("MESSAGE_TARGET must be set")
		.parse()
		.expect("MESSAGE_TARGET must be a valid integer");

	let warn_spacing: u64 = std::env::var("WARN_SPACING")
		.expect("WARN_SPACING must be set")
		.parse()
		.expect("WARN_SPACING must be a valid integer");
	let warn_spacing = Duration::from_secs(warn_spacing);

	let warning = Arc::new(AtomicBool::new(false));

	tokio::spawn(async move {
		loop {
			tokio::select! {
				msg = socket.recv() => {
					match msg {
						Some(Ok(Message::Text(text))) => {
							if let Ok(event) = serde_json::from_str::<Event>(&text) {
								match event {
									Event::Warn { title } => {
										if warning.load(Ordering::Relaxed) {
											continue;
										}
										warning.store(true, Ordering::Relaxed);
										let warning_clone = Arc::clone(&warning);
										tokio::spawn(async move {
											tokio::time::sleep(warn_spacing).await;
											warning_clone.store(false, Ordering::Relaxed);
										});

										let msg = SegmentBuilder::new().text(format!("Warn {title}")).build();
										if let Ok(client) = OnebotClient::instance().await {
											let _ = client.send_private_msg(user_id, msg, None).await;
										}
									}
									Event::Reminder { title } => {
										let msg = SegmentBuilder::new()
											.text(format!("Reminder {title}"))
											.build();
										if let Ok(client) = OnebotClient::instance().await {
											let _ = client.send_private_msg(user_id, msg, None).await;
										}
									}
								}
							}
						}
						Some(Ok(Message::Close(_))) | None => {
							println!("WebSocket client disconnected");
							break;
						}
						Some(Err(e)) => {
							eprintln!("WebSocket error: {}", e);
							break;
						}
						_ => {} // 忽略 Ping/Pong 等，axum 通常自动处理
					}
				}
			}
		}
	});
}
