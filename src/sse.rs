use crate::config::{Host, Port};
use crate::event::{Event as ReportEvent, EventConsumer};
use anyhow::anyhow;
use axum::extract::State;
use axum::response::Sse;
use axum::response::sse::Event;
use axum::routing::get;
use futures::Stream;
use futures::StreamExt;
use std::net::Ipv4Addr;
use tokio::sync::broadcast;
use tokio::sync::broadcast::Sender;
use tokio_stream::wrappers::BroadcastStream;
use tracing::info;

async fn sse_handler(
	State(sender): State<broadcast::Sender<ReportEvent>>,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>>> {
	let receiver = sender.subscribe();
	let stream = BroadcastStream::new(receiver).map(|msg| match msg {
		Ok(data) => Ok(Event::default().data(toml::to_string(&data).unwrap())),
		Err(_) => Err(axum::Error::new("broadcast error")),
	});

	Sse::new(stream)
		.keep_alive(axum::response::sse::KeepAlive::new().interval(std::time::Duration::from_secs(5)))
}

pub struct SseServerController {
	sender: Option<Sender<ReportEvent>>,
	host: Host,
	port: Port,
}

impl SseServerController {
	pub fn new(host: Host, port: Port) -> Self {
		Self {
			sender: None,
			host,
			port,
		}
	}
}

impl EventConsumer for SseServerController {
	fn inject(&mut self, sender: Sender<ReportEvent>) {
		self.sender = Some(sender)
	}

	fn start(&self) -> anyhow::Result<()> {
		if self.sender.is_none() {
			return Err(anyhow!("There is no sender"));
		}
		let sender = self.sender.clone().unwrap();

		let host = self.host;
		let port = self.port;

		tokio::spawn(async move {
			let addr = format!("{}.{}.{}.{}:{}", host[0], host[1], host[2], host[3], port);
			let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
			let app = axum::Router::new()
				.route("/events", get(sse_handler))
				.with_state(sender);
			info!("SSE server run on {}", addr);
			axum::serve(listener, app).await.unwrap();
		});
		Ok(())
	}
}
