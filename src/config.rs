use serde::Deserialize;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
	pub warn_voice_path: PathBuf,
	pub reminder_voice_path: PathBuf,
	pub chars: Vec<Character>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Character {
	pub title: String,
	pub warn_region: WarnRegion,
	pub reminder_point: ReminderPoint,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WarnRegion {
	pub x: u32,
	pub y_start: u32,
	pub y_end: u32,
	pub step: u32,
	pub target_rgb: TargetRGB,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReminderPoint {
	pub now: Point,
	pub enemy: Point,
	pub now_target_rgb: TargetRGB,
	pub enemy_target_rgb: TargetRGB,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TargetRGB {
	pub r: u8,
	pub g: u8,
	pub b: u8,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Point {
	pub x: u32,
	pub y: u32,
}

impl Config {
	pub async fn init() -> anyhow::Result<Self> {
		let current_dir = std::env::current_dir()?;
		let config_path = current_dir.join("settings.toml");
		let mut file = File::open(config_path).await?;
		let mut config_str = String::new();
		file.read_to_string(&mut config_str).await?;
		Ok(toml::from_str(&config_str)?)
	}
}

#[cfg(test)]
mod tests {
	use crate::config::Config;
	use tokio::test;

	#[test]
	async fn test() {
		let toml_str = r#"
			warn_voice_path = "C:/path.mp3"
			reminder_voice_path = "C:/path.mp3"

			[[chars]]
			title = "TITLE"
			[chars.warn_region]
			x = 1
			y_start = 1
			y_end = 1
			step = 1
			[chars.warn_region.target_rgb]
			r = 1
			g = 1
			b = 1

			[chars.reminder_point]
			[chars.reminder_point.now]
			x = 1
			y = 1
			[chars.reminder_point.enemy]
			x = 1
			y = 1
			[chars.reminder_point.now_target_rgb]
			r = 1
			g = 1
			b = 1
			[chars.reminder_point.enemy_target_rgb]
			r = 1
			g = 1
			b = 1
		"#
		.trim();
		let data: Config = toml::from_str(toml_str).unwrap();
		println!("{data:#?}");
	}
}
