#[macro_export]
macro_rules! test_name {
	() => {{
		let file_name = file!();
		let file_name = file_name.replace("/", "-");
		let file_name = file_name.replace("\\", "-");
		format!("{}-line-{}", file_name, line!())
	}};
}
