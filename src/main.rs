use crate::config::Config;
use crate::eve::EveClient;
use crate::eve_monitor::EveMonitor;
use crate::voice_player::VoicePlayer;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;
use tracing::{info, instrument, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod config;
mod eve;
mod eve_monitor;
mod image_checker;
mod voice_player;

#[instrument]
async fn entry_point() -> anyhow::Result<()> {
	let config = Config::init().await?;
	let voice_player = VoicePlayer::new(&config.warn_voice_path, &config.reminder_voice_path).await?;
	let (mut warn_monitors, mut reminder_monitors): (Vec<_>, Vec<_>) = config
		.chars
		.iter()
		.map(|item| EveMonitor::new(item.clone()))
		.filter_map(|monitor| monitor.ok())
		.filter_map(|monitor| {
			if monitor.start_capture().is_ok() {
				Some(monitor)
			} else {
				None
			}
		})
		.map(|monitor| {
			(
				monitor.subscribe_warn_points(),
				monitor.subscribe_reminder(),
			)
		})
		.unzip();

	loop {
		let mut warn_tasks = get_tasks(&mut warn_monitors);
		let mut reminder_tasks = get_tasks(&mut reminder_monitors);

		tokio::select! {
			_ = warn_tasks.next() => {
				let _ = voice_player.play_warn().await;
			}
			_ = reminder_tasks.next() => {
				let _ = voice_player.play_reminder().await;
			}
		}
	}
}

fn get_tasks(
	monitors: &mut [broadcast::Receiver<()>],
) -> FuturesUnordered<impl Future<Output = Result<(), RecvError>>> {
	let tasks: FuturesUnordered<_> = monitors.iter_mut().map(|monitor| monitor.recv()).collect();
	tasks
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
	tracing_subscriber::registry()
		.with(tracing_subscriber::fmt::layer())
		.init();
	let args = std::env::args().skip(1).collect::<Vec<_>>();
	if let Some(first) = args.first()
		&& first == &String::from("--capture-only")
	{
		capture_only().await
	} else {
		entry_point().await
	}
}
