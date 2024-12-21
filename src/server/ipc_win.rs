use interprocess::local_socket::traits::ListenerExt;
use interprocess::local_socket::{Listener, ListenerOptions, ToFsName};
use interprocess::os::windows::local_socket::NamedPipe;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::cmd::ActivityCmd;
use crate::log;

use super::ipc_utils::{handle_stream, IpcFacilitator};

#[derive(Clone)]
pub struct IpcConnector {
  socket: Arc<Mutex<Listener>>,
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
    let socket = Self::create_socket(None);
    *self.socket.lock().unwrap() = socket;
  }

  /**
   * Create a new thread that will recieve messages from the socket
   */
  fn start(&mut self) {
    let connector = self.clone();

    std::thread::spawn(move || {
      let socket = connector.socket.lock().unwrap();

      for stream in socket.incoming() {
        // Little baby delay to keep things smooth
        std::thread::sleep(std::time::Duration::from_millis(5));

        let mut clone = connector.clone();

        match stream {
          Ok(mut stream) => {
            std::thread::spawn(move || handle_stream(&mut clone, &mut stream));
          }
          Err(err) => {
            log!("[IPC] Error: {}", err);
            break;
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
    Self {
      socket: Arc::new(Mutex::new(Self::create_socket(None))),
      did_handshake: false,
      client_id: "".to_string(),
      pid: 0,
      nonce: "".to_string(),
      event_sender,
    }
  }

  fn create_socket(tries: Option<u8>) -> Listener {
    // Define the path to the named pipe
    let pipe_path = r"\\.\pipe\discord-ipc";

    // Append tried number to name if applicable
    let pipe_path = match tries {
      Some(tries) => format!("{}-{}", pipe_path, tries),
      None => format!("{}-{}", pipe_path, 0),
    };

    let listener =
      ListenerOptions::new().name(pipe_path.clone().to_fs_name::<NamedPipe>().unwrap());

    let socket = match listener.create_sync() {
      Ok(socket) => socket,
      Err(err) => {
        log!("[IPC] Failed to create IPC socket: {}", err);

        if tries.unwrap_or(0) < 9 {
          return Self::create_socket(Some(tries.unwrap_or(0) + 1));
        } else {
          panic!("[IPC] Failed to create socket: {}", err);
        }
      }
    };

    log!("[IPC] Created IPC socket: {}", pipe_path);

    socket
  }
}
