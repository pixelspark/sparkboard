use rppal::{gpio::Level};
use spidev::{Spidev};
use rppal::gpio::{OutputPin, InputPin};
use std::thread::{sleep};
use std::time::Duration;
use std::error::Error;
use std::io::prelude::*;

pub struct EPD7in5v2 {
	spi: Spidev,
	busy: InputPin,
	cs: OutputPin,
	dc: OutputPin,
	rst: OutputPin
}

const EPD7IN5V2_WIDTH: usize = 800;
const EPD7IN5V2_HEIGHT: usize = 480;

impl EPD7in5v2 {
	pub fn new(spi: Spidev, busy: InputPin, cs: OutputPin, dc: OutputPin, rst: OutputPin) -> EPD7in5v2 {
		EPD7in5v2 {
			spi,
			busy,
			cs,
			dc,
			rst
		}
	}

	pub fn width() -> usize {
		EPD7IN5V2_WIDTH
	}

	pub fn height() -> usize {
		EPD7IN5V2_HEIGHT
	}

	// https://github.com/waveshare/e-Paper/blob/master/RaspberryPi_JetsonNano/c/lib/e-Paper/EPD_7in5_V2.c#L38
	fn reset(&mut self) {
		self.cs.set_high();
		log::debug!("reset");
		self.rst.set_high();
		sleep(Duration::from_millis(200));
		self.rst.set_low();
		sleep(Duration::from_millis(2));
		self.rst.set_high();
		sleep(Duration::from_millis(200));
		log::debug!("reset done; busy={:?}", self.busy.read());
	}

	pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
		self.reset();
		self.send(0x01, &[0x07, 0x07, 0x3f, 0x3f])?; // Power setting VGH=20V, VGL=-20V, VDH=15V, VDL=-15V
		self.send_command(0x04)?; // POWER ON
		sleep(Duration::from_millis(100));
		self.wait_until_idle()?;

		self.send(0x00, &[0x1F])?; // Panel setting  KW-3f   KWR-2F	BWROTP 0f	BWOTP 1f
		self.send(0x61, &[0x03, 0x20, 0x01, 0xE0])?; // TRES (resolution). 800x480

		self.send(0x15, &[0x00])?;
		self.send(0x50, &[0x10, 0x07])?; // VCOM, data interval settings

		self.send(0x60, &[0x22])?;	// TCON setting
		self.send(0x65, &[0x00, 0x00, 0x00, 0x00])?; // Resolution setting. 800x480

		Ok(())
	}

	pub fn clear(&mut self, black: bool) -> Result<(), Box<dyn Error>> {
		let width = if EPD7IN5V2_WIDTH % 8 == 0 {
			EPD7IN5V2_WIDTH / 8
		}
		else {
			EPD7IN5V2_WIDTH / 8 + 1
		};
		let height = EPD7IN5V2_HEIGHT;
		log::debug!("clear w={:?} h={:?}", width, height);

		let zeroes = vec![0x00 as u8; height * width];
		let ffs = vec![0xFF as u8; height * width];

		if black {
			//self.send(0x10, &zeroes)?;
			self.send(0x13, &ffs)?;
		}
		else {
			self.send(0x10, &zeroes)?;
			self.send(0x13, &zeroes)?;
		}

		self.turn_on_display()?;
		Ok(())
	}

	pub fn draw(&mut self, buffer: &[u8]) -> Result<(), Box<dyn Error>> {
		if buffer.len() != (EPD7IN5V2_HEIGHT * EPD7IN5V2_WIDTH / 8) {
			panic!("invalid buffer size");
		}

		self.send(0x13, buffer)?;
		self.turn_on_display()?;
		Ok(())
	}

	fn spi_transfer(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
		self.cs.set_low();
		for chunk in data.chunks(128) {
			if self.spi.write(chunk)? != chunk.len() {
				panic!("too little bytes written to SPI");
			}
		}
		self.cs.set_high();
		Ok(())
	}

	fn send(&mut self, cmd: u8, data: &[u8]) -> Result<(), Box<dyn Error>> {
		if data.len() > 10 {
			log::debug!("send {:02x?} with data {:02x?} (len={:?})", cmd, &data[0..10], data.len());
		}
		else {
			log::debug!("send {:02x?} with data {:02x?}", cmd, data);
		}
		self.send_command(cmd)?;
		self.send_data(data)?;
		Ok(())
	}

	fn send_command(&mut self, cmd: u8) -> Result<(), Box<dyn Error>> {
		self.dc.set_low();
		log::debug!("send cmd {:02x?} busy={:?}", cmd, self.busy.read());
		self.spi_transfer(&[cmd])?;
		Ok(())
	}

	fn send_data(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
		self.dc.set_high();
		self.spi_transfer(data)?;
		Ok(())
	}

	// https://github.com/waveshare/e-Paper/blob/master/RaspberryPi_JetsonNano/c/lib/e-Paper/EPD_7in5_V2.c#L78
	fn wait_until_idle(&mut self) -> Result<(), Box<dyn Error>> {
		//assert!(self.busy.is_high(), "expect busy pin to be high before waiting until idle");
		let mut n = 0;
		log::debug!("wait_until_idle busy={:?} {:?} {:?}", self.busy.read(), self.busy.read(), self.busy.read());
		loop {
			n += 1;
			self.send_command(0x71)?;
			sleep(Duration::from_millis(20));
			if self.busy.read() == Level::High {
				sleep(Duration::from_millis(100));
				log::debug!("busy release n={:?}", n);
				return Ok(());
			}
		}
	}

	// https://github.com/waveshare/e-Paper/blob/master/RaspberryPi_JetsonNano/c/lib/e-Paper/EPD_7in5_V2.c#L95
	fn turn_on_display(&mut self) -> Result<(), Box<dyn Error>> {
		log::debug!("turn_on_display");
		self.send_command(0x12)?; // Display refresh
		sleep(Duration::from_millis(100));
		self.wait_until_idle()?;
		log::debug!("turn_on_display done");
		Ok(())
	}

	// https://github.com/waveshare/e-Paper/blob/master/RaspberryPi_JetsonNano/c/lib/e-Paper/EPD_7in5_V2.c#L217
	pub fn sleep(&mut self) -> Result<(), Box<dyn Error>> {
		self.send_command(0x02)?; // Power off
		self.wait_until_idle()?;
		self.send(0x07, &[0xA5])?; // deep sleep
		Ok(())
	}
}