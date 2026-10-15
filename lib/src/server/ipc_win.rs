use interprocess::local_socket::traits::ListenerExt;
use interprocess::local_socket::{Listener, ListenerOptions, ToFsName};
use interprocess::os::windows::local_socket::{ListenerOptionsExt, NamedPipe};
use interprocess::os::windows::security_descriptor::SecurityDescriptor;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::cmd::ActivityCmd;
use crate::log;

use super::ipc_utils::{handle_stream, IpcFacilitator};

#[derive(Clone)]
pub struct IpcConnector {
  socket: Option<Arc<Mutex<Listener>>>,
  did_handshake: bool,
  pub client_id: String,
  pub pid: u64,
  pub nonce: String,

  event_sender: mpsc::Sender<ActivityCmd>,
}

impl IpcFacilitator for IpcConnector {
  fn handshake(&self) -> bool {
    self.did_handshake
  }

  fn set_handshake(&mut self, handshake: bool) {
    self.did_handshake = handshake;
  }

  fn client_id(&self) -> String {
    self.client_id.clone()
  }

  fn set_client_id(&mut self, client_id: String) {
    self.client_id = client_id;
  }

  fn pid(&self) -> u64 {
    self.pid
  }

  fn set_pid(&mut self, pid: u64) {
    self.pid = pid;
  }

  fn nonce(&self) -> String {
    self.nonce.clone()
  }

  fn set_nonce(&mut self, nonce: String) {
    self.nonce = nonce;
  }

  fn recreate_socket(&mut self) {
    match Self::create_socket(None) {
      Err(err) => log!("[IPC] Could not recreate socket: {}", err),
      Ok(socket) => match self.socket.as_ref() {
        None => self.socket = Some(Arc::new(Mutex::new(socket))),
        Some(arc) => *arc.lock().unwrap() = socket,
      },
    }
  }

  /**
   * Create a new thread that will recieve messages from the socket
   */
  fn start(&mut self) {
    let connector = self.clone();

    std::thread::spawn(move || {
      match connector.socket.as_ref() {
        None => log!("[IPC] IPC connector is disabled"),
        Some(arc) => {
          let socket = arc.lock().unwrap();

          for stream in socket.incoming() {
            // Little baby delay to keep things smooth
            std::thread::sleep(std::time::Duration::from_millis(5));

            let mut clone = connector.clone();

            match stream {
              Ok(mut stream) => {
                log!("[IPC] Incoming stream...");
                std::thread::spawn(move || handle_stream(&mut clone, &mut stream));
              }
              Err(err) => {
                log!("[IPC] Error: {}", err);
                break;
              }
            }
          }
        }
      }
    });
  }

  fn event_sender(&mut self) -> &mut mpsc::Sender<ActivityCmd> {
    &mut self.event_sender
  }
}

impl IpcConnector {
  /**
   * Create a socket and return a new IpcConnector
   */
  pub fn new(event_sender: mpsc::Sender<ActivityCmd>) -> Self {
    let socket = match Self::create_socket(None) {
      Err(err) => {
        log!(
          "[IPC] IPC connector disabled, could not create pipe: {}",
          err
        );
        None
      }
      Ok(socket) => Some(Arc::new(Mutex::new(socket))),
    };

    Self {
      socket,
      did_handshake: false,
      client_id: "".to_string(),
      pid: 0,
      nonce: "".to_string(),
      event_sender,
    }
  }

  fn create_socket(tries: Option<u8>) -> Result<Listener, Box<dyn std::error::Error>> {
    // Define the path to the named pipe
    let pipe_path = r"\\.\pipe\discord-ipc";

    // Append tried number to name if applicable
    let pipe_path = match tries {
      Some(tries) => format!("{}-{}", pipe_path, tries),
      None => format!("{}-{}", pipe_path, 0),
    };

    let name = match pipe_path.as_str().to_fs_name::<NamedPipe>() {
      Err(err) => {
        log!("[IPC] Pipe name is unusable: {}", err);
        return Err("Invalid IPC pipe name".to_string().into());
      }
      Ok(name) => name,
    };

    let listener = ListenerOptions::new()
      .name(name)
      .security_descriptor(SecurityDescriptor::default());

    let socket = match listener.create_sync() {
      Ok(socket) => socket,
      Err(err) => {
        log!("[IPC] Failed to create IPC socket: {}", err);

        if tries.unwrap_or(0) < 9 {
          return Self::create_socket(Some(tries.unwrap_or(0) + 1));
        } else {
          return Err("Failed to create IPC socket".to_string().into());
        }
      }
    };

    log!("[IPC] Created IPC socket: {}", pipe_path);

    Ok(socket)
  }
}
