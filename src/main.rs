use rppal::gpio::Gpio;
use spidev::{SpiModeFlags, Spidev, SpidevOptions};
use std::{error::Error};
use std::process::Command;
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;

mod epd;
mod epd7in5_v2;
mod epd7in5bc;
mod fetch;
mod graphics;

use chrono::prelude::*;
use clap::{App, Arg, SubCommand};
use epd::EPDDisplay;
use epd7in5_v2::EPD7in5v2;
use epd7in5bc::EPD7in5bc;
use fetch::fetch;
use graphics::Surface;

use embedded_graphics::{
	fonts::{Font12x16, Font6x12, Text},
	pixelcolor::BinaryColor,
	prelude::*,
	primitives::Circle,
	style::PrimitiveStyle,
	text_style,
};

fn main() -> Result<(), Box<dyn Error>> {
	env_logger::init();

	let matches = App::new("SparkBoard")
		.version("1.0")
		.author("Tommy van der Vorst")
		.about("E-paper dashboard driver")
		.arg(
			Arg::with_name("device")
				.short("d")
				.long("config")
				.default_value("/dev/spidev0.0")
				.value_name("PATH")
				.help("SPI device to use")
				.takes_value(true),
		)
		.arg(
			Arg::with_name("type")
				.short("t")
				.long("type")
				.value_name("DEVICETYPE")
				.help("Type of device")
				.possible_value("7in5v2")
				.possible_value("7in5bc")
				.default_value("7in5v2")
				.takes_value(true),
		)
		.subcommand(SubCommand::with_name("test").about("perform tests"))
		.subcommand(
			SubCommand::with_name("fetch")
				.about("Periodically fetch a PNG image from a URL and display")
				.arg(
					Arg::with_name("interval")
						.short("i")
						.long("interval")
						.value_name("SECONDS")
						.help(
							"Number of seconds to wait between each fetch (excludes time taken by e-Paper to refresh)",
						)
						.default_value("0"),
				)
				.arg(
					Arg::with_name("wifi")
						.long("wifi")
						.value_name("INTERFACE")
						.help("Wait until Wi-Fi interface connects"),
				)
				.arg(Arg::with_name("url").takes_value(true).help("URL to fetch")),
		)
		.get_matches();

	// Configure SPI
	let spi_path = matches.value_of("device").unwrap_or("/dev/spidev0.0");
	log::info!("Using SPI device at {:?}", spi_path);
	let mut spi = Spidev::open(spi_path)?;
	let options = SpidevOptions::new()
		.bits_per_word(8)
		.max_speed_hz(10_000_000)
		.mode(SpiModeFlags::SPI_MODE_0)
		.build();
	spi.configure(&options)?;

	// Configure GPIO
	let gpio = Gpio::new()?;
	let cs = gpio.get(8)?.into_output(); // CE0 = BCM8
	let busy = gpio.get(24)?.into_input(); // BCM24, physical 18
	let dc = gpio.get(25)?.into_output(); // BCM25
	let rst = gpio.get(17)?.into_output(); // BCM17

	let device_type = matches.value_of("type").unwrap_or("7in5v2");

	if let Some(_subcommand_matches) = matches.subcommand_matches("test") {
		match device_type {
			"7in5bc" => {
				let mut display = EPD7in5bc::new(spi, busy, cs, dc, rst);
				test_7in5bc(&mut display)?;
			}
			"7in5v2" => {
				let mut display = EPD7in5v2::new(spi, busy, cs, dc, rst);
				test_7in5v2(&mut display)?;
			}
			_ => panic!("invalid device type: {:?}", device_type),
		}
	} else if let Some(subcommand_matches) = matches.subcommand_matches("fetch") {
		let url = subcommand_matches.value_of("url").expect("please provide a URL");
		let interval = u64::from_str(subcommand_matches.value_of("interval").unwrap())?;

		let mut display: Box<dyn EPDDisplay> = match device_type {
			"7in5bc" => Box::new(EPD7in5bc::new(spi, busy, cs, dc, rst)),
			"7in5v2" => Box::new(EPD7in5v2::new(spi, busy, cs, dc, rst)),
			_ => panic!("invalid device type: {:?}", device_type),
		};

		if let Some(wifi) = subcommand_matches.value_of("wifi") {
			log::info!("wait for Wi-Fi device {:?}...", wifi);
			wait_for_wifi(&mut display, wifi)?;
		}

		loop {
			log::info!("fetching new image from {:?}", url);
			match fetch(url) {
				Ok(buffer) => {
					log::info!("displaying new image");
					display.init()?;
					display.draw(&buffer)?;
					sleep(Duration::from_millis(500));
					display.sleep()?;

					if interval > 0 {
						log::info!("sleeping for {:?} seconds", interval);
						sleep(Duration::from_secs(interval));
					} else {
						return Ok(());
					}
				}
				Err(e) => {
					log::error!("fetch of {:?} failed: {:?}", url, e);
					display_string(&mut display, &format!("fetch failed: {:?}", e))?;
					log::info!("sleeping for five seconds before trying again...");
					sleep(Duration::from_secs(5));
				}
			}
		}
	}

	Ok(())
}

fn wait_for_wifi(display: &mut Box<dyn EPDDisplay>, interface: &str) -> Result<(), Box<dyn Error>> {
	loop {
		log::info!("Waiting for Wi-Fi interface {:?}", interface);
		// Check Wi-Fi status
		let wifi_status = Command::new("/usr/bin/wpa_cli")
			.arg(&format!("-i{}", interface))
			.arg("-s/var/run/wpa_supplicant")
			.arg("status")
			.output()?;
		log::info!("wifi: status={:?}", wifi_status);

		if !wifi_status.status.success() {
			log::info!("wpa_cli command was not sucessful; try again in a few seconds...");
			let status_string = String::from_utf8(wifi_status.stdout).unwrap();
			let message = format!("Waiting for Wi-Fi: {:?}", status_string);
			display_string(display, &message)?;
		}
		else {
			let status_string = String::from_utf8(wifi_status.stdout).unwrap();
			if status_string.contains("wpa_state=COMPLETED\n") {
				// Still scanning
				log::info!("Not scanning anymore!");
				return Ok(());
			}
			display_string(display, &status_string)?;
		}
	}
}

fn display_string(display: &mut Box<dyn EPDDisplay>, text: &str) -> Result<(), Box<dyn Error>> {
	display.init()?;
	let mut image = Surface::new(display.width() as u32, display.height() as u32);

	let mut y = 10;
	for txt in text.chars().collect::<Vec<char>>().chunks(100) {
		let line = txt.iter().collect::<String>();
		let _ = Text::new(&line, Point::new(10, y))
			.into_styled(text_style!(
				font = Font6x12,
				text_color = BinaryColor::Off,
				background_color = BinaryColor::On
			))
			.draw(&mut image);
		y += 14;
	}

	display.draw(image.buffer())?;
	display.sleep()?;
	Ok(())
}

fn test_7in5v2(display: &mut EPD7in5v2) -> Result<(), Box<dyn Error>> {
	display.init()?;
	display.clear(false)?;
	sleep(Duration::from_millis(500));

	let mut image = Surface::new(display.width() as u32, display.height() as u32);

	loop {
		let _ = Text::new("Sparkboard (C) Pixelspark, 2021", Point::new(10, 10))
			.into_styled(text_style!(
				font = Font12x16,
				text_color = BinaryColor::Off,
				background_color = BinaryColor::On
			))
			.draw(&mut image);

		// Draw time
		let local: DateTime<Local> = Local::now();
		let time_string = format!("Time: {:?}", local);

		let _ = Text::new(&time_string, Point::new(10, 36))
			.into_styled(text_style!(
				font = Font12x16,
				text_color = BinaryColor::Off,
				background_color = BinaryColor::On
			))
			.draw(&mut image);

		let w = display.width() as i32;
		let h = display.height() as i32;
		let _ = Circle::new(Point::new(w - 20, h - 20), 10)
			.into_styled(PrimitiveStyle::with_stroke(BinaryColor::Off, 1))
			.draw(&mut image);

		display.draw(image.buffer())?;
		sleep(Duration::from_millis(500));
		display.sleep()?;
		log::debug!("display sleeping");
		sleep(Duration::from_millis(60000));
		display.init()?;
	}
}

fn test_7in5bc(display: &mut EPD7in5bc) -> Result<(), Box<dyn Error>> {
	display.init()?;
	display.clear(false)?;
	sleep(Duration::from_millis(500));

	let mut image = Surface::new(display.width() as u32, display.height() as u32);
	let mut color_image = Surface::new(display.width() as u32, display.height() as u32);

	loop {
		let _ = Text::new("Sparkboard (C) Pixelspark, 2021", Point::new(10, 10))
			.into_styled(text_style!(
				font = Font12x16,
				text_color = BinaryColor::Off,
				background_color = BinaryColor::On
			))
			.draw(&mut image);

		// Draw time
		let local: DateTime<Local> = Local::now();
		let time_string = format!("Time: {:?}", local);

		let _ = Text::new(&time_string, Point::new(10, 36))
			.into_styled(text_style!(
				font = Font12x16,
				text_color = BinaryColor::Off,
				background_color = BinaryColor::On
			))
			.draw(&mut color_image);

		display.draw_bichromatic(image.buffer(), color_image.buffer())?;
		sleep(Duration::from_millis(500));
		display.sleep()?;
		log::debug!("display sleeping");
		sleep(Duration::from_millis(60000));
		display.init()?;
	}
}
