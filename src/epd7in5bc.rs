use spidev::{Spidev};
use rppal::gpio::{OutputPin, InputPin};
use std::thread::{sleep};
use std::time::Duration;
use std::error::Error;

use crate::epd::EPD;

pub struct EPD7in5bc {
	epd: EPD
}

const EPD7IN5BC_WIDTH: usize = 640;
const EPD7IN5BC_HEIGHT: usize = 384;

impl EPD7in5bc {
	pub fn new(spi: Spidev, busy: InputPin, cs: OutputPin, dc: OutputPin, rst: OutputPin) -> EPD7in5bc {
		EPD7in5bc {
			epd: EPD {
				spi,
				busy,
				cs,
				dc,
				rst
			}
		}
	}

	pub fn width() -> usize {
		EPD7IN5BC_WIDTH
	}

	pub fn height() -> usize {
		EPD7IN5BC_HEIGHT
	}

	// https://github.com/waveshare/e-Paper/blob/master/RaspberryPi_JetsonNano/c/lib/e-Paper/EPD_7in5bc.c#L161
	pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
		self.epd.reset();
		self.epd.send(0x01, &[0x37, 0x00])?; // POWER_SETTING
		self.epd.send(0x00, &[0xCF, 0x08])?; // PANEL_SETTING
		self.epd.send(0x30, &[0x3A])?; // PLL_CONTROL
		self.epd.send(0x82, &[0x28])?; // VCM_DC_SETTING: all temperature range
		self.epd.send(0x06, &[0xC7, 0xCC, 0x15])?; // BOOSTER_SOFT_START
		self.epd.send(0x50, &[0x77])?; // VCOM AND DATA INTERVAL SETTING
		self.epd.send(0x60, &[0x22])?; // TCON_SETTING
		self.epd.send(0x65, &[0x00])?; // FLASH CONTROL
		self.epd.send(0x61, &[
			(EPD7IN5BC_WIDTH >> 8) as u8,
			(EPD7IN5BC_WIDTH & 0xFF) as u8,
			(EPD7IN5BC_HEIGHT >> 8) as u8,
			(EPD7IN5BC_HEIGHT & 0xFF) as u8
		])?; // TCON_RESOLUTION
		self.epd.send(0xE5, &[0x03])?; // FLASH MODE
		Ok(())
	}

	pub fn clear(&mut self, black: bool) -> Result<(), Box<dyn Error>> {
		let width = EPD7IN5BC_WIDTH / 8;
		let height = EPD7IN5BC_HEIGHT;
		log::debug!("clear w={:?} h={:?}", width, height);

		let pixels = vec![0x33 as u8; height * width * 4];
		self.epd.send(0x10, &pixels)?;
		self.turn_on_display()?;
		Ok(())
	}

	// https://github.com/waveshare/e-Paper/blob/master/RaspberryPi_JetsonNano/c/lib/e-Paper/EPD_7in5bc.c#L229
	pub fn draw(&mut self, black_buffer: &[u8], color_buffer: &[u8]) -> Result<(), Box<dyn Error>> {
		if black_buffer.len() != (EPD7IN5BC_HEIGHT * EPD7IN5BC_WIDTH / 8) {
			panic!("invalid buffer size");
		}
		if color_buffer.len() != (EPD7IN5BC_HEIGHT * EPD7IN5BC_WIDTH / 8) {
			panic!("invalid color buffer size");
		}

		let full_size = EPD7IN5BC_HEIGHT * EPD7IN5BC_WIDTH / 2;
		let mut full_image = vec![0x00; full_size]; // one byte for two pixels: 0=black, 3=white, 4=color; 0x44 = two adjacent color pixels
		let mut i = 0;
		for y in 0..EPD7IN5BC_HEIGHT {
			for x in (0..(EPD7IN5BC_WIDTH / 8)) {
				let eight_pixels_black = black_buffer[y * (EPD7IN5BC_WIDTH / 8) + x];
				let eight_pixels_color = color_buffer[y * (EPD7IN5BC_WIDTH / 8) + x];

				for k in (0..8).step_by(2) {
					let left_black = ((eight_pixels_black << k) & 0x80) != 0;
					let right_black = ((eight_pixels_black << (k + 1)) & 0x80) != 0;
					let left_color = ((eight_pixels_color << k) & 0x80) != 0;
					let right_color = ((eight_pixels_color << (k + 1)) & 0x80) != 0;
					let pixel = if left_color { 0x40 } else if left_black { 0x00 } else { 0x30 } |
						if right_color { 0x04 } else if right_black { 0x00 } else { 0x03 };
					full_image[i] = pixel;
					i += 1;
				}
			}
		}

		self.epd.send(0x10, &full_image)?;
		self.turn_on_display()?;
		Ok(())
	}

	// https://github.com/waveshare/e-Paper/blob/master/RaspberryPi_JetsonNano/c/lib/e-Paper/EPD_7in5bc.c#L148
	fn turn_on_display(&mut self) -> Result<(), Box<dyn Error>> {
		log::debug!("turn_on_display");
		self.epd.send_command(0x04)?; // Power on
		self.epd.wait_until_idle()?;
		self.epd.send_command(0x12)?; // Display refresh
		sleep(Duration::from_millis(100));
		self.epd.wait_until_idle()?;
		log::debug!("turn_on_display done");
		Ok(())
	}

	// https://github.com/waveshare/e-Paper/blob/master/RaspberryPi_JetsonNano/c/lib/e-Paper/EPD_7in5bc.c#L325
	pub fn sleep(&mut self) -> Result<(), Box<dyn Error>> {
		self.epd.send_command(0x02)?; // Power off
		self.epd.wait_until_idle()?;
		self.epd.send(0x07, &[0xA5])?; // deep sleep
		Ok(())
	}
}