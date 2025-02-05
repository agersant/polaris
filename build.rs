#[cfg(windows)]
fn main() {
	let mut res = winres::WindowsResource::new();
	res.set_icon("./res/windows/application/icon_polaris_512.ico");
	res.compile().unwrap();
	embed_resource::compile(
		"res/windows/application/polaris-manifest.rc",
		embed_resource::NONE,
	);
}

#[cfg(unix)]
fn main() {}
