use std::sync::{mpsc, Arc, Mutex};
use std::env;
use std::path::PathBuf;
use crate::cmd::ActivityCmd;

use super::ipc_utils::Handshake;
use super::ipc_utils::PacketType;

fn get_socket_path() -> PathBuf {
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

  PathBuf::from(format!("{}/discord-ipc", tmp_dir))
}

#[derive(Clone)]
pub struct IpcConnector {
  socket: Arc<Mutex<u32>>,
  pub did_handshake: bool,
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
    Self::create_socket(None);

    Self {
      socket: Arc::new(Mutex::new(0)),
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
  pub fn close(&mut self) {}

  /**
   * ACTUALLY create a socket, and return the handle
   */
  fn create_socket(tries: Option<u8>) {}

  pub fn start(&mut self) {}

  fn encode(r_type: PacketType, data: String) {}
}
