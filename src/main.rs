extern crate scrap;

use launchpad::{mk2::{LaunchpadMk2, MidiLaunchpadMk2, Location}, RGBColor};

use scrap::{Capturer, Display};

use structopt::StructOpt;

use std::io::ErrorKind::WouldBlock;
use std::thread;
use std::time::Duration;
use std::time::Instant;

#[derive(StructOpt, Debug)]
#[structopt(name = "backlight")]
struct Options {
    /// Output brightness, float in range [0, 1].
    #[structopt(short, long, parse(try_from_str = parse_float_range), default_value = "1.0")]
    brightness: f32,
    /// Output intensity, float in range [0, 1].
    #[structopt(short, long, parse(try_from_str = parse_float_range), default_value = "1.0")]
    intensity: f32,
    /// Target frames per second.
    #[structopt(short, long, default_value = "60")]
    fps: usize,
    /// Index of screen to display.
    /// If unset, the primary display will be used.
    #[structopt(short, long)]
    display: Option<usize>,
}

fn parse_float_range(s: &str) -> Result<f32, String> {
    let value: f32 = s.parse()
        .map_err(|_| "could not parse float")?;

    if value >= 0. && value <= 1. {
        Ok(value)
    } else {
        Err("must be in range [0, 1]".to_owned())
    }
}

fn main() -> launchpad::Result<()> {
    // let brightness: f64 = value_t!(matches.value_of("brightness"), f64).unwrap_or(1.0);
    // let intensity: f64 = value_t!(matches.value_of("intensity"), f64).unwrap_or(1.0);
    // let display_index: clap::Result<usize> = value_t!(matches.value_of("display"), usize);
    let opt = Options::from_args();

    // launchpad device
    let mut launchpad = MidiLaunchpadMk2::autodetect()?;

    // clear at start
    launchpad.light_all(6)?;
    thread::sleep(Duration::from_secs(1));
    launchpad.light_all(0)?;

    // screen capture
    let display = if let Some(display) = opt.display {
        Display::all().expect("Could not retrieve displays").remove(display)
    } else {
        Display::primary().expect("Could not retrieve primary display")
    };

    let mut capturer = Capturer::new(display).expect("Couldn't begin capture");
    let (width, height) = (capturer.width(), capturer.height());

    loop {
        let t_a = Instant::now();

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

        let t_b = Instant::now();

        let stride = buffer.len() / height;
        // let count = (buffer.len() / 4) as u128;

        // find average

        let cell_width = width / 8;
        let cell_height = height / 8;

        let count = (cell_width * cell_height) as u128 / 4;

        let mut lights = Vec::with_capacity(64);

        // for each of the pads
        for pad_y in 0..8 {
            for pad_x in 0..8 {
                let mut sum = (0u128, 0u128, 0u128);

                for y in ((cell_height * pad_y)..((pad_y + 1) * cell_height)).step_by(2) {
                    for x in ((cell_width * pad_x)..((pad_x + 1) * cell_width)).step_by(2) {
                        let i = stride * y + 4 * x;

                        sum.0 += buffer[i + 2] as u128;
                        sum.1 += buffer[i + 1] as u128;
                        sum.2 += buffer[i] as u128;
                    }
                }

                // calculate average
                let color = (sum.0 / count, sum.1 / count, sum.2 / count);

                // map to 0.0-1.0
                let color = (color.0 as f32 / 255., color.1 as f32 / 255., color.2 as f32 / 255.);

                // intensify
                let exp = 1.5;
                let color = (color.0.powf(exp), color.1.powf(exp), color.2.powf(exp));

                // normalize
                let length = (color.0 * color.0 + color.1 * color.1 + color.2 * color.2).sqrt();
                //let length = length.powf(1.1); // curve a bit
                let color = (color.0 / length, color.1 / length, color.2 / length);

                // to 0-255 for launchpad
                let color = RGBColor::new(
                    (color.0 * 255.) as u8,
                    (color.1 * 255.) as u8,
                    (color.2 * 255.) as u8,
                );

                // done!
                lights.push((Location::Pad(7 - pad_x as u8, 7 - pad_y as u8), color));
            }
        }

        launchpad.light_multi_rgb(lights)?;

        thread::sleep(Duration::from_millis(60));
    }

    Ok(())
}
