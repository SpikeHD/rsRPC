pub mod client_connector;
pub mod process;

#[cfg(target_os = "windows")]
pub mod ipc_win;

#[cfg(not(target_os = "windows"))]
pub mod ipc_unix;

#[cfg(target_os = "windows")]
mod platform {
    pub use super::ipc_win as ipc;
}

#[cfg(not(target_os = "windows"))]
mod platform {
    pub use super::ipc_unix as ipc;
}

pub use platform::ipc;
