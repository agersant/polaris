use std;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use uuid;
use winapi;

const IDI_POLARIS_TRAY: isize = 0x102;
const UID_NOTIFICATION_ICON: u32 = 0;
const MESSAGE_NOTIFICATION_ICON: u32 = winapi::um::winuser::WM_USER + 1;
const MESSAGE_NOTIFICATION_ICON_QUIT: u32 = winapi::um::winuser::WM_USER + 2;

pub trait ToWin {
	type Out;
	fn to_win(&self) -> Self::Out;
}

impl<'a> ToWin for &'a str {
	type Out = Vec<u16>;

	fn to_win(&self) -> Self::Out {
		OsStr::new(self)
			.encode_wide()
			.chain(std::iter::once(0))
			.collect()
	}
}

impl ToWin for uuid::Uuid {
	type Out = winapi::shared::guiddef::GUID;

	fn to_win(&self) -> Self::Out {
		let bytes = self.as_bytes();
		let end = [bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
		           bytes[15]];

		winapi::shared::guiddef::GUID {
			Data1: ((bytes[0] as u32) << 24 | (bytes[1] as u32) << 16 | (bytes[2] as u32) << 8 |
			        (bytes[3] as u32)),
			Data2: ((bytes[4] as u16) << 8 | (bytes[5] as u16)),
			Data3: ((bytes[6] as u16) << 8 | (bytes[7] as u16)),
			Data4: end,
		}
	}
}

pub trait Constructible {
	type Out;
	fn new() -> Self::Out;
}

impl Constructible for winapi::um::shellapi::NOTIFYICONDATAW {
	type Out = winapi::um::shellapi::NOTIFYICONDATAW;

	fn new() -> Self::Out {

		let mut version_union: winapi::um::shellapi::NOTIFYICONDATAW_u = unsafe { std::mem::zeroed() };
		unsafe {
			let version = version_union.uVersion_mut();
			*version = winapi::um::shellapi::NOTIFYICON_VERSION_4;
		}

		winapi::um::shellapi::NOTIFYICONDATAW {
			cbSize: std::mem::size_of::<winapi::um::shellapi::NOTIFYICONDATAW>() as u32,
			hWnd: std::ptr::null_mut(),
			uFlags: 0,
			guidItem: uuid::Uuid::nil().to_win(),
			hIcon: std::ptr::null_mut(),
			uID: 0,
			uCallbackMessage: 0,
			szTip: [0; 128],
			dwState: 0,
			dwStateMask: 0,
			szInfo: [0; 256],
			u: version_union,
			szInfoTitle: [0; 64],
			dwInfoFlags: 0,
			hBalloonIcon: std::ptr::null_mut(),
		}
	}
}

fn create_window() -> Option<winapi::shared::windef::HWND> {

	let class_name = "Polaris-class".to_win();
	let window_name = "Polaris-window".to_win();

	unsafe {
		let module_handle = winapi::um::libloaderapi::GetModuleHandleW(std::ptr::null());
		let wnd = winapi::um::winuser::WNDCLASSW {
			style: 0,
			lpfnWndProc: Some(window_proc),
			hInstance: module_handle,
			hIcon: std::ptr::null_mut(),
			hCursor: std::ptr::null_mut(),
			lpszClassName: class_name.as_ptr(),
			hbrBackground: winapi::um::winuser::COLOR_WINDOW as winapi::shared::windef::HBRUSH,
			lpszMenuName: std::ptr::null_mut(),
			cbClsExtra: 0,
			cbWndExtra: 0,
		};

		let atom = winapi::um::winuser::RegisterClassW(&wnd);
		if atom == 0 {
			return None;
		}

		let window_handle = winapi::um::winuser::CreateWindowExW(0,
		                                            atom as winapi::shared::ntdef::LPCWSTR,
		                                            window_name.as_ptr(),
		                                            winapi::um::winuser::WS_DISABLED,
		                                            0,
		                                            0,
		                                            0,
		                                            0,
		                                            winapi::um::winuser::GetDesktopWindow(),
		                                            std::ptr::null_mut(),
		                                            std::ptr::null_mut(),
		                                            std::ptr::null_mut());

		if window_handle.is_null() {
			return None;
		}

		return Some(window_handle);
	}
}

fn add_notification_icon(window: winapi::shared::windef::HWND) {

	let mut tooltip = [0 as winapi::um::winnt::WCHAR; 128];
	for (&x, p) in "Polaris".to_win().iter().zip(tooltip.iter_mut()) {
		*p = x;
	}

	unsafe {
		let module = winapi::um::libloaderapi::GetModuleHandleW(std::ptr::null());
		let icon = winapi::um::winuser::LoadIconW(module, std::mem::transmute(IDI_POLARIS_TRAY));
		let mut flags = winapi::um::shellapi::NIF_MESSAGE | winapi::um::shellapi::NIF_TIP;
		if !icon.is_null() {
			flags |= winapi::um::shellapi::NIF_ICON;
		}

		let mut icon_data = winapi::um::shellapi::NOTIFYICONDATAW::new();
		icon_data.hWnd = window;
		icon_data.uID = UID_NOTIFICATION_ICON;
		icon_data.uFlags = flags;
		icon_data.hIcon = icon;
		icon_data.uCallbackMessage = MESSAGE_NOTIFICATION_ICON;
		icon_data.szTip = tooltip;

		winapi::um::shellapi::Shell_NotifyIconW(winapi::um::shellapi::NIM_ADD, &mut icon_data);
	}
}

fn remove_notification_icon(window: winapi::shared::windef::HWND) {
	let mut icon_data = winapi::um::shellapi::NOTIFYICONDATAW::new();
	icon_data.hWnd = window;
	icon_data.uID = UID_NOTIFICATION_ICON;
	unsafe {
		winapi::um::shellapi::Shell_NotifyIconW(winapi::um::shellapi::NIM_DELETE, &mut icon_data);
	}
}

fn open_notification_context_menu(window: winapi::shared::windef::HWND) {
	info!("Opening notification icon context menu");
	let quit_string = "Quit Polaris".to_win();

	unsafe {
		let context_menu = winapi::um::winuser::CreatePopupMenu();
		if context_menu.is_null() {
			return;
		}
		winapi::um::winuser::InsertMenuW(context_menu,
		                    0,
		                    winapi::um::winuser::MF_STRING,
		                    MESSAGE_NOTIFICATION_ICON_QUIT as usize,
		                    quit_string.as_ptr());

		let mut cursor_position = winapi::shared::windef::POINT { x: 0, y: 0 };
		winapi::um::winuser::GetCursorPos(&mut cursor_position);

		winapi::um::winuser::SetForegroundWindow(window);
		let flags = winapi::um::winuser::TPM_RIGHTALIGN | winapi::um::winuser::TPM_BOTTOMALIGN |
		            winapi::um::winuser::TPM_RIGHTBUTTON;
		winapi::um::winuser::TrackPopupMenu(context_menu,
		                       flags,
		                       cursor_position.x,
		                       cursor_position.y,
		                       0,
		                       window,
		                       std::ptr::null_mut());
		winapi::um::winuser::PostMessageW(window, 0, 0, 0);

		info!("Closing notification context menu");
		winapi::um::winuser::DestroyMenu(context_menu);
	}
}

fn quit(window: winapi::shared::windef::HWND) {
	info!("Shutting down UI");
	unsafe {
		winapi::um::winuser::PostMessageW(window, winapi::um::winuser::WM_CLOSE, 0, 0);
	}
}

pub fn run() {
	info!("Starting up UI (Windows)");

	create_window().expect("Could not initialize window");

	let mut message = winapi::um::winuser::MSG {
		hwnd: std::ptr::null_mut(),
		message: 0,
		wParam: 0,
		lParam: 0,
		time: 0,
		pt: winapi::shared::windef::POINT { x: 0, y: 0 },
	};

	loop {
		let status: i32;
		unsafe {
			status = winapi::um::winuser::GetMessageW(&mut message, std::ptr::null_mut(), 0, 0);
			if status == -1 {
				panic!("GetMessageW error: {}", winapi::um::errhandlingapi::GetLastError());
			}
			if status == 0 {
				break;
			}
			winapi::um::winuser::TranslateMessage(&message);
			winapi::um::winuser::DispatchMessageW(&message);
		}
	}
}

pub unsafe extern "system" fn window_proc(window: winapi::shared::windef::HWND,
                                          msg: winapi::shared::minwindef::UINT,
                                          w_param: winapi::shared::minwindef::WPARAM,
                                          l_param: winapi::shared::minwindef::LPARAM)
                                          -> winapi::shared::minwindef::LRESULT {
	match msg {

		winapi::um::winuser::WM_CREATE => {
			add_notification_icon(window);
		}

		MESSAGE_NOTIFICATION_ICON => {
			match winapi::shared::minwindef::LOWORD(l_param as winapi::shared::minwindef::DWORD) as u32 {
				winapi::um::winuser::WM_RBUTTONUP => {
					open_notification_context_menu(window);
				}
				_ => (),
			}
		}

		winapi::um::winuser::WM_COMMAND => {
			match winapi::shared::minwindef::LOWORD(w_param as winapi::shared::minwindef::DWORD) as u32 {
				MESSAGE_NOTIFICATION_ICON_QUIT => {
					quit(window);
				}
				_ => (),
			}
		}

		winapi::um::winuser::WM_DESTROY => {
			remove_notification_icon(window);
			winapi::um::winuser::PostQuitMessage(0);
		}

		_ => (),
	};

	return winapi::um::winuser::DefWindowProcW(window, msg, w_param, l_param);
}
