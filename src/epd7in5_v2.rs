use crate::epd::{EPDDisplay, EPD};
use rppal::gpio::{InputPin, OutputPin};
use spidev::Spidev;
use std::error::Error;
use std::thread::sleep;
use std::time::Duration;
pub struct EPD7in5v2 {
	epd: EPD,
}

const EPD7IN5V2_WIDTH: usize = 800;
const EPD7IN5V2_HEIGHT: usize = 480;

impl EPD7in5v2 {
	pub fn new(spi: Spidev, busy: InputPin, cs: OutputPin, dc: OutputPin, rst: OutputPin) -> EPD7in5v2 {
		EPD7in5v2 {
			epd: EPD { spi, busy, cs, dc, rst },
		}
	}

	pub fn clear(&mut self, black: bool) -> Result<(), Box<dyn Error>> {
		let width = if EPD7IN5V2_WIDTH % 8 == 0 {
			EPD7IN5V2_WIDTH / 8
		} else {
			EPD7IN5V2_WIDTH / 8 + 1
		};
		let height = EPD7IN5V2_HEIGHT;
		log::debug!("clear w={:?} h={:?}", width, height);

		let zeroes = vec![0x00 as u8; height * width];
		let ffs = vec![0xFF as u8; height * width];

		if black {
			//self.send(0x10, &zeroes)?;
			self.epd.send(0x13, &ffs)?;
		} else {
			self.epd.send(0x10, &zeroes)?;
			self.epd.send(0x13, &zeroes)?;
		}

		self.turn_on_display()?;
		Ok(())
	}

	// https://github.com/waveshare/e-Paper/blob/master/RaspberryPi_JetsonNano/c/lib/e-Paper/EPD_7in5_V2.c#L95
	fn turn_on_display(&mut self) -> Result<(), Box<dyn Error>> {
		log::debug!("turn_on_display");
		self.epd.send_command(0x12)?; // Display refresh
		sleep(Duration::from_millis(100));
		self.epd.wait_until_idle()?;
		log::debug!("turn_on_display done");
		Ok(())
	}
}

impl EPDDisplay for EPD7in5v2 {
	// https://github.com/waveshare/e-Paper/blob/master/RaspberryPi_JetsonNano/c/lib/e-Paper/EPD_7in5_V2.c#L217
	fn sleep(&mut self) -> Result<(), Box<dyn Error>> {
		self.epd.send_command(0x02)?; // Power off
		self.epd.wait_until_idle()?;
		self.epd.send(0x07, &[0xA5])?; // deep sleep
		Ok(())
	}

	fn draw(&mut self, buffer: &[u8]) -> Result<(), Box<dyn Error>> {
		if buffer.len() != (EPD7IN5V2_HEIGHT * EPD7IN5V2_WIDTH / 8) {
			panic!("invalid buffer size");
		}

		self.epd.send(0x13, buffer)?;
		self.turn_on_display()?;
		Ok(())
	}

	fn width(&self) -> usize {
		EPD7IN5V2_WIDTH
	}

	fn height(&self) -> usize {
		EPD7IN5V2_HEIGHT
	}

	fn init(&mut self) -> Result<(), Box<dyn Error>> {
		self.epd.reset();
		self.epd.send(0x01, &[0x07, 0x07, 0x3f, 0x3f])?; // Power setting VGH=20V, VGL=-20V, VDH=15V, VDL=-15V
		self.epd.send_command(0x04)?; // POWER ON
		sleep(Duration::from_millis(100));
		self.epd.wait_until_idle()?;

		self.epd.send(0x00, &[0x1F])?; // Panel setting  KW-3f   KWR-2F	BWROTP 0f	BWOTP 1f
		self.epd.send(0x61, &[0x03, 0x20, 0x01, 0xE0])?; // TRES (resolution). 800x480

		self.epd.send(0x15, &[0x00])?;
		self.epd.send(0x50, &[0x10, 0x07])?; // VCOM, data interval settings

		self.epd.send(0x60, &[0x22])?; // TCON setting
		self.epd.send(0x65, &[0x00, 0x00, 0x00, 0x00])?; // Resolution setting. 800x480

		Ok(())
	}
}
