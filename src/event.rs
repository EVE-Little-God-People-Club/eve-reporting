use async_trait::async_trait;
use tokio::sync::broadcast::{self, Sender};

#[derive(Clone, Debug)]
pub enum Event {
	Warn { title: String },
	Reminder { title: String },
}

#[async_trait]
pub trait EventProducer {
	fn inject(&mut self, sender: Sender<Event>);
	fn start(&self) -> anyhow::Result<()>;
}

#[async_trait]
pub trait EventConsumer {
	fn inject(&mut self, sender: Sender<Event>);
	fn start(&self) -> anyhow::Result<()>;
}

pub struct EventCenter {
	pub sender: Sender<Event>,
	pub producers: Vec<Box<dyn EventProducer>>,
	pub consumers: Vec<Box<dyn EventConsumer>>,
}

impl EventCenter {
	pub fn init() -> Self {
		let (sender, _) = broadcast::channel(16);
		Self {
			sender,
			producers: Vec::new(),
			consumers: Vec::new(),
		}
	}

	pub fn add_producer(&mut self, mut producer: Box<dyn EventProducer>) -> anyhow::Result<()> {
		producer.inject(self.sender.clone());
		producer.start()?;
		self.producers.push(producer);
		Ok(())
	}

	pub fn add_consumer(&mut self, mut consumer: Box<dyn EventConsumer>) -> anyhow::Result<()> {
		consumer.inject(self.sender.clone());
		consumer.start()?;
		self.consumers.push(consumer);
		Ok(())
	}

	// pub async fn and_add_producer(mut self, producer: Box<dyn EventProducer>) -> anyhow::Result<Self> {
	// 	self.add_producer(producer).await?;
	// 	Ok(self)
	// }
	//
	// pub async fn and_add_consumer(mut self, consumer: Box<dyn EventConsumer>) -> anyhow::Result<Self> {
	// 	self.add_consumer(consumer).await?;
	// 	Ok(self)
	// }
}
