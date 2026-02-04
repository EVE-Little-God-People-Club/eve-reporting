use image::RgbaImage;

pub trait ImageChecker {
	fn check_point_rgb(&self, point: [u32; 2], target_rgb: [u8; 3]) -> bool;
	fn check_point_rgb_list(&self, point: [u32; 2], target_rgb: &[[u8; 3]]) -> bool {
		target_rgb
			.iter()
			.any(|rgb| self.check_point_rgb(point, *rgb))
	}
}

impl ImageChecker for RgbaImage {
	fn check_point_rgb(&self, point: [u32; 2], target_rgb: [u8; 3]) -> bool {
		let [x, y] = point;
		let (width, height) = self.dimensions();
		if x >= width || y >= height {
			return false;
		}
		let pixel = self.get_pixel(x, y);
		let [rt, gt, bt] = target_rgb;
		pixel[0] == rt && pixel[1] == gt && pixel[2] == bt
	}
}
