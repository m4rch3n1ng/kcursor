use cursor_icon::CursorIcon;
use kcursor::CursorTheme;
use png::Encoder as PngEncoder;
use std::{fs::File, io::BufWriter};

fn main() {
	let theme = CursorTheme::load("Breeze_Light").unwrap();
	let icon = theme.icon(CursorIcon::Wait).unwrap();

	let size = 48;
	let frames = icon.frames(size).unwrap();
	let frame = &frames[0];

	let file = File::create("image.png").unwrap();
	let file = BufWriter::new(file);

	let mut encoder = PngEncoder::new(file, frame.width, frame.height);
	encoder.set_color(png::ColorType::Rgba);
	encoder.set_depth(png::BitDepth::Eight);

	let mut writer = encoder.write_header().unwrap();
	writer.write_image_data(&frame.pixels).unwrap();
}
