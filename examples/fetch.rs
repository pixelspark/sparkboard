use std::path::Path;
use bytes::{Buf};
use dither::prelude::*;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
	let client = reqwest::Client::builder()
	.user_agent("Sparkboard/1.0")
	.danger_accept_invalid_certs(true)
	.build()?;

	let resp = client.get("https://tynas.local:8002/2/TKjkMUq7eoL/png").send().await?;
	let png = resp.bytes().await?;
	println!("{:#?}", png.len());

	// Read PNG
	let decoder = png::Decoder::new(png.reader());
	let (info, mut reader) = decoder.read_info().unwrap();
	// Allocate the output buffer.
	let mut buf = vec![0; info.buffer_size()];
	// Read the next frame. An APNG might contain multiple frames.
	reader.next_frame(&mut buf).unwrap();

	// Dither
	let mut source: Vec<RGB<u8>> = vec![RGB(0,0,0); (info.width * info.height) as usize];
	let w = info.width as usize;

	for x in 0..(info.width as usize) {
		for y in 0..(info.height as usize) {
			let r = buf[(x + w * y) * 4];
			let g = buf[(x + w * y) * 4 + 1];
			let b = buf[(x + w * y) * 4 + 2];
			//let avg = ((r as u64 + g as u64 + b as u64) / 3) as u8;
			let avg = r.min(g).min(b);

			source[x + w * y] = RGB(
				avg, avg, avg
			)
		}
	}

	let source_image = Img::new(source, info.width).unwrap();
	// source_image.clone().save(Path::new(r"in.png"));
	let source_image_f = source_image.convert_with(|rgb| rgb.convert_with(f64::from));

	let quantize = dither::create_quantize_n_bits_func(1)?;
	let dest = dither::ditherer::STUCKI.dither(source_image_f, RGB::map_across(quantize))
		.convert_with(|rgb| rgb.convert_with(clamp_f64_to_u8));

	dest.save(Path::new(r"out.png"))?;

	Ok(())
}