extern crate winapi;
extern crate winapi_util;

use std::sync::{Arc, Mutex};
use winapi::um::namedpipeapi as pipeapi;
use winapi::um::winbase;
use winapi::um::handleapi;
use std::ptr;
use std::ffi::c_void;

use crate::logger;

pub struct PipeHandle(*mut c_void);
unsafe impl Send for PipeHandle {}

#[derive(Clone)]
pub struct IpcConnector {
  pub socket: Arc<Mutex<PipeHandle>>,
}

impl IpcConnector {
  /**
   * Create the socket
   */
  pub fn new() -> Self {
    let pipe_handle = Self::create_socket(None);

    Self {
      socket: Arc::new(Mutex::new(PipeHandle(pipe_handle))),
    }
  }

  fn create_socket(tries: Option<u8>) -> *mut c_void {
    // Define the path to the named pipe
    let pipe_path = r"\\.\pipe\discord-ipc";

    // Append tried number to name if applicable
    let pipe_path = match tries {
      Some(tries) => format!("{}-{}", pipe_path, tries),
      None => format!("{}-{}", pipe_path, 0),
    };

    let pipe_path_wide: Vec<u16> = pipe_path.encode_utf16().chain(std::iter::once(0)).collect();

    // Open the named pipe
    let pipe_handle = unsafe {
      pipeapi::CreateNamedPipeW(
        pipe_path_wide.as_ptr(),
        winbase::PIPE_ACCESS_DUPLEX,
        winbase::PIPE_TYPE_BYTE | winbase::PIPE_READMODE_BYTE | winbase::PIPE_WAIT,
        1, // Maximum number of instances
        1024, // Out buffer size
        1024, // In buffer size
        0, // Default timeout
        ptr::null_mut(),
      )
    };
    
    let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };

    // Retry if needed
    if pipe_handle == handleapi::INVALID_HANDLE_VALUE {
      // Retry if we haven't tried too many times
      if tries.unwrap_or(0) < 9 {
        return Self::create_socket(Some(tries.unwrap_or(0) + 1));
      } else {
        panic!("Failed to create socket: {}", error_code);
      }
    }

    eprintln!("Created IPC socket: {}", pipe_path);

    pipe_handle
  }

  /**
   * Create a new thread that will recieve messages from the socket
   */
  pub fn start(&mut self) {
    let clone = self.clone();

    std::thread::spawn(move || {
      let socket = clone.socket.lock().unwrap();

      // Forever recieve messages from the socket
      loop {
        let mut buffer: [u8; 1024] = [0; 1024];
        let mut bytes_read: u32 = 0;

        unsafe {
          winapi::um::fileapi::ReadFile(
            (*socket).0,
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as u32,
            &mut bytes_read,
            ptr::null_mut(),
          );
        }

        // If the buffer is empty or full of nothing, just stop there
        if bytes_read == 0 || buffer.iter().all(|&x| x == 0) {
          continue;
        }

        // Convert the buffer to a string
        let message = String::from_utf8_lossy(&buffer).to_string();

        // Log the message
        logger::log(&format!("Recieved message: {}", message));
      }
    });
  }
}