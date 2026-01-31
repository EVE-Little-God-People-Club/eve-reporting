use image::{ImageBuffer, RgbaImage};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
	EnumWindows, FindWindowW, GetWindowTextW, IsWindowVisible,
};
use windows::core::{BOOL, HSTRING};
use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;
use windows_capture::settings::{
	ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
	MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
};
use windows_capture::window::Window;

pub fn find_all_windows_hwnd_and_title() -> anyhow::Result<Vec<(HWND, Option<String>)>> {
	unsafe extern "system" fn enum_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
		if unsafe { !IsWindowVisible(hwnd).as_bool() } {
			return BOOL::from(true);
		}

		let windows = unsafe { &mut *(lparam.0 as *mut Vec<(HWND, Option<String>)>) };

		let mut buffer = [0u16; 512];
		let len = unsafe { GetWindowTextW(hwnd, &mut buffer) };
		let title = if len > 0 {
			Some(String::from_utf16_lossy(&buffer[..len as usize]))
		} else {
			None
		};

		windows.push((hwnd, title));

		BOOL::from(true)
	}

	let mut list = Vec::new();

	unsafe { EnumWindows(Some(enum_window_proc), LPARAM(&mut list as *mut _ as isize))? };

	Ok(list)
}

struct ClientCapture {
	sender: broadcast::Sender<Arc<RgbaImage>>,
}

impl GraphicsCaptureApiHandler for ClientCapture {
	type Flags = broadcast::Sender<Arc<RgbaImage>>;
	type Error = anyhow::Error;

	fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
		Ok(Self { sender: ctx.flags })
	}

	fn on_frame_arrived(
		&mut self,
		frame: &mut Frame,
		_capture_control: InternalCaptureControl,
	) -> Result<(), Self::Error> {
		let frame_buffer = frame.buffer()?;
		let mut buffer = Vec::new();
		let buffer = frame_buffer.as_nopadding_buffer(&mut buffer);
		let image_opt: Option<RgbaImage> =
			ImageBuffer::from_raw(frame_buffer.width(), frame_buffer.height(), buffer.to_vec());
		if let Some(image) = image_opt {
			let _ = self.sender.send(Arc::new(image));
		};
		Ok(())
	}
}

pub struct EveClient {
	pub hwnd: HWND,
	pub title: String,
	capture_sender: broadcast::Sender<Arc<RgbaImage>>,
}

impl EveClient {
	pub fn new(hwnd: HWND, title: String) -> Self {
		let (capture_sender, _) = broadcast::channel(16);
		Self {
			hwnd,
			title,
			capture_sender,
		}
	}

	// pub fn new_from_hwnd(hwnd: HWND) -> Self {
	// 	let mut buffer = [0u16; 512];
	// 	let len = unsafe { GetWindowTextW(hwnd, &mut buffer) };
	// 	let title = if len > 0 {
	// 		String::from_utf16_lossy(&buffer[..len as usize])
	// 	} else {
	// 		"".to_string()
	// 	};
	// 	Self::new(hwnd, title)
	// }

	pub fn new_from_title(title: impl ToString) -> anyhow::Result<Self> {
		let title = title.to_string();
		let hwnd = unsafe { FindWindowW(None, &HSTRING::from(&title))? };
		Ok(Self::new(hwnd, title))
	}

	pub fn find_all_eve_client_hwnd_and_title() -> anyhow::Result<Vec<(HWND, String)>> {
		let windows = find_all_windows_hwnd_and_title()?;
		let result = windows
			.into_iter()
			.filter_map(|(hwnd, title)| title.map(|title| (hwnd, title)))
			.filter(|(_, title)| title.starts_with("EVE - "))
			.collect::<Vec<_>>();

		Ok(result)
	}

	pub fn get_all_eve_client() -> anyhow::Result<Vec<Self>> {
		Ok(
			Self::find_all_eve_client_hwnd_and_title()?
				.into_iter()
				.map(|(hwnd, title)| Self::new(hwnd, title))
				.collect(),
		)
	}

	pub fn start_capture(&self) -> anyhow::Result<()> {
		let item = Window::from_raw_hwnd(self.hwnd.0);
		let settings = Settings::new(
			// Item to capture
			item,
			// Capture cursor settings
			CursorCaptureSettings::WithoutCursor,
			// Draw border settings
			DrawBorderSettings::WithoutBorder,
			// Secondary window settings, if you want to include secondary windows in the capture
			SecondaryWindowSettings::Default,
			// Minimum update interval, if you want to change the frame rate limit (default is 60 FPS or 16.67 ms)
			MinimumUpdateIntervalSettings::Custom(Duration::from_millis(500)),
			// Dirty region settings,
			DirtyRegionSettings::Default,
			// The desired color format for the captured frame.
			ColorFormat::Rgba8,
			// Additional flags for the capture settings that will be passed to the user-defined `new` function.
			self.capture_sender.clone(),
		);
		tokio::task::spawn_blocking(move || ClientCapture::start(settings));
		Ok(())
	}

	pub fn get_capture_receiver(&self) -> broadcast::Receiver<Arc<RgbaImage>> {
		self.capture_sender.subscribe()
	}
}
