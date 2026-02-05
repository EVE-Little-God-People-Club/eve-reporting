use crate::config::Config;
use crate::eve::EveClient;
use crate::eve_monitor::EveMonitor;
use crate::event::EventCenter;
use std::time::Duration;
use tokio::fs::File;
use tokio::task::JoinHandle;
use tracing::{error, info, instrument, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod config;
mod eve;
mod eve_monitor;
mod event;
mod image_checker;
mod notification;
mod reverse_websocket;
mod sse;
mod voice_player;

#[instrument]
async fn entry_point() -> anyhow::Result<()> {
	let config = Config::init()
		.await
		.inspect_err(|e| error!("reading config failed: {e}"))?;
	let mut event_center = EventCenter::init();
	config
		.report_methods
		.iter()
		.filter_map(|cfg| cfg.to_consumer())
		.for_each(|method| {
			event_center
				.add_consumer(method)
				.inspect_err(|e| warn!("There is a error when start report method: {e}"))
				.unwrap_or(())
		});
	config
		.characters
		.into_iter()
		.filter_map(|cfg| EveMonitor::new(cfg).ok())
		.for_each(|monitor| {
			event_center
				.add_producer(Box::new(monitor))
				.inspect_err(|e| warn!("There is a error when start eve monitor: {e}"))
				.unwrap_or(())
		});
	if let Err(e) = tokio::signal::ctrl_c().await {
		warn!("Failed to listen ctrl-c signal: {}", e);
		loop {
			tokio::time::sleep(Duration::from_hours(1)).await;
		}
	} else {
		info!("EXIT SIGNAL BY USER");
		std::process::exit(0);
	}
}

#[instrument]
async fn capture_only() -> anyhow::Result<()> {
	info!("capture only mode");
	let mut tasks: Vec<JoinHandle<anyhow::Result<()>>> = Vec::new();
	EveClient::get_all_eve_client()?
		.into_iter()
		.inspect(|client| info!("search client: {}", client.title))
		.filter(|client| {
			client
				.start_capture()
				.inspect_err(|e| {
					warn!(
						"client {} start capture failed with error: {}",
						client.title, e
					)
				})
				.is_ok()
		})
		.inspect(|client| info!("client {} start capture successfully", client.title))
		.map(|client| (client.get_capture_receiver(), client.title))
		.for_each(|(mut capture_receiver, title)| {
			tasks.push(tokio::spawn(async move {
				info!("saving {title} capture");
				let current_dir = std::env::current_dir()?;
				let img_path = current_dir.join(format!("{title}.png"));
				let img = capture_receiver.recv().await?;
				img.save(img_path)?;
				Ok(())
			}))
		});

	for task in tasks {
		if let Err(e) = task.await? {
			warn!("error throw while saving: {e}")
		}
	}

	std::process::exit(0);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let subscriber = tracing_subscriber::registry().with(tracing_subscriber::fmt::layer());
	let args = std::env::args().skip(1).collect::<Vec<_>>();
	if args.contains(&"--log-file".to_string()) {
		let file = File::create("reporting.log").await?.into_std().await;
		let file_layer = tracing_subscriber::fmt::layer()
			.with_writer(file)
			.with_ansi(false);
		subscriber.with(file_layer).init();
	} else {
		subscriber.init();
	}
	if args.contains(&"--capture-only".to_string()) {
		capture_only().await?;
	} else {
		entry_point().await?;
	}
	Ok(())
}
