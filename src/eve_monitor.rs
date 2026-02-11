use crate::config::*;
use crate::eve::EveClient;
use crate::event::{Event, EventProducer};
use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::broadcast::Sender;

pub struct EveMonitor {
	pub eve_client: EveClient,
	pub character_config: Character,
	pub sender: Option<Sender<Event>>,
}

#[async_trait]
impl EventProducer for EveMonitor {
	fn inject(&mut self, sender: Sender<Event>) {
		self.sender = Some(sender)
	}

	fn start(&self) -> anyhow::Result<()> {
		if self.sender.is_none() {
			return Err(anyhow!("There are no sender"));
		}
		self.eve_client.start_capture()?;
		let mut capture_receiver = self.eve_client.get_capture_receiver();
		let char_cfg = self.character_config.clone();
		let sender = self.sender.clone().unwrap();
		let title = self.eve_client.title.clone();
		tokio::spawn(async move {
			let mut state = (false, false);
			loop {
				if let Ok(capture) = capture_receiver.recv().await {
					if char_cfg.warn_region.check_in_image(&capture) {
						let _ = sender.send(Event::Warn {
							title: title.clone(),
						});
					}
					if char_cfg
						.reminder_regions
						.check_reminder(&capture, &mut state)
					{
						let _ = sender.send(Event::Reminder {
							title: title.clone(),
						});
					}
				}
			}
		});
		Ok(())
	}
}

impl EveMonitor {
	pub fn new(character_config: Character) -> anyhow::Result<Self> {
		let eve_client = EveClient::new_from_title(&character_config.title)?;
		Ok(Self {
			eve_client,
			character_config,
			sender: None,
		})
	}
}
