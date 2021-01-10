use embedded_graphics::prelude::*;
use embedded_graphics::pixelcolor::BinaryColor;

pub struct Surface {
	buffer: Vec<u8>,
	width: u32,
	height: u32
}

impl Surface {
	pub fn new(width: u32, height: u32) -> Surface {
		Surface {
			buffer: vec![0; (width * height) as usize / 8],
			width,
			height
		}
	}

	fn find_position(&self, x: i32, y: i32) -> (usize, u8) {
		if x < 0 || y < 0 {
			return (0, 0);
		}

		(
			((x as u32) / 8 + (self.width / 8) * (y as u32)) as usize,
			0x80 >> ((x as u32) % 8),
		)
	}

	pub fn buffer(&self) -> &[u8] {
		&self.buffer
	}
}

impl DrawTarget<BinaryColor> for Surface {
	type Error = core::convert::Infallible;

	fn draw_pixel(&mut self, pixel: Pixel<BinaryColor>) -> Result<(), Self::Error> {
		let Pixel(point, color) = pixel;
		let (index, bit) = self.find_position(point.x, point.y);
		match color {
			BinaryColor::On => {
				self.buffer[index] &= !bit;
			}
			// White
			BinaryColor::Off => {
				self.buffer[index] |= bit;
			}
		}
		Ok(())
	}

	fn size(&self) -> Size {
		Size::new(self.width, self.height)
	}
}