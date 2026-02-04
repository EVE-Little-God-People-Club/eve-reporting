use crate::config::*;
use crate::eve::EveClient;
use crate::event::{Event, EventProducer};
use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::broadcast::Sender;
//
// pub struct EveMonitor {
// 	pub eve_client: EveClient,
// 	pub warn_points: Vec<(u32, u32)>,
// 	pub reminder_now_point: (u32, u32),
// 	pub reminder_enemy_point: (u32, u32),
// 	pub warn_points_rgb: (u8, u8, u8),
// 	pub reminder_now_rgb: (u8, u8, u8),
// 	pub reminder_enemy_rgb: (u8, u8, u8),
// 	pub warn_points_sender: broadcast::Sender<()>,
// 	pub reminder_sender: broadcast::Sender<()>,
// }
//
// impl EveMonitor {
// 	pub fn new(char_config: Character) -> anyhow::Result<Self> {
// 		let eve_client = EveClient::new_from_title(&char_config.title)?;
// 		let (warn_points, reminder_now_point, reminder_enemy_point) = Self::get_all_point(&char_config);
// 		let w_rgb = &char_config.warn_region.target_rgb;
// 		let n_rgb = &char_config.reminder_region.now_target_rgb;
// 		let e_rgb = &char_config.reminder_region.enemy_target_rgb;
//
// 		let warn_points_rgb = (w_rgb.r, w_rgb.g, w_rgb.b);
// 		let reminder_now_rgb = (n_rgb.r, n_rgb.g, n_rgb.b);
// 		let reminder_enemy_rgb = (e_rgb.r, e_rgb.g, e_rgb.b);
//
// 		let (warn_points_sender, _) = broadcast::channel(16);
// 		let (reminder_sender, _) = broadcast::channel(16);
// 		Ok(Self {
// 			eve_client,
// 			warn_points,
// 			reminder_enemy_point,
// 			reminder_now_point,
// 			warn_points_rgb,
// 			reminder_now_rgb,
// 			reminder_enemy_rgb,
// 			warn_points_sender,
// 			reminder_sender,
// 		})
// 	}
//
// 	pub fn subscribe_warn_points(&self) -> broadcast::Receiver<()> {
// 		self.warn_points_sender.subscribe()
// 	}
//
// 	pub fn subscribe_reminder(&self) -> broadcast::Receiver<()> {
// 		self.reminder_sender.subscribe()
// 	}
//
// 	pub fn start_capture(&self) -> anyhow::Result<()> {
// 		self.eve_client.start_capture()?;
// 		let mut capture_receiver = self.eve_client.get_capture_receiver();
// 		let warn_points = self.warn_points.clone();
// 		let warn_points_rgb = self.warn_points_rgb;
// 		let warn_points_sender = self.warn_points_sender.clone();
// 		let reminder_now_point = self.reminder_now_point;
// 		let reminder_now_rgb = self.reminder_now_rgb;
// 		let reminder_enemy_point = self.reminder_enemy_point;
// 		let reminder_enemy_rgb = self.reminder_enemy_rgb;
// 		let reminder_sender = self.reminder_sender.clone();
// 		tokio::spawn(async move {
// 			let mut reminder_state = (false, false);
// 			loop {
// 				if let Ok(image) = capture_receiver.recv().await {
// 					Self::check_all_points(
// 						&image,
// 						&warn_points,
// 						warn_points_rgb,
// 						reminder_now_point,
// 						reminder_now_rgb,
// 						reminder_enemy_point,
// 						reminder_enemy_rgb,
// 						&mut reminder_state,
// 						&warn_points_sender,
// 						&reminder_sender,
// 					)
// 				}
// 			}
// 		});
// 		Ok(())
// 	}
//
// 	#[allow(clippy::too_many_arguments)]
// 	fn check_all_points(
// 		image: &RgbaImage,
// 		warn_points: &[(u32, u32)],
// 		warn_points_rgb: (u8, u8, u8),
// 		reminder_now_point: (u32, u32),
// 		reminder_now_rgb: (u8, u8, u8),
// 		reminder_enemy_point: (u32, u32),
// 		reminder_enemy_rgb: (u8, u8, u8),
// 		reminder_state: &mut (bool, bool),
// 		warn_points_sender: &broadcast::Sender<()>,
// 		reminder_sender: &broadcast::Sender<()>,
// 	) {
// 		if Self::check_warn_points(image, warn_points, warn_points_rgb) {
// 			let _ = warn_points_sender.send(());
// 		}
// 		if Self::check_reminder(
// 			image,
// 			reminder_now_point,
// 			reminder_now_rgb,
// 			reminder_enemy_point,
// 			reminder_enemy_rgb,
// 			reminder_state,
// 		) {
// 			let _ = reminder_sender.send(());
// 		}
// 	}
//
// 	fn check_warn_points(image: &RgbaImage, points: &[(u32, u32)], rgb: (u8, u8, u8)) -> bool {
// 		points
// 			.iter()
// 			.any(|point| image.check_point_rgb(point, &rgb))
// 	}
//
// 	fn check_reminder(
// 		image: &RgbaImage,
// 		now_point: (u32, u32),
// 		now_rgb: (u8, u8, u8),
// 		enemy_point: (u32, u32),
// 		enemy_rgb: (u8, u8, u8),
// 		state: &mut (bool, bool),
// 	) -> bool {
// 		if !state.0 && image.check_point_rgb(&now_point, &now_rgb) {
// 			state.0 = true;
// 			false
// 		} else if state.0 && !state.1 && image.check_point_rgb(&enemy_point, &enemy_rgb) {
// 			state.1 = true;
// 			false
// 		} else if state.0 && state.1 && !image.check_point_rgb(&enemy_point, &enemy_rgb) {
// 			state.0 = false;
// 			state.1 = false;
// 			true
// 		} else {
// 			false
// 		}
// 	}
//
// 	#[allow(clippy::type_complexity)]
// 	fn get_all_point(char_config: &Character) -> (Vec<(u32, u32)>, (u32, u32), (u32, u32)) {
// 		let mut ptr = char_config.warn_region.y_start;
// 		let mut vec = Vec::new();
// 		loop {
// 			if ptr > char_config.warn_region.y_end {
// 				break;
// 			}
// 			vec.push((char_config.warn_region.x, ptr));
// 			ptr += char_config.warn_region.step;
// 		}
// 		(
// 			vec,
// 			(
// 				char_config.reminder_region.now.x,
// 				char_config.reminder_region.now.y,
// 			),
// 			(
// 				char_config.reminder_region.enemy.x,
// 				char_config.reminder_region.enemy.y,
// 			),
// 		)
// 	}
// }

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
