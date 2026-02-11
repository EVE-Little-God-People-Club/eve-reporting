use crate::event::EventConsumer;
use crate::image_checker::ImageChecker;
use crate::notification::NotifyController;
use crate::reverse_websocket::ReverseWebsocketController;
use crate::sse::SseServerController;
use crate::voice_player::VoicePlayerController;
use anyhow::anyhow;
use image::RgbaImage;
use serde::Deserialize;
use std::fmt::Debug;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use strum::EnumIs;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tracing::{debug, info, instrument, trace};
use url::Url;

pub type Point = [u32; 2];
pub type Host = [u8; 4];
pub type Port = u16;
pub type Rgb = [u8; 3];
pub type VecRgb = Vec<Rgb>;

#[derive(Debug, Deserialize, Clone)]
pub struct Region {
	pub start: Point,
	pub end: Point,
	pub rgb: VecRgb,
}

pub struct PointIter {
	start: [u32; 2],
	end: [u32; 2],
	current_x: u32,
	current_y: u32,
	finished: bool,
}

impl Iterator for PointIter {
	type Item = Point;

	fn next(&mut self) -> Option<Self::Item> {
		if self.finished {
			return None;
		}

		let point = [self.current_x, self.current_y];

		if self.current_y < self.end[1] {
			self.current_y += 1;
		} else if self.current_x < self.end[0] {
			self.current_x += 1;
			self.current_y = self.start[1];
		} else {
			self.finished = true;
		}

		Some(point)
	}
}

impl Region {
	pub fn check_in_image(&self, image: &RgbaImage) -> bool {
		self.iter().any(|point| {
			let result = image.check_point_rgb_list(point, &self.rgb);
			if result {
				trace!(?point, ?self.rgb, "find rgb");
			}
			result
		})
	}

	pub fn iter(&self) -> PointIter {
		PointIter {
			start: self.start,
			end: self.end,
			current_x: self.start[0],
			current_y: self.start[1],
			finished: false,
		}
	}
}

impl IntoIterator for Region {
	type Item = Point;
	type IntoIter = PointIter;
	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
	pub report_methods: Vec<ReportMethod>,
	pub characters: Vec<Character>,
}

impl Config {
	pub fn check_report_methods(&self) -> bool {
		let mut has_voice = false;
		let mut has_notification = false;

		for method in &self.report_methods {
			if method.is_voice() {
				if has_voice {
					return false;
				}
				has_voice = true;
			} else if method.is_notification() {
				if has_notification {
					return false;
				}
				has_notification = true;
			}
		}

		true
	}
}

#[derive(Debug, Deserialize, Clone)]
pub struct Character {
	pub title: String,
	pub warn_region: Region,
	#[serde(flatten)]
	pub reminder_regions: ReminderRegions,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReminderRegions {
	pub reminder_now_region: Region,
	pub reminder_enemy_region: Region,
}

impl ReminderRegions {
	pub fn check_reminder(&self, image: &RgbaImage, state: &mut (bool, bool)) -> bool {
		trace!(?state, "check reminder call");
		if !state.0 && !state.1 && self.reminder_now_region.check_in_image(image) {
			state.0 = true;
			false
		} else if state.0 && !state.1 && self.reminder_enemy_region.check_in_image(image) {
			state.1 = true;
			false
		} else if state.0 && state.1 && !self.reminder_enemy_region.check_in_image(image) {
			state.0 = false;
			state.1 = false;
			debug!(level = "reminder", ?state);
			true
		} else {
			false
		}
	}
}

fn deserialize_url<'de, D>(deserializer: D) -> Result<Url, D::Error>
where
	D: serde::Deserializer<'de>,
{
	let s = String::deserialize(deserializer)?;
	match Url::parse(&s) {
		Ok(url) => Ok(url),
		Err(err) => Err(serde::de::Error::custom(err.to_string())),
	}
}

fn default_try_forever() -> bool {
	false
}

fn default_try_spacing() -> Duration {
	Duration::from_secs(5)
}

#[derive(Debug, Deserialize, Clone, EnumIs)]
#[serde(tag = "type")]
pub enum ReportMethod {
	Voice {
		warn_voice_path: PathBuf,
		reminder_voice_path: PathBuf,
	},
	Sse {
		host: Host,
		port: Port,
	},
	Notification,
	ReverseWebsocket {
		#[serde(deserialize_with = "deserialize_url")]
		url: Url,
		#[serde(default = "default_try_forever")]
		try_forever: bool,
		#[serde(default = "default_try_spacing")]
		try_spacing: Duration,
	},
}

impl ReportMethod {
	pub fn to_consumer(&self) -> Option<Box<dyn EventConsumer>> {
		match self {
			Self::Voice {
				warn_voice_path,
				reminder_voice_path,
			} => Some(Box::new(VoicePlayerController::new(
				warn_voice_path,
				reminder_voice_path,
			))),
			Self::Notification => Some(Box::new(NotifyController::new())),
			Self::Sse { host, port } => Some(Box::new(SseServerController::new(*host, *port))),
			Self::ReverseWebsocket {
				url,
				try_forever,
				try_spacing,
			} => Some(Box::new(ReverseWebsocketController::new(
				url.clone(),
				*try_forever,
				*try_spacing,
			))),
		}
	}
}

impl Config {
	#[instrument]
	pub async fn init() -> anyhow::Result<Self> {
		let current_dir = std::env::current_dir()?;
		let config_path = current_dir.join("settings.toml");
		info!("reading config from {config_path:?}");
		let mut file = File::open(config_path).await?;
		let mut config_str = String::new();
		file.read_to_string(&mut config_str).await?;
		info!("read config successful");
		Self::from_str(&config_str)
	}
}

impl FromStr for Config {
	type Err = anyhow::Error;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let toml = toml::from_str::<Config>(s)?;
		if !toml.check_report_methods() {
			Err(anyhow!("There are duplicates"))
		} else {
			Ok(toml)
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::config::Config;
	use std::str::FromStr;
	use tokio::test;

	#[test]
	async fn example_config() {
		let toml_str = r#"
			[[report_methods]]
			type = "Voice"
			warn_voice_path = "C:\\warn_voice.mp3"
			reminder_voice_path = "C:\\reminder_voice.mp3"

			[[report_methods]]
			type = "Notification"

			[[report_methods]]
			type = "Sse"
			host = [127, 0, 0, 1]
			port = 8080

			[[report_methods]]
			type = "ReverseWebsocket"
			url = "wss://example.com"
			try_forever = true
			try_spacing = {
				secs = 5,
				nanos = 0
			}

			[[characters]]
			title = "EVE - CHAR1"
			warn_region.start = [1, 1]
			warn_region.end = [1, 1]
			warn_region.rgb = [
				[1, 1, 1]
			]
			reminder_now_region.start = [1, 1]
			reminder_now_region.end = [1, 1]
			reminder_now_region.rgb = [
				[1, 1, 1]
			]
			reminder_enemy_region.start = [1, 1]
			reminder_enemy_region.end = [1, 1]
			reminder_enemy_region.rgb = [
				[1, 1, 1]
			]

			[[characters]]
			title = "EVE - CHAR2"
			warn_region.start = [1, 1]
			warn_region.end = [1, 1]
			warn_region.rgb = [
				[1, 1, 1]
			]
			reminder_now_region.start = [1, 1]
			reminder_now_region.end = [1, 1]
			reminder_now_region.rgb = [
				[1, 1, 1]
			]
			reminder_enemy_region.start = [1, 1]
			reminder_enemy_region.end = [1, 1]
			reminder_enemy_region.rgb = [
				[1, 1, 1]
			]
		"#
		.trim();
		let data = Config::from_str(toml_str).unwrap();
		println!("{data:#?}");
	}
}
