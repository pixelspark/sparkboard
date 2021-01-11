use rppal::gpio::Gpio;
use spidev::{SpiModeFlags, Spidev, SpidevOptions};
use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

mod epd;
mod epd7in5_v2;
mod epd7in5bc;
mod fetch;
mod graphics;

use chrono::prelude::*;
use clap::{App, Arg, SubCommand};
use epd7in5_v2::EPD7in5v2;
use epd7in5bc::EPD7in5bc;
use fetch::fetch;
use graphics::Surface;

use embedded_graphics::{
	fonts::{Font12x16, Text},
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
				.value_name("/dev/spidev0.0")
				.help("SPI device to use")
				.takes_value(true),
		)
		.arg(
			Arg::with_name("type")
				.short("t")
				.long("type")
				.value_name("7in5v2")
				.help("Type of device. Acceptable values are: 7in5bc, 7in5v2")
				.takes_value(true),
		)
		.subcommand(SubCommand::with_name("test").about("perform tests"))
		.subcommand(
			SubCommand::with_name("fetch")
				.about("fetch PNG and display")
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
		let buffer = fetch(url)?;
		match device_type {
			"7in5bc" => {
				let mut display = EPD7in5bc::new(spi, busy, cs, dc, rst);
				display.init()?;
				let zeroes = vec![0x00; EPD7in5bc::width() * EPD7in5bc::height() / 8];
				display.draw(&buffer, &zeroes)?;
				sleep(Duration::from_millis(500));
				display.sleep()?;
			}
			"7in5v2" => {
				let mut display = EPD7in5v2::new(spi, busy, cs, dc, rst);
				display.init()?;
				display.draw(&buffer)?;
				sleep(Duration::from_millis(500));
				display.sleep()?;
			}
			_ => panic!("invalid device type: {:?}", device_type),
		}
	}

	Ok(())
}

fn test_7in5v2(display: &mut EPD7in5v2) -> Result<(), Box<dyn Error>> {
	display.init()?;
	display.clear(false)?;
	sleep(Duration::from_millis(500));

	let mut image = Surface::new(EPD7in5v2::width() as u32, EPD7in5v2::height() as u32);

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

		let w = EPD7in5v2::width() as i32;
		let h = EPD7in5v2::height() as i32;
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

	let mut image = Surface::new(EPD7in5bc::width() as u32, EPD7in5bc::height() as u32);
	let mut color_image = Surface::new(EPD7in5bc::width() as u32, EPD7in5bc::height() as u32);

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

		display.draw(image.buffer(), color_image.buffer())?;
		sleep(Duration::from_millis(500));
		display.sleep()?;
		log::debug!("display sleeping");
		sleep(Duration::from_millis(60000));
		display.init()?;
	}
}
