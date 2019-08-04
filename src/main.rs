extern crate scrap;

use scrap::{Capturer, Display};

use std::io::ErrorKind::WouldBlock;
use std::thread;
use std::time::Duration;
use launchpad::{LaunchpadMk2, RGBColor};

fn main() {
	// launchpad device
	let mut launchpad = LaunchpadMk2::autodetect();

	launchpad.light_all_raw(0);

	// screen capture
	let display = Display::all().expect("Couldn't find display").remove(1);
	let mut capturer = Capturer::new(display).expect("Couldn't begin capture");
	let (width, height) = (capturer.width(), capturer.height());

	loop {
		let buffer = match capturer.frame() {
			Ok(buffer) => buffer,
			Err(error) => {
				if error.kind() == WouldBlock {
					thread::sleep(Duration::from_millis(15));
					continue;
				} else {
					panic!("Error: {}", error);
				}
			}
		};

		let stride = buffer.len() / height;
		let count = (buffer.len() / 4) as u128;

		// find average

		let width = width / 8;
		let height = height / 8;

		let count = (width * height) as u128;

		// for each of the pads
		for pad_y in 0..8 {
			for pad_x in 0..8 {
				let mut sum = (0u128, 0u128, 0u128);

				for y in (height * pad_y)..((pad_y + 1) * height) {
					for x in width * pad_x..(pad_x + 1) * width {
						let i = stride * y + 4 * x;

						sum.0 += buffer[i + 2] as u128;
						sum.1 += buffer[i + 1] as u128;
						sum.2 += buffer[i] as u128;
					}
				}

				// calculate average
				let color = (sum.0 / count, sum.1 / count, sum.2 / count);

				// map to 0.0-1.0
				let color = (color.0 as f32 / 255f32, color.1 as f32 / 255f32, color.2 as f32 / 255f32);

				// normalize
				let length = (color.0 * color.0 + color.1 * color.1 + color.2 * color.2).sqrt();
				let length = length.powf(0.3); // curve a bit
				let color = (color.0 / length, color.1 / length, color.2 / length);

				// to 0-255 for launchpad
				let color = RGBColor::new((color.0 * 255f32) as u8, (color.1 * 255f32) as u8, (color.2 * 255f32) as u8);

				// done!
				launchpad.light_single(7 - pad_x as u8, 7 - pad_y as u8, &color);
			}
		}

		thread::sleep(Duration::from_millis(5));
	}
}
