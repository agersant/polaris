#[cfg(windows)]
fn main() {
	let mut res = winres::WindowsResource::new();
	res.set_icon("./res/windows/application/icon_polaris_512.ico");
	res.compile().unwrap();
}

#[cfg(unix)]
fn main() {}
