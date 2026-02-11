use crate::event::{Event, EventConsumer};
use anyhow::anyhow;
use futures::SinkExt;
use std::time::Duration;
use tokio::sync::broadcast::Sender;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};
use url::Url;

pub struct ReverseWebsocketController {
	sender: Option<Sender<Event>>,
	url: Url,
	try_forever: bool,
	try_spacing: Duration,
}

impl ReverseWebsocketController {
	pub fn new(url: Url, try_forever: bool, try_spacing: Duration) -> Self {
		Self {
			url,
			sender: None,
			try_forever,
			try_spacing,
		}
	}
}

impl EventConsumer for ReverseWebsocketController {
	fn inject(&mut self, sender: Sender<Event>) {
		self.sender = Some(sender)
	}

	fn start(&self) -> anyhow::Result<()> {
		if self.sender.is_none() {
			return Err(anyhow!("There is no sender"));
		}
		let sender = self.sender.clone().unwrap();
		let url = self.url.clone();
		let try_forever = self.try_forever;
		let try_spacing = self.try_spacing;
		tokio::spawn(async move {
			let task = async || {
				info!("try to connect {}", url);
				let mut receiver = sender.subscribe();
				let (mut ws_stream, _) = tokio_tungstenite::connect_async(url.to_string()).await?;
				info!("reverse websocket connect successful");
				loop {
					if let Ok(event) = receiver.recv().await {
						match serde_json::to_string(&event) {
							Ok(data) => {
								ws_stream.send(Message::Text(data.into())).await?;
							}
							Err(e) => warn!("event serialize failed: {}", e),
						}
					}
				}
			};

			if try_forever {
				loop {
					let result: anyhow::Result<()> = task().await;
					let _ = result.inspect_err(|e| warn!("cannot connect ws server: {}", e));
					tokio::time::sleep(try_spacing).await;
				}
			} else {
				let result: anyhow::Result<()> = task().await;
				let _ = result.inspect_err(|e| warn!("cannot connect ws server: {}", e));
			}
		});

		Ok(())
	}
}
