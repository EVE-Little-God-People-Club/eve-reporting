use image::RgbaImage;

pub trait ImageChecker {
	fn check_point_rgb(&self, point: &(u32, u32), target_rgb: &(u8, u8, u8)) -> bool;
}

impl ImageChecker for RgbaImage {
	fn check_point_rgb(&self, point: &(u32, u32), target_rgb: &(u8, u8, u8)) -> bool {
		let (x, y) = *point;
		let (width, height) = self.dimensions();
		if x >= width || y >= height {
			return false;
		}
		let pixel = self.get_pixel(x, y);
		let (rt, gt, bt) = *target_rgb;
		pixel[0] == rt && pixel[1] == gt && pixel[2] == bt
	}
}
