#[cfg(all(windows, feature = "ui"))]
mod windows;

#[cfg(all(windows, feature = "ui"))]
pub use self::windows::*;

#[cfg(not(all(windows, feature = "ui")))]
mod headless;

#[cfg(not(all(windows, feature = "ui")))]
pub use self::headless::*;
