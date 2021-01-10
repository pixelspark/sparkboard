use spidev::{Spidev, SpidevOptions, SpiModeFlags};
use rppal::gpio::{Gpio};
use std::thread::{sleep};
use std::time::Duration;
use std::error::Error;

mod epd7in5_v2;
mod graphics;
use epd7in5_v2::EPD7in5v2;
use graphics::Surface;

use embedded_graphics::{
	prelude::*,
	style::PrimitiveStyle,
	text_style,
	fonts::{Font12x16, Font6x8, Text},
	pixelcolor::BinaryColor
};

fn main() -> Result<(), Box<dyn Error>> {
	env_logger::init();

	let mut spi = Spidev::open("/dev/spidev0.0")?;
	let options = SpidevOptions::new()
		 .bits_per_word(8)
		 .max_speed_hz(10_000_000)
		 .mode(SpiModeFlags::SPI_MODE_0)
		 .build();
	spi.configure(&options)?;

	let gpio = Gpio::new()?;
	let cs = gpio.get(8)?.into_output(); // CE0 = BCM8
	let busy = gpio.get(24)?.into_input(); // BCM24, physical 18
	let dc = gpio.get(25)?.into_output(); // BCM25
	let rst = gpio.get(17)?.into_output(); // BCM17

	let mut display = EPD7in5v2::new(spi, busy, cs, dc, rst);
	display.init()?;

	display.clear(false)?;
	sleep(Duration::from_millis(500));
	println!("cleared");
	/*display.clear(true)?;
	sleep(Duration::from_millis(500));*/

	let mut image = Surface::new(EPD7in5v2::width() as u32, EPD7in5v2::height() as u32);

	let _ = Text::new("Na een hele zondag puzzelen werkt mijn e-paper!", Point::new(10, 10)).into_styled(text_style!(
			font = Font12x16,
			text_color = BinaryColor::Off,
			background_color = BinaryColor::On
        ))
        .draw(&mut image);

	display.draw(image.buffer())?;
	sleep(Duration::from_millis(500));

	display.sleep()?;

	Ok(())
}