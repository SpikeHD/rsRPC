use std::ffi::c_void;
use std::ptr;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use winapi::um::handleapi;
use winapi::um::namedpipeapi as pipeapi;
use winapi::um::winbase;

use crate::cmd::{ActivityCmd, ActivityCmdArgs};
use crate::log;
use crate::server::utils;

use super::ipc_utils::Handshake;
use super::ipc_utils::PacketType;

pub struct PipeHandle(*mut c_void);
unsafe impl Send for PipeHandle {}

#[derive(Clone)]
pub struct IpcConnector {
  socket: Arc<Mutex<PipeHandle>>,
  did_handshake: bool,
  pub client_id: String,
  pub pid: u64,
  pub nonce: String,

  event_sender: mpsc::Sender<ActivityCmd>,
}

impl IpcConnector {
  /**
   * Create a socket and return a new IpcConnector
   */
  pub fn new(event_sender: mpsc::Sender<ActivityCmd>) -> Self {
    let pipe_handle = Self::create_socket(None);

    Self {
      socket: Arc::new(Mutex::new(PipeHandle(pipe_handle))),
      did_handshake: false,
      client_id: "".to_string(),
      pid: 0,
      nonce: "".to_string(),
      event_sender,
    }
  }

  /**
   * Close and delete the socket
   */
  pub fn close(&mut self) {
    let socket = self.socket.lock().unwrap();

    unsafe {
      winapi::um::namedpipeapi::DisconnectNamedPipe(socket.0);
      winapi::um::handleapi::CloseHandle(socket.0);
    }
  }

  /**
   * ACTUALLY create a socket, and return the handle
   */
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
        1,    // Maximum number of instances
        1024, // Out buffer size
        1024, // In buffer size
        0,    // Default timeout
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

    log!("[IPC] Created IPC socket: {}", pipe_path);

    pipe_handle
  }

  /**
   * Create a new thread that will recieve messages from the socket
   */
  pub fn start(&mut self) {
    let mut clone = self.clone();

    std::thread::spawn(move || {
      let mut socket = clone.socket.lock().unwrap();

      // Forever recieve messages from the socket
      loop {
        // Little baby delay to keep things smooth
        std::thread::sleep(std::time::Duration::from_millis(15));

        // buffer is unknown size
        let mut init_buffer: Vec<u8> = vec![0; 8];
        let mut bytes_read: u32 = 0;

        unsafe {
          winapi::um::fileapi::ReadFile(
            socket.0,
            init_buffer.as_mut_ptr() as *mut c_void,
            init_buffer.len() as u32,
            &mut bytes_read,
            ptr::null_mut(),
          );
        }

        // If the buffer is empty or full of nothing, just stop there
        if bytes_read == 0 || init_buffer.iter().all(|&x| x == 0) {
          continue;
        }

        let r_type = PacketType::from_u32(u32::from_le_bytes([
          init_buffer[0],
          init_buffer[1],
          init_buffer[2],
          init_buffer[3],
        ]));
        let data_size = u32::from_le_bytes([
          init_buffer[4],
          init_buffer[5],
          init_buffer[6],
          init_buffer[7],
        ]);

        // Read the rest into a new buffer and convert to string
        let mut buffer: Vec<u8> = vec![0; data_size as usize];
        let mut bytes_read: u32 = 0;

        unsafe {
          winapi::um::fileapi::ReadFile(
            socket.0,
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as u32,
            &mut bytes_read,
            ptr::null_mut(),
          );
        }

        let message = String::from_utf8(buffer).unwrap();

        // Log the message
        log!("[IPC] Recieved message: {}", message);

        match r_type {
          PacketType::Handshake => {
            let Ok(data) = serde_json::from_str::<Handshake>(&message) else {
              log!("[IPC] Error parsing handshake");
              continue;
            };

            if data.v != 1 {
              panic!("Invalid version: {}", data.v);
            }

            // Send CONNECTION_REPONSE
            let resp = encode(PacketType::Frame, utils::CONNECTION_REPONSE.to_string());

            unsafe {
              winapi::um::fileapi::WriteFile(
                socket.0,
                resp.as_ptr() as *mut c_void,
                resp.len() as u32,
                ptr::null_mut(),
                ptr::null_mut(),
              );
            }

            clone.did_handshake = true;
            clone.client_id = data.client_id;
          }
          PacketType::Frame => {
            log!("[IPC] Recieved frame");
            if !clone.did_handshake {
              log!("[IPC] Did not handshake yet, ignoring frame");
              continue;
            }

            let mut activity_cmd = match serde_json::from_str::<ActivityCmd>(&message) {
              Ok(a) => a,
              Err(err) => {
                log!("[IPC] Error parsing activity command: {}", err);
                continue;
              }
            };

            activity_cmd.application_id = Some(clone.client_id.clone());

            clone.pid = activity_cmd.args.pid;
            clone.nonce.clone_from(&activity_cmd.nonce);

            match clone.event_sender.send(activity_cmd) {
              Ok(_) => (),
              Err(err) => log!("[IPC] Error sending activity command: {}", err),
            };
          }
          PacketType::Close => {
            log!("[IPC] Recieved close");

            // Send message with an empty activity
            let activity_cmd = ActivityCmd {
              application_id: Some(clone.client_id.clone()),
              cmd: "SET_ACTIVITY".to_string(),
              args: ActivityCmdArgs {
                pid: clone.pid,
                activity: None,
              },
              nonce: clone.nonce.clone(),
            };

            match clone.event_sender.send(activity_cmd) {
              Ok(_) => (),
              Err(err) => log!("[IPC] Error sending activity command: {}", err),
            };

            // reset values
            clone.did_handshake = false;
            clone.client_id = "".to_string();
            clone.pid = 0;

            // Delete and recreate socket
            unsafe {
              winapi::um::namedpipeapi::DisconnectNamedPipe(socket.0);
              winapi::um::handleapi::CloseHandle(socket.0);
            }

            let pipe_handle = Self::create_socket(None);

            *socket = PipeHandle(pipe_handle);
          }
          PacketType::Ping => {
            log!("[IPC] Recieved ping");

            // Send a pong
            let resp = encode(PacketType::Pong, message);

            unsafe {
              winapi::um::fileapi::WriteFile(
                socket.0,
                resp.as_ptr() as *mut c_void,
                resp.len() as u32,
                ptr::null_mut(),
                ptr::null_mut(),
              );
            }
          }
          PacketType::Pong => {
            log!("[IPC] Recieved pong");
          }
        }
      }
    });

    fn encode(r_type: PacketType, data: String) -> Vec<u8> {
      let mut buffer: Vec<u8> = Vec::new();

      // Write the packet type
      buffer.extend_from_slice(&u32::to_le_bytes(r_type as u32));

      // Write the data size
      buffer.extend_from_slice(&u32::to_le_bytes(data.len() as u32));

      // Write the data
      buffer.extend_from_slice(data.as_bytes());

      buffer
    }
  }
}
