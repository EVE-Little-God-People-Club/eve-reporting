use crate::event::{Event, EventConsumer};
use anyhow::anyhow;
use notify_rust::Notification;
use tokio::sync::broadcast::Sender;

pub struct NotifyController {
	sender: Option<Sender<Event>>,
}

impl NotifyController {
	pub fn new() -> Self {
		Self { sender: None }
	}
}

impl EventConsumer for NotifyController {
	fn inject(&mut self, sender: Sender<Event>) {
		self.sender = Some(sender)
	}
	fn start(&self) -> anyhow::Result<()> {
		if self.sender.is_none() {
			return Err(anyhow!("There is no sender"));
		}
		let mut receiver = self.sender.clone().unwrap().subscribe();

		tokio::spawn(async move {
			loop {
				if let Ok(event) = receiver.recv().await {
					match event {
						Event::Warn { title } => {
							let _ = Notification::new().summary("Warn").body(&title).show();
						}
						Event::Reminder { title } => {
							let _ = Notification::new().summary("Reminder").body(&title).show();
						}
					}
				}
			}
		});

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use notify_rust::Notification;
	use tokio::test;

	#[test]
	async fn test() {
		let mut nt = Notification::new();
		nt.summary("A");
		nt.body("B");
		nt.show().unwrap();
	}
}
