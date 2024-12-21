use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use crate::cmd::{ActivityCmd, ActivityCmdArgs};
use crate::log;
use crate::server::utils;

use super::ipc_utils::encode;
use super::ipc_utils::Handshake;
use super::ipc_utils::PacketType;

fn get_socket_path() -> String {
  let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR").unwrap_or_default();
  let tmpdir = env::var("TMPDIR").unwrap_or_default();
  let tmp = env::var("TMP").unwrap_or_default();
  let temp = env::var("TEMP").unwrap_or_default();
  let tmp_dir = if !xdg_runtime_dir.is_empty() {
    xdg_runtime_dir
  } else if !tmpdir.is_empty() {
    tmpdir
  } else if !tmp.is_empty() {
    tmp
  } else {
    temp
  };

  // Append a / to the temp dir if it doesn't have one
  let tmp_dir = if tmp_dir.ends_with('/') {
    tmp_dir
  } else {
    format!("{}/", tmp_dir)
  };

  format!("{}discord-ipc", tmp_dir)
}

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
    // Delete and recreate socket
    let mut socket = clone.socket.lock().unwrap();

    let socket_addr = socket.local_addr().unwrap();
    let path = socket_addr
      .as_pathname()
      .unwrap_or(std::path::Path::new(""));

    std::fs::remove_file(path).unwrap_or_default();

    *socket = Self::create_socket(None);
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
    let socket = Self::create_socket(None);

    Self {
      socket: Arc::new(Mutex::new(socket)),
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
  pub fn _close(&mut self) {
    let socket_addr = self.socket.lock().unwrap().local_addr().unwrap();
    let path = socket_addr
      .as_pathname()
      .unwrap_or(std::path::Path::new(""));
    std::fs::remove_file(path).unwrap_or_default();
  }

  /**
   * ACTUALLY create a socket, and return the handle
   */
  fn create_socket(tries: Option<u8>) -> UnixListener {
    let socket_path = get_socket_path();
    let tries = tries.unwrap_or(0);

    log!("[IPC] Creating socket: {}-{}", socket_path, tries);

    let socket = UnixListener::bind(format!("{}-{}", socket_path, tries));

    match socket {
      Ok(socket) => socket,
      Err(err) => {
        if tries >= 10 {
          log!("[IPC] Could not create IPC socket after 10 tries: {}", err);
          panic!("Could not create IPC socket after 10 tries");
        }

        std::thread::sleep(Duration::from_millis(500));

        Self::create_socket(Some(tries + 1))
      }
    }
  }
}
