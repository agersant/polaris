use log::info;
use native_windows_derive::NwgUi;
use native_windows_gui::{self as nwg, NativeUi};

const TRAY_ICON: &[u8] =
	include_bytes!("../../res/windows/application/icon_polaris_outline_16.png");

#[derive(Default, NwgUi)]
pub struct SystemTray {
	#[nwg_control]
	window: nwg::MessageWindow,

	#[nwg_resource(source_bin: Some(TRAY_ICON))]
	icon: nwg::Icon,

	#[nwg_control(icon: Some(&data.icon), tip: Some("Polaris"))]
	#[nwg_events(MousePressLeftUp: [SystemTray::show_menu], OnContextMenu: [SystemTray::show_menu])]
	tray: nwg::TrayNotification,

	#[nwg_control(parent: window, popup: true)]
	tray_menu: nwg::Menu,

	#[nwg_control(parent: tray_menu, text: "Quit Polaris")]
	#[nwg_events(OnMenuItemSelected: [SystemTray::exit])]
	exit_menu_item: nwg::MenuItem,
}

impl SystemTray {
	fn show_menu(&self) {
		let (x, y) = nwg::GlobalCursor::position();
		self.tray_menu.popup(x, y);
	}

	fn exit(&self) {
		nwg::stop_thread_dispatch();
	}
}

pub fn run() {
	info!("Starting up UI (Windows system tray)");
	nwg::init().expect("Failed to init Native Windows GUI");
	let _ui = SystemTray::build_ui(Default::default()).expect("Failed to build tray UI");
	nwg::dispatch_thread_events();
}
