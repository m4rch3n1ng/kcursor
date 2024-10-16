// official ref
// https://invent.kde.org/plasma/kwin/-/blob/master/src/utils/svgcursorreader.cpp?ref_type=heads

use resvg::{
	tiny_skia::Pixmap,
	usvg::{Transform, Tree},
};
use serde::Deserialize;
use std::{
	borrow::Cow,
	collections::HashMap,
	fmt::Debug,
	path::{Path, PathBuf},
	sync::{Arc, LazyLock},
};

fn xdg_data_dirs() -> Vec<PathBuf> {
	let Some(data_dirs) = std::env::var_os("XDG_DATA_DIRS") else {
		return vec![PathBuf::from("/usr/share/icons")];
	};

	std::env::split_paths(&data_dirs)
		.map(|mut path| {
			path.push("icons");
			path
		})
		.collect()
}

fn user_theme_dirs() -> Vec<PathBuf> {
	let home = std::env::var_os("XDG_HOME")
		.or_else(|| std::env::var_os("HOME"))
		.expect("$HOME is not set");
	let home = PathBuf::from(home);

	let xdg_data_home = std::env::var_os("XDG_DATA_HOME")
		.map(PathBuf::from)
		.unwrap_or_else(|| home.join(".local/share"));

	vec![xdg_data_home.join("icons"), home.join(".icons")]
}

static CURSOR_DIRS: LazyLock<Vec<PathBuf>> = LazyLock::new(|| {
	let mut user_dirs = user_theme_dirs();
	user_dirs.extend(xdg_data_dirs());
	user_dirs
});

#[derive(Debug)]
pub struct CursorTheme {
	cache: HashMap<String, Arc<CursorIcon>>,
}

impl CursorTheme {
	pub fn load(name: &str) -> Option<Self> {
		let mut cache = HashMap::new();

		CursorTheme::discover(name, &mut cache);

		if cache.is_empty() {
			None
		} else {
			Some(CursorTheme { cache })
		}
	}

	fn discover(icon: &str, cache: &mut HashMap<String, Arc<CursorIcon>>) {
		let mut stack = vec![Cow::Borrowed(icon)];

		while let Some(name) = stack.pop() {
			let mut inherits = None;

			for path in &*CURSOR_DIRS {
				let path = path.join(&*name);
				if path.is_dir() {
					let scalable = path.join("cursors_scalable");
					if scalable.is_dir() {
						CursorTheme::discover_svg_cursors(scalable, cache);
					} else {
						let xcursors = path.join("cursors");
						if xcursors.is_dir() {
							CursorTheme::discover_x_cursors(xcursors, cache);
						}
					}

					if inherits.is_none() {
						let index = path.join("index.theme");
						if let Some(it) = theme_inherits(index) {
							inherits = Some(it)
						}
					}
				}
			}

			if let Some(it) = inherits {
				stack.push(Cow::Owned(it));
			}
		}
	}

	fn discover_svg_cursors(directory: PathBuf, cache: &mut HashMap<String, Arc<CursorIcon>>) {
		let (entries, symlinks) = directory
			.read_dir()
			.unwrap()
			.filter_map(Result::ok)
			.filter(|entry| entry.metadata().is_ok())
			.partition::<Vec<_>, _>(|entry| !entry.metadata().unwrap().is_symlink());

		for entry in entries.into_iter().chain(symlinks.into_iter()) {
			let shape = entry.file_name();
			let shape = shape.into_string().unwrap();

			if cache.contains_key(&shape) {
				continue;
			}

			if entry.metadata().unwrap().is_symlink() {
				let symlink = entry.path();
				let target = std::fs::read_link(&symlink).unwrap();

				assert_eq!(target.file_name(), Some(target.as_os_str()));
				let target = target.into_os_string().into_string().unwrap();

				if let Some(target) = cache.get(&target) {
					cache.insert(shape, target.clone());
				}
			} else {
				let path = entry.path();
				cache.insert(shape, Arc::new(CursorIcon::Svg { path }));
			}
		}
	}

	fn discover_x_cursors(directory: PathBuf, cache: &mut HashMap<String, Arc<CursorIcon>>) {
		let (entries, symlinks): (Vec<_>, Vec<_>) = directory
			.read_dir()
			.unwrap()
			.filter_map(Result::ok)
			.filter(|entry| entry.metadata().is_ok())
			.partition(|entry| !entry.metadata().unwrap().is_symlink());

		for entry in entries.into_iter().chain(symlinks) {
			let shape = entry.file_name();
			let shape = shape.into_string().unwrap();

			if cache.contains_key(&shape) {
				continue;
			}

			if entry.metadata().unwrap().is_symlink() {
				let symlink = entry.path();
				let target = std::fs::read_link(&symlink).unwrap();

				assert_eq!(target.file_name(), Some(target.as_os_str()));
				let target = target.into_os_string().into_string().unwrap();

				if let Some(target) = cache.get(&target) {
					cache.insert(shape, target.clone());
				}
			} else {
				let path = entry.path();
				cache.insert(shape, Arc::new(CursorIcon::X { path }));
			}
		}
	}

	pub fn icon(&self, icon: &str) -> Option<&CursorIcon> {
		self.cache.get(icon).map(Arc::as_ref)
	}
}

/// does the theme inherit from another theme?
///
/// adapted from the [xcursor crate](https://github.com/esposm03/xcursor-rs)
fn theme_inherits(path: PathBuf) -> Option<String> {
	let content = std::fs::read_to_string(path).ok()?;

	fn is_xcursor_space_or_separator(&ch: &char) -> bool {
		ch.is_whitespace() || ch == ';' || ch == ','
	}

	const INHERITS: &str = "Inherits";
	for line in content.lines() {
		if !line.starts_with(INHERITS) {
			continue;
		}

		let chars = &line[INHERITS.len()..].trim_start();
		let mut chars = chars.chars();

		if chars.next() != Some('=') {
			continue;
		}

		let inherits = chars
			.skip_while(is_xcursor_space_or_separator)
			.take_while(|ch| !is_xcursor_space_or_separator(ch))
			.collect::<String>();

		if !inherits.is_empty() {
			return Some(inherits);
		}
	}

	None
}

#[derive(Debug)]
pub enum CursorIcon {
	Svg { path: PathBuf },
	X { path: PathBuf },
}

impl CursorIcon {
	pub fn frames(&self, size: u32) -> Option<Vec<Image>> {
		match self {
			CursorIcon::Svg { path } => {
				let metadata = path.join("metadata.json");
				let metadata = std::fs::read_to_string(metadata).ok()?;
				let metadata = serde_json::from_str::<Vec<Meta>>(&metadata).ok()?;

				if metadata.is_empty() {
					return None;
				}

				let images = metadata
					.into_iter()
					.map(|meta| Image::render_svg(path, size, meta));

				Some(images.collect())
			}
			CursorIcon::X { path } => {
				let content = std::fs::read(path).ok()?;
				let images = xcursor::parser::parse_xcursor(&content)?;
				if images.is_empty() {
					return None;
				}

				let nearest = images
					.iter()
					.min_by_key(|img| u32::abs_diff(img.size, size))
					.unwrap();
				let nearest_size = nearest.size;

				let frames = images
					.into_iter()
					.filter(|img| img.size == nearest_size)
					.map(Image::from_xcursor)
					.collect();
				Some(frames)
			}
		}
	}
}

pub struct Image {
	/// size of the image
	pub size: u32,

	/// x hotspot in scaled pixels
	pub xhot: u32,
	// y hotspot in scaled pixels
	pub yhot: u32,

	/// delay in ms
	pub delay: Option<u32>,

	/// pixels in rgba format
	pub pixels: Vec<u8>,
}

impl Debug for Image {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Frame")
			.field("size", &self.size)
			.field("xhot", &self.xhot)
			.field("yhot", &self.yhot)
			.field("delay", &self.delay)
			.field("pixels", &[&..])
			.finish()
	}
}

impl Image {
	fn from_xcursor(xcursor: xcursor::parser::Image) -> Self {
		Image {
			size: xcursor.size,

			xhot: xcursor.xhot,
			yhot: xcursor.yhot,

			delay: Some(xcursor.delay),

			pixels: xcursor.pixels_rgba,
		}
	}

	fn render_svg(path: &Path, size: u32, meta: Meta) -> Self {
		let usvg_opts = resvg::usvg::Options::default();

		let data = path.join(meta.filename);
		let data = std::fs::read(data).unwrap();

		let tree = Tree::from_data(&data, &usvg_opts).unwrap();
		let transform = Transform::from_scale(
			size as f32 / tree.size().height(),
			size as f32 / tree.size().width(),
		);

		let scale = size as f32 / meta.nominal_size;
		let (xhot, yhot) = (meta.hotspot_x * scale, meta.hotspot_y * scale);

		let mut pixmap = Pixmap::new(size, size).unwrap();
		resvg::render(&tree, transform, &mut pixmap.as_mut());

		let pixels = pixmap.take();

		Image {
			size,

			xhot: xhot as u32,
			yhot: yhot as u32,

			delay: meta.delay,

			pixels,
		}
	}
}

#[derive(Debug, Deserialize)]
struct Meta {
	filename: String,

	hotspot_x: f32,
	hotspot_y: f32,
	nominal_size: f32,

	#[serde(default)]
	delay: Option<u32>,
}
