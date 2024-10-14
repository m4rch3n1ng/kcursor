use kcursor::CursorTheme;

fn main() {
	let _theme = CursorTheme::load("Breeze_Light").unwrap();
	dbg!(_theme);
}
