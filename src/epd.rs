use spidev::{Spidev};
use rppal::gpio::{OutputPin, InputPin};
use rppal::{gpio::Level};
use std::thread::{sleep};
use std::time::Duration;
use std::error::Error;
use std::io::prelude::*;

pub struct EPD {
	pub(crate) spi: Spidev,
	pub(crate) busy: InputPin,
	pub(crate) cs: OutputPin,
	pub(crate) dc: OutputPin,
	pub(crate) rst: OutputPin
}

impl EPD {
	pub fn spi_transfer(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
		self.cs.set_low();
		for chunk in data.chunks(128) {
			if self.spi.write(chunk)? != chunk.len() {
				panic!("too little bytes written to SPI");
			}
		}
		self.cs.set_high();
		Ok(())
	}

	pub fn send(&mut self, cmd: u8, data: &[u8]) -> Result<(), Box<dyn Error>> {
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

	pub fn send_command(&mut self, cmd: u8) -> Result<(), Box<dyn Error>> {
		self.dc.set_low();
		log::debug!("send cmd {:02x?} busy={:?}", cmd, self.busy.read());
		self.spi_transfer(&[cmd])?;
		Ok(())
	}

	pub fn send_data(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
		self.dc.set_high();
		self.spi_transfer(data)?;
		Ok(())
	}

	// https://github.com/waveshare/e-Paper/blob/master/RaspberryPi_JetsonNano/c/lib/e-Paper/EPD_7in5_V2.c#L38
	// https://github.com/waveshare/e-Paper/blob/master/RaspberryPi_JetsonNano/c/lib/e-Paper/EPD_7in5bc.c#L92
	pub fn reset(&mut self) {
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

	// https://github.com/waveshare/e-Paper/blob/master/RaspberryPi_JetsonNano/c/lib/e-Paper/EPD_7in5_V2.c#L78
	pub fn wait_until_idle(&mut self) -> Result<(), Box<dyn Error>> {
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
}