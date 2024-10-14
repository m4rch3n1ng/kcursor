// official ref
// https://invent.kde.org/plasma/kwin/-/blob/master/src/utils/svgcursorreader.cpp?ref_type=heads

use std::{
	borrow::Cow,
	collections::HashMap,
	path::PathBuf,
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
