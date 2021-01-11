use bytes::Buf;
use dither::prelude::*;
use std::error::Error;

/** Fetch a URL and return it as a black and white (packed) bitmap. Size of the buffer will be width * height / 8. */
pub fn fetch(url: &str) -> std::result::Result<Vec<u8>, Box<dyn Error>> {
	let client = reqwest::blocking::Client::builder()
		.user_agent("Sparkboard/1.0")
		.danger_accept_invalid_certs(true)
		.build()?;

	let resp = client.get(url).send()?;
	let png = resp.bytes()?;
	log::info!("got image: {:#?} bytes", png.len());

	// Read PNG
	let decoder = png::Decoder::new(png.reader());
	let (info, mut reader) = decoder.read_info().unwrap();
	log::info!("image size {:?}x{:?}", info.width, info.height);

	// Allocate the output buffer.
	let mut buf = vec![0; info.buffer_size()];
	// Read the next frame. An APNG might contain multiple frames.
	reader.next_frame(&mut buf).unwrap();

	// Dither
	let mut source: Vec<RGB<u8>> = vec![RGB(0, 0, 0); (info.width * info.height) as usize];
	let w = info.width as usize;

	for x in 0..(info.width as usize) {
		for y in 0..(info.height as usize) {
			let r = buf[(x + w * y) * 4];
			let g = buf[(x + w * y) * 4 + 1];
			let b = buf[(x + w * y) * 4 + 2];
			//let avg = ((r as u64 + g as u64 + b as u64) / 3) as u8;
			let avg = r.min(g).min(b);

			source[x + w * y] = RGB(avg, avg, avg)
		}
	}

	let source_image = Img::new(source, info.width).unwrap();
	let source_image_f = source_image.convert_with(|rgb| rgb.convert_with(f64::from));

	let quantize = dither::create_quantize_n_bits_func(1)?;
	let dest = dither::ditherer::STUCKI
		.dither(source_image_f, RGB::map_across(quantize))
		.convert_with(|rgb| rgb.convert_with(clamp_f64_to_u8));

	// Build BW buffer
	let w = if info.width % 8 == 0 {
		info.width
	} else {
		info.width + 1
	};
	let mut buffer = vec![0x00 as u8; (w * info.height / 8) as usize];

	for y in 0..info.height {
		for x in 0..(w / 8) {
			// Pack eight consecutive pixels to a single byte
			let mut byte: u8 = 0;
			for k in 0..8 {
				let source_x = x * 8 + k;
				let source_pixel = dest[(source_x, y)];
				let black = source_pixel.0 > 0 || source_pixel.1 > 0 || source_pixel.2 > 0;
				// Shift current set of pixels left by one
				byte <<= 1;
				if !black {
					// Set rightmost bit
					byte |= 1;
				}
			}

			buffer[(y * (w / 8) + x) as usize] = byte;
		}
	}

	Ok(buffer)
}
