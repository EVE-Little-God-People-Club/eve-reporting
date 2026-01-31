use rodio::{OutputStream, Sink};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::fs::File as AFile;

pub struct VoicePlayer {
	warn_voice_path: PathBuf,
	reminder_voice_path: PathBuf,
	warn_playing: Arc<AtomicBool>,
	reminder_playing: Arc<AtomicBool>,
	stream_handle: Arc<OutputStream>,
}

impl VoicePlayer {
	pub async fn new(warn_voice_path: &Path, reminder_voice_path: &Path) -> anyhow::Result<Self> {
		let warn_playing = Arc::new(AtomicBool::new(false));
		let reminder_playing = Arc::new(AtomicBool::new(false));
		let stream_handle = Arc::new(rodio::OutputStreamBuilder::open_default_stream()?);
		Ok(Self {
			warn_voice_path: warn_voice_path.to_path_buf(),
			reminder_voice_path: reminder_voice_path.to_path_buf(),
			warn_playing,
			reminder_playing,
			stream_handle,
		})
	}

	pub async fn play_warn(&self) -> anyhow::Result<()> {
		let file = AFile::open(&self.warn_voice_path).await?.into_std().await;
		let playing = Arc::clone(&self.warn_playing);
		self.play_voice(file, playing);
		Ok(())
	}

	pub async fn play_reminder(&self) -> anyhow::Result<()> {
		let file = AFile::open(&self.reminder_voice_path)
			.await?
			.into_std()
			.await;
		let playing = Arc::clone(&self.reminder_playing);
		self.play_voice(file, playing);
		Ok(())
	}

	fn play_voice(&self, file: File, playing: Arc<AtomicBool>) {
		if playing.load(Ordering::Relaxed) {
			return;
		}
		let stream_handle = Arc::clone(&self.stream_handle);
		playing.fetch_not(Ordering::Relaxed);
		tokio::task::spawn_blocking(move || {
			let task = || {
				let decoder = rodio::Decoder::new(file)?;
				let sink = Sink::connect_new(stream_handle.mixer());
				sink.append(decoder);
				sink.sleep_until_end();
				playing.fetch_not(Ordering::Relaxed);
				Ok::<(), anyhow::Error>(())
			};
			if task().is_err() {
				playing.fetch_not(Ordering::Relaxed);
			}
		});
	}
}
#[cfg(test)]
mod tests {
	use crate::voice_player::VoicePlayer;
	use std::path::PathBuf;
	use std::time::Duration;
	use tokio::test;

	#[test]
	async fn test() {
		let warn = PathBuf::from("D:\\dev\\rust\\project\\eve\\reporting\\warn.mp3");
		let reminder = PathBuf::from("D:\\dev\\rust\\project\\eve\\reporting\\reminder.mp3");
		let vp = VoicePlayer::new(&warn, &reminder).await.unwrap();
		vp.play_reminder().await.unwrap();
		tokio::time::sleep(Duration::from_secs(1)).await;
		vp.play_reminder().await.unwrap();
		tokio::time::sleep(Duration::from_secs(3)).await;
		vp.play_reminder().await.unwrap();
	}
}
