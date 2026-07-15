#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod implementation;

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod implementation;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
compile_error!("KanColle Browser supports game windows only on Windows and macOS");

pub(super) use implementation::*;
